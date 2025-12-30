use super::{
    component::ComponentId,
    dropdown::DropdownRender,
    editor_keymap::{shifted_char, KEYMAP_SCORE},
    editor_keymap_legend::NormalModeOverride,
    render_editor::Source,
    suggestive_editor::{Decoration, Info},
};
use crate::{
    app::{Dimension, Dispatch, ToHostApp},
    buffer::Buffer,
    char_index_range::range_intersects,
    components::component::Component,
    context::LocalSearchConfig,
    edit::{Action, ActionGroup, Edit, EditTransaction},
    git::{hunk::SimpleHunkKind, DiffMode, GitOperation as _, GitRepo},
    list::grep::RegexConfig,
    lsp::completion::PositionalEdit,
    position::Position,
    quickfix_list::QuickfixListItem,
    rectangle::Rectangle,
    search::parse_search_config,
    selection::{CharIndex, Selection, SelectionMode, SelectionSet},
    selection_mode::{ast_grep, NamingConventionAgnostic},
};
use crate::{
    app::{Dispatches, RequestParams, Scope},
    buffer::Line,
    char_index_range::CharIndexRange,
    clipboard::CopiedTexts,
    context::{Context, GlobalMode, LocalSearchConfigMode, Search},
    lsp::{completion::CompletionItemEdit, process::ResponseContext},
    selection_mode::{self, regex::get_regex},
    surround::EnclosureKind,
    transformation::{MyRegex, Transformation},
};
use crate::{grid::LINE_NUMBER_VERTICAL_BORDER, selection_mode::PositionBasedSelectionMode};
use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use event::KeyEvent;
use itertools::{Either, Itertools};
use my_proc_macros::key;
use nonempty::NonEmpty;
use ropey::Rope;
use shared::canonicalized_path::CanonicalizedPath;
use std::{
    cell::{Ref, RefCell, RefMut},
    ops::{Not, Range},
    path::PathBuf,
    rc::Rc,
};
use DispatchEditor::*;

#[derive(PartialEq, Clone, Debug, Eq)]
pub(crate) enum Mode {
    Normal,
    Insert,
    MultiCursor,
    FindOneChar(IfCurrentNotFound),
    Swap,
    Replace,
    Delete,
}

#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub(crate) enum PriorChange {
    EnterMultiCursorMode,
    EnableSelectionExtension,
}

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct Jump {
    pub(crate) character: char,
    pub(crate) selection: Selection,
}

impl Component for Editor {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn editor(&self) -> &Editor {
        self
    }

    fn editor_mut(&mut self) -> &mut Editor {
        self
    }

    fn set_content(&mut self, str: &str, context: &Context) -> Result<(), anyhow::Error> {
        self.update_buffer(str);
        self.clamp(context)
    }

    fn title(&self, context: &Context) -> String {
        let title = self.title.clone();
        title
            .or_else(|| self.title_impl(context))
            .unwrap_or_else(|| "[No title]".to_string())
    }

    fn set_title(&mut self, title: String) {
        self.title = Some(title);
    }

    fn handle_paste_event(
        &mut self,
        content: String,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        self.insert(&content, context)
    }

    fn get_cursor_position(&self) -> anyhow::Result<Position> {
        self.buffer
            .borrow()
            .char_to_position(self.get_cursor_char_index())
    }

    fn set_rectangle(&mut self, rectangle: Rectangle, context: &Context) {
        // Only update and recalculate scroll offset
        // if the new rectangle is different from the current rectangle

        if self.rectangle != rectangle {
            self.rectangle = rectangle;
            self.recalculate_scroll_offset(context);
        }
    }

    fn rectangle(&self) -> &Rectangle {
        &self.rectangle
    }

    fn handle_key_event(
        &mut self,
        context: &Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        self.handle_key_event(context, event)
    }

    fn handle_mouse_event(
        &mut self,
        mouse_event: crossterm::event::MouseEvent,
    ) -> anyhow::Result<Dispatches> {
        const SCROLL_HEIGHT: usize = 2;
        match mouse_event.kind {
            MouseEventKind::ScrollUp => {
                self.apply_scroll(Direction::Start, SCROLL_HEIGHT);
                Ok(Default::default())
            }
            MouseEventKind::ScrollDown => {
                self.apply_scroll(Direction::End, SCROLL_HEIGHT);
                Ok(Default::default())
            }
            MouseEventKind::Down(MouseButton::Left) => Ok(Default::default()),
            _ => Ok(Default::default()),
        }
    }

    #[cfg(test)]
    fn handle_events(&mut self, events: &[event::KeyEvent]) -> anyhow::Result<Dispatches> {
        let context = Context::default();
        Ok(events
            .iter()
            .map(|event| -> anyhow::Result<_> {
                Ok(self.handle_key_event(&context, event.clone())?.into_vec())
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .into())
    }

    fn handle_event(
        &mut self,
        context: &Context,
        event: event::event::Event,
    ) -> anyhow::Result<Dispatches> {
        match event {
            event::event::Event::Key(event) => self.handle_key_event(context, event),
            event::event::Event::Paste(content) => self.paste_text(
                Direction::End,
                CopiedTexts::new(NonEmpty::singleton(content)),
                context,
                false,
            ),
            event::event::Event::Mouse(event) => self.handle_mouse_event(event),
            _ => Ok(Default::default()),
        }
    }

    fn handle_dispatch_editor(
        &mut self,
        context: &mut Context,
        dispatch: DispatchEditor,
    ) -> anyhow::Result<Dispatches> {
        let last_visible_line = self.last_visible_line(context);
        match dispatch {
            #[cfg(test)]
            AlignViewTop => self.align_selection_to_top(),
            #[cfg(test)]
            AlignViewBottom => self.align_selection_to_bottom(context),
            #[cfg(test)]
            AlignViewCenter => self.align_selection_to_center(context),
            Transform(transformation) => return self.transform_selection(transformation, context),
            SetSelectionMode(if_current_not_found, selection_mode) => {
                return self.set_selection_mode(
                    if_current_not_found,
                    selection_mode,
                    context,
                    None,
                );
            }
            SetSelectionModeWithPriorChange(if_current_not_found, selection_mode, prior_change) => {
                return self.set_selection_mode(
                    if_current_not_found,
                    selection_mode,
                    context,
                    prior_change,
                );
            }
            FindOneChar(if_current_not_found) => {
                self.enter_single_character_mode(if_current_not_found)
            }
            MoveSelection(movement) => return self.handle_movement(context, movement),
            MoveSelectionWithPriorChange(movement, prior_change) => {
                return self.handle_movement_with_prior_change(context, movement, prior_change)
            }
            Copy => return self.copy(),
            ReplaceWithCopiedText { cut } => return self.replace_with_copied_text(context, cut, 0),
            SelectAll => return self.select_all(context),
            SetContent(content) => self.set_content(&content, context)?,
            EnableSelectionExtension => self.enable_selection_extension(),
            DisableSelectionExtension => self.disable_selection_extension(),
            EnterInsertMode(direction) => return self.enter_insert_mode(direction, context),
            Insert(string) => return self.insert(&string, context),
            #[cfg(test)]
            MatchLiteral(literal) => return self.match_literal(&literal, context),
            EnterNormalMode => self.enter_normal_mode(context)?,
            CursorAddToAllSelections => self.add_cursor_to_all_selections(context)?,
            CursorKeepPrimaryOnly => self.cursor_keep_primary_only(),
            EnterSwapMode => self.enter_swap_mode(),
            ReplacePattern { config } => {
                let selection_set = self.selection_set.clone();
                let (_, selection_set, dispatches, _) =
                    self.buffer_mut()
                        .replace(config, selection_set, last_visible_line)?;
                return Ok(self
                    .update_selection_set(selection_set, false, context)
                    .chain(self.get_document_did_change_dispatch())
                    .chain(dispatches));
            }
            Undo => {
                let dispatches = self.undo(context);
                return dispatches;
            }
            KillLine(direction) => return self.kill_line(direction, context),
            #[cfg(test)]
            Reset => self.reset(),
            DeleteWordBackward { short } => return self.delete_word_backward(short, context),
            Backspace => return self.backspace(context),
            MoveToLineStart => return self.move_to_line_start(context),
            MoveToLineEnd => return self.move_to_line_end(),
            SelectLine(movement) => return self.select_line(movement, context),
            Redo => return self.redo(context),
            DeleteOne => return self.delete_one(context),
            Change => return self.change(context),
            ChangeCut => return self.change_cut(context),
            #[cfg(test)]
            SetRectangle(rectangle) => self.set_rectangle(rectangle, context),
            ScrollPageDown => return self.scroll_page_down(context),
            ScrollPageUp => return self.scroll_page_up(context),
            ShowJumps {
                use_current_selection_mode,
                prior_change,
            } => return self.show_jumps(use_current_selection_mode, context, prior_change),
            SwitchViewAlignment => self.switch_view_alignment(context),
            #[cfg(test)]
            SetScrollOffset(n) => self.set_scroll_offset(n),
            #[cfg(test)]
            SetLanguage(language) => self.set_language(*language)?,
            #[cfg(test)]
            ApplySyntaxHighlight => {
                self.apply_syntax_highlighting(context)?;
            }
            Save => return self.do_save(false, context),
            ForceSave => {
                return self.do_save(true, context);
            }
            ReplaceCurrentSelectionWith(string) => {
                return self
                    .replace_current_selection_with(|_| Some(Rope::from_str(&string)), context)
            }
            SelectLineAt(index) => {
                return Ok(self.select_line_at(index, context)?.into_vec().into())
            }
            Surround(open, close) => return self.surround(open, close, context),
            EnterReplaceMode => self.enter_replace_mode(),
            Paste => return self.paste(context, true),
            PasteNoGap => return self.paste(context, false),
            SwapCursor => self.swap_cursor(context),
            SetDecorations(decorations) => self.buffer_mut().set_decorations(&decorations),
            MoveCharacterBack => self.selection_set.move_left(&self.cursor_direction),
            MoveCharacterForward => {
                let len_chars = self.buffer().len_chars();
                self.selection_set
                    .move_right(&self.cursor_direction, len_chars)
            }
            Open => return self.open(context),
            GoBack => self.go_back(context),
            GoForward => self.go_forward(context),
            SelectSurround { enclosure, kind } => {
                return self.select_surround(enclosure, kind, context)
            }
            DeleteSurround(enclosure) => return self.delete_surround(enclosure, context),
            ChangeSurround { from, to } => return self.change_surround(from, Some(to), context),
            ReplaceWithPattern => return self.replace_with_pattern(context),
            Replace(movement) => return self.replace_with_movement(&movement, context),
            ApplyPositionalEdits(edits) => {
                return self.apply_positional_edits(
                    edits
                        .into_iter()
                        .map(|edit| match edit {
                            CompletionItemEdit::PositionalEdit(positional_edit) => positional_edit,
                        })
                        .collect_vec(),
                    context,
                )
            }
            ReplaceWithPreviousCopiedText => {
                let history_offset = self.copied_text_history_offset.decrement();
                return self.replace_with_copied_text(context, false, history_offset);
            }
            ReplaceWithNextCopiedText => {
                let history_offset = self.copied_text_history_offset.increment();
                return self.replace_with_copied_text(context, false, history_offset);
            }
            MoveToLastChar => return Ok(self.move_to_last_char(context)),
            PipeToShell { command } => return self.pipe_to_shell(command, context),
            ShowCurrentTreeSitterNodeSexp => return self.show_current_tree_sitter_node_sexp(),
            Indent => return self.indent(context),
            Dedent => return self.dedent(context),
            CyclePrimarySelection(direction) => self.cycle_primary_selection(direction),
            SwapExtensionAnchor => self.selection_set.swap_anchor(),
            CollapseSelection(direction) => return self.collapse_selection(context, direction),
            FilterSelectionMatchingSearch { maintain, search } => {
                self.mode = Mode::Normal;
                let search_config = parse_search_config(&search)?;
                return Ok(self.filter_selection_matching_search(
                    search_config.local_config(),
                    maintain,
                    context,
                ));
            }
            EnterNewline => return self.enter_newline(context),
            DeleteCurrentCursor(direction) => self.delete_current_cursor(direction),
            BreakSelection => return self.break_selection(context),
            ShowHelp => return self.show_help(context),
            HandleEsc => {
                self.disable_selection_extension();
                self.mode = Mode::Normal;
                return Ok(Dispatches::one(Dispatch::RemainOnlyCurrentComponent));
            }
            ToggleReveal(reveal) => self.toggle_reveal(reveal),
            SearchCurrentSelection(if_current_not_found, scope) => {
                return Ok(self.search_current_selection(if_current_not_found, scope))
            }
            ExecuteCompletion { replacement, edit } => {
                return self.execute_completion(replacement, edit, context)
            }
            ToggleLineComment => return self.toggle_line_comment(context),
            ToggleBlockComment => return self.toggle_block_comment(context),
            ShowKeymapLegendExtend => {
                return Ok(Dispatches::one(Dispatch::ShowKeymapLegend(
                    self.extend_mode_keymap_legend_config(context),
                )))
            }
            RepeatSearch(scope, if_current_not_found, prior_change) => {
                return self.repeat_search(context, scope, if_current_not_found, prior_change)
            }
            RevertHunk(diff_mode) => return self.revert_hunk(context, diff_mode),
            GitBlame => return self.git_blame(context),
            ReloadFile { force } => return self.reload(force),
            MergeContent {
                content_filesystem,
                content_editor,
                path,
            } => return self.merge_content(context, path, content_editor, content_filesystem),
            ClearIncrementalSearchMatches => self.clear_incremental_search_matches(),
            GoToFile => return self.go_to_file(),
            SearchClipboardContent(scope) => {
                return Ok(self.search_clipboard_content(scope, context))
            }
            PressSpace => return Ok(self.press_space(context)),
            PathRenamed {
                source,
                destination,
            } => self.handle_path_renamed(source, destination),
            CopyAbsolutePath => return self.copy_current_file_absolute_path(),
            CopyRelativePath => return self.copy_current_file_relative_path(context),
            DeleteWithMovement(movement) => return self.delete_with_movement(context, movement),
            EnterDeleteMode => self.mode = Mode::Delete,
            AlignSelections(direction) => return self.align_selections(direction, context),
        }
        Ok(Default::default())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Reveal {
    CurrentSelectionMode,
    Cursor,
    Mark,
}

impl Clone for Editor {
    fn clone(&self) -> Self {
        Editor {
            mode: self.mode.clone(),
            selection_set: self.selection_set.clone(),
            jumps: None,
            cursor_direction: self.cursor_direction.clone(),
            scroll_offset: self.scroll_offset,
            rectangle: self.rectangle.clone(),
            buffer: self.buffer.clone(),
            title: self.title.clone(),
            id: self.id,
            current_view_alignment: None,
            regex_highlight_rules: Vec::new(),
            copied_text_history_offset: Default::default(),
            normal_mode_override: self.normal_mode_override.clone(),
            reveal: self.reveal.clone(),
            visible_line_ranges: Default::default(),
            incremental_search_matches: self.incremental_search_matches.clone(),
        }
    }
}

pub(crate) struct Editor {
    pub(crate) mode: Mode,
    pub(crate) regex_highlight_rules: Vec<RegexHighlightRule>,

    pub(crate) selection_set: SelectionSet,

    pub(crate) jumps: Option<Vec<Jump>>,
    pub(crate) cursor_direction: Direction,

    /// This means the number of lines to be skipped from the top during rendering.
    /// 2 means the first line to be rendered on the screen if the 3rd line of the text.
    scroll_offset: usize,
    rectangle: Rectangle,
    buffer: Rc<RefCell<Buffer>>,
    title: Option<String>,
    id: ComponentId,
    pub(crate) current_view_alignment: Option<ViewAlignment>,
    copied_text_history_offset: Counter,
    pub(crate) normal_mode_override: Option<NormalModeOverride>,
    pub(crate) reveal: Option<Reveal>,

    /// This is only used when Ki is running as an embedded component,
    /// for example, inside VS Code.
    visible_line_ranges: Option<Vec<Range<usize>>>,

    pub(crate) incremental_search_matches: Option<Vec<Range<usize>>>,
}

#[derive(Default)]
struct Counter {
    value: isize,
}

impl Counter {
    fn decrement(&mut self) -> isize {
        self.value -= 1;
        self.value
    }

    fn increment(&mut self) -> isize {
        self.value += 1;
        self.value
    }

    #[cfg(test)]
    fn value(&self) -> isize {
        self.value
    }

    fn reset(&mut self) {
        self.value = 0;
    }
}

pub(crate) struct RegexHighlightRule {
    pub(crate) regex: regex::Regex,
    pub(crate) capture_styles: Vec<RegexHighlightRuleCaptureStyle>,
}

pub(crate) struct RegexHighlightRuleCaptureStyle {
    /// 0 means the entire match.
    /// Refer https://docs.rs/regex/latest/regex/struct.Regex.html#method.captures
    pub(crate) capture_name: &'static str,
    pub(crate) source: Source,
}

impl RegexHighlightRuleCaptureStyle {
    pub(crate) fn new(capture_name: &'static str, source: Source) -> Self {
        Self {
            capture_name,
            source,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    /// Also means Backward or Previous
    Start,
    /// Also means Forward or Next
    End,
}

impl Direction {
    pub(crate) fn reverse(&self) -> Self {
        match self {
            Direction::Start => Direction::End,
            Direction::End => Direction::Start,
        }
    }

    #[cfg(test)]
    pub(crate) fn default() -> Direction {
        Direction::Start
    }

    pub(crate) fn format_action(&self, action: &str) -> String {
        match self {
            Direction::Start => format!("← {action}"),
            Direction::End => format!("{action} →"),
        }
    }

    pub(crate) fn to_if_current_not_found(&self) -> IfCurrentNotFound {
        match self {
            Direction::Start => IfCurrentNotFound::LookBackward,
            Direction::End => IfCurrentNotFound::LookForward,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub(crate) enum IfCurrentNotFound {
    LookForward,
    LookBackward,
}
impl IfCurrentNotFound {
    pub(crate) fn inverse(&self) -> IfCurrentNotFound {
        use IfCurrentNotFound::*;
        match self {
            LookForward => LookBackward,
            LookBackward => LookForward,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// This enum is to be used for binding keys
pub(crate) enum Movement {
    Right,
    Left,
    Last,
    Current(IfCurrentNotFound),
    Up,
    Down,
    First,
    /// 0-based
    Index(usize),
    Jump(CharIndexRange),
    Expand,
    Previous,
    Next,
}
impl Movement {
    pub(crate) fn into_movement_applicandum(
        self,
        sticky_column_index: &Option<usize>,
    ) -> MovementApplicandum {
        match self {
            Movement::Right => MovementApplicandum::Right,
            Movement::Left => MovementApplicandum::Left,
            Movement::Last => MovementApplicandum::Last,
            Movement::Current(if_current_not_found) => {
                MovementApplicandum::Current(if_current_not_found)
            }
            Movement::Up => MovementApplicandum::Up {
                sticky_column_index: *sticky_column_index,
            },
            Movement::Down => MovementApplicandum::Down {
                sticky_column_index: *sticky_column_index,
            },
            Movement::First => MovementApplicandum::First,
            Movement::Index(index) => MovementApplicandum::Index(index),
            Movement::Jump(chars) => MovementApplicandum::Jump(chars),
            Movement::Expand => MovementApplicandum::Expand,
            Movement::Previous => MovementApplicandum::Previous,
            Movement::Next => MovementApplicandum::Next,
        }
    }

    fn reverse(&self) -> Movement {
        match self {
            Movement::Left => Movement::Right,
            Movement::Right => Movement::Left,
            Movement::Up => Movement::Down,
            Movement::Down => Movement::Up,
            Movement::First => Movement::Last,
            Movement::Last => Movement::First,
            Movement::Previous => Movement::Next,
            Movement::Next => Movement::Previous,
            _ => *self,
        }
    }

    fn to_direction(self) -> Direction {
        use Movement::*;
        match self {
            Right | Next | Last => Direction::End,
            Left | Previous | First => Direction::Start,
            _ => Direction::End,
        }
    }

    fn downgrade(&self) -> Movement {
        match self {
            Movement::Right => Movement::Next,
            Movement::Left => Movement::Previous,
            _ => *self,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// This enum is to be used internally, not exposed to keybindings
/// Applicandum = To Be Applied
pub(crate) enum MovementApplicandum {
    Right,
    Left,
    Last,
    Current(IfCurrentNotFound),
    Up {
        sticky_column_index: Option<usize>,
    },
    Down {
        sticky_column_index: Option<usize>,
    },
    First,
    /// 0-based
    Index(usize),
    Jump(CharIndexRange),
    Expand,
    Next,
    Previous,
}

impl Editor {
    /// Returns (hidden_parent_lines, visible_parent_lines)
    pub(crate) fn get_parent_lines(&self) -> anyhow::Result<(Vec<Line>, Vec<Line>)> {
        let position = self.get_cursor_position()?;

        self.get_parent_lines_given_line_index_and_scroll_offset(position.line, self.scroll_offset)
    }

    // BOTTLENECK 5: This causes hiccup when navigating 10,000 lines CSV
    pub(crate) fn get_parent_lines_given_line_index_and_scroll_offset(
        &self,
        line_index: usize,
        scroll_offset: usize,
    ) -> anyhow::Result<(Vec<Line>, Vec<Line>)> {
        let parent_lines = self.buffer().get_parent_lines(line_index)?;
        Ok(parent_lines
            .into_iter()
            .partition(|line| line.line < scroll_offset))
    }

    pub(crate) fn show_info(&mut self, info: Info, context: &Context) -> Result<(), anyhow::Error> {
        self.set_title(info.title());
        self.set_decorations(info.decorations());
        self.set_content(info.content(), context)
    }

    pub(crate) fn render_dropdown(
        &mut self,
        context: &mut Context,
        render: &DropdownRender,
    ) -> anyhow::Result<Dispatches> {
        self.apply_dispatches(
            context,
            [
                SetContent(render.content.clone()),
                SetDecorations(render.decorations.clone()),
                SelectLineAt(render.highlight_line_index),
            ]
            .to_vec(),
        )
    }

    pub(crate) fn from_text(language: Option<tree_sitter::Language>, text: &str) -> Self {
        Self {
            selection_set: SelectionSet::default(),
            jumps: None,
            mode: Mode::Normal,
            cursor_direction: Direction::Start,
            scroll_offset: 0,
            rectangle: Rectangle::default(),
            buffer: Rc::new(RefCell::new(Buffer::new(language, text))),
            title: None,
            id: ComponentId::new(),
            current_view_alignment: None,
            regex_highlight_rules: Vec::new(),
            copied_text_history_offset: Default::default(),

            normal_mode_override: None,
            reveal: None,
            visible_line_ranges: Default::default(),
            incremental_search_matches: Default::default(),
        }
    }

    pub(crate) fn from_buffer(buffer: Rc<RefCell<Buffer>>) -> Self {
        // Select the first line of the file

        let first_line_range = selection_mode::LineTrimmed
            .get_current_selection_by_cursor(
                &buffer.borrow(),
                CharIndex(0),
                IfCurrentNotFound::LookForward,
            )
            .unwrap_or_default()
            .and_then(|byte_range| {
                buffer
                    .borrow()
                    .byte_range_to_char_index_range(byte_range.range())
                    .ok()
            })
            .unwrap_or_default();

        let selection = Selection {
            range: first_line_range,
            initial_range: None,
            info: None,
        };

        let selection_set = SelectionSet::default().set_selections(NonEmpty::new(selection));

        Self {
            selection_set,
            jumps: None,
            mode: Mode::Normal,
            cursor_direction: Direction::Start,
            scroll_offset: 0,
            rectangle: Rectangle::default(),
            buffer,
            title: None,
            id: ComponentId::new(),
            current_view_alignment: None,
            regex_highlight_rules: Vec::new(),
            copied_text_history_offset: Default::default(),
            normal_mode_override: None,
            reveal: None,
            visible_line_ranges: Default::default(),
            incremental_search_matches: Default::default(),
        }
    }

    /// The returned value includes leading whitespaces but elides trailing newline character
    pub(crate) fn current_line(&self) -> anyhow::Result<String> {
        let cursor = self.get_cursor_char_index();
        Ok(self
            .buffer
            .borrow()
            .get_line_by_char_index(cursor)?
            .to_string()
            .trim_end_matches("\n")
            .to_string())
    }

    pub(crate) fn get_current_word(&self) -> anyhow::Result<String> {
        let cursor = self.get_cursor_char_index();
        self.buffer.borrow().get_word_before_char_index(cursor)
    }

    pub(crate) fn select_line(
        &mut self,
        movement: Movement,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        self.select(SelectionMode::Line, movement, context)
    }

    pub(crate) fn select_line_at(
        &mut self,
        line: usize,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        let start = self.buffer.borrow().line_to_char(line)?;
        let selection_set = SelectionSet::new(NonEmpty::singleton(Selection::new(
            (start
                ..start
                    + self
                        .buffer
                        .borrow()
                        .get_line_by_char_index(start)?
                        .len_chars())
                .into(),
        )));

        Ok(self.update_selection_set(selection_set, false, context))
    }

    #[cfg(test)]
    pub(crate) fn reset(&mut self) {
        self.selection_set.escape_highlight_mode();
    }

    pub(crate) fn update_selection_set(
        &mut self,
        selection_set: SelectionSet,
        store_history: bool,
        context: &Context,
    ) -> Dispatches {
        let show_info = selection_set
            .map(|selection| selection.info())
            .into_iter()
            .flatten()
            .reduce(Info::join)
            .map(Dispatch::ShowGlobalInfo);
        self.cursor_direction = Direction::Start;
        if store_history {
            self.buffer_mut()
                .push_selection_set_history(selection_set.clone());
        }
        self.set_selection_set(selection_set, context);
        Dispatches::default().append_some(show_info)
    }

    pub(crate) fn char_index_range_to_selection_set(
        &self,
        range: CharIndexRange,
    ) -> anyhow::Result<SelectionSet> {
        let mode = if self.buffer().given_range_is_node(&range) {
            SelectionMode::SyntaxNode
        } else {
            SelectionMode::Custom
        };
        let primary = self
            .selection_set
            .primary_selection()
            .clone()
            .set_range(range);
        Ok(SelectionSet::new(NonEmpty::new(primary)).set_mode(mode))
    }

    /// Scroll offset recalculation is always based on the position of the cursor
    fn recalculate_scroll_offset(&mut self, context: &Context) {
        // Update scroll_offset if primary selection is out of view.
        let primary_selection_range = self.selection_set.primary_selection().extended_range();
        let line_range = self
            .buffer()
            .char_index_range_to_line_range(primary_selection_range)
            .unwrap_or_default();
        let render_area = self.render_area(context);
        let out_of_viewport = |row: usize| {
            row.saturating_sub(self.scroll_offset) > render_area.height.saturating_sub(1)
                || row < self.scroll_offset
        };
        if out_of_viewport(line_range.start) || out_of_viewport(line_range.end.saturating_sub(1)) {
            self.align_selection_to_center(context);
            self.current_view_alignment = None;
        }
    }

    /// Aligns the selection to show the target line within the available viewport.
    ///
    /// This function attempts to position the target line optimally within the visible area
    /// by iteratively testing different scroll offsets. Due to the complex interaction between
    /// hidden parent lines (contextual lines) and text wrapping, there's no deterministic way
    /// to calculate the optimal scroll offset directly.
    ///
    /// The algorithm works backwards from the maximum possible offset, testing each position
    /// until it finds one where the target line is visible in the rendered grid. This finds
    /// the best alignment position for both center and bottom alignment scenarios.
    fn align_selection<F: Fn(usize) -> usize>(
        &mut self,
        context: &Context,
        line_range_to_target: impl Fn(Range<usize>) -> usize,
        available_height_multiplier: F,
    ) {
        let primary_selection_range = self.selection_set.primary_selection().extended_range();
        let line_range = self
            .buffer()
            .char_index_range_to_line_range(primary_selection_range)
            .unwrap_or_default();

        let available_height = self
            .rectangle
            .height
            .saturating_sub(self.window_title_height(context));

        let target_line_index = if available_height <= line_range.len() {
            // Use cursor row if there are not enough spaces to fit all lines of the selection
            self.cursor_row()
        } else {
            line_range_to_target(line_range.clone())
        };
        for i in (0..available_height_multiplier(available_height)).rev() {
            let new_scroll_offset = target_line_index.saturating_sub(i);
            let grid = self.get_grid_with_scroll_offset(
                context,
                false,
                new_scroll_offset,
                Default::default(),
            );
            let grid_string = grid.grid.to_string();
            let grid_string_lines = grid_string.lines().collect_vec();
            let target_line_number =
                format!("{}{}", target_line_index + 1, LINE_NUMBER_VERTICAL_BORDER);
            let target_line_index_in_range = grid_string_lines
                .iter()
                .any(|line| line.contains(&target_line_number));
            if target_line_index_in_range {
                self.scroll_offset = new_scroll_offset;
                return;
            }
        }
    }

    pub(crate) fn align_selection_to_bottom(&mut self, context: &Context) {
        self.align_selection(
            context,
            // We need to subtract line_range.end by one because it is exclusive
            |line_range| line_range.end.saturating_sub(1),
            |height| height,
        )
    }

    /// If the primary selection has multiple lines
    /// then the middle line will be used for center alignment
    fn align_selection_to_center(&mut self, context: &Context) {
        self.align_selection(
            context,
            |line_range| line_range.start + (line_range.len() as f32 / 2.0).floor() as usize,
            |height| height / 2,
        );
    }

    /// If the primary selection has multiple lines
    /// then the first line will be used for top alignment
    pub(crate) fn align_selection_to_top(&mut self) {
        let selection_first_line = self
            .buffer()
            .char_to_line({
                let range = self.selection_set.primary_selection().extended_range();
                match self.cursor_direction {
                    Direction::Start => range.start,
                    Direction::End => range.end,
                }
            })
            .unwrap_or_default();
        self.scroll_offset = selection_first_line;
    }

    fn cursor_row(&self) -> usize {
        self.buffer()
            .char_to_line(self.get_cursor_char_index())
            .unwrap_or_default()
    }

    pub(crate) fn select(
        &mut self,
        selection_mode: SelectionMode,
        movement: Movement,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        //  There are a few selection modes where Current make sense.
        if let Some(selection_set) = self.get_selection_set(&selection_mode, movement, context)? {
            Ok(self.update_selection_set(selection_set, true, context))
        } else {
            Ok(Default::default())
        }
    }

    fn jump_characters(context: &Context) -> Vec<char> {
        let chars = context
            .keyboard_layout_kind()
            .get_keyboard_layout()
            .iter()
            .flatten()
            .zip(KEYMAP_SCORE.iter().flatten())
            .sorted_by_key(|(_, score)| **score)
            .map(|(char, _)| char.chars().next().unwrap());
        chars.clone().chain(chars.map(shifted_char)).collect()
    }

    pub(crate) fn get_selection_mode_trait_object(
        &self,
        selection: &Selection,
        use_current_selection_mode: bool,
        working_directory: &shared::canonicalized_path::CanonicalizedPath,
        quickfix_list_items: Vec<&QuickfixListItem>,
        marks: &[CharIndexRange],
    ) -> anyhow::Result<Box<dyn selection_mode::SelectionModeTrait>> {
        if use_current_selection_mode {
            self.selection_set.mode().clone()
        } else {
            SelectionMode::Subword
        }
        .to_selection_mode_trait_object(
            &self.buffer(),
            selection,
            &self.cursor_direction,
            working_directory,
            quickfix_list_items,
            marks,
        )
    }

    fn jump_from_selection(
        &mut self,
        selection: &Selection,
        use_current_selection_mode: bool,
        context: &Context,
    ) -> anyhow::Result<()> {
        let chars = Self::jump_characters(context);

        let object = self.get_selection_mode_trait_object(
            selection,
            use_current_selection_mode,
            context.current_working_directory(),
            context.quickfix_list_items(),
            &context.get_marks(self.path()),
        )?;

        let line_ranges = if let Some(ranges) = &self.visible_line_ranges {
            ranges.clone()
        } else {
            Some(self.visible_line_range())
                .into_iter()
                .chain(self.hidden_parent_line_ranges()?)
                .collect_vec()
        };
        let jumps = object.jumps(
            &selection_mode::SelectionModeParams {
                buffer: &self.buffer(),
                current_selection: selection,
                cursor_direction: &self.cursor_direction,
            },
            chars,
            line_ranges,
        )?;
        self.jumps = Some(jumps);

        Ok(())
    }

    pub(crate) fn show_jumps(
        &mut self,
        use_current_selection_mode: bool,
        context: &Context,
        prior_change: Option<PriorChange>,
    ) -> anyhow::Result<Dispatches> {
        self.handle_prior_change(prior_change);
        self.jump_from_selection(
            &self.selection_set.primary_selection().clone(),
            use_current_selection_mode,
            context,
        )?;
        Ok(Dispatches::one(self.dispatch_jumps_changed()))
    }

    fn delete_with_movement(
        &mut self,
        context: &Context,
        movement: Movement,
    ) -> anyhow::Result<Dispatches> {
        // to copy deleted item to clipboard copy_dispatch should be self.copy()?
        let copy_dispatches: Dispatches = Default::default();
        let edit_transaction = EditTransaction::from_action_groups({
            let buffer = self.buffer();
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let current_range = selection.extended_range();

                    // Let the current_range be at least one character long
                    // so even if the current_range is empty, the user can
                    // still delete the character which is apparently under the cursor.
                    let current_range = if current_range.len() == 0 {
                        (current_range.start
                            ..(current_range.start + 1).min(CharIndex(self.buffer().len_chars())))
                            .into()
                    } else {
                        current_range
                    };

                    let default = {
                        let start = current_range.start;
                        (current_range, (start..start + 1).into())
                    };

                    let get_selection = |movement: &Movement| {
                        // The start selection is used for getting the next/previous selection
                        // It cannot be the extended selection, otherwise the next/previous selection
                        // will not be found

                        let start_selection = &selection
                            .clone()
                            .collapsed_to_anchor_range(&movement.to_direction());

                        let result_selection = Selection::get_selection_(
                            &buffer,
                            start_selection,
                            self.selection_set.mode(),
                            &movement.into_movement_applicandum(
                                self.selection_set.sticky_column_index(),
                            ),
                            &self.cursor_direction,
                            context,
                        )
                        .ok()
                        .flatten()?;

                        if result_selection.selection.range() == start_selection.range() {
                            None
                        } else {
                            Some(result_selection)
                        }
                    };
                    let (delete_range, select_range) = (|| {
                        // Perform a "delete until the other selection" instead
                        // Other selection is a selection which is before/after the current selection
                        if let Some(other_selection) = get_selection(&movement)
                            .or_else(|| get_selection(&movement.reverse()))
                            // If no selection is found using `movement`, then try downgrading it.
                            // Downgrading is only applicable for the Left/Right movement,
                            // which transform into Previous/Next.
                            // This is necessary, because in some cases, there are no longer meaningful selections,
                            // and only meaningless selections are left,
                            // so we will have to "downgrade" the movement so that we can obtain the meaningless selections.
                            .or_else(|| get_selection(&movement.downgrade()))
                            .or_else(|| get_selection(&movement.downgrade().reverse()))
                        {
                            // The other_selection is only consider valid
                            // if it does not intersect with the range to be deleted
                            if !other_selection
                                .selection
                                .range()
                                .intersects_with(&current_range)
                            {
                                let other_range = other_selection.selection.range();
                                if other_range == current_range {
                                    return default;
                                } else if other_range.start >= current_range.end {
                                    let delete_range: CharIndexRange =
                                        (current_range.start..other_range.start).into();
                                    let select_range = {
                                        other_selection
                                            .selection
                                            .extended_range()
                                            .shift_left(delete_range.len())
                                    };
                                    return (delete_range, select_range);
                                } else {
                                    let delete_range: CharIndexRange =
                                        (other_range.end..current_range.end).into();
                                    let select_range = other_selection.selection.range();
                                    return (delete_range, select_range);
                                }
                            }
                        }

                        // If the other selection not found, then only deletes the selection
                        // without moving forward or backward
                        let range = selection.extended_range();
                        (range, (range.start..range.start).into())
                    })();
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit::new(
                                self.buffer().rope(),
                                delete_range,
                                Rope::new(),
                            )),
                            Action::Select(
                                selection
                                    .clone()
                                    .set_range(select_range)
                                    .set_initial_range(None),
                            ),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect()
        });
        let dispatches = self.apply_edit_transaction(edit_transaction, context)?;
        Ok(copy_dispatches.chain(dispatches))
    }

    fn enter_newline(&mut self, context: &Context) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups({
            let buffer = self.buffer();
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let cursor = selection.extended_range().start;
                    let current_line_index = buffer.char_to_line(cursor)?;

                    let current_line = buffer.get_line_by_line_index(current_line_index);

                    let indent = "\n".to_string()
                        + current_line
                            .map(|line| {
                                line.to_string()
                                    .chars()
                                    .take_while(|c| c.is_whitespace() && c != &'\n')
                                    .join("")
                            })
                            .unwrap_or_default()
                            .as_str();

                    let range_start = cursor + indent.chars().count();
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit::new(
                                self.buffer().rope(),
                                (cursor..cursor).into(),
                                indent.into(),
                            )),
                            Action::Select(
                                selection
                                    .clone()
                                    .set_range((range_start..range_start).into())
                                    .set_initial_range(None),
                            ),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect()
        });
        self.apply_edit_transaction(edit_transaction, context)
    }

    pub(crate) fn copy(&mut self) -> anyhow::Result<Dispatches> {
        Ok(Dispatches::one(Dispatch::SetClipboardContent {
            copied_texts: CopiedTexts::new(self.selection_set.map(|selection| {
                self.buffer()
                    .slice(&selection.extended_range())
                    .ok()
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            })),
        }))
    }

    fn replace_current_selection_with<F>(
        &mut self,
        f: F,
        context: &Context,
    ) -> anyhow::Result<Dispatches>
    where
        F: Fn(Rope) -> Option<Rope>,
    {
        let edit_transactions = self.selection_set.map(|selection| {
            let content = self
                .buffer()
                .slice(&selection.extended_range())
                .ok()
                .unwrap_or_default();
            if let Some(result) = f(content) {
                let range = selection.extended_range();
                let start = range.start;
                EditTransaction::from_action_groups(
                    [ActionGroup::new(
                        [
                            Action::Edit(Edit::new(self.buffer().rope(), range, result.clone())),
                            Action::Select(Selection::new({
                                let start = start + result.len_chars();
                                (start..start).into()
                            })),
                        ]
                        .to_vec(),
                    )]
                    .to_vec(),
                )
            } else {
                EditTransaction::from_action_groups(vec![])
            }
        });
        let edit_transaction = EditTransaction::merge(edit_transactions.into());
        self.apply_edit_transaction(edit_transaction, context)
    }

    fn try_replace_current_long_word(
        &mut self,
        replacement: String,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        let replacement: Rope = replacement.into();
        let buffer = self.buffer();
        let edit_transactions = self.selection_set.map(move |selection| {
            let rope = buffer.rope();
            let current_char_index = selection.range().start;
            let word_start = rope
                .chars()
                .enumerate()
                .take(current_char_index.0)
                .collect_vec()
                .iter()
                .rev()
                .take_while(|(_, c)| c.is_alphanumeric() || c == &'_' || c == &'-')
                .last()
                .map(|(char_index, _)| CharIndex(*char_index))
                .unwrap_or(current_char_index);
            let range: CharIndexRange = (word_start..selection.range().start).into();
            let start = range.start;
            EditTransaction::from_action_groups(
                [ActionGroup::new(
                    [
                        Action::Edit(Edit::new(rope, range, replacement.clone())),
                        Action::Select(Selection::new({
                            let start = start + replacement.len_chars();
                            (start..start).into()
                        })),
                    ]
                    .to_vec(),
                )]
                .to_vec(),
            )
        });
        let edit_transaction = EditTransaction::merge(edit_transactions.into());
        self.apply_edit_transaction(edit_transaction, context)
    }

    fn paste_text(
        &mut self,
        direction: Direction,
        copied_texts: CopiedTexts,
        context: &Context,
        with_gap: bool,
    ) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups({
            self.get_selection_set_with_gap(&direction, context)?
                .into_iter()
                .enumerate()
                .map(|(index, (selection, gap))| {
                    let gap = if with_gap { gap } else { Rope::new() };
                    let current_range = selection.extended_range();
                    let insertion_range_start = match direction {
                        Direction::Start => current_range.start,
                        Direction::End => current_range.end,
                    };
                    let insertion_range = insertion_range_start..insertion_range_start;
                    let copied_text: Rope = copied_texts.get(index).into();
                    let copied_text_len = copied_text.len_chars();

                    let (selection_range, paste_text) = if self.mode == Mode::Normal {
                        let range: CharIndexRange =
                            (insertion_range_start..insertion_range_start + copied_text_len).into();
                        let selection_range = match direction {
                            Direction::Start => range,
                            Direction::End => range.shift_right(gap.len_chars()),
                        };
                        let paste_text = {
                            match direction {
                                Direction::Start => {
                                    let mut paste_text = copied_text;
                                    paste_text.append(gap);
                                    paste_text
                                }
                                Direction::End => {
                                    let mut gap = gap;
                                    gap.append(copied_text);
                                    gap
                                }
                            }
                        };
                        (selection_range, paste_text)
                    } else {
                        let start = insertion_range_start + copied_text_len;
                        let selection_range = (start..start).into();
                        let paste_text = copied_text;
                        (selection_range, paste_text)
                    };
                    ActionGroup::new(
                        [
                            Action::Edit(Edit::new(
                                self.buffer().rope(),
                                insertion_range.into(),
                                paste_text,
                            )),
                            Action::Select(
                                selection.set_range(selection_range).set_initial_range(None),
                            ),
                        ]
                        .to_vec(),
                    )
                })
                .collect()
        });
        self.apply_edit_transaction(edit_transaction, context)
    }

    pub(crate) fn paste(
        &mut self,
        context: &mut Context,
        with_gap: bool,
    ) -> anyhow::Result<Dispatches> {
        let clipboards_differ: bool = !context.clipboards_synced();
        let Some(copied_texts) = context.get_clipboard_content(0) else {
            return Ok(Default::default());
        };
        let direction = self.cursor_direction.reverse();
        // out-of-sync paste should also add the content to clipboard history
        if clipboards_differ {
            context.add_clipboard_history(copied_texts.clone());
        }

        self.paste_text(direction, copied_texts, context, with_gap)
    }

    /// If `cut` if true, the replaced text will override the clipboard.
    ///
    /// If `history_offset` is 0, it means select the latest copied text;
    ///   +n means select the nth next copied text (cycle to the first copied text if current copied text is the latest)
    ///   -n means select the nth previous copied text (cycle to the last copied text if current copied text is the first)
    pub(crate) fn replace_with_copied_text(
        &mut self,
        context: &Context,
        cut: bool,
        history_offset: isize,
    ) -> anyhow::Result<Dispatches> {
        let dispatches = if cut {
            self.copy()?
        } else {
            Default::default()
        };

        let Some(copied_texts) = context.get_clipboard_content(history_offset) else {
            return Ok(Default::default());
        };

        Ok(self
            .transform_selection(
                Transformation::ReplaceWithCopiedText { copied_texts },
                context,
            )?
            .chain(dispatches))
    }

    pub(crate) fn apply_edit_transaction(
        &mut self,
        edit_transaction: EditTransaction,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        // Apply the transaction to the buffer
        let last_visible_line = self.last_visible_line(context);
        let (new_selection_set, dispatches, diff_edits) =
            self.buffer.borrow_mut().apply_edit_transaction(
                &edit_transaction,
                self.selection_set.clone(),
                self.mode != Mode::Insert,
                true,
                last_visible_line,
            )?;

        // Create a BufferEditTransaction dispatch for external integrations
        let buffer_edit_dispatch = if context.is_running_as_embedded() {
            edit_transaction
                .edits()
                .is_empty()
                .not()
                .then(|| {
                    // Get the path for the buffer
                    self.buffer().path().map(|path| {
                        // Create a dispatch to send buffer edit transaction to external integrations
                        Dispatches::one(Dispatch::ToHostApp(ToHostApp::BufferEditTransaction {
                            path,
                            edits: diff_edits,
                        }))
                    })
                })
                .flatten()
                .unwrap_or_default()
        } else {
            Dispatches::default()
        };

        self.set_selection_set(new_selection_set, context);

        self.recalculate_scroll_offset(context);

        self.clamp(context)?;

        // Add the buffer edit transaction dispatch
        let dispatches = self
            .get_document_did_change_dispatch()
            .chain(buffer_edit_dispatch)
            .chain(dispatches);

        Ok(dispatches)
    }

    pub(crate) fn get_document_did_change_dispatch(&mut self) -> Dispatches {
        [Dispatch::DocumentDidChange {
            component_id: self.id(),
            batch_id: self.buffer().batch_id().clone(),
            path: self.buffer().path(),
            content: self.buffer().rope().to_string(),
            language: self.buffer().language(),
        }]
        .into_iter()
        .collect_vec()
        .into()
    }

    pub(crate) fn undo(&mut self, context: &Context) -> anyhow::Result<Dispatches> {
        self.undo_or_redo(true, context)
    }

    pub(crate) fn redo(&mut self, context: &Context) -> anyhow::Result<Dispatches> {
        self.undo_or_redo(false, context)
    }

    pub(crate) fn swap_cursor(&mut self, context: &Context) {
        self.cursor_direction = match self.cursor_direction {
            Direction::Start => Direction::End,
            Direction::End => Direction::Start,
        };
        self.recalculate_scroll_offset(context)
    }

    pub(crate) fn get_selection_set(
        &self,
        mode: &SelectionMode,
        movement: Movement,
        context: &Context,
    ) -> anyhow::Result<Option<SelectionSet>> {
        self.selection_set.generate(
            &self.buffer.borrow(),
            mode,
            &movement.into_movement_applicandum(self.selection_set.sticky_column_index()),
            &self.cursor_direction,
            context,
        )
    }

    pub(crate) fn get_cursor_char_index(&self) -> CharIndex {
        self.selection_set
            .primary_selection()
            .to_char_index(&self.cursor_direction)
    }

    pub(crate) fn enable_selection_extension(&mut self) {
        self.selection_set.enable_selection_extension();
    }

    pub(crate) fn disable_selection_extension(&mut self) {
        self.selection_set.unset_initial_range();
    }

    pub(crate) fn handle_key_event(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        match self.handle_universal_key(key_event, context)? {
            HandleEventResult::Ignored(key_event) => {
                if let Some(jumps) = self.jumps.take() {
                    self.handle_jump_mode(context, key_event, jumps)
                } else if let Mode::Insert = self.mode {
                    self.handle_insert_mode(key_event, context)
                } else if let Mode::FindOneChar(_) = self.mode {
                    self.handle_find_one_char_mode(
                        IfCurrentNotFound::LookForward,
                        key_event,
                        context,
                    )
                } else {
                    let keymap_legend_config = self.get_current_keymap_legend_config(context);

                    if let Some(keymap) = keymap_legend_config.keymaps().get(&key_event) {
                        return Ok(keymap.get_dispatches());
                    }
                    log::info!("unhandled event: {key_event:?}");
                    Ok(vec![].into())
                }
            }
            HandleEventResult::Handled(dispatches) => Ok(dispatches),
        }
    }

    fn handle_jump_mode(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
        jumps: Vec<Jump>,
    ) -> anyhow::Result<Dispatches> {
        match key_event {
            key!("esc") => {
                self.jumps = None;
                Ok(Default::default())
            }
            key => {
                let KeyCode::Char(c) = key.code else {
                    return Ok(Default::default());
                };
                let matching_jumps = jumps
                    .iter()
                    .filter(|jump| c == jump.character)
                    .collect_vec();
                match matching_jumps.split_first() {
                    None => Ok(Default::default()),
                    Some((jump, [])) => {
                        let dispatches =
                            self.handle_movement(context, Movement::Jump(jump.selection.range()))?;
                        self.mode = Mode::Normal;
                        Ok(dispatches.append_some(if context.is_running_as_embedded() {
                            // We need to manually send a SelectionChanged dispatch here
                            // because although SelectionChanged is dispatched automatically in most cases, it is not for this case.
                            Some(self.dispatch_selection_changed())
                        } else {
                            None
                        }))
                    }
                    Some(_) => {
                        self.jumps = Some(
                            matching_jumps
                                .into_iter()
                                .zip(Self::jump_characters(context).into_iter().cycle())
                                .map(|(jump, character)| Jump {
                                    character,
                                    ..jump.clone()
                                })
                                .collect_vec(),
                        );
                        Ok(Default::default())
                    }
                }
            }
        }
        .map(|dispatches| dispatches.append(self.dispatch_jumps_changed()))
    }

    // This is similar to Ki's Change, except it enters normal mode
    pub(crate) fn delete_one(&mut self, context: &Context) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let delete_range = selection.extended_range();

                    // Ensure the delete range is at least one character long

                    let delete_range = if delete_range.len() == 0
                        && delete_range.start < CharIndex(self.buffer().len_chars())
                    {
                        (delete_range.start..delete_range.start + 1).into()
                    } else {
                        delete_range
                    };

                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit::new(
                                self.buffer().rope(),
                                delete_range,
                                Rope::new(),
                            )),
                            Action::Select(
                                selection
                                    .clone()
                                    .set_range((delete_range.start..delete_range.start + 1).into())
                                    .set_initial_range(None),
                            ),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect(),
        );

        let _ = self.enter_normal_mode(context);

        self.apply_edit_transaction(edit_transaction, context)
    }

    /// Similar to Change in Vim, but does not copy the current selection
    pub(crate) fn change(&mut self, context: &Context) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let range = selection.extended_range();
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit::new(self.buffer().rope(), range, Rope::new())),
                            Action::Select(
                                selection
                                    .clone()
                                    .set_range((range.start..range.start).into())
                                    .set_initial_range(None),
                            ),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect(),
        );

        Ok(self
            .apply_edit_transaction(edit_transaction, context)?
            .chain(self.enter_insert_mode(Direction::Start, context)?))
    }

    pub(crate) fn change_cut(&mut self, context: &Context) -> anyhow::Result<Dispatches> {
        Ok(self.copy()?.chain(self.change(context)?))
    }

    pub(crate) fn insert(&mut self, s: &str, context: &Context) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| {
                    let range = selection.extended_range();
                    let new_char_index = range.start + s.chars().count();
                    ActionGroup::new(
                        [
                            Action::Edit(Edit::new(
                                self.buffer().rope(),
                                {
                                    let start = selection.to_char_index(&Direction::End);
                                    (start..start).into()
                                },
                                Rope::from_str(s),
                            )),
                            Action::Select(
                                selection
                                    .clone()
                                    .set_range((new_char_index..new_char_index).into()),
                            ),
                        ]
                        .to_vec(),
                    )
                })
                .into(),
        );

        self.apply_edit_transaction(edit_transaction, context)
    }

    pub(crate) fn get_request_params(&self) -> Option<RequestParams> {
        let position = self.get_cursor_position().ok()?;
        self.path().map(|path| RequestParams {
            path,
            position,
            context: ResponseContext {
                scope: None,
                description: None,
            },
        })
    }

    pub(crate) fn set_selection_mode(
        &mut self,
        if_current_not_found: IfCurrentNotFound,
        selection_mode: SelectionMode,
        context: &Context,
        prior_change: Option<PriorChange>,
    ) -> anyhow::Result<Dispatches> {
        self.clear_incremental_search_matches();
        self.handle_prior_change(prior_change);
        if self.mode == Mode::MultiCursor {
            let selection_set = self.selection_set.clone().set_mode(selection_mode.clone());
            let selection_set = if let Some(all_selections) = selection_set.all_selections(
                &self.buffer.borrow(),
                &self.cursor_direction,
                context,
            )? {
                let selection_set = {
                    let selections = self
                        .selection_set
                        .map(|selection| {
                            let range = selection.extended_range();
                            all_selections
                                .iter()
                                .filter(|selection| {
                                    range.is_supserset_of(&selection.extended_range())
                                })
                                .collect_vec()
                        })
                        .into_iter()
                        .flatten()
                        .collect_vec();
                    if let Some((head, tail)) = selections.split_first() {
                        selection_set.set_selections(NonEmpty {
                            head: (**head).clone(),
                            tail: tail.iter().map(|selection| (**selection).clone()).collect(),
                        })
                    } else {
                        selection_set
                    }
                };
                selection_set
            } else {
                selection_set
            };
            Ok(self
                .update_selection_set(selection_set, true, context)
                .append(Dispatch::ToEditor(EnterNormalMode)))
        } else {
            if self.reveal == Some(Reveal::CurrentSelectionMode)
                && self.selection_set.mode() != &selection_mode
            {
                self.reveal = None
            }
            self.move_selection_with_selection_mode_without_global_mode(
                Movement::Current(if_current_not_found),
                selection_mode.clone(),
                context,
            )
            .map(|dispatches| {
                Dispatches::one(Dispatch::SetGlobalMode(None))
                    .chain(dispatches)
                    .append_some(if selection_mode.is_contiguous() {
                        None
                    } else {
                        Some(Dispatch::SetLastNonContiguousSelectionMode(Either::Left(
                            selection_mode,
                        )))
                    })
            })
        }
    }

    fn move_selection_with_selection_mode(
        &mut self,
        context: &Context,
        movement: Movement,
        selection_mode: SelectionMode,
    ) -> anyhow::Result<Dispatches> {
        if let Some(global_mode) = &context.mode() {
            match global_mode {
                GlobalMode::QuickfixListItem => {
                    Ok(vec![Dispatch::GotoQuickfixListItem(movement)].into())
                }
            }
        } else {
            self.move_selection_with_selection_mode_without_global_mode(
                movement,
                selection_mode,
                context,
            )
        }
    }

    pub(crate) fn handle_movement(
        &mut self,
        context: &Context,
        movement: Movement,
    ) -> anyhow::Result<Dispatches> {
        self.copied_text_history_offset.reset();
        match self.mode {
            Mode::Normal => self.move_selection_with_selection_mode(
                context,
                movement,
                self.selection_set.mode().clone(),
            ),
            Mode::Swap => self.swap(movement, context),
            Mode::Replace => self.replace_with_movement(&movement, context),
            Mode::MultiCursor => self
                .add_cursor(
                    &movement.into_movement_applicandum(self.selection_set.sticky_column_index()),
                    context,
                )
                .map(|_| Default::default()),
            Mode::Delete => self.delete_with_movement(context, movement),
            Mode::FindOneChar(_) | Mode::Insert => Ok(Default::default()),
        }
    }

    pub(crate) fn toggle_marks(&mut self) -> Dispatches {
        let selections = self
            .selection_set
            .map(|selection| selection.extended_range());

        self.path()
            .map(|path| {
                Dispatches::one(Dispatch::SaveMarks {
                    path,
                    marks: selections.iter().copied().collect(),
                })
            })
            .unwrap_or_default()
    }

    pub(crate) fn path(&self) -> Option<CanonicalizedPath> {
        self.editor().buffer().path()
    }

    pub(crate) fn enter_insert_mode(
        &mut self,
        direction: Direction,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        self.set_selection_set(
            self.selection_set
                .apply(self.selection_set.mode.clone(), |selection| {
                    let range = selection.extended_range();
                    let char_index = match direction {
                        Direction::Start => range.start,
                        Direction::End => range.end,
                    };
                    Ok(selection
                        .clone()
                        .set_range((char_index..char_index).into())
                        .set_initial_range(None))
                })?,
            context,
        );
        self.mode = Mode::Insert;
        self.cursor_direction = Direction::Start;
        Ok(Dispatches::one(Dispatch::RequestSignatureHelp))
    }

    pub(crate) fn enter_normal_mode(&mut self, context: &Context) -> anyhow::Result<()> {
        if self.mode == Mode::Insert {
            // This is necessary for cursor to not overflow after exiting insert mode
            self.set_selection_set(
                self.selection_set
                    .apply(self.selection_set.mode.clone(), |selection| {
                        let range = {
                            if let Ok(position) = self
                                .buffer()
                                .char_to_position(selection.extended_range().start)
                            {
                                let start = selection.extended_range().start
                                    - if position.column > 0 { 1 } else { 0 };
                                (start..start + 1).into()
                            } else {
                                selection.extended_range()
                            }
                        };
                        Ok(selection.clone().set_range(range))
                    })?,
                context,
            );
            self.clamp(context)?;
            self.buffer_mut().reparse_tree()?;
        }
        // TODO: continue from here, need to add test: upon exiting insert mode, should close all panels
        // Maybe we should call this function the exit_insert_mode?

        self.mode = Mode::Normal;
        self.selection_set.unset_initial_range();
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn jump_chars(&self) -> Vec<char> {
        self.jumps()
            .into_iter()
            .map(|jump| jump.character)
            .collect_vec()
    }

    pub(crate) fn jumps(&self) -> Vec<&Jump> {
        self.jumps
            .as_ref()
            .map(|jumps| jumps.iter().collect())
            .unwrap_or_default()
    }

    // TODO: handle mouse click
    #[allow(dead_code)]
    pub(crate) fn set_cursor_position(
        &mut self,
        row: usize,
        column: usize,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        let start = (self.buffer.borrow().line_to_char(row)?) + column;
        let primary = self
            .selection_set
            .primary_selection()
            .clone()
            .set_range((start..start).into());
        Ok(self.update_selection_set(
            self.selection_set
                .clone()
                .set_selections(NonEmpty::new(primary)),
            true,
            context,
        ))
    }

    /// Get the selection that preserves the syntactic structure of the current selection.
    ///
    /// Returns a valid edit transaction if there is any, otherwise `Left(current_selection)`.
    fn get_valid_selection(
        &self,
        current_selection: &Selection,
        selection_mode: &SelectionMode,
        movement: &Movement,
        get_actual_edit_transaction: impl Fn(
            /* current */ &Selection,
            /* next */ &Selection,
        ) -> anyhow::Result<EditTransaction>,
        context: &Context,
    ) -> anyhow::Result<Either<Selection, EditTransaction>> {
        let current_selection = current_selection.clone();

        let buffer = self.buffer.borrow();

        // Loop until the edit transaction does not result in errorneous node
        let mut next_selection = Selection::get_selection_(
            &buffer,
            // Collapse selection so that "Swapping extended selection" works
            &current_selection
                .clone()
                .collapsed_to_anchor_range(&movement.to_direction()),
            selection_mode,
            &movement.into_movement_applicandum(self.selection_set.sticky_column_index()),
            &self.cursor_direction,
            context,
        )?
        .unwrap_or_else(|| current_selection.clone().into())
        .selection;

        if next_selection.eq(&current_selection) {
            return Ok(Either::Left(current_selection));
        }

        loop {
            let edit_transaction =
                get_actual_edit_transaction(&current_selection, &next_selection)?;
            let current_node = buffer.get_current_node(&current_selection, false)?;

            let new_buffer = {
                let mut new_buffer = self.buffer.borrow().clone();
                if new_buffer
                    .apply_edit_transaction(
                        &edit_transaction,
                        self.selection_set.clone(),
                        true,
                        true,
                        self.last_visible_line(context),
                    )
                    .is_err()
                {
                    continue;
                }
                new_buffer
            };

            let text_at_next_selection: Rope = buffer.slice(&next_selection.extended_range())?;

            let next_nodes = edit_transaction
                .selections()
                .into_iter()
                .map(|selection| -> anyhow::Result<_> {
                    new_buffer.get_current_node(selection, false)
                })
                .collect::<Result<Vec<_>, _>>()?;

            // Why don't we just use `tree.root_node().has_error()` instead?
            // Because I assume we want to be able to swap even if some part of the tree
            // contains error
            if !selection_mode.is_node()
                || (!text_at_next_selection.to_string().trim().is_empty()
                    && next_nodes
                        .iter()
                        .all(|next_node| match (current_node, next_node) {
                            (Some(current_node), Some(next_node)) => {
                                current_node.byte_range().len() == next_node.byte_range().len()
                            }
                            (_, _) => true,
                        })
                    && !new_buffer.has_syntax_error_at(edit_transaction.range()))
            {
                return Ok(Either::Right(get_actual_edit_transaction(
                    &current_selection,
                    &next_selection,
                )?));
            }

            // Get the next selection
            let new_selection = Selection::get_selection_(
                &buffer,
                &next_selection,
                selection_mode,
                &movement.into_movement_applicandum(self.selection_set.sticky_column_index()),
                &self.cursor_direction,
                context,
            )?
            .unwrap_or_else(|| next_selection.clone().into())
            .selection;

            if next_selection.eq(&new_selection) {
                return Ok(Either::Left(current_selection));
            }

            next_selection = new_selection;
        }
    }

    fn make_swap_action_groups(
        rope: &Rope,
        first_selection: &Selection,
        first_selection_range: CharIndexRange,
        first_selection_text: Rope,
        second_selection_range: CharIndexRange,
        second_selection_text: Rope,
    ) -> Vec<ActionGroup> {
        let new_select_range: CharIndexRange = (second_selection_range.start
            ..(second_selection_range.start + first_selection_text.len_chars()))
            .into();

        let selection = first_selection.clone().apply_offset(
            (new_select_range.start.0 as isize) - (first_selection_range.start.0 as isize),
        );
        [
            ActionGroup::new(
                [Action::Edit(Edit::new(
                    rope,
                    first_selection_range,
                    second_selection_text.clone(),
                ))]
                .to_vec(),
            ),
            ActionGroup::new(
                [
                    Action::Edit(Edit::new(
                        rope,
                        second_selection_range,
                        first_selection_text.clone(),
                    )),
                    Action::Select(selection),
                ]
                .to_vec(),
            ),
        ]
        .to_vec()
    }

    /// Replace the next selection with the current selection without
    /// making the syntax node invalid.
    fn replace_faultlessly(
        &mut self,
        selection_mode: &SelectionMode,
        movement: Movement,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        let buffer = self.buffer.borrow().clone();
        let get_edit_transaction =
            |current_selection: &Selection, next_selection: &Selection| -> anyhow::Result<_> {
                let current_selection_range = current_selection.extended_range();
                let next_selection_range = next_selection
                    .extended_range()
                    // Subtract the current selection range to prevent duplication during swap.
                    // Without this, overlapping selections would duplicate the overlapping text.
                    //
                    // Example: In "foo bar spam", if current = "foo" and next = "foo bar",
                    // swapping without subtraction would produce "foo bar foo spam" instead of "bar foo spam".
                    .subtracts(&current_selection_range);

                let text_at_current_selection: Rope = buffer.slice(&current_selection_range)?;
                let text_at_next_selection: Rope = buffer.slice(&next_selection_range)?;
                Ok(EditTransaction::from_action_groups(
                    Self::make_swap_action_groups(
                        buffer.rope(),
                        current_selection,
                        current_selection_range,
                        text_at_current_selection,
                        next_selection_range,
                        text_at_next_selection,
                    ),
                ))
            };

        let edit_transactions = self
            .selection_set
            .map(|selection| {
                self.get_valid_selection(
                    selection,
                    selection_mode,
                    &movement,
                    get_edit_transaction,
                    context,
                )
            })
            .into_iter()
            .filter_map(|transaction| transaction.ok())
            .filter_map(|transaction| transaction.map_right(Some).right_or(None))
            .collect_vec();

        self.apply_edit_transaction(EditTransaction::merge(edit_transactions), context)
    }

    pub(crate) fn swap(
        &mut self,
        movement: Movement,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        match movement {
            Movement::Last => self.swap_till_last(context),
            Movement::First => self.swap_till_first(context),
            _ => self.replace_faultlessly(&self.selection_set.mode().clone(), movement, context),
        }
    }

    /// Swaps the current selection with the text range from
    /// the first occurrence until just before the current selection.
    fn swap_till_first(&mut self, context: &Context) -> anyhow::Result<Dispatches> {
        let selection_mode = self.selection_set.mode().clone();
        let edit_transaction = {
            let buffer = self.buffer.borrow();
            EditTransaction::from_action_groups(
                self.selection_set
                    .map(|current_selection| {
                        // Select from the first until before current
                        let selection_mode = selection_mode
                            .to_selection_mode_trait_object(
                                &buffer,
                                current_selection,
                                &self.cursor_direction,
                                context.current_working_directory(),
                                context.quickfix_list_items(),
                                &context.get_marks(self.path()),
                            )
                            .ok()?;

                        let params = selection_mode::SelectionModeParams {
                            buffer: &buffer,
                            current_selection: &current_selection
                                .clone()
                                .collapsed_to_anchor_range(&Direction::Start),
                            cursor_direction: &self.cursor_direction,
                        };
                        let first = selection_mode.first(&params).ok()??.range();
                        // Find the before current selection
                        let before_current = selection_mode.left(&params).ok()??.range();
                        let first_range = current_selection.extended_range();
                        let second_range: CharIndexRange =
                            (first.start()..before_current.end()).into();
                        // Swap the range with the last selection
                        Some(Self::make_swap_action_groups(
                            buffer.rope(),
                            current_selection,
                            first_range,
                            buffer.slice(&first_range).ok()?,
                            second_range,
                            buffer.slice(&second_range).ok()?,
                        ))
                    })
                    .into_iter()
                    .flatten()
                    .flatten()
                    .collect(),
            )
        };
        self.apply_edit_transaction(edit_transaction, context)
    }

    /// Swaps the current selection with the text range from
    /// just after the current selection until the last occurrence.
    fn swap_till_last(&mut self, context: &Context) -> anyhow::Result<Dispatches> {
        let selection_mode = self.selection_set.mode().clone();
        let edit_transaction = {
            let buffer = self.buffer.borrow();
            EditTransaction::from_action_groups(
                self.selection_set
                    .map(|current_selection| {
                        let selection_mode = selection_mode
                            .to_selection_mode_trait_object(
                                &buffer,
                                current_selection,
                                &self.cursor_direction,
                                context.current_working_directory(),
                                context.quickfix_list_items(),
                                &context.get_marks(self.path()),
                            )
                            .ok()?;
                        let params = selection_mode::SelectionModeParams {
                            buffer: &buffer,
                            current_selection: &current_selection
                                .clone()
                                .collapsed_to_anchor_range(&Direction::End),
                            cursor_direction: &self.cursor_direction,
                        };

                        // Select from the first until before current
                        let last = selection_mode.last(&params).ok()??.range();
                        // Find the before current selection
                        let after_current = selection_mode.right(&params).ok()??.range();
                        let first_range = current_selection.extended_range();
                        let second_range: CharIndexRange =
                            (after_current.start()..last.end()).into();
                        // Swap the range with the last selection
                        Some(Self::make_swap_action_groups(
                            buffer.rope(),
                            current_selection,
                            first_range,
                            buffer.slice(&first_range).ok()?,
                            second_range,
                            buffer.slice(&second_range).ok()?,
                        ))
                    })
                    .into_iter()
                    .flatten()
                    .flatten()
                    .collect(),
            )
        };
        self.apply_edit_transaction(edit_transaction, context)
    }

    pub(crate) fn add_cursor(
        &mut self,
        movement: &MovementApplicandum,
        context: &Context,
    ) -> anyhow::Result<()> {
        let mut add_selection = |movement: &MovementApplicandum| {
            self.selection_set.add_selection(
                &self.buffer.borrow(),
                movement,
                &self.cursor_direction,
                context,
            )
        };
        match movement {
            MovementApplicandum::First => {
                while let Ok(true) = add_selection(&MovementApplicandum::Left) {}
            }
            MovementApplicandum::Last => {
                while let Ok(true) = add_selection(&MovementApplicandum::Right) {}
            }
            other_movement => {
                add_selection(other_movement)?;
            }
        };
        self.recalculate_scroll_offset(context);
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn get_selected_texts(&self) -> Vec<String> {
        let buffer = self.buffer.borrow();
        let mut selections = self
            .selection_set
            .map(|selection| -> anyhow::Result<_> {
                Ok((
                    selection.extended_range(),
                    buffer.slice(&selection.extended_range())?.to_string(),
                ))
            })
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        selections.sort_by(|a, b| a.0.start.0.cmp(&b.0.start.0));
        selections
            .into_iter()
            .map(|selection| selection.1)
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn text(&self) -> String {
        let buffer = self.buffer.borrow().clone();
        buffer.rope().slice(0..buffer.len_chars()).to_string()
    }

    pub(crate) fn dimension(&self) -> Dimension {
        self.rectangle.dimension()
    }

    fn apply_scroll(&mut self, direction: Direction, scroll_height: usize) {
        self.scroll_offset = match direction {
            Direction::Start => self.scroll_offset.saturating_sub(scroll_height),
            Direction::End => self.scroll_offset.saturating_add(scroll_height),
        };
    }

    pub(crate) fn backspace(&mut self, context: &Context) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| {
                    let start = CharIndex(selection.extended_range().start.0.saturating_sub(1));
                    ActionGroup::new(
                        [
                            Action::Edit(Edit::new(
                                self.buffer().rope(),
                                (start..selection.extended_range().start).into(),
                                Rope::from(""),
                            )),
                            Action::Select(selection.clone().set_range((start..start).into())),
                        ]
                        .to_vec(),
                    )
                })
                .into(),
        );

        self.apply_edit_transaction(edit_transaction, context)
    }

    pub(crate) fn delete_word_backward(
        &mut self,
        short: bool,
        context: &Context,
    ) -> Result<Dispatches, anyhow::Error> {
        let action_groups = self
            .selection_set
            .map(|current_selection| -> anyhow::Result<_> {
                let current_range = current_selection.extended_range();
                if current_range.start.0 == 0 && current_range.end.0 == 0 {
                    // Do nothing if cursor is at the beginning of the file
                    return Ok(ActionGroup::new(Vec::new()));
                }

                let len_chars = self.buffer().rope().len_chars();
                let start = CharIndex(current_range.start.0.min(len_chars).saturating_sub(1));

                let previous_word = {
                    let get_word = |movement: Movement| {
                        Selection::get_selection_(
                            &self.buffer(),
                            &current_selection.clone().set_range((start..start).into()),
                            &if short {
                                SelectionMode::Subword
                            } else {
                                SelectionMode::Word
                            },
                            &movement.into_movement_applicandum(
                                self.selection_set.sticky_column_index(),
                            ),
                            &self.cursor_direction,
                            context,
                        )
                        .map(|option| option.unwrap_or_else(|| current_selection.clone().into()))
                    };
                    let current_word =
                        get_word(Movement::Current(IfCurrentNotFound::LookBackward))?.selection;
                    if current_word.extended_range().start <= start {
                        current_word
                    } else {
                        get_word(Movement::Previous)?.selection
                    }
                };

                let previous_word_range = previous_word.extended_range();
                let end = previous_word_range
                    .end
                    .min(current_range.start)
                    .max(start + 1);
                let start = previous_word_range.start;
                Ok(ActionGroup::new(
                    [
                        Action::Edit(Edit::new(
                            self.buffer().rope(),
                            (start..end).into(),
                            Rope::from(""),
                        )),
                        Action::Select(current_selection.clone().set_range((start..start).into())),
                    ]
                    .to_vec(),
                ))
            })
            .into_iter()
            .flatten()
            .collect();
        let edit_transaction = EditTransaction::from_action_groups(action_groups);
        self.apply_edit_transaction(edit_transaction, context)
    }

    /// Replace the parent node of the current node with the current node
    pub(crate) fn replace_with_movement(
        &mut self,
        movement: &Movement,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        let buffer = self.buffer.borrow().clone();
        let edit_transactions = self.selection_set.map(|selection| {
            let get_edit_transaction =
                |current_selection: &Selection, other_selection: &Selection| -> anyhow::Result<_> {
                    let range = current_selection
                        .extended_range()
                        .start
                        .min(other_selection.extended_range().start)
                        ..current_selection
                            .extended_range()
                            .end
                            .max(other_selection.extended_range().end);
                    let new: Rope = buffer.slice(&current_selection.extended_range())?;

                    let new_len_chars = new.len_chars();
                    Ok(EditTransaction::from_action_groups(
                        [ActionGroup::new(
                            [
                                Action::Edit(Edit::new(
                                    self.buffer().rope(),
                                    range.clone().into(),
                                    new,
                                )),
                                Action::Select(current_selection.clone().set_range(
                                    (range.start..(range.start + new_len_chars)).into(),
                                )),
                            ]
                            .to_vec(),
                        )]
                        .to_vec(),
                    ))
                };
            self.get_valid_selection(
                selection,
                self.selection_set.mode(),
                movement,
                get_edit_transaction,
                context,
            )
        });
        let edit_transaction = EditTransaction::merge(
            edit_transactions
                .into_iter()
                .filter_map(|edit_transaction| edit_transaction.ok())
                .filter_map(|edit_transaction| edit_transaction.map_right(Some).right_or(None))
                .collect(),
        );
        self.apply_edit_transaction(edit_transaction, context)
    }

    pub(crate) fn buffer(&self) -> Ref<Buffer> {
        self.buffer.borrow()
    }

    pub(crate) fn buffer_rc(&self) -> Rc<RefCell<Buffer>> {
        self.buffer.clone()
    }

    pub(crate) fn buffer_mut(&mut self) -> RefMut<Buffer> {
        self.buffer.borrow_mut()
    }

    fn update_buffer(&mut self, s: &str) {
        self.buffer.borrow_mut().update(s)
    }

    fn scroll(
        &mut self,
        direction: Direction,
        scroll_height: usize,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        let position = self
            .selection_set
            .primary_selection()
            .extended_range()
            .start
            .to_position(&self.buffer());
        let line = if direction == Direction::End {
            position.line.saturating_add(scroll_height)
        } else {
            position.line.saturating_sub(scroll_height)
        }
        .min(self.buffer().len_lines().saturating_sub(1));
        let position = Position { line, column: 0 };
        let start = position.to_char_index(&self.buffer())?;
        let selection_mode = self.selection_set.mode().clone();
        self.selection_set = SelectionSet::new(NonEmpty::new(
            self.selection_set
                .primary_selection()
                .clone()
                .set_range((start..start).into()),
        ))
        .set_mode(selection_mode);
        self.handle_movement(context, Movement::Current(IfCurrentNotFound::LookForward))
    }

    /// This returns a vector of selections
    /// with a gap that is the maximum of previous-current gap and current-next gap.
    ///
    /// Used by `Self::paste` and `Self::open`.
    fn get_selection_set_with_gap(
        &self,
        direction: &Direction,
        context: &Context,
    ) -> anyhow::Result<Vec<(Selection, Rope)>> {
        self.selection_set
            .map(|selection| {
                let object = self.get_selection_mode_trait_object(
                    selection,
                    true,
                    context.current_working_directory(),
                    context.quickfix_list_items(),
                    &context.get_marks(self.path()),
                )?;
                let buffer = self.buffer.borrow();
                let gap = object.get_paste_gap(
                    &selection_mode::SelectionModeParams {
                        buffer: &buffer,
                        current_selection: selection,
                        cursor_direction: &self.cursor_direction,
                    },
                    direction,
                );
                // Ensure the gap only contain at most one newline character
                let gap: String = gap
                    .chars()
                    .scan(false, |newline_found, c| {
                        if c == '\n' {
                            if *newline_found {
                                None
                            } else {
                                *newline_found = true;
                                Some(c)
                            }
                        } else {
                            Some(c)
                        }
                    })
                    .collect();
                Ok((selection.clone(), gap.into()))
            })
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>()
    }

    fn open(&mut self, context: &Context) -> Result<Dispatches, anyhow::Error> {
        let direction = self.cursor_direction.reverse();
        let edit_transaction = EditTransaction::from_action_groups(
            self.get_selection_set_with_gap(&direction, context)?
                .into_iter()
                .map(|(selection, gap)| {
                    let gap = if gap.len_chars() == 0 {
                        Rope::from_str(" ")
                    } else {
                        gap
                    };
                    let gap_len = gap.len_chars();
                    ActionGroup::new(
                        [
                            Action::Edit(Edit::new(
                                self.buffer().rope(),
                                {
                                    let start = match direction {
                                        Direction::Start => selection.range().start,
                                        Direction::End => selection.range().end,
                                    };
                                    (start..start).into()
                                },
                                gap,
                            )),
                            Action::Select(selection.clone().set_range({
                                let start = match direction {
                                    Direction::Start => selection.range().start,
                                    Direction::End => selection.range().end + gap_len,
                                };
                                (start..start).into()
                            })),
                        ]
                        .to_vec(),
                    )
                })
                .collect_vec(),
        );

        Ok(self
            .apply_edit_transaction(edit_transaction, context)?
            .append(Dispatch::ToEditor(EnterInsertMode(direction))))
    }

    pub(crate) fn apply_positional_edits(
        &mut self,
        edits: Vec<PositionalEdit>,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups(
            edits
                .into_iter()
                .filter_map(|edit| {
                    let range = edit.range.start.to_char_index(&self.buffer()).ok()?
                        ..edit.range.end.to_char_index(&self.buffer()).ok()?;

                    let action_edit = Action::Edit(Edit::new(
                        self.buffer().rope(),
                        range.clone().into(),
                        edit.new_text.into(),
                    ));

                    Some(ActionGroup::new(vec![action_edit]))
                })
                .chain(
                    // This is necessary to retain the current selection set
                    self.selection_set
                        .map(|selection| ActionGroup::new(vec![Action::Select(selection.clone())])),
                )
                .collect(),
        );
        self.apply_edit_transaction(edit_transaction, context)
    }

    pub(crate) fn save(&mut self, context: &Context) -> anyhow::Result<Dispatches> {
        self.do_save(false, context)
    }

    fn do_save(&mut self, force: bool, context: &Context) -> anyhow::Result<Dispatches> {
        let last_visible_line = self.last_visible_line(context);

        let (dispatches, path) = if context.is_running_as_embedded() {
            (Dispatches::default(), self.path())
        } else {
            self.buffer
                .borrow_mut()
                .save(self.selection_set.clone(), force, last_visible_line)?
        };

        let Some(path) = path else {
            return Ok(Default::default());
        };

        self.clamp(context)?;
        self.cursor_keep_primary_only();
        self.enter_normal_mode(context)?;
        Ok(Dispatches::one(Dispatch::RemainOnlyCurrentComponent)
            .append(Dispatch::DocumentDidSave { path })
            .chain(self.get_document_did_change_dispatch())
            .append(Dispatch::RemainOnlyCurrentComponent)
            .chain(dispatches)
            .append_some(if self.selection_set.mode().is_contiguous() {
                Some(Dispatch::ToEditor(MoveSelection(Movement::Current(
                    IfCurrentNotFound::LookForward,
                ))))
            } else {
                None
            }))
    }

    /// Clamp everything that might be out of bound after the buffer content is modified elsewhere
    fn clamp(&mut self, context: &Context) -> anyhow::Result<()> {
        let len_chars = self.buffer().len_chars();
        self.set_selection_set(self.selection_set.clamp(CharIndex(len_chars))?, context);

        let len_lines = self.buffer().len_lines();
        self.scroll_offset = self.scroll_offset.clamp(0, len_lines);

        Ok(())
    }

    pub(crate) fn surround(
        &mut self,
        open: String,
        close: String,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let old = self.buffer().slice(&selection.extended_range())?;
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit::new(
                                self.buffer().rope(),
                                selection.extended_range(),
                                format!("{open}{old}{close}").into(),
                            )),
                            Action::Select(
                                selection.clone().set_range(
                                    (selection.extended_range().start
                                        ..selection.extended_range().end
                                            + open.chars().count()
                                            + close.chars().count())
                                        .into(),
                                ),
                            ),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect_vec(),
        );
        Ok(self
            .apply_edit_transaction(edit_transaction, context)?
            .append(Dispatch::ToEditor(DisableSelectionExtension)))
    }

    fn transform_selection(
        &mut self,
        transformation: Transformation,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map_with_index(|index, selection| -> anyhow::Result<_> {
                    let new: Rope = transformation
                        .apply(
                            index,
                            self.buffer()
                                .slice(&selection.extended_range())?
                                .to_string(),
                        )?
                        .into();
                    let new_char_count = new.chars().count();
                    let range = selection.extended_range();
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit::new(self.buffer().rope(), range, new)),
                            Action::Select(
                                selection
                                    .clone()
                                    .set_range((range.start..range.start + new_char_count).into())
                                    .set_initial_range(None),
                            ),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .try_collect()?,
        );
        self.apply_edit_transaction(edit_transaction, context)
    }

    pub(crate) fn display_mode(&self) -> String {
        if self.jumps.is_some() {
            "JUMP".to_string()
        } else {
            match &self.mode {
                Mode::Normal => {
                    let prefix = if self.selection_set.is_extended() {
                        "+"
                    } else {
                        ""
                    };
                    format!("{prefix}NORM")
                }
                Mode::Insert => "INST".to_string(),
                Mode::MultiCursor => "MULTI".to_string(),
                Mode::FindOneChar(_) => "ONE".to_string(),
                Mode::Swap => "SWAP".to_string(),
                Mode::Replace => "RPLCE".to_string(),
                Mode::Delete => "DELTE".to_string(),
            }
        }
    }

    pub(crate) fn display_selection_mode(&self) -> String {
        let selection_mode = self.selection_set.mode().display();
        let cursor_count = self.selection_set.len();
        format!("{selection_mode: <5}x{cursor_count}")
    }

    pub(crate) fn visible_line_range(&self) -> Range<usize> {
        self.visible_line_range_given_scroll_offset_and_height(
            self.scroll_offset,
            self.rectangle.height,
        )
    }

    pub(crate) fn visible_line_range_given_scroll_offset_and_height(
        &self,
        scroll_offset: usize,
        height: usize,
    ) -> Range<usize> {
        let start = scroll_offset;
        let end = (start + height).min(self.buffer().len_lines());

        start..end
    }

    pub(crate) fn add_cursor_to_all_selections(
        &mut self,
        context: &Context,
    ) -> Result<(), anyhow::Error> {
        self.mode = Mode::Normal;
        self.reveal = Some(Reveal::Cursor);
        self.selection_set
            .add_all(&self.buffer.borrow(), &self.cursor_direction, context)?;
        self.recalculate_scroll_offset(context);
        Ok(())
    }

    pub(crate) fn dispatch_selection_changed(&self) -> Dispatch {
        Dispatch::ToHostApp(ToHostApp::SelectionChanged {
            component_id: self.id(),
            selections: {
                // Rearrange the selection such that selections[cursor_index] will be the first selection
                let cursor_index = self.selection_set.cursor_index;
                let (current, others): (Vec<_>, Vec<_>) = self
                    .selection_set
                    .selections()
                    .into_iter()
                    .enumerate()
                    .partition(|(i, _)| *i == cursor_index);

                current
                    .into_iter()
                    .map(|(_, sel)| sel.clone())
                    .chain(others.into_iter().map(|(_, sel)| sel.clone()))
                    .collect()
            },
        })
    }

    pub(crate) fn cursor_keep_primary_only(&mut self) {
        self.mode = Mode::Normal;
        if self.reveal == Some(Reveal::Cursor) {
            self.reveal = None;
        }
        self.selection_set.only();
    }

    fn enter_single_character_mode(&mut self, if_current_not_found: IfCurrentNotFound) {
        self.mode = Mode::FindOneChar(if_current_not_found);
    }

    fn handle_find_one_char_mode(
        &mut self,
        if_current_not_found: IfCurrentNotFound,
        key_event: KeyEvent,
        context: &Context,
    ) -> Result<Dispatches, anyhow::Error> {
        match key_event.code {
            KeyCode::Char(c) => {
                self.mode = Mode::Normal;
                self.set_selection_mode(
                    if_current_not_found,
                    SelectionMode::Find {
                        search: Search {
                            search: c.to_string(),
                            mode: LocalSearchConfigMode::Regex(crate::list::grep::RegexConfig {
                                escaped: true,
                                case_sensitive: true,
                                match_whole_word: false,
                            }),
                        },
                    },
                    context,
                    None,
                )
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                Ok(Default::default())
            }
            _ => Ok(Default::default()),
        }
    }

    pub(crate) fn set_decorations(&mut self, decorations: &[super::suggestive_editor::Decoration]) {
        self.buffer.borrow_mut().set_decorations(decorations)
    }

    fn half_page_height(&self) -> usize {
        let height = if let Some(visible_line_ranges) = self.visible_line_ranges.as_ref() {
            visible_line_ranges
                .iter()
                .map(|range| range.len())
                .max()
                .unwrap_or_default()
        } else {
            self.dimension().height
        };
        height / 2
    }

    #[cfg(test)]
    pub(crate) fn match_literal(
        &mut self,
        search: &str,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        self.set_selection_mode(
            IfCurrentNotFound::LookForward,
            SelectionMode::Find {
                search: Search {
                    mode: LocalSearchConfigMode::Regex(crate::list::grep::RegexConfig {
                        escaped: true,
                        case_sensitive: false,
                        match_whole_word: false,
                    }),
                    search: search.to_string(),
                },
            },
            context,
            None,
        )
    }

    pub(crate) fn move_to_line_start(&mut self, context: &Context) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups({
            let buffer = self.buffer();
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let cursor = selection.to_char_index(&self.cursor_direction);
                    let line = buffer.char_to_line(cursor)?;
                    let char_index = buffer.line_to_char(line)?;
                    Ok(ActionGroup::new(
                        [Action::Select(
                            selection
                                .clone()
                                .set_range((char_index..char_index).into())
                                .set_initial_range(None),
                        )]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect()
        });
        self.apply_edit_transaction(edit_transaction, context)
    }

    pub(crate) fn move_to_line_end(&mut self) -> anyhow::Result<Dispatches> {
        Ok([
            Dispatch::ToEditor(SelectLine(Movement::Current(
                IfCurrentNotFound::LookForward,
            ))),
            Dispatch::ToEditor(EnterInsertMode(Direction::End)),
        ]
        .to_vec()
        .into())
    }

    pub(crate) fn select_all(&mut self, context: &mut Context) -> anyhow::Result<Dispatches> {
        self.handle_dispatch_editors(
            context,
            [
                DisableSelectionExtension,
                (MoveSelection(Movement::First)),
                EnableSelectionExtension,
                (MoveSelection(Movement::Last)),
            ]
            .to_vec(),
        )
    }

    fn move_selection_with_selection_mode_without_global_mode(
        &mut self,
        movement: Movement,
        selection_mode: SelectionMode,
        context: &Context,
    ) -> Result<Dispatches, anyhow::Error> {
        let dispatches = self.select(selection_mode, movement, context)?;
        self.current_view_alignment = None;

        Ok(dispatches)
    }

    pub(crate) fn scroll_page_down(
        &mut self,
        context: &Context,
    ) -> Result<Dispatches, anyhow::Error> {
        self.scroll(Direction::End, self.half_page_height(), context)
    }

    pub(crate) fn scroll_page_up(
        &mut self,
        context: &Context,
    ) -> Result<Dispatches, anyhow::Error> {
        self.scroll(Direction::Start, self.half_page_height(), context)
    }

    #[cfg(test)]
    pub(crate) fn current_view_alignment(&self) -> Option<ViewAlignment> {
        self.current_view_alignment
    }

    pub(crate) fn switch_view_alignment(&mut self, context: &Context) {
        self.current_view_alignment = Some(match self.current_view_alignment {
            Some(ViewAlignment::Top) => {
                self.align_selection_to_center(context);
                ViewAlignment::Center
            }
            Some(ViewAlignment::Center) => {
                self.align_selection_to_bottom(context);
                ViewAlignment::Bottom
            }
            None | Some(ViewAlignment::Bottom) => {
                self.align_selection_to_top();
                ViewAlignment::Top
            }
        })
    }

    fn undo_or_redo(&mut self, undo: bool, context: &Context) -> Result<Dispatches, anyhow::Error> {
        let last_visible_line = self.last_visible_line(context);

        // Call the appropriate buffer method to perform undo/redo
        let result = if undo {
            self.buffer_mut().undo(last_visible_line)
        } else {
            self.buffer_mut().redo(last_visible_line)
        }?;

        // Create dispatches for document changes and buffer edit transaction
        let dispatches = match result {
            Some((selection_set, diff_edits, edits)) => {
                // Update selection set
                let dispatches = self.update_selection_set(selection_set, false, context);

                // Create a BufferEditTransaction dispatch for external integrations
                let dispatch = if let Some(path) = self.buffer().path() {
                    // Create a dispatch to send buffer edit transaction
                    Dispatches::one(Dispatch::ToHostApp(ToHostApp::BufferEditTransaction {
                        path: path.clone(),
                        edits: diff_edits,
                    }))
                    .append(Dispatch::AppliedEdits { path, edits })
                } else {
                    Default::default()
                };

                dispatches.chain(dispatch)
            }
            Option::None => Dispatches::default(),
        };

        // Add document did change dispatch
        let dispatches = dispatches.chain(self.get_document_did_change_dispatch());

        // Also send a mode change notification to ensure external integrations are in sync
        let dispatches = dispatches.append(Dispatch::ToHostApp(ToHostApp::ModeChanged));

        log::trace!("undo_or_redo: Returning dispatches");

        Ok(dispatches)
    }

    #[cfg(test)]
    pub(crate) fn set_scroll_offset(&mut self, scroll_offset: usize) {
        self.scroll_offset = scroll_offset
    }

    #[cfg(test)]
    pub(crate) fn set_language(
        &mut self,
        language: shared::language::Language,
    ) -> anyhow::Result<()> {
        self.buffer_mut().set_language(language)
    }

    pub(crate) fn render_area(&self, context: &Context) -> Dimension {
        let Dimension { height, width } = self.dimension();
        Dimension {
            height: height.saturating_sub(self.window_title_height(context)),
            width,
        }
    }

    #[cfg(test)]
    pub(crate) fn apply_syntax_highlighting(
        &mut self,
        context: &mut Context,
    ) -> anyhow::Result<()> {
        let source_code = self.text();
        let mut buffer = self.buffer_mut();
        if let Some(language) = buffer.language() {
            let highlighted_spans = context.highlight(language, &source_code)?;
            buffer.update_highlighted_spans(Default::default(), highlighted_spans);
        }
        Ok(())
    }

    pub(crate) fn apply_dispatches(
        &mut self,
        context: &mut Context,
        dispatches: Vec<DispatchEditor>,
    ) -> anyhow::Result<Dispatches> {
        let mut result = Vec::new();
        for dispatch in dispatches {
            result.extend(self.handle_dispatch_editor(context, dispatch)?.into_vec());
        }
        Ok(result.into())
    }

    fn enter_swap_mode(&mut self) {
        self.mode = Mode::Swap
    }

    fn kill_line(
        &mut self,
        direction: Direction,
        context: &Context,
    ) -> Result<Dispatches, anyhow::Error> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let buffer = self.buffer();
                    let cursor = selection.get_anchor(&self.cursor_direction);
                    let line_range = buffer.get_line_range_by_char_index(cursor)?;
                    let (delete_range, cursor_start) = match direction {
                        Direction::Start => {
                            let start = line_range.start();
                            let range = (start..cursor).into();
                            let slice = buffer.slice(&range)?.to_string();
                            let start = if slice.is_empty() { start - 1 } else { start };
                            let range = (start..cursor).into();
                            (range, start)
                        }
                        Direction::End => {
                            let range = (cursor..line_range.end()).into();
                            let slice = buffer.slice(&range)?.to_string();
                            let range = if slice == "\n" || range.end.0 == buffer.len_chars() {
                                range
                            } else {
                                (cursor..(line_range.end() - 1)).into()
                            };
                            (range, cursor)
                        }
                    };
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit::new(
                                self.buffer().rope(),
                                delete_range,
                                Rope::new(),
                            )),
                            Action::Select(
                                selection
                                    .clone()
                                    .set_range((cursor_start..cursor_start).into()),
                            ),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect_vec(),
        );
        let dispatches = self
            .apply_edit_transaction(edit_transaction, context)?
            .chain(self.enter_insert_mode(Direction::Start, context)?);
        Ok(dispatches)
    }

    fn enter_replace_mode(&mut self) {
        self.mode = Mode::Replace
    }

    pub(crate) fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub(crate) fn set_regex_highlight_rules(
        &mut self,
        regex_highlight_rules: Vec<RegexHighlightRule>,
    ) {
        self.regex_highlight_rules = regex_highlight_rules
    }

    fn go_back(&mut self, context: &Context) {
        let selection_set = self.buffer_mut().previous_selection_set();
        if let Some(selection_set) = selection_set {
            self.set_selection_set(selection_set, context)
        }
    }

    fn go_forward(&mut self, context: &Context) {
        let selection_set = self.buffer_mut().next_selection_set();
        if let Some(selection_set) = selection_set {
            self.set_selection_set(selection_set, context)
        }
    }

    pub(crate) fn set_selection_set(&mut self, selection_set: SelectionSet, context: &Context) {
        self.selection_set = selection_set;
        self.recalculate_scroll_offset(context)
    }

    pub(crate) fn set_char_index_range(
        &mut self,
        range: CharIndexRange,
        context: &Context,
    ) -> Result<Dispatches, anyhow::Error> {
        let selection_set = self.char_index_range_to_selection_set(range)?;
        Ok(self.update_selection_set(selection_set, true, context))
    }

    fn select_surround(
        &mut self,
        enclosure: EnclosureKind,
        kind: SurroundKind,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        self.disable_selection_extension();
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let buffer = self.buffer();
                    let cursor_char_index = selection.get_anchor(&self.cursor_direction);
                    if let Some((open_index, close_index)) =
                        crate::surround::get_surrounding_indices(
                            &buffer.content(),
                            enclosure,
                            cursor_char_index,
                            false,
                        )
                    {
                        let offset = match kind {
                            SurroundKind::Inside => 1,
                            SurroundKind::Around => 0,
                        };
                        let range = ((open_index + offset)..(close_index + 1 - offset)).into();
                        Ok(ActionGroup::new(
                            [Action::Select(selection.clone().set_range(range))].to_vec(),
                        ))
                    } else {
                        Ok(ActionGroup::new(Default::default()))
                    }
                })
                .into_iter()
                .flatten()
                .collect_vec(),
        );
        let _ = self.set_selection_mode(
            IfCurrentNotFound::LookForward,
            SelectionMode::Custom,
            context,
            None,
        );
        self.disable_selection_extension();
        self.apply_edit_transaction(edit_transaction, context)
    }

    fn delete_surround(
        &mut self,
        enclosure: EnclosureKind,
        context: &Context,
    ) -> Result<Dispatches, anyhow::Error> {
        self.change_surround(enclosure, None, context)
    }

    fn change_surround(
        &mut self,
        from: EnclosureKind,
        to: Option<EnclosureKind>,
        context: &Context,
    ) -> Result<Dispatches, anyhow::Error> {
        self.disable_selection_extension();
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let buffer = self.buffer();
                    let cursor_char_index = selection.get_anchor(&self.cursor_direction);
                    if let Some((open_index, close_index)) =
                        crate::surround::get_surrounding_indices(
                            &buffer.content(),
                            from,
                            cursor_char_index,
                            true,
                        )
                    {
                        let open_range = (open_index..open_index + 1).into();
                        let close_range = (close_index..close_index + 1).into();
                        let (new_open, new_close) = to
                            .as_ref()
                            .map(|to| to.open_close_symbols_str())
                            .unwrap_or(("", ""));
                        let select_range = (open_index + 1 - new_open.chars().count()
                            ..(close_index + new_close.chars().count()))
                            .into();
                        Ok([
                            ActionGroup::new(
                                [Action::Edit(Edit::new(
                                    self.buffer().rope(),
                                    open_range,
                                    new_open.into(),
                                ))]
                                .to_vec(),
                            ),
                            ActionGroup::new(
                                [Action::Edit(Edit::new(
                                    self.buffer().rope(),
                                    close_range,
                                    new_close.into(),
                                ))]
                                .to_vec(),
                            ),
                            ActionGroup::new(
                                [Action::Select(selection.clone().set_range(select_range))]
                                    .to_vec(),
                            ),
                        ]
                        .to_vec())
                    } else {
                        Ok(Default::default())
                    }
                })
                .into_iter()
                .flatten()
                .flatten()
                .collect_vec(),
        );
        let _ = self.set_selection_mode(
            IfCurrentNotFound::LookForward,
            SelectionMode::Custom,
            context,
            None,
        );
        self.apply_edit_transaction(edit_transaction, context)
    }

    fn replace_with_pattern(&mut self, context: &Context) -> Result<Dispatches, anyhow::Error> {
        let config = context.local_search_config(Scope::Local);
        match config.mode {
            LocalSearchConfigMode::AstGrep => {
                let edits = if let Some(language) = self.buffer().treesitter_language() {
                    selection_mode::AstGrep::replace(
                        language,
                        &self.content(),
                        &config.search(),
                        &config.replacement(),
                    )?
                } else {
                    Default::default()
                };
                let edit_transaction = EditTransaction::from_action_groups(
                    self.selection_set
                        .map(|selection| -> anyhow::Result<_> {
                            let byte_range = self
                                .buffer()
                                .char_index_range_to_byte_range(selection.extended_range())?;
                            if let Some((range, new)) = edits.iter().find_map(|edit| {
                                let new: Rope =
                                    String::from_utf8(edit.inserted_text.clone()).ok()?.into();
                                let range = edit.position..edit.position + edit.deleted_length;
                                if byte_range == range {
                                    Some((range, new))
                                } else {
                                    None
                                }
                            }) {
                                let range = self.buffer().byte_range_to_char_index_range(&range)?;
                                let new_len_chars = new.len_chars();
                                Ok(ActionGroup::new(
                                    [
                                        Action::Edit(Edit::new(self.buffer().rope(), range, new)),
                                        Action::Select(selection.clone().set_range(
                                            (range.start..range.start + new_len_chars).into(),
                                        )),
                                    ]
                                    .to_vec(),
                                ))
                            } else {
                                Ok(ActionGroup::new(Default::default()))
                            }
                        })
                        .into_iter()
                        .flatten()
                        .collect_vec(),
                );
                self.apply_edit_transaction(edit_transaction, context)
            }
            LocalSearchConfigMode::Regex(regex_config) => self.transform_selection(
                Transformation::RegexReplace {
                    regex: MyRegex(regex_config.to_regex(&config.search())?),
                    replacement: config.replacement(),
                },
                context,
            ),
            LocalSearchConfigMode::NamingConventionAgnostic => self.transform_selection(
                Transformation::NamingConventionAgnosticReplace {
                    search: config.search(),
                    replacement: config.replacement(),
                },
                context,
            ),
        }
    }

    #[cfg(test)]
    pub(crate) fn copied_text_history_offset(&self) -> isize {
        self.copied_text_history_offset.value()
    }

    fn hidden_parent_line_ranges(&self) -> anyhow::Result<Vec<Range<usize>>> {
        let (hidden_parent_lines, _) = self.get_parent_lines()?;
        Ok(hidden_parent_lines
            .into_iter()
            .map(|line| (line.line..line.line + 1))
            .collect_vec())
    }

    fn move_to_last_char(&mut self, context: &Context) -> Dispatches {
        let last_cursor_index = CharIndex(self.buffer().len_chars());
        self.update_selection_set(
            SelectionSet::new(NonEmpty::singleton(
                Selection::default().set_range((last_cursor_index..last_cursor_index).into()),
            )),
            false,
            context,
        )
        .append(Dispatch::ToEditor(EnterInsertMode(Direction::Start)))
    }

    fn pipe_to_shell(
        &mut self,
        command: String,
        context: &Context,
    ) -> Result<Dispatches, anyhow::Error> {
        self.transform_selection(Transformation::PipeToShell { command }, context)
    }

    fn show_current_tree_sitter_node_sexp(&self) -> Result<Dispatches, anyhow::Error> {
        let buffer = self.buffer();
        let node = buffer.get_current_node(self.selection_set.primary_selection(), false)?;
        let info = node
            .map(|node| node.to_sexp())
            .unwrap_or("[No node found]".to_string());
        Ok(Dispatches::one(Dispatch::ShowGlobalInfo(Info::new(
            "Tree-sitter node S-expression".to_string(),
            info,
        ))))
    }

    fn indent(&mut self, context: &Context) -> Result<Dispatches, anyhow::Error> {
        let indentation: Rope = std::iter::repeat_n(INDENT_CHAR, INDENT_WIDTH)
            .collect::<String>()
            .into();
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let original_range = selection.extended_range();
                    let line_range = self
                        .buffer()
                        .char_index_range_to_line_range(original_range)?;
                    let linewise_range = self
                        .buffer()
                        .line_range_to_full_char_index_range(line_range.clone())?;
                    let content = self.buffer().slice(&linewise_range)?;
                    let modified_lines = content
                        .lines()
                        .filter(|line| line.len_chars() > 0)
                        .map(|line| {
                            (
                                indentation.len_chars() as isize,
                                format!("{indentation}{line}"),
                            )
                        })
                        .collect_vec();
                    let length_changes = modified_lines
                        .iter()
                        .map(|(length_change, _)| *length_change)
                        .collect_vec();
                    let length_change: isize = length_changes.into_iter().sum();
                    let new: Rope = modified_lines
                        .into_iter()
                        .map(|(_, line)| line)
                        .join("")
                        .into();
                    let select_range = {
                        let offset: isize = INDENT_WIDTH as isize;
                        let start = original_range.start.apply_offset(offset);
                        let original_len = original_range.len();
                        let end =
                            (start + original_len + length_change as usize).apply_offset(-offset);
                        start..end
                    };

                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit::new(self.buffer().rope(), linewise_range, new)),
                            Action::Select(selection.clone().set_range(select_range.into())),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect_vec(),
        );
        self.apply_edit_transaction(edit_transaction, context)
    }

    fn dedent(&mut self, context: &Context) -> Result<Dispatches, anyhow::Error> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let original_range = selection.extended_range();
                    let line_range = self
                        .buffer()
                        .char_index_range_to_line_range(original_range)?;
                    let linewise_range = self
                        .buffer()
                        .line_range_to_full_char_index_range(line_range.clone())?;
                    let content = self.buffer().slice(&linewise_range)?;
                    let get_remove_leading_char_count = |line: &str| {
                        let leading_indent_count =
                            line.chars().take_while(|c| c == &INDENT_CHAR).count();
                        leading_indent_count.min(INDENT_WIDTH)
                    };
                    let modified_lines = content
                        .lines()
                        .filter(|line| line.len_chars() > 0)
                        .map(|line| {
                            let remove_leading_char_count =
                                get_remove_leading_char_count(&line.to_string());
                            (
                                -(remove_leading_char_count as isize),
                                line.slice(remove_leading_char_count..).to_string(),
                            )
                        })
                        .collect_vec();
                    let length_changes = modified_lines
                        .iter()
                        .map(|(length_change, _)| *length_change)
                        .collect_vec();
                    let first_length_change = length_changes.first().cloned().unwrap_or_default();
                    let length_change: isize = length_changes.into_iter().sum();
                    let new: Rope = modified_lines
                        .into_iter()
                        .map(|(_, line)| line)
                        .join("")
                        .into();
                    let select_range = {
                        let offset: isize = first_length_change;
                        let start = original_range.start.apply_offset(offset);
                        let original_len = original_range.len();

                        let end = if line_range.len() <= 1 {
                            start + original_len
                        } else {
                            (start
                                + original_len
                                    .saturating_sub(length_change.unsigned_abs())
                                    .max(1))
                            .apply_offset(-offset)
                        };
                        start..end
                    };
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit::new(self.buffer().rope(), linewise_range, new)),
                            Action::Select(selection.clone().set_range(select_range.into())),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect_vec(),
        );
        self.apply_edit_transaction(edit_transaction, context)
    }

    #[cfg(test)]
    pub(crate) fn primary_selection(&self) -> anyhow::Result<String> {
        Ok(self
            .buffer()
            .slice(
                &self
                    .editor()
                    .selection_set
                    .primary_selection()
                    .extended_range(),
            )?
            .to_string())
    }

    fn cycle_primary_selection(&mut self, direction: Direction) {
        self.selection_set.cycle_primary_selection(direction)
    }

    fn handle_dispatch_editors(
        &mut self,
        context: &mut Context,
        dispatch_editors: Vec<DispatchEditor>,
    ) -> Result<Dispatches, anyhow::Error> {
        Ok(Dispatches::new(
            dispatch_editors
                .into_iter()
                .map(|dispatch_editor| self.handle_dispatch_editor(context, dispatch_editor))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flat_map(|dispatches| dispatches.into_vec())
                .collect(),
        ))
    }

    fn collapse_selection(
        &mut self,
        context: &mut Context,
        direction: Direction,
    ) -> anyhow::Result<Dispatches> {
        let set_column_selection_mode =
            SetSelectionMode(IfCurrentNotFound::LookForward, SelectionMode::Character);
        match direction {
            Direction::Start => self.handle_dispatch_editor(context, set_column_selection_mode),
            Direction::End => self
                .handle_dispatch_editors(context, [SwapCursor, set_column_selection_mode].to_vec()),
        }
    }

    fn filter_selection_matching_search(
        &mut self,
        local_search_config: &crate::context::LocalSearchConfig,
        keep: bool,
        context: &Context,
    ) -> Dispatches {
        let search = local_search_config.search();
        let selections = self.selection_set.selections();
        let filtered = selections
            .iter()
            .filter_map(|selection| -> Option<_> {
                let range = selection.extended_range();
                let haystack = self.buffer().slice(&range).unwrap_or_default().to_string();
                let is_match = match local_search_config.mode {
                    LocalSearchConfigMode::Regex(regex_config) => get_regex(&search, regex_config)
                        .ok()?
                        .is_match(&haystack)
                        .ok()?,
                    LocalSearchConfigMode::AstGrep => false,
                    LocalSearchConfigMode::NamingConventionAgnostic => {
                        selection_mode::NamingConventionAgnostic::new(search.clone())
                            .find_all(&haystack)
                            .is_empty()
                            .not()
                    }
                };
                if keep && is_match || !keep && !is_match {
                    Some(selection.clone())
                } else {
                    None
                }
            })
            .collect_vec();
        let selections = match filtered.split_first() {
            Some((head, tail)) => NonEmpty {
                head: head.clone(),
                tail: tail.to_vec(),
            },
            None => selections.clone(),
        };
        self.update_selection_set(
            self.selection_set.clone().set_selections(selections),
            true,
            context,
        )
    }

    fn delete_current_cursor(&mut self, direction: Direction) {
        self.selection_set.delete_current_selection(direction)
    }

    fn break_selection(&mut self, context: &Context) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups({
            let buffer = self.buffer();
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let select_range = selection.extended_range();

                    let line_index = buffer.char_to_line(select_range.start)?;
                    let line_char_index = buffer.line_to_char(line_index)?;
                    let indentation: String = {
                        let line = buffer
                            .get_line_by_line_index(line_index)
                            .map(|slice| slice.to_string())
                            .unwrap_or_default();
                        line.chars().take_while(|c| c.is_whitespace()).collect()
                    };
                    let current = buffer
                        .slice(&select_range)
                        .map(|slice| slice.to_string())
                        .unwrap_or_default();

                    // The edit range should include the leading whitespaces of the current selection
                    // if the current selection is not befored by purely whitespaces
                    let edit_range = {
                        let line_leading_content = buffer
                            .slice(&(line_char_index..select_range.start).into())?
                            .to_string();

                        let leading_whitespaces_count = line_leading_content
                            .chars()
                            .rev()
                            .take_while(|c| c.is_whitespace())
                            .count();

                        if leading_whitespaces_count < line_leading_content.chars().count() {
                            ((select_range.start - leading_whitespaces_count)..select_range.end)
                                .into()
                        } else {
                            select_range
                        }
                    };

                    Ok([
                        ActionGroup::new(
                            [Action::Edit(Edit::new(
                                self.buffer().rope(),
                                edit_range,
                                format!("\n{indentation}{current}").into(),
                            ))]
                            .to_vec(),
                        ),
                        ActionGroup::new(
                            [Action::Select(
                                selection
                                    .clone()
                                    .set_range(select_range)
                                    .set_initial_range(None),
                            )]
                            .to_vec(),
                        ),
                    ])
                })
                .into_iter()
                .flatten()
                .flatten()
                .collect()
        });
        self.apply_edit_transaction(edit_transaction, context)
    }

    pub(crate) fn insert_mode_keymaps(
        &self,
        include_universal_keymaps: bool,
        context: &Context,
    ) -> super::keymap_legend::Keymaps {
        self.insert_mode_keymap_legend_config(include_universal_keymaps, context)
            .keymaps()
    }

    pub(crate) fn set_normal_mode_override(&mut self, normal_mode_override: NormalModeOverride) {
        self.normal_mode_override = Some(normal_mode_override)
    }

    fn show_help(&self, context: &Context) -> Result<Dispatches, anyhow::Error> {
        Ok(Dispatches::one(Dispatch::ShowKeymapLegend(
            self.get_current_keymap_legend_config(context),
        )))
    }

    fn get_current_keymap_legend_config(
        &self,
        context: &Context,
    ) -> super::keymap_legend::KeymapLegendConfig {
        match self.mode {
            Mode::Insert => self.insert_mode_keymap_legend_config(true, context),
            _ => self.normal_mode_keymap_legend_config(context, None, None),
        }
    }

    fn toggle_reveal(&mut self, reveal: Reveal) {
        self.reveal = match &self.reveal {
            Some(current_reveal) if &reveal == current_reveal => None,
            _ => Some(reveal),
        }
    }

    pub(crate) fn reveal(&self) -> std::option::Option<Reveal> {
        self.reveal.clone()
    }

    #[cfg(test)]
    pub(crate) fn selection_extension_enabled(&self) -> bool {
        self.selection_set.is_extended()
    }

    pub(crate) fn current_primary_selection(&self) -> anyhow::Result<String> {
        Ok(self
            .buffer()
            .slice(&self.selection_set.primary_selection().extended_range())?
            .to_string())
    }

    fn search_current_selection(
        &mut self,
        if_current_not_found: IfCurrentNotFound,
        scope: Scope,
    ) -> Dispatches {
        self.current_primary_selection()
            .map(|search| self.search_for_content(if_current_not_found, scope, search.to_string()))
            .unwrap_or_default()
    }

    fn search_clipboard_content(&mut self, scope: Scope, context: &Context) -> Dispatches {
        context
            .get_clipboard_content(0)
            .map(|copied_texts| {
                self.search_for_content(
                    self.cursor_direction.reverse().to_if_current_not_found(),
                    scope,
                    copied_texts.get(0),
                )
            })
            .unwrap_or_default()
    }

    fn search_for_content(
        &mut self,
        if_current_not_found: IfCurrentNotFound,
        scope: Scope,
        content: String,
    ) -> Dispatches {
        let dispatches = Dispatches::one(Dispatch::UpdateLocalSearchConfig {
            scope,
            if_current_not_found,
            update: crate::app::LocalSearchConfigUpdate::Config(
                LocalSearchConfig::new(LocalSearchConfigMode::Regex(RegexConfig::literal()))
                    .set_search(content.to_string())
                    .clone(),
            ),
            run_search_after_config_updated: true,
            component_id: None,
        })
        .append(Dispatch::PushPromptHistory {
            key: super::prompt::PromptHistoryKey::Search,
            line: format!("l/{}", content.replace("/", r#"\/"#)),
        });

        self.disable_selection_extension();
        dispatches
    }

    pub(crate) fn current_selection_range(&self) -> CharIndexRange {
        self.selection_set.primary_selection().extended_range()
    }

    pub(crate) fn window_title_height(&self, context: &Context) -> usize {
        self.title(context).lines().count()
    }

    fn execute_completion(
        &mut self,
        replacement: String,
        edit: Option<CompletionItemEdit>,
        context: &Context,
    ) -> Result<Dispatches, anyhow::Error> {
        // Only apply `edit` if there's no more than one cursor
        match edit {
            Some(edit) if self.selection_set.len() == 1 => self.apply_positional_edits(
                Some(edit)
                    .into_iter()
                    .map(|edit| match edit {
                        CompletionItemEdit::PositionalEdit(positional_edit) => positional_edit,
                    })
                    .collect_vec(),
                context,
            ),
            // Otherwise, replace word under cursor(s) with replacement
            _ => self.try_replace_current_long_word(replacement, context),
        }
    }

    fn last_visible_line(&self, context: &Context) -> usize {
        (self.render_area(context).height + self.scroll_offset).saturating_sub(1)
    }

    pub(crate) fn update_current_line(
        &mut self,
        context: &Context,
        replacement: &str,
    ) -> Result<Dispatches, anyhow::Error> {
        let current_line_index = self.buffer().char_to_line(
            self.selection_set
                .primary_selection()
                .range()
                .as_char_index(&self.cursor_direction),
        )?;
        let edit_transaction = EditTransaction::from_action_groups({
            let line_char_index = self.buffer().line_to_char(current_line_index)?;
            let line_length = self
                .buffer()
                .get_line_by_line_index(current_line_index)
                .ok_or_else(|| {
                    anyhow::anyhow!("Unable to get line at line index {current_line_index:?}")
                })?
                .len_chars();

            [ActionGroup::new(
                [Action::Edit(Edit::new(
                    self.buffer().rope(),
                    (line_char_index..line_char_index + line_length).into(),
                    replacement.into(),
                ))]
                .to_vec(),
            )]
            .to_vec()
        });
        self.apply_edit_transaction(edit_transaction, context)
    }

    pub(crate) fn set_visible_line_ranges(&mut self, visible_line_ranges: Vec<Range<usize>>) {
        let max_line_index = self.buffer().len_lines().saturating_sub(1);
        self.visible_line_ranges = Some(
            visible_line_ranges
                .into_iter()
                .map(|range| {
                    // Clamp range.end to buffer bounds because the VS Code adapter may send out-of-bounds ranges.
                    // VS Code tends to under-report visible line ranges, so our adapter adds safety padding
                    // to ensure we don't miss any visible content. This padding can cause the reported ranges
                    // to extend beyond the actual buffer size. For example, if the buffer has 100 lines
                    // (indices 0-99), the adapter might report a visible range like 95..105.
                    let end = range.end.min(max_line_index);
                    range.start..(end + 1)
                })
                .collect(),
        )
    }

    fn dispatch_jumps_changed(&self) -> Dispatch {
        Dispatch::ToHostApp(ToHostApp::JumpsChanged {
            component_id: self.id(),
            jumps: self
                .jumps
                .as_ref()
                .map(|jumps| {
                    jumps
                        .iter()
                        .map(|jump| (jump.character, jump.selection.range.start))
                        .collect_vec()
                })
                .unwrap_or_default(),
        })
    }

    pub(crate) fn dispatch_selection_mode_changed(&self) -> Dispatch {
        Dispatch::ToHostApp(ToHostApp::SelectionModeChanged(
            self.selection_set.mode().clone(),
        ))
    }

    pub(crate) fn update_content(
        &mut self,
        new_content: &str,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        let current_selection_set = self.selection_set.clone();
        let last_visible_line = self.last_visible_line(context);
        self.buffer_mut()
            .update_content(new_content, current_selection_set, last_visible_line)
    }

    fn toggle_line_comment(&mut self, context: &Context) -> anyhow::Result<Dispatches> {
        let Some(prefix) = self
            .buffer()
            .language()
            .and_then(|langugae| langugae.line_comment_prefix())
        else {
            return Ok(Default::default());
        };
        self.transform_selection(Transformation::ToggleLineComment { prefix }, context)
    }

    fn toggle_block_comment(&mut self, context: &Context) -> anyhow::Result<Dispatches> {
        let Some((open, close)) = self
            .buffer()
            .language()
            .and_then(|langugae| langugae.block_comment_affixes())
        else {
            return Ok(Default::default());
        };
        self.transform_selection(Transformation::ToggleBlockComment { open, close }, context)
    }

    fn handle_movement_with_prior_change(
        &mut self,
        context: &mut Context,
        movement: Movement,
        prior_change: Option<PriorChange>,
    ) -> Result<Dispatches, anyhow::Error> {
        self.handle_prior_change(prior_change);
        self.handle_movement(context, movement)
    }

    pub(crate) fn handle_prior_change(&mut self, prior_change: Option<PriorChange>) {
        if let Some(prior_change) = prior_change {
            match prior_change {
                PriorChange::EnterMultiCursorMode => self.mode = Mode::MultiCursor,
                PriorChange::EnableSelectionExtension => self.enable_selection_extension(),
            }
        }
    }

    fn repeat_search(
        &mut self,
        context: &Context,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        prior_change: Option<PriorChange>,
    ) -> anyhow::Result<Dispatches> {
        let Some(search) = context.local_search_config(scope).last_search() else {
            return Ok(Dispatches::default());
        };
        self.handle_prior_change(prior_change);
        let dispatches = Dispatches::one(Dispatch::UpdateLocalSearchConfig {
            scope,
            if_current_not_found,
            update: crate::app::LocalSearchConfigUpdate::Config(
                LocalSearchConfig::new(search.mode)
                    .set_search(search.search)
                    .clone(),
            ),
            run_search_after_config_updated: true,
            component_id: None,
        });
        Ok(dispatches)
    }

    fn revert_hunk(
        &mut self,
        context: &Context,
        diff_mode: DiffMode,
    ) -> anyhow::Result<Dispatches> {
        let Some(path) = self.buffer().path() else {
            return Ok(Default::default());
        };
        let hunks = path.simple_hunks(
            &self.buffer().content(),
            &diff_mode,
            context.current_working_directory(),
        )?;
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let buffer = self.buffer();
                    let line_range = buffer.char_index_range_to_line_range(selection.range())?;
                    let Some(matching_hunk) = hunks.iter().find(|hunk| {
                        range_intersects(&hunk.new_line_range, &line_range)
                            || hunk.new_line_range.start == line_range.start
                    }) else {
                        // Do nothing if this selection does not intersect with any hunks
                        return Ok(ActionGroup::new(
                            [Action::Select(selection.clone())].to_vec(),
                        ));
                    };

                    let edit_range = {
                        let start = buffer.line_to_char(matching_hunk.new_line_range.start)?;
                        (start..start + Rope::from_str(&matching_hunk.new_content).len_chars())
                            .into()
                    };
                    let replacement: Rope = matching_hunk.old_content.clone().into();

                    // If the hunk is a removed hunk, we need to append a newline char
                    let replacement = if matching_hunk.kind == SimpleHunkKind::Delete {
                        format!("{}\n", matching_hunk.old_content).into()
                    } else {
                        replacement
                    };
                    let select_range = {
                        let start = buffer.line_to_char(matching_hunk.new_line_range.start)?;
                        (start..start + replacement.len_chars()).into()
                    };
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit::new(self.buffer().rope(), edit_range, replacement)),
                            Action::Select(selection.clone().set_range(select_range)),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect_vec(),
        );
        self.apply_edit_transaction(edit_transaction, context)
    }

    fn git_blame(&self, context: &mut Context) -> Result<Dispatches, anyhow::Error> {
        let Some(file_path) = self.buffer().path() else {
            return Ok(Default::default());
        };
        let line_numbers: Vec<usize> = self
            .selection_set
            .map(|selection| self.buffer().char_to_line(selection.range.start))
            .into_iter()
            .try_collect()?;

        let git_blame_infos: Vec<_> = line_numbers
            .into_iter()
            .map(|line_number| {
                crate::git::blame::blame_line(
                    context.current_working_directory(),
                    &file_path,
                    line_number,
                )
            })
            .try_collect()?;

        let info = git_blame_infos
            .into_iter()
            .map(|info| info.display())
            .join("\n==========\n");

        Ok(Dispatches::one(Dispatch::ShowEditorInfo(Info::new(
            "Git blame".to_string(),
            info,
        ))))
    }

    fn merge_content(
        &mut self,
        context: &Context,
        file_path: CanonicalizedPath,
        content_editor: String,
        content_filesystem: String,
    ) -> anyhow::Result<Dispatches> {
        let original = file_path
            .content_at_last_commit(
                &DiffMode::UnstagedAgainstCurrentBranch,
                &GitRepo::try_from(context.current_working_directory())?,
            )
            .unwrap_or_default();
        let merged = match diffy::merge(&original, &content_editor, &content_filesystem) {
            Ok(merged_without_conflicts) => merged_without_conflicts,
            Err(merged_with_conflicts) => merged_with_conflicts,
        };
        let dispatches = self.update_content(&merged, context)?;
        Ok(dispatches.chain(self.do_save(true, context)?))
    }

    fn reload(&mut self, force: bool) -> Result<Dispatches, anyhow::Error> {
        let dispatches = self.buffer_mut().reload(force)?;
        Ok(dispatches.chain(self.get_document_did_change_dispatch()))
    }

    fn clear_incremental_search_matches(&mut self) {
        self.incremental_search_matches = None
    }

    pub(crate) fn set_incremental_search_config(&mut self, config: LocalSearchConfig) {
        let content = self.content();
        let search = config.search();
        if search.is_empty() {
            // Don't set incremental search matches if the search string is empty
            // otherwise it will cause performance issue with rendering
            // as empty string matches every character of the file.
            return;
        }
        let matches = match config.mode {
            LocalSearchConfigMode::Regex(regex_config) => regex_config
                .to_regex(&search)
                .map(|regex| {
                    regex
                        .find_iter(&content)
                        .filter_map(|m| Some(m.ok()?.range()))
                        .collect_vec()
                })
                .unwrap_or_default(),
            LocalSearchConfigMode::AstGrep => ast_grep::AstGrep::new(&self.buffer(), &search)
                .map(|result| result.find_all().map(|m| m.range()).collect_vec())
                .unwrap_or_default(),
            LocalSearchConfigMode::NamingConventionAgnostic => {
                NamingConventionAgnostic::new(search)
                    .find_all(&content)
                    .into_iter()
                    .map(|(range, _)| range.range().clone())
                    .collect_vec()
            }
        };
        self.incremental_search_matches = Some(matches)
    }

    pub(crate) fn initialize_incremental_search_matches(&mut self) {
        self.incremental_search_matches = Some(Vec::new())
    }

    fn go_to_file(&self) -> Result<Dispatches, anyhow::Error> {
        Ok(Dispatches::one(Dispatch::OpenFile {
            path: self.current_primary_selection()?.try_into()?,
            owner: crate::buffer::BufferOwner::User,
            focus: true,
        }))
    }

    fn press_space(&self, context: &Context) -> Dispatches {
        match self.mode {
            Mode::Normal => Dispatches::one(Dispatch::ShowKeymapLegend(
                self.space_keymap_legend_config(context),
            )),
            Mode::Insert => Dispatches::default(),
            _ => Dispatches::one(Dispatch::ToEditor(EnterNormalMode)),
        }
    }

    fn handle_path_renamed(&mut self, source: PathBuf, destination: CanonicalizedPath) {
        let Some(path) = self.path() else { return };
        if path.to_path_buf() == &source {
            self.buffer_mut().update_path(destination)
        }
    }

    fn copy_current_file_absolute_path(&self) -> Result<Dispatches, anyhow::Error> {
        if let Some(path) = self.path() {
            Ok(Dispatches::one(Dispatch::SetClipboardContent {
                copied_texts: CopiedTexts::new(NonEmpty::new(path.display_absolute())),
            }))
        } else {
            Err(anyhow::anyhow!(
                "Failed to copy file path as the current buffer does not have a file path."
            ))
        }
    }

    fn copy_current_file_relative_path(
        &self,
        context: &Context,
    ) -> Result<Dispatches, anyhow::Error> {
        if let Some(path) = self.path() {
            Ok(Dispatches::one(Dispatch::SetClipboardContent {
                copied_texts: CopiedTexts::new(NonEmpty::new(
                    path.display_relative_to(context.current_working_directory())?,
                )),
            }))
        } else {
            Err(anyhow::anyhow!(
                "Failed to copy file path as the current buffer does not have a file path."
            ))
        }
    }

    fn align_selections(
        &mut self,
        direction: Direction,
        context: &Context,
    ) -> Result<Dispatches, anyhow::Error> {
        let max_column = self
            .selection_set
            .map(|selection| -> anyhow::Result<_> {
                let char_index = selection.to_char_index(&direction);
                let position = self.buffer().char_to_position(char_index)?;
                Ok(position.column)
            })
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .max()
            .ok_or_else(|| anyhow::anyhow!("Unable to obtain max column"))?;

        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let original_range = selection.extended_range();
                    let char_index = selection.to_char_index(&direction);
                    let position = self.buffer().char_to_position(char_index)?;
                    let extra_leading_whitespaces_count =
                        max_column.saturating_sub(position.column);
                    let content = self.buffer().slice(&original_range)?;
                    let new_content =
                        format!("{}{}", " ".repeat(extra_leading_whitespaces_count), content);
                    let select_range = original_range.shift_right(extra_leading_whitespaces_count);
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit::new(
                                self.buffer().rope(),
                                original_range,
                                new_content.into(),
                            )),
                            Action::Select(selection.clone().set_range(select_range)),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect_vec(),
        );
        self.apply_edit_transaction(edit_transaction, context)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub(crate) enum ViewAlignment {
    Top,
    Center,
    Bottom,
}

pub(crate) enum HandleEventResult {
    Handled(Dispatches),
    Ignored(KeyEvent),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DispatchEditor {
    Surround(String, String),
    #[cfg(test)]
    SetScrollOffset(usize),
    ShowJumps {
        use_current_selection_mode: bool,
        prior_change: Option<PriorChange>,
    },
    ScrollPageDown,
    ScrollPageUp,
    #[cfg(test)]
    AlignViewTop,
    #[cfg(test)]
    AlignViewBottom,
    #[cfg(test)]
    AlignViewCenter,
    Transform(Transformation),
    SetSelectionMode(IfCurrentNotFound, SelectionMode),
    SetSelectionModeWithPriorChange(IfCurrentNotFound, SelectionMode, Option<PriorChange>),
    Save,
    ForceSave,
    FindOneChar(IfCurrentNotFound),
    MoveSelection(Movement),
    /// This is used for initiating modes such as Multicursor and Extend.
    MoveSelectionWithPriorChange(Movement, Option<PriorChange>),
    SwitchViewAlignment,
    Copy,
    GoBack,
    GoForward,
    SelectAll,
    SetContent(String),
    SetDecorations(Vec<Decoration>),
    #[cfg(test)]
    SetRectangle(Rectangle),
    EnableSelectionExtension,
    DisableSelectionExtension,
    DeleteOne,
    Change,
    ChangeCut,
    EnterInsertMode(Direction),
    ReplaceWithCopiedText {
        cut: bool,
    },
    ReplaceWithPattern,
    SelectLine(Movement),
    Backspace,
    Insert(String),
    MoveToLineStart,
    MoveToLineEnd,
    #[cfg(test)]
    MatchLiteral(String),
    SelectSurround {
        enclosure: EnclosureKind,
        kind: SurroundKind,
    },
    Open,
    EnterNormalMode,
    EnterSwapMode,
    EnterReplaceMode,
    CursorAddToAllSelections,
    CyclePrimarySelection(Direction),
    CursorKeepPrimaryOnly,
    ReplacePattern {
        config: crate::context::LocalSearchConfig,
    },
    Undo,
    Redo,
    KillLine(Direction),
    #[cfg(test)]
    Reset,
    DeleteWordBackward {
        short: bool,
    },
    #[cfg(test)]
    SetLanguage(Box<shared::language::Language>),
    #[cfg(test)]
    ApplySyntaxHighlight,
    ReplaceCurrentSelectionWith(String),
    SelectLineAt(usize),
    Paste,
    PasteNoGap,
    SwapCursor,
    MoveCharacterBack,
    MoveCharacterForward,
    DeleteSurround(EnclosureKind),
    ChangeSurround {
        from: EnclosureKind,
        to: EnclosureKind,
    },
    Replace(Movement),
    ApplyPositionalEdits(Vec<CompletionItemEdit>),
    ReplaceWithPreviousCopiedText,
    ReplaceWithNextCopiedText,
    MoveToLastChar,
    PipeToShell {
        command: String,
    },
    ShowCurrentTreeSitterNodeSexp,
    Indent,
    Dedent,
    SwapExtensionAnchor,
    CollapseSelection(Direction),
    FilterSelectionMatchingSearch {
        search: String,
        maintain: bool,
    },
    EnterNewline,
    DeleteCurrentCursor(Direction),
    BreakSelection,
    ShowHelp,
    HandleEsc,
    ToggleReveal(Reveal),
    SearchCurrentSelection(IfCurrentNotFound, Scope),
    ExecuteCompletion {
        replacement: String,
        edit: Option<CompletionItemEdit>,
    },
    ToggleLineComment,
    ToggleBlockComment,
    ShowKeymapLegendExtend,
    RepeatSearch(Scope, IfCurrentNotFound, Option<PriorChange>),
    RevertHunk(DiffMode),
    GitBlame,
    ReloadFile {
        force: bool,
    },
    MergeContent {
        content_filesystem: String,
        content_editor: String,
        path: CanonicalizedPath,
    },
    ClearIncrementalSearchMatches,
    GoToFile,
    SearchClipboardContent(Scope),
    PressSpace,
    PathRenamed {
        source: PathBuf,
        destination: CanonicalizedPath,
    },
    CopyAbsolutePath,
    CopyRelativePath,
    DeleteWithMovement(Movement),
    EnterDeleteMode,
    AlignSelections(Direction),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub(crate) enum SurroundKind {
    Inside,
    Around,
}

impl std::fmt::Display for SurroundKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SurroundKind::Inside => write!(f, "Inside"),
            SurroundKind::Around => write!(f, "Outside"),
        }
    }
}

const INDENT_CHAR: char = ' ';
const INDENT_WIDTH: usize = 4;
