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

use nonempty::NonEmpty;
use shared::canonicalized_path::CanonicalizedPath;
use std::{
    cell::{Ref, RefCell, RefMut},
    ops::{Not, Range},
    rc::Rc,
};

use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use event::KeyEvent;
use itertools::{Either, Itertools};
use my_proc_macros::key;
use ropey::Rope;

use crate::{
    app::{Dimension, Dispatch},
    buffer::Buffer,
    components::component::Component,
    edit::{Action, ActionGroup, Edit, EditTransaction},
    lsp::completion::PositionalEdit,
    position::Position,
    rectangle::Rectangle,
    selection::{CharIndex, Selection, SelectionMode, SelectionSet},
};

use DispatchEditor::*;

use super::{
    component::ComponentId,
    dropdown::DropdownRender,
    render_editor::Source,
    suggestive_editor::{Decoration, Info},
};

#[derive(PartialEq, Clone, Debug, Eq)]
pub(crate) enum Mode {
    Normal,
    Insert,
    MultiCursor,
    FindOneChar(IfCurrentNotFound),
    Exchange,
    UndoTree,
    Replace,
    V,
}

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct Jump {
    pub(crate) character: char,
    pub(crate) selection: Selection,
}
const WINDOW_TITLE_HEIGHT: usize = 1;

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

    fn set_content(&mut self, str: &str) -> Result<(), anyhow::Error> {
        self.update_buffer(str);
        self.clamp()
    }

    fn title(&self, context: &Context) -> String {
        let title = self.title.clone();
        title
            .or_else(|| {
                let path = self.buffer().path()?;
                let current_working_directory = context.current_working_directory();
                let string = path
                    .display_relative_to(current_working_directory)
                    .unwrap_or_else(|_| path.display_absolute());
                let icon = path.icon();
                Some(format!(" {} {}", icon, string))
            })
            .unwrap_or_else(|| "[No title]".to_string())
    }

    fn set_title(&mut self, title: String) {
        self.title = Some(title);
    }

    fn handle_paste_event(&mut self, content: String) -> anyhow::Result<Dispatches> {
        self.insert(&content)
    }

    fn get_cursor_position(&self) -> anyhow::Result<Position> {
        self.buffer
            .borrow()
            .char_to_position(self.get_cursor_char_index())
    }

    fn set_rectangle(&mut self, rectangle: Rectangle) {
        self.rectangle = rectangle;
        self.recalculate_scroll_offset();
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
        const SCROLL_HEIGHT: usize = 1;
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
        match dispatch {
            #[cfg(test)]
            AlignViewTop => self.align_cursor_to_top(),
            #[cfg(test)]
            AlignViewBottom => self.align_cursor_to_bottom(),
            Transform(transformation) => return self.transform_selection(transformation),
            SetSelectionMode(if_current_not_found, selection_mode) => {
                return self.set_selection_mode(if_current_not_found, selection_mode);
            }

            FindOneChar(if_current_not_found) => {
                self.enter_single_character_mode(if_current_not_found)
            }

            MoveSelection(direction) => return self.handle_movement(context, direction),
            Copy {
                use_system_clipboard,
            } => return self.copy(use_system_clipboard),
            ReplaceWithCopiedText {
                cut,
                use_system_clipboard,
            } => return self.replace_with_copied_text(context, cut, use_system_clipboard, 0),
            SelectAll => return self.select_all(context),
            SetContent(content) => self.set_content(&content)?,
            EnableSelectionExtension => self.enable_selection_extension(),
            EnterVMode => self.enter_v_mode(),
            EnterUndoTreeMode => return Ok(self.enter_undo_tree_mode()),
            EnterInsertMode(direction) => return self.enter_insert_mode(direction),
            Delete(direction) => return self.delete(direction, None),
            Insert(string) => return self.insert(&string),
            #[cfg(test)]
            MatchLiteral(literal) => return self.match_literal(&literal),
            ToggleMark => self.toggle_marks(),
            EnterNormalMode => self.enter_normal_mode()?,
            CursorAddToAllSelections => self.add_cursor_to_all_selections()?,
            CursorKeepPrimaryOnly => self.cursor_keep_primary_only(),
            EnterExchangeMode => self.enter_exchange_mode(),
            ReplacePattern { config } => {
                let selection_set = self.selection_set.clone();
                let (_, selection_set) = self.buffer_mut().replace(config, selection_set)?;
                return Ok(self
                    .update_selection_set(selection_set, false)
                    .chain(self.get_document_did_change_dispatch()));
            }
            Undo => {
                let dispatches = self.undo();
                return dispatches;
            }
            KillLine(direction) => return self.kill_line(direction),
            #[cfg(test)]
            Reset => self.reset(),
            DeleteWordBackward { short } => return self.delete_word_backward(short),
            Backspace => return self.backspace(),
            MoveToLineStart => return self.move_to_line_start(),
            MoveToLineEnd => return self.move_to_line_end(),
            SelectLine(movement) => return self.select_line(movement),
            Redo => return self.redo(),
            Change => return self.change(),
            ChangeCut {
                use_system_clipboard,
            } => return self.change_cut(use_system_clipboard),
            #[cfg(test)]
            SetRectangle(rectangle) => self.set_rectangle(rectangle),
            ScrollPageDown => return self.scroll_page_down(),
            ScrollPageUp => return self.scroll_page_up(),
            ShowJumps {
                use_current_selection_mode,
            } => self.show_jumps(use_current_selection_mode)?,
            SwitchViewAlignment => self.switch_view_alignment(),
            #[cfg(test)]
            SetScrollOffset(n) => self.set_scroll_offset(n),
            #[cfg(test)]
            SetLanguage(language) => self.set_language(language)?,
            #[cfg(test)]
            ApplySyntaxHighlight => {
                self.apply_syntax_highlighting(context)?;
            }
            Save => return self.save(),
            ReplaceCurrentSelectionWith(string) => {
                return self.replace_current_selection_with(|_| Some(Rope::from_str(&string)))
            }
            SelectLineAt(index) => return Ok(self.select_line_at(index)?.into_vec().into()),
            EnterMultiCursorMode => self.enter_multicursor_mode(),
            Surround(open, close) => return self.enclose(open, close),
            ShowKeymapLegendInsertMode => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.insert_mode_keymap_legend_config(),
                )]
                .to_vec()
                .into())
            }
            ShowKeymapLegendHelp => {
                return Ok(
                    [Dispatch::ShowKeymapLegend(self.help_keymap_legend_config())]
                        .to_vec()
                        .into(),
                )
            }
            ShowKeymapLegendNormalMode => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.normal_mode_keymap_legend_config(context),
                )]
                .to_vec()
                .into())
            }
            EnterReplaceMode => self.enter_replace_mode(),
            Paste {
                direction,
                use_system_clipboard,
            } => return self.paste(direction, context, use_system_clipboard),
            SwapCursorWithAnchor => self.swap_cursor_with_anchor(),
            SetDecorations(decorations) => self.buffer_mut().set_decorations(&decorations),
            MoveCharacterBack => self.selection_set.move_left(&self.cursor_direction),
            MoveCharacterForward => {
                let len_chars = self.buffer().len_chars();
                self.selection_set
                    .move_right(&self.cursor_direction, len_chars)
            }
            Open(direction) => return self.open(direction),
            TryReplaceCurrentLongWord(replacement) => {
                return self.try_replace_current_long_word(replacement)
            }
            GoBack => self.go_back(),
            GoForward => self.go_forward(),
            SelectSurround { enclosure, kind } => return self.select_surround(enclosure, kind),
            DeleteSurround(enclosure) => return self.delete_surround(enclosure),
            ChangeSurround { from, to } => return self.change_surround(from, Some(to)),
            ReplaceWithPattern => return self.replace_with_pattern(context),
            Replace(movement) => return self.replace_with_movement(&movement),
            ApplyPositionalEdits(edits) => {
                return self.apply_positional_edits(
                    edits
                        .into_iter()
                        .map(|edit| match edit {
                            CompletionItemEdit::PositionalEdit(positional_edit) => positional_edit,
                        })
                        .collect_vec(),
                )
            }
            ReplaceWithPreviousCopiedText => {
                let history_offset = self.copied_text_history_offset.decrement();
                return self.replace_with_copied_text(context, false, false, history_offset);
            }
            ReplaceWithNextCopiedText => {
                let history_offset = self.copied_text_history_offset.increment();
                return self.replace_with_copied_text(context, false, false, history_offset);
            }
            MoveToLastChar => return Ok(self.move_to_last_char()),
            PipeToShell { command } => return self.pipe_to_shell(command),
            ShowCurrentTreeSitterNodeSexp => return self.show_current_tree_sitter_node_sexp(),
            Indent => return self.indent(),
            Dedent => return self.dedent(),
            CyclePrimarySelection(direction) => self.cycle_primary_selection(direction),
            SwapExtensionDirection => self.selection_set.swap_initial_range_direction(),
            CollapseSelection(direction) => return self.collapse_selection(context, direction),
            FilterSelectionMatchingSearch { keep, search } => {
                return Ok(self.filter_selection_matching_search(
                    context.get_local_search_config(Scope::Local),
                    search,
                    keep,
                ))
            }
        }
        Ok(Default::default())
    }
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
    scroll_offset: u16,
    rectangle: Rectangle,
    buffer: Rc<RefCell<Buffer>>,
    title: Option<String>,
    id: ComponentId,
    pub(crate) current_view_alignment: Option<ViewAlignment>,
    copied_text_history_offset: Counter,
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

    fn to_movement(&self) -> Movement {
        match self {
            Direction::Start => Movement::Previous,
            Direction::End => Movement::Next,
        }
    }

    pub(crate) fn format_action(&self, action: &str) -> String {
        match self {
            Direction::Start => format!("◁ {action}"),
            Direction::End => format!("{action} ▷"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub(crate) enum IfCurrentNotFound {
    LookForward,
    LookBackward,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Movement {
    Right,
    Left,
    Next,
    Previous,
    Last,
    Current(IfCurrentNotFound),
    Up,
    Down,
    First,
    /// 0-based
    Index(usize),
    Jump(CharIndexRange),
    ToParentLine,
    Expand,
    Shrink,
}

impl Editor {
    /// Returns (hidden_parent_lines, visible_parent_lines)
    pub(crate) fn get_parent_lines(&self) -> anyhow::Result<(Vec<Line>, Vec<Line>)> {
        let position = self.get_cursor_position()?;

        let parent_lines = self.buffer().get_parent_lines(position.line)?;
        Ok(parent_lines
            .into_iter()
            .partition(|line| line.line < self.scroll_offset as usize))
    }

    pub(crate) fn show_info(&mut self, info: Info) -> Result<(), anyhow::Error> {
        self.set_title(info.title());
        self.set_decorations(info.decorations());
        self.set_content(info.content())
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
        }
    }

    pub(crate) fn from_buffer(buffer: Rc<RefCell<Buffer>>) -> Self {
        Self {
            selection_set: SelectionSet::default(),
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
        }
    }

    pub(crate) fn current_line(&self) -> anyhow::Result<String> {
        let cursor = self.get_cursor_char_index();
        Ok(self
            .buffer
            .borrow()
            .get_line_by_char_index(cursor)?
            .to_string()
            .trim()
            .into())
    }

    pub(crate) fn get_current_word(&self) -> anyhow::Result<String> {
        let cursor = self.get_cursor_char_index();
        self.buffer.borrow().get_word_before_char_index(cursor)
    }

    pub(crate) fn select_line(&mut self, movement: Movement) -> anyhow::Result<Dispatches> {
        self.select(SelectionMode::Line, movement)
    }

    pub(crate) fn select_line_at(&mut self, line: usize) -> anyhow::Result<Dispatches> {
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

        Ok(self.update_selection_set(selection_set, false))
    }

    #[cfg(test)]
    pub(crate) fn reset(&mut self) {
        self.selection_set.escape_highlight_mode();
    }

    pub(crate) fn update_selection_set(
        &mut self,
        selection_set: SelectionSet,
        store_history: bool,
    ) -> Dispatches {
        let show_info = selection_set
            .map(|selection| selection.info())
            .into_iter()
            .flatten()
            .reduce(Info::join)
            .map(Dispatch::ShowEditorInfo);
        self.cursor_direction = Direction::Start;
        if store_history {
            self.buffer_mut()
                .push_selection_set_history(selection_set.clone());
        }
        self.set_selection_set(selection_set);
        Dispatches::default().append_some(show_info)
    }

    pub(crate) fn position_range_to_selection_set(
        &self,
        range: Range<Position>,
    ) -> anyhow::Result<SelectionSet> {
        let range = (self.buffer().position_to_char(range.start)?
            ..self.buffer().position_to_char(range.end)?)
            .into();

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

    fn cursor_row(&self) -> u16 {
        self.get_cursor_char_index()
            .to_position(&self.buffer.borrow())
            .line as u16
    }

    fn recalculate_scroll_offset(&mut self) {
        // Update scroll_offset if primary selection is out of view.
        let cursor_row = self.cursor_row();
        let render_area = self.render_area();
        if cursor_row.saturating_sub(self.scroll_offset) > render_area.height.saturating_sub(1)
            || cursor_row < self.scroll_offset
        {
            self.align_cursor_to_center();
            self.current_view_alignment = None;
        }
    }

    pub(crate) fn align_cursor_to_bottom(&mut self) {
        self.scroll_offset = self.cursor_row().saturating_sub(
            self.rectangle
                .height
                .saturating_sub(1)
                .saturating_sub(WINDOW_TITLE_HEIGHT as u16),
        );
    }

    pub(crate) fn align_cursor_to_top(&mut self) {
        self.scroll_offset = self.cursor_row();
    }

    fn align_cursor_to_center(&mut self) {
        self.scroll_offset = self
            .cursor_row()
            .saturating_sub((self.rectangle.height as f64 / 2.0).ceil() as u16);
    }

    pub(crate) fn select(
        &mut self,
        selection_mode: SelectionMode,
        movement: Movement,
    ) -> anyhow::Result<Dispatches> {
        //  There are a few selection modes where Current make sense.
        if let Some(selection_set) = self.get_selection_set(&selection_mode, movement)? {
            Ok(self.update_selection_set(selection_set, true))
        } else {
            Ok(Default::default())
        }
    }

    fn jump_characters() -> Vec<char> {
        ('a'..='z').chain('A'..='Z').chain('0'..='9').collect_vec()
    }

    pub(crate) fn get_selection_mode_trait_object(
        &self,
        selection: &Selection,
        use_current_selection_mode: bool,
    ) -> anyhow::Result<Box<dyn selection_mode::SelectionMode>> {
        if use_current_selection_mode {
            self.selection_set.mode.clone()
        } else {
            SelectionMode::Subword
        }
        .to_selection_mode_trait_object(&self.buffer(), selection, &self.cursor_direction)
    }

    fn jump_from_selection(
        &mut self,
        selection: &Selection,
        use_current_selection_mode: bool,
    ) -> anyhow::Result<()> {
        let chars = Self::jump_characters();

        let object = self.get_selection_mode_trait_object(selection, use_current_selection_mode)?;

        let line_ranges = Some(self.visible_line_range())
            .into_iter()
            .chain(self.hidden_parent_line_ranges()?)
            .collect_vec();
        let jumps = object.jumps(
            selection_mode::SelectionModeParams {
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

    pub(crate) fn show_jumps(&mut self, use_current_selection_mode: bool) -> anyhow::Result<()> {
        self.jump_from_selection(
            &self.selection_set.primary_selection().clone(),
            use_current_selection_mode,
        )
    }

    pub(crate) fn delete(
        &mut self,
        direction: Direction,
        use_system_clipboard: Option<bool>,
    ) -> anyhow::Result<Dispatches> {
        let copy_dispatches = if let Some(use_system_clipboard) = use_system_clipboard {
            self.copy(use_system_clipboard)?
        } else {
            Default::default()
        };
        let edit_transaction = EditTransaction::from_action_groups({
            let buffer = self.buffer();
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let current_range = selection.extended_range();
                    let default = {
                        let start = current_range.start;
                        (current_range, (start..start + 1).into())
                    };

                    let get_selection = |direction: &Direction| {
                        // The start selection is used for getting the next/previous selection
                        // It cannot be the extended selection, otherwise the next/previous selection
                        // will not be found
                        let start_selection =
                            &selection.clone().collapsed_to_anchor_range(direction);
                        Selection::get_selection_(
                            &buffer,
                            start_selection,
                            &self.selection_set.mode,
                            &direction.to_movement(),
                            &self.cursor_direction,
                        )
                        .ok()
                        .flatten()
                    };
                    let (delete_range, select_range) = {
                        if !self.selection_set.mode.is_contiguous() {
                            default
                        }
                        // If the selection mode is contiguous,
                        // perform a "kill next/previous" instead
                        else if let Some(other_selection) = get_selection(&direction)
                            .or_else(|| get_selection(&direction.reverse()))
                        {
                            let other_range = other_selection.selection.range();
                            if other_range == current_range {
                                default
                            } else if other_range.start >= current_range.end {
                                let delete_range: CharIndexRange =
                                    (current_range.start..other_range.start).into();
                                let select_range = {
                                    other_selection
                                        .selection
                                        .extended_range()
                                        .shift_left(delete_range.len())
                                };
                                (delete_range, select_range)
                            } else {
                                let delete_range: CharIndexRange =
                                    (other_range.end..current_range.end).into();
                                let select_range = other_selection.selection.range();
                                (delete_range, select_range)
                            }
                        }
                        // If the other selection not found, then only deletes the selection
                        // without moving forward or backward
                        else {
                            let range = selection.extended_range();
                            (range, (range.start..range.start).into())
                        }
                    };
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit {
                                range: delete_range,
                                new: Rope::new(),
                            }),
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
        let dispatches = self.apply_edit_transaction(edit_transaction)?;
        Ok(copy_dispatches.chain(dispatches))
    }

    pub(crate) fn copy(&mut self, use_system_clipboard: bool) -> anyhow::Result<Dispatches> {
        Ok(Dispatches::one(Dispatch::SetClipboardContent {
            use_system_clipboard,
            copied_texts: CopiedTexts::new(self.selection_set.map(|selection| {
                self.buffer()
                    .slice(&selection.extended_range())
                    .ok()
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            })),
        }))
    }

    fn replace_current_selection_with<F>(&mut self, f: F) -> anyhow::Result<Dispatches>
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
                            Action::Edit(Edit {
                                range,
                                new: result.clone(),
                            }),
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
        self.apply_edit_transaction(edit_transaction)
    }

    fn try_replace_current_long_word(&mut self, replacement: String) -> anyhow::Result<Dispatches> {
        let replacement: Rope = replacement.into();
        let buffer = self.buffer();
        let edit_transactions = self.selection_set.map(move |selection| {
            let current_char_index = selection.range().start;
            let word_start = buffer
                .rope()
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
                        Action::Edit(Edit {
                            range,
                            new: replacement.clone(),
                        }),
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
        self.apply_edit_transaction(edit_transaction)
    }

    fn paste_text(
        &mut self,
        direction: Direction,
        copied_texts: CopiedTexts,
    ) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups({
            self.get_selection_set_with_gap(&direction)
                .into_iter()
                .enumerate()
                .map(|(index, (selection, gap))| {
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
                            Action::Edit(Edit {
                                range: insertion_range.into(),
                                new: paste_text,
                            }),
                            Action::Select(
                                selection.set_range(selection_range).set_initial_range(None),
                            ),
                        ]
                        .to_vec(),
                    )
                })
                .collect()
        });
        self.apply_edit_transaction(edit_transaction)
    }

    pub(crate) fn paste(
        &mut self,
        direction: Direction,
        context: &Context,
        use_system_clipboard: bool,
    ) -> anyhow::Result<Dispatches> {
        let Some(copied_texts) = context.get_clipboard_content(use_system_clipboard, 0)? else {
            return Ok(Default::default());
        };
        self.paste_text(direction, copied_texts)
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
        use_system_clipboard: bool,
        history_offset: isize,
    ) -> anyhow::Result<Dispatches> {
        let dispatches = if cut {
            self.copy(use_system_clipboard)?
        } else {
            Default::default()
        };

        let Some(copied_texts) =
            context.get_clipboard_content(use_system_clipboard, history_offset)?
        else {
            return Ok(Default::default());
        };

        Ok(self
            .transform_selection(Transformation::ReplaceWithCopiedText { copied_texts })?
            .chain(dispatches))
    }

    fn apply_edit_transaction(
        &mut self,
        edit_transaction: EditTransaction,
    ) -> anyhow::Result<Dispatches> {
        let new_selection_set = self.buffer.borrow_mut().apply_edit_transaction(
            &edit_transaction,
            self.selection_set.clone(),
            self.mode != Mode::Insert,
        )?;

        self.set_selection_set(new_selection_set);

        self.recalculate_scroll_offset();

        Ok(self.get_document_did_change_dispatch())
    }

    pub(crate) fn get_document_did_change_dispatch(&mut self) -> Dispatches {
        [Dispatch::DocumentDidChange {
            component_id: self.id(),
            path: self.buffer().path(),
            content: self.buffer().rope().to_string(),
            language: self.buffer().language(),
        }]
        .into_iter()
        .chain(if self.mode == Mode::UndoTree {
            Some(self.show_undo_tree_dispatch())
        } else {
            None
        })
        .collect_vec()
        .into()
    }

    pub(crate) fn enter_undo_tree_mode(&mut self) -> Dispatches {
        self.mode = Mode::UndoTree;
        [self.show_undo_tree_dispatch()].to_vec().into()
    }

    pub(crate) fn show_undo_tree_dispatch(&self) -> Dispatch {
        Dispatch::ShowGlobalInfo(Info::new(
            "Undo Tree History".to_string(),
            self.buffer().display_history(),
        ))
    }

    pub(crate) fn undo(&mut self) -> anyhow::Result<Dispatches> {
        let result = self.navigate_undo_tree(Movement::Left)?;
        Ok(result)
    }

    pub(crate) fn redo(&mut self) -> anyhow::Result<Dispatches> {
        self.navigate_undo_tree(Movement::Right)
    }

    pub(crate) fn swap_cursor_with_anchor(&mut self) {
        self.cursor_direction = match self.cursor_direction {
            Direction::Start => Direction::End,
            Direction::End => Direction::Start,
        };
        self.recalculate_scroll_offset()
    }

    fn get_selection_set(
        &self,
        mode: &SelectionMode,
        movement: Movement,
    ) -> anyhow::Result<Option<SelectionSet>> {
        self.selection_set.generate(
            &self.buffer.borrow(),
            mode,
            &movement,
            &self.cursor_direction,
        )
    }

    pub(crate) fn get_cursor_char_index(&self) -> CharIndex {
        self.selection_set
            .primary_selection()
            .to_char_index(&self.cursor_direction)
    }

    pub(crate) fn enter_v_mode(&mut self) {
        self.mode = Mode::V
    }

    pub(crate) fn enable_selection_extension(&mut self) {
        self.selection_set.enable_selection_extension();
    }

    pub(crate) fn handle_key_event(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        match self.handle_universal_key(key_event)? {
            HandleEventResult::Ignored(key_event) => {
                if let Some(jumps) = self.jumps.take() {
                    self.handle_jump_mode(context, key_event, jumps)
                } else {
                    match &self.mode {
                        Mode::Normal => self.handle_normal_mode(context, key_event),
                        Mode::Insert => self.handle_insert_mode(key_event),
                        Mode::MultiCursor => self.handle_multi_cursor_mode(context, key_event),
                        Mode::FindOneChar(if_current_not_found) => {
                            self.handle_find_one_char_mode(*if_current_not_found, key_event)
                        }
                        Mode::Exchange => self.handle_normal_mode(context, key_event),
                        Mode::UndoTree => self.handle_normal_mode(context, key_event),
                        Mode::Replace => self.handle_normal_mode(context, key_event),
                        Mode::V => self.handle_v_mode(context, key_event),
                    }
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
                    Some((jump, [])) => Ok(self
                        .handle_movement(context, Movement::Jump(jump.selection.extended_range()))?
                        .append(Dispatch::ToEditor(EnterNormalMode))),
                    Some(_) => {
                        self.jumps = Some(
                            matching_jumps
                                .into_iter()
                                .zip(Self::jump_characters().into_iter().cycle())
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
    }

    /// Similar to Change in Vim, but does not copy the current selection
    pub(crate) fn change(&mut self) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let range = selection.extended_range();
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit {
                                range,
                                new: Rope::new(),
                            }),
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
            .apply_edit_transaction(edit_transaction)?
            .chain(self.enter_insert_mode(Direction::Start)?))
    }

    pub(crate) fn change_cut(&mut self, use_system_clipboard: bool) -> anyhow::Result<Dispatches> {
        Ok(self.copy(use_system_clipboard)?.chain(self.change()?))
    }

    pub(crate) fn insert(&mut self, s: &str) -> anyhow::Result<Dispatches> {
        let edit_transaction =
            EditTransaction::from_action_groups(
                self.selection_set
                    .map(|selection| {
                        let range = selection.extended_range();
                        ActionGroup::new(
                            [
                                Action::Edit(Edit {
                                    range: {
                                        let start = selection.to_char_index(&Direction::End);
                                        (start..start).into()
                                    },
                                    new: Rope::from_str(s),
                                }),
                                Action::Select(selection.clone().set_range(
                                    (range.start + s.len()..range.start + s.len()).into(),
                                )),
                            ]
                            .to_vec(),
                        )
                    })
                    .into(),
            );

        self.apply_edit_transaction(edit_transaction)
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
    ) -> anyhow::Result<Dispatches> {
        self.move_selection_with_selection_mode_without_global_mode(
            Movement::Current(if_current_not_found),
            selection_mode.clone(),
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
            self.move_selection_with_selection_mode_without_global_mode(movement, selection_mode)
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
                self.selection_set.mode.clone(),
            ),
            Mode::Exchange => self.exchange(movement),
            Mode::Replace => self.replace_with_movement(&movement),
            Mode::UndoTree => self.navigate_undo_tree(movement),
            Mode::MultiCursor => self.add_cursor(&movement).map(|_| Default::default()),
            _ => Ok(Default::default()),
        }
    }

    pub(crate) fn toggle_marks(&mut self) {
        let selections = self
            .selection_set
            .map(|selection| selection.extended_range());
        self.buffer_mut().save_marks(selections.into())
    }

    pub(crate) fn path(&self) -> Option<CanonicalizedPath> {
        self.editor().buffer().path()
    }

    pub(crate) fn enter_insert_mode(&mut self, direction: Direction) -> anyhow::Result<Dispatches> {
        self.set_selection_set(self.selection_set.apply(
            self.selection_set.mode.clone(),
            |selection| {
                let range = selection.extended_range();
                let char_index = match direction {
                    Direction::Start => range.start,
                    Direction::End => range.end,
                };
                Ok(selection
                    .clone()
                    .set_range((char_index..char_index).into())
                    .set_initial_range(None))
            },
        )?);
        self.mode = Mode::Insert;
        self.cursor_direction = Direction::Start;
        Ok(Dispatches::one(Dispatch::RequestSignatureHelp))
    }

    pub(crate) fn enter_normal_mode(&mut self) -> anyhow::Result<()> {
        if self.mode == Mode::Insert {
            // This is necessary for cursor to not overflow after exiting insert mode
            self.set_selection_set(self.selection_set.apply(
                self.selection_set.mode.clone(),
                |selection| {
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
                },
            )?);
            self.clamp()?;
            self.buffer_mut().reparse_tree()?
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
        row: u16,
        column: u16,
    ) -> anyhow::Result<Dispatches> {
        let start = (self.buffer.borrow().line_to_char(row as usize)?) + column.into();
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
        ))
    }

    /// Get the selection that preserves the syntactic structure of the current selection.
    ///
    /// Returns a valid edit transaction if there is any, otherwise `Left(current_selection)`.
    fn get_valid_selection(
        &self,
        current_selection: &Selection,
        selection_mode: &SelectionMode,
        direction: &Movement,
        get_actual_edit_transaction: impl Fn(
            /* current */ &Selection,
            /* next */ &Selection,
        ) -> anyhow::Result<EditTransaction>,
    ) -> anyhow::Result<Either<Selection, EditTransaction>> {
        let current_selection = current_selection.clone();

        let buffer = self.buffer.borrow();

        // Loop until the edit transaction does not result in errorneous node
        let mut next_selection = Selection::get_selection_(
            &buffer,
            &current_selection,
            selection_mode,
            direction,
            &self.cursor_direction,
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
                    .apply_edit_transaction(&edit_transaction, self.selection_set.clone(), true)
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
            // Because I assume we want to be able to exchange even if some part of the tree
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
                direction,
                &self.cursor_direction,
            )?
            .unwrap_or_else(|| next_selection.clone().into())
            .selection;

            if next_selection.eq(&new_selection) {
                return Ok(Either::Left(current_selection));
            }

            next_selection = new_selection;
        }
    }

    /// Replace the next selection with the current selection without
    /// making the syntax node invalid
    fn replace_faultlessly(
        &mut self,
        selection_mode: &SelectionMode,
        movement: Movement,
    ) -> anyhow::Result<Dispatches> {
        let buffer = self.buffer.borrow().clone();
        let get_edit_transaction = |current_selection: &Selection,
                                    next_selection: &Selection|
         -> anyhow::Result<_> {
            let current_selection_range = current_selection.extended_range();
            let text_at_current_selection: Rope = buffer.slice(&current_selection_range)?;
            let text_at_next_selection: Rope = buffer.slice(&next_selection.extended_range())?;

            Ok(EditTransaction::from_action_groups(
                [
                    ActionGroup::new(
                        [Action::Edit(Edit {
                            range: current_selection_range,
                            new: text_at_next_selection.clone(),
                        })]
                        .to_vec(),
                    ),
                    ActionGroup::new(
                        [
                            Action::Edit(Edit {
                                range: next_selection.extended_range(),
                                new: text_at_current_selection.clone(),
                            }),
                            Action::Select(
                                current_selection.clone().set_range(
                                    (next_selection.extended_range().start
                                        ..(next_selection.extended_range().start
                                            + text_at_current_selection.len_chars()))
                                        .into(),
                                ),
                            ),
                        ]
                        .to_vec(),
                    ),
                ]
                .to_vec(),
            ))
        };

        let edit_transactions = self.selection_set.map(|selection| {
            self.get_valid_selection(selection, selection_mode, &movement, get_edit_transaction)
        });

        self.apply_edit_transaction(EditTransaction::merge(
            edit_transactions
                .into_iter()
                .filter_map(|transaction| transaction.ok())
                .filter_map(|transaction| transaction.map_right(Some).right_or(None))
                .collect(),
        ))
    }

    pub(crate) fn exchange(&mut self, movement: Movement) -> anyhow::Result<Dispatches> {
        let mode = self.selection_set.mode.clone();
        self.replace_faultlessly(&mode, movement)
    }

    pub(crate) fn add_cursor(&mut self, movement: &Movement) -> anyhow::Result<()> {
        self.selection_set.add_selection(
            &self.buffer.borrow(),
            movement,
            &self.cursor_direction,
        )?;
        self.recalculate_scroll_offset();
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
            Direction::Start => self.scroll_offset.saturating_sub(scroll_height as u16),
            Direction::End => self.scroll_offset.saturating_add(scroll_height as u16),
        };
    }

    pub(crate) fn backspace(&mut self) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| {
                    let start = CharIndex(selection.extended_range().start.0.saturating_sub(1));
                    ActionGroup::new(
                        [
                            Action::Edit(Edit {
                                range: (start..selection.extended_range().start).into(),
                                new: Rope::from(""),
                            }),
                            Action::Select(selection.clone().set_range((start..start).into())),
                        ]
                        .to_vec(),
                    )
                })
                .into(),
        );

        self.apply_edit_transaction(edit_transaction)
    }

    pub(crate) fn delete_word_backward(
        &mut self,
        short: bool,
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
                            &movement,
                            &self.cursor_direction,
                        )
                        .map(|option| option.unwrap_or_else(|| current_selection.clone().into()))
                    };
                    let current_word =
                        get_word(Movement::Current(IfCurrentNotFound::LookBackward))?.selection;
                    if current_word.extended_range().start <= start {
                        current_word
                    } else {
                        get_word(Movement::Left)?.selection
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
                        Action::Edit(Edit {
                            range: (start..end).into(),
                            new: Rope::from(""),
                        }),
                        Action::Select(current_selection.clone().set_range((start..start).into())),
                    ]
                    .to_vec(),
                ))
            })
            .into_iter()
            .flatten()
            .collect();
        let edit_transaction = EditTransaction::from_action_groups(action_groups);
        self.apply_edit_transaction(edit_transaction)
    }

    /// Replace the parent node of the current node with the current node
    pub(crate) fn replace_with_movement(
        &mut self,
        movement: &Movement,
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
                                Action::Edit(Edit {
                                    range: range.clone().into(),
                                    new,
                                }),
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
                &self.selection_set.mode,
                movement,
                get_edit_transaction,
            )
        });
        let edit_transaction = EditTransaction::merge(
            edit_transactions
                .into_iter()
                .filter_map(|edit_transaction| edit_transaction.ok())
                .filter_map(|edit_transaction| edit_transaction.map_right(Some).right_or(None))
                .collect(),
        );
        self.apply_edit_transaction(edit_transaction)
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

    fn scroll(&mut self, direction: Direction, scroll_height: usize) -> anyhow::Result<Dispatches> {
        let dispatch = self.update_selection_set(
            self.selection_set
                .apply(self.selection_set.mode.clone(), |selection| {
                    let position = selection.extended_range().start.to_position(&self.buffer());
                    let line = if direction == Direction::End {
                        position.line.saturating_add(scroll_height)
                    } else {
                        position.line.saturating_sub(scroll_height)
                    };
                    let position = Position { line, ..position };
                    let start = position.to_char_index(&self.buffer())?;
                    Ok(selection.clone().set_range((start..start).into()))
                })?,
            false,
        );
        self.align_cursor_to_center();

        Ok(dispatch)
    }

    /// This returns a vector of selections
    /// with a gap that is the maximum of previous-current gap and current-next gap.
    ///
    /// Used for `Self::open` and `Self::paste`.
    fn get_selection_set_with_gap(&self, direction: &Direction) -> Vec<(Selection, Rope)> {
        self.selection_set
            .map(|selection| {
                let buffer = self.buffer.borrow();
                let get_in_between_gap = |movement: Movement| -> Option<Rope> {
                    let other = Selection::get_selection_(
                        &buffer,
                        selection,
                        &self.selection_set.mode,
                        &movement,
                        &self.cursor_direction,
                    )
                    .ok()??
                    .selection;
                    if other.range() == selection.range() {
                        None
                    } else {
                        let current_range = selection.range();
                        let other_range = other.range();
                        let in_between_range = current_range.end.min(other_range.end)
                            ..current_range.start.max(other_range.start);
                        buffer.slice(&in_between_range.into()).ok()
                    }
                };
                let gap = if !self.selection_set.mode.is_contiguous() {
                    Rope::from_str("")
                } else {
                    match (
                        get_in_between_gap(if self.selection_set.mode.is_syntax_node() {
                            Movement::Next
                        } else {
                            Movement::Right
                        }),
                        get_in_between_gap(if self.selection_set.mode.is_syntax_node() {
                            Movement::Previous
                        } else {
                            Movement::Left
                        }),
                    ) {
                        (None, None) => Default::default(),
                        (None, Some(gap)) | (Some(gap), None) => gap,
                        (Some(next_gap), Some(prev_gap)) => {
                            let larger = next_gap.len_chars() > prev_gap.len_chars();
                            match (direction, larger) {
                                (Direction::Start, true) => prev_gap,
                                (Direction::Start, false) => next_gap,
                                (Direction::End, true) => next_gap,
                                (Direction::End, false) => prev_gap,
                            }
                        }
                    }
                };
                (selection.clone(), gap)
            })
            .into_iter()
            .collect_vec()
    }

    fn open(&mut self, direction: Direction) -> Result<Dispatches, anyhow::Error> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.get_selection_set_with_gap(&direction)
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
                            Action::Edit(Edit {
                                range: {
                                    let start = match direction {
                                        Direction::Start => selection.range().start,
                                        Direction::End => selection.range().end,
                                    };
                                    (start..start).into()
                                },
                                new: gap,
                            }),
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
            .apply_edit_transaction(edit_transaction)?
            .append(Dispatch::ToEditor(EnterInsertMode(direction))))
    }

    pub(crate) fn apply_positional_edits(
        &mut self,
        edits: Vec<PositionalEdit>,
    ) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups(
            edits
                .into_iter()
                .filter_map(|edit| {
                    let range = edit.range.start.to_char_index(&self.buffer()).ok()?
                        ..edit.range.end.to_char_index(&self.buffer()).ok()?;

                    let action_edit = Action::Edit(Edit {
                        range: range.clone().into(),
                        new: edit.new_text.into(),
                    });

                    Some(ActionGroup::new(vec![action_edit]))
                })
                .chain(
                    // This is necessary to retain the current selection set
                    self.selection_set
                        .map(|selection| ActionGroup::new(vec![Action::Select(selection.clone())])),
                )
                .collect(),
        );
        self.apply_edit_transaction(edit_transaction)
    }

    pub(crate) fn save(&mut self) -> anyhow::Result<Dispatches> {
        let Some(path) = self.buffer.borrow_mut().save(self.selection_set.clone())? else {
            return Ok(Default::default());
        };

        self.clamp()?;
        self.cursor_keep_primary_only();
        self.enter_normal_mode()?;
        Ok(Dispatches::one(Dispatch::RemainOnlyCurrentComponent)
            .append(Dispatch::DocumentDidSave { path })
            .chain(self.get_document_did_change_dispatch())
            .append(Dispatch::RemainOnlyCurrentComponent)
            .append_some(if self.selection_set.mode.is_contiguous() {
                Some(Dispatch::ToEditor(MoveSelection(Movement::Current(
                    IfCurrentNotFound::LookForward,
                ))))
            } else {
                None
            }))
    }

    /// Clamp everything that might be out of bound after the buffer content is modified elsewhere
    fn clamp(&mut self) -> anyhow::Result<()> {
        let len_chars = self.buffer().len_chars();
        self.set_selection_set(self.selection_set.clamp(CharIndex(len_chars))?);

        let len_lines = self.buffer().len_lines();
        self.scroll_offset = self.scroll_offset.clamp(0, len_lines as u16);

        Ok(())
    }

    pub(crate) fn enclose(&mut self, open: String, close: String) -> anyhow::Result<Dispatches> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let old = self.buffer().slice(&selection.extended_range())?;
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit {
                                range: selection.extended_range(),
                                new: format!("{}{}{}", open, old, close).into(),
                            }),
                            Action::Select(
                                selection.clone().set_range(
                                    (selection.extended_range().start
                                        ..selection.extended_range().end + 2)
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

        self.apply_edit_transaction(edit_transaction)
    }

    fn transform_selection(
        &mut self,
        transformation: Transformation,
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
                            Action::Edit(Edit { range, new }),
                            Action::Select(
                                selection
                                    .clone()
                                    .set_range((range.start..range.start + new_char_count).into()),
                            ),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .try_collect()?,
        );
        self.apply_edit_transaction(edit_transaction)
    }

    pub(crate) fn display_mode(&self) -> String {
        let prefix = if self.selection_set.is_extended() {
            "+"
        } else {
            ""
        };
        let core = match &self.mode {
            Mode::Normal => "MOVE",
            Mode::Insert => "INSERT",
            Mode::MultiCursor => "MULTI CURSOR",
            Mode::FindOneChar(_) => "FIND ONE CHAR",
            Mode::Exchange => "EXCHANGE",
            Mode::UndoTree => "UNDO TREE",
            Mode::Replace => "REPLACE",
            Mode::V => "V",
        }
        .to_string();
        format!("{prefix}{core}")
    }

    pub(crate) fn display_selection_mode(&self) -> String {
        let selection_mode = self.selection_set.mode.display();
        let cursor_count = self.selection_set.len();
        let result = format!("{} x {}", selection_mode, cursor_count);
        if self.jumps.is_some() {
            format!("{} (JUMP)", result)
        } else {
            result
        }
    }

    pub(crate) fn visible_line_range(&self) -> Range<usize> {
        let start = self.scroll_offset;
        let end = (start as usize + self.rectangle.height as usize).min(self.buffer().len_lines());

        start as usize..end
    }

    fn handle_multi_cursor_mode(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> Result<Dispatches, anyhow::Error> {
        match key_event {
            key!("esc") => {
                self.mode = Mode::Normal;
                Ok(Default::default())
            }

            other => self.handle_normal_mode(context, other),
        }
    }

    pub(crate) fn add_cursor_to_all_selections(&mut self) -> Result<(), anyhow::Error> {
        self.selection_set
            .add_all(&self.buffer.borrow(), &self.cursor_direction)?;
        self.recalculate_scroll_offset();
        Ok(())
    }

    pub(crate) fn cursor_keep_primary_only(&mut self) {
        self.selection_set.only();
    }

    fn enter_single_character_mode(&mut self, if_current_not_found: IfCurrentNotFound) {
        self.mode = Mode::FindOneChar(if_current_not_found);
    }

    fn handle_find_one_char_mode(
        &mut self,
        if_current_not_found: IfCurrentNotFound,
        key_event: KeyEvent,
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
        (self.dimension().height / 2) as usize
    }

    #[cfg(test)]
    pub(crate) fn match_literal(&mut self, search: &str) -> anyhow::Result<Dispatches> {
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
        )
    }

    pub(crate) fn move_to_line_start(&mut self) -> anyhow::Result<Dispatches> {
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
        self.apply_edit_transaction(edit_transaction)
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
                (MoveSelection(Movement::First)),
                (EnableSelectionExtension),
                (MoveSelection(Movement::Last)),
            ]
            .to_vec(),
        )
    }

    fn move_selection_with_selection_mode_without_global_mode(
        &mut self,
        movement: Movement,
        selection_mode: SelectionMode,
    ) -> Result<Dispatches, anyhow::Error> {
        let dispatches = self.select(selection_mode, movement)?;
        self.current_view_alignment = None;

        Ok(dispatches)
    }

    pub(crate) fn scroll_page_down(&mut self) -> Result<Dispatches, anyhow::Error> {
        self.scroll(Direction::End, self.half_page_height())
    }

    pub(crate) fn scroll_page_up(&mut self) -> Result<Dispatches, anyhow::Error> {
        self.scroll(Direction::Start, self.half_page_height())
    }

    #[cfg(test)]
    pub(crate) fn current_view_alignment(&self) -> Option<ViewAlignment> {
        self.current_view_alignment
    }

    pub(crate) fn switch_view_alignment(&mut self) {
        self.current_view_alignment = Some(match self.current_view_alignment {
            Some(ViewAlignment::Top) => {
                self.align_cursor_to_center();
                ViewAlignment::Center
            }
            Some(ViewAlignment::Center) => {
                self.align_cursor_to_bottom();
                ViewAlignment::Bottom
            }
            None | Some(ViewAlignment::Bottom) => {
                self.align_cursor_to_top();
                ViewAlignment::Top
            }
        })
    }

    fn navigate_undo_tree(&mut self, movement: Movement) -> Result<Dispatches, anyhow::Error> {
        let selection_set = self.buffer_mut().undo_tree_apply_movement(movement)?;

        Ok(selection_set
            .map(|selection_set| self.update_selection_set(selection_set, false))
            .unwrap_or_default()
            .chain(self.get_document_did_change_dispatch()))
    }

    #[cfg(test)]
    pub(crate) fn set_scroll_offset(&mut self, scroll_offset: u16) {
        self.scroll_offset = scroll_offset
    }

    #[cfg(test)]
    pub(crate) fn set_language(
        &mut self,
        language: shared::language::Language,
    ) -> anyhow::Result<()> {
        self.buffer_mut().set_language(language)
    }

    pub(crate) fn render_area(&self) -> Dimension {
        let Dimension { height, width } = self.dimension();
        Dimension {
            height: height.saturating_sub(WINDOW_TITLE_HEIGHT as u16),
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
            buffer.update_highlighted_spans(highlighted_spans);
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

    fn enter_exchange_mode(&mut self) {
        self.mode = Mode::Exchange
    }

    fn kill_line(&mut self, direction: Direction) -> Result<Dispatches, anyhow::Error> {
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
                            Action::Edit(Edit {
                                range: delete_range,
                                new: Rope::new(),
                            }),
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
            .apply_edit_transaction(edit_transaction)?
            .chain(self.enter_insert_mode(Direction::Start)?);
        Ok(dispatches)
    }

    fn enter_multicursor_mode(&mut self) {
        self.mode = Mode::MultiCursor
    }

    fn enter_replace_mode(&mut self) {
        self.mode = Mode::Replace
    }

    pub(crate) fn scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    pub(crate) fn set_regex_highlight_rules(
        &mut self,
        regex_highlight_rules: Vec<RegexHighlightRule>,
    ) {
        self.regex_highlight_rules = regex_highlight_rules
    }

    fn go_back(&mut self) {
        let selection_set = self.buffer_mut().previous_selection_set();
        if let Some(selection_set) = selection_set {
            self.set_selection_set(selection_set)
        }
    }

    fn go_forward(&mut self) {
        let selection_set = self.buffer_mut().next_selection_set();
        if let Some(selection_set) = selection_set {
            self.set_selection_set(selection_set)
        }
    }

    fn set_selection_set(&mut self, selection_set: SelectionSet) {
        self.selection_set = selection_set;
        self.recalculate_scroll_offset()
    }

    pub(crate) fn set_position_range(
        &mut self,
        range: Range<Position>,
    ) -> Result<Dispatches, anyhow::Error> {
        let selection_set = self.position_range_to_selection_set(range)?;
        Ok(self.update_selection_set(selection_set, true))
    }

    fn select_surround(
        &mut self,
        enclosure: EnclosureKind,
        kind: SurroundKind,
    ) -> anyhow::Result<Dispatches> {
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
        let _ = self.set_selection_mode(IfCurrentNotFound::LookForward, SelectionMode::Custom);
        self.apply_edit_transaction(edit_transaction)
    }

    fn delete_surround(&mut self, enclosure: EnclosureKind) -> Result<Dispatches, anyhow::Error> {
        self.change_surround(enclosure, None)
    }

    fn change_surround(
        &mut self,
        from: EnclosureKind,
        to: Option<EnclosureKind>,
    ) -> Result<Dispatches, anyhow::Error> {
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
                                [Action::Edit(Edit {
                                    range: open_range,
                                    new: new_open.into(),
                                })]
                                .to_vec(),
                            ),
                            ActionGroup::new(
                                [Action::Edit(Edit {
                                    range: close_range,
                                    new: new_close.into(),
                                })]
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
        let _ = self.set_selection_mode(IfCurrentNotFound::LookForward, SelectionMode::Custom);
        self.apply_edit_transaction(edit_transaction)
    }

    fn replace_with_pattern(&mut self, context: &Context) -> Result<Dispatches, anyhow::Error> {
        let config = context.local_search_config();
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
                                        Action::Edit(Edit { range, new }),
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
                self.apply_edit_transaction(edit_transaction)
            }
            LocalSearchConfigMode::Regex(regex_config) => {
                self.transform_selection(Transformation::RegexReplace {
                    regex: MyRegex(regex_config.to_regex(&config.search())?),
                    replacement: config.replacement(),
                })
            }
            LocalSearchConfigMode::CaseAgnostic => {
                self.transform_selection(Transformation::CaseAgnosticReplace {
                    search: config.search(),
                    replacement: config.replacement(),
                })
            }
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

    fn move_to_last_char(&mut self) -> Dispatches {
        let last_cursor_index = CharIndex(self.buffer().len_chars());
        self.update_selection_set(
            SelectionSet::new(NonEmpty::singleton(
                Selection::default().set_range((last_cursor_index..last_cursor_index).into()),
            )),
            false,
        )
        .append(Dispatch::ToEditor(EnterInsertMode(Direction::Start)))
    }

    fn pipe_to_shell(&mut self, command: String) -> Result<Dispatches, anyhow::Error> {
        self.transform_selection(Transformation::PipeToShell { command })
    }

    fn show_current_tree_sitter_node_sexp(&self) -> Result<Dispatches, anyhow::Error> {
        let buffer = self.buffer();
        let node = buffer.get_current_node(self.selection_set.primary_selection(), false)?;
        let info = node
            .map(|node| node.to_sexp())
            .unwrap_or("[No node found]".to_string());
        Ok(Dispatches::one(Dispatch::ShowEditorInfo(Info::new(
            "Tree-sitter node S-expression".to_string(),
            info,
        ))))
    }

    fn indent(&mut self) -> Result<Dispatches, anyhow::Error> {
        let indentation: Rope = std::iter::repeat(INDENT_CHAR)
            .take(INDENT_WIDTH)
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
                                format!("{}{}", indentation, line),
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
                            Action::Edit(Edit {
                                range: linewise_range,
                                new,
                            }),
                            Action::Select(selection.clone().set_range(select_range.into())),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect_vec(),
        );
        self.apply_edit_transaction(edit_transaction)
    }

    fn dedent(&mut self) -> Result<Dispatches, anyhow::Error> {
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
                            Action::Edit(Edit {
                                range: linewise_range,
                                new,
                            }),
                            Action::Select(selection.clone().set_range(select_range.into())),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect_vec(),
        );
        self.apply_edit_transaction(edit_transaction)
    }

    #[cfg(test)]
    pub(crate) fn primary_selection(&self) -> anyhow::Result<String> {
        println!(
            "Editor::primary_selection range = {:?}",
            &self
                .editor()
                .selection_set
                .primary_selection()
                .extended_range()
        );
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

    fn handle_v_mode(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> Result<Dispatches, anyhow::Error> {
        self.mode = Mode::Normal;
        if let Some(keymap) = self.visual_mode_initialized_keymaps().get(&key_event) {
            Ok(keymap.get_dispatches())
        } else {
            self.enable_selection_extension();
            self.handle_normal_mode(context, key_event)
        }
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
            SetSelectionMode(IfCurrentNotFound::LookForward, SelectionMode::Column);
        match direction {
            Direction::Start => self.handle_dispatch_editor(context, set_column_selection_mode),
            Direction::End => self.handle_dispatch_editors(
                context,
                [SwapCursorWithAnchor, set_column_selection_mode].to_vec(),
            ),
        }
    }

    fn filter_selection_matching_search(
        &mut self,
        local_search_config: &crate::context::LocalSearchConfig,
        search: String,
        keep: bool,
    ) -> Dispatches {
        let selections = self.selection_set.selections();
        let filtered = selections
            .iter()
            .filter_map(|selection| -> Option<_> {
                let range = selection.extended_range();
                let haystack = self.buffer().slice(&range).unwrap_or_default().to_string();
                let is_match = match local_search_config.mode {
                    LocalSearchConfigMode::Regex(regex_config) => {
                        get_regex(&search, regex_config).ok()?.is_match(&haystack)
                    }
                    LocalSearchConfigMode::AstGrep => false,
                    LocalSearchConfigMode::CaseAgnostic => {
                        selection_mode::CaseAgnostic::new(search.clone())
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
        self.update_selection_set(self.selection_set.clone().set_selections(selections), true)
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
    SetScrollOffset(u16),
    ShowJumps {
        use_current_selection_mode: bool,
    },
    ScrollPageDown,
    ScrollPageUp,
    #[cfg(test)]
    AlignViewTop,
    #[cfg(test)]
    AlignViewBottom,
    Transform(Transformation),
    SetSelectionMode(IfCurrentNotFound, SelectionMode),
    Save,
    FindOneChar(IfCurrentNotFound),
    MoveSelection(Movement),
    SwitchViewAlignment,
    Copy {
        use_system_clipboard: bool,
    },
    GoBack,
    GoForward,
    SelectAll,
    SetContent(String),
    SetDecorations(Vec<Decoration>),
    #[cfg(test)]
    SetRectangle(Rectangle),
    EnableSelectionExtension,
    EnterVMode,
    Change,
    ChangeCut {
        use_system_clipboard: bool,
    },
    EnterUndoTreeMode,
    EnterInsertMode(Direction),
    ReplaceWithCopiedText {
        cut: bool,
        use_system_clipboard: bool,
    },
    ReplaceWithPattern,
    SelectLine(Movement),
    Backspace,
    Delete(Direction),
    Insert(String),
    MoveToLineStart,
    MoveToLineEnd,
    #[cfg(test)]
    MatchLiteral(String),
    SelectSurround {
        enclosure: EnclosureKind,
        kind: SurroundKind,
    },
    Open(Direction),
    ToggleMark,
    EnterNormalMode,
    EnterExchangeMode,
    EnterReplaceMode,
    EnterMultiCursorMode,
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
    SetLanguage(shared::language::Language),
    #[cfg(test)]
    ApplySyntaxHighlight,
    ReplaceCurrentSelectionWith(String),
    TryReplaceCurrentLongWord(String),
    SelectLineAt(usize),
    ShowKeymapLegendNormalMode,
    ShowKeymapLegendInsertMode,
    Paste {
        direction: Direction,
        use_system_clipboard: bool,
    },
    SwapCursorWithAnchor,
    MoveCharacterBack,
    MoveCharacterForward,
    ShowKeymapLegendHelp,
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
    SwapExtensionDirection,
    CollapseSelection(Direction),
    FilterSelectionMatchingSearch {
        search: String,
        keep: bool,
    },
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
