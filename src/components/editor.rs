use crate::{
    app::{FilePickerKind, RequestParams, Scope},
    buffer::Line,
    char_index_range::CharIndexRange,
    components::component::Cursor,
    context::{Context, GlobalMode, Search, SearchKind},
    grid::{CellUpdate, Style, StyleKey},
    lsp::process::ResponseContext,
    selection::{Filter, FilterKind, FilterTarget, Filters},
    selection_mode::{self, inside::InsideKind, ByteRange},
    soft_wrap,
};

use shared::{canonicalized_path::CanonicalizedPath, language::Language};
use std::{
    cell::{Ref, RefCell, RefMut},
    ops::Range,
    rc::Rc,
};

use convert_case::{Case, Casing};
use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use event::KeyEvent;
use itertools::{Either, Itertools};
use lsp_types::DiagnosticSeverity;
use my_proc_macros::key;
use ropey::Rope;

use crate::{
    app::{Dimension, Dispatch},
    buffer::Buffer,
    components::component::Component,
    edit::{Action, ActionGroup, Edit, EditTransaction},
    grid::Grid,
    lsp::completion::PositionalEdit,
    position::Position,
    quickfix_list::QuickfixListType,
    rectangle::Rectangle,
    selection::{CharIndex, Selection, SelectionMode, SelectionSet},
};

use super::{
    component::{ComponentId, GetGridResult},
    keymap_legend::{Keymap, KeymapLegendConfig},
    suggestive_editor::Info,
};

#[derive(PartialEq, Clone, Debug, Eq)]
pub enum Mode {
    Normal,
    Insert,
    MultiCursor,
    FindOneChar,
    Exchange,
    UndoTree,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Jump {
    pub character: char,
    pub selection: Selection,
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
        context.current_working_directory();
        let title = self.title.clone();
        title
            .or_else(|| {
                let path = self.buffer().path()?;
                let current_working_directory = context.current_working_directory()?;
                let string = path
                    .display_relative_to(current_working_directory)
                    .unwrap_or_else(|_| path.display_absolute());
                Some(format!(" {} {}", string, path.icon()))
            })
            .unwrap_or_else(|| "[No title]".to_string())
    }

    fn set_title(&mut self, title: String) {
        self.title = Some(title);
    }

    fn get_grid(&self, context: &mut Context) -> GetGridResult {
        let editor = self;
        let Dimension { height, width } = editor.render_area();
        let buffer = editor.buffer();
        let rope = buffer.rope();

        let highlighted_spans = buffer.highlighted_spans();
        let diagnostics = context.get_diagnostics(self.path());

        let len_lines = rope.len_lines().max(1) as u16;
        let max_line_number_len = len_lines.to_string().len() as u16;
        let line_number_separator_width = 1;
        let (hidden_parent_lines, visible_parent_lines) =
            self.get_parent_lines().unwrap_or_default();

        let top_offset = hidden_parent_lines.len() as u16;

        let scroll_offset = self.scroll_offset;

        let visible_lines = &rope
            .lines()
            .skip(scroll_offset as usize)
            .take(height as usize)
            .map(|slice| slice.to_string())
            .collect_vec();

        let content_container_width = (width
            .saturating_sub(max_line_number_len)
            .saturating_sub(line_number_separator_width))
            as usize;

        let wrapped_lines = soft_wrap::soft_wrap(&visible_lines.join(""), content_container_width);

        let parent_lines_numbers = visible_parent_lines
            .iter()
            .chain(hidden_parent_lines.iter())
            .map(|line| line.line)
            .collect_vec();

        let visible_lines_grid: Grid = Grid::new(Dimension {
            height: (height as usize).max(wrapped_lines.wrapped_lines_count()) as u16,
            width,
        });

        let selection = &editor.selection_set.primary;
        // If the buffer selection is updated less recently than the window's scroll offset,

        // use the window's scroll offset.

        let lines = wrapped_lines
            .lines()
            .iter()
            .flat_map(|line| {
                let line_number = line.line_number();
                line.lines()
                    .into_iter()
                    .enumerate()
                    .map(|(index, line)| RenderLine {
                        line_number: line_number + (scroll_offset as usize),
                        content: line,
                        wrapped: index > 0,
                    })
                    .collect_vec()
            })
            .collect::<Vec<_>>();
        let theme = context.theme();

        let possible_selections = self
            .possible_selections_in_line_number_range(&self.selection_set.primary, context)
            .unwrap_or_default()
            .into_iter()
            .flat_map(|range| buffer.byte_range_to_char_index_range(range.range()))
            .flat_map(|bookmark| {
                range_to_cell_update(&buffer, bookmark, theme, StyleKey::UiPossibleSelection)
            })
            .collect_vec();

        let bookmarks = buffer.bookmarks().into_iter().flat_map(|bookmark| {
            range_to_cell_update(&buffer, bookmark, theme, StyleKey::UiBookmark)
        });

        let secondary_selections = &editor.selection_set.secondary;

        fn range_to_cell_update(
            buffer: &Buffer,
            range: CharIndexRange,
            theme: &crate::themes::Theme,
            source: StyleKey,
        ) -> Vec<CellUpdate> {
            range
                .iter()
                .filter_map(|char_index| {
                    let position = buffer.char_to_position(char_index).ok()?;
                    let style = theme.get_style(&source);
                    Some(CellUpdate::new(position).style(style).source(Some(source)))
                })
                .collect()
        }

        fn char_index_to_cell_update(
            buffer: &Buffer,
            char_index: CharIndex,
            style: Style,
        ) -> Option<CellUpdate> {
            buffer
                .char_to_position(char_index)
                .ok()
                .map(|position| CellUpdate::new(position).style(style))
        }

        let primary_selection = range_to_cell_update(
            &buffer,
            selection.extended_range(),
            theme,
            StyleKey::UiPrimarySelection,
        );

        let primary_selection_anchors = selection.anchors().into_iter().flat_map(|range| {
            range_to_cell_update(&buffer, range, theme, StyleKey::UiPrimarySelectionAnchors)
        });

        let primary_selection_primary_cursor = char_index_to_cell_update(
            &buffer,
            selection.to_char_index(&editor.cursor_direction),
            Style::default(),
        )
        .map(|cell_update| cell_update.set_is_cursor(true));

        let primary_selection_secondary_cursor = if self.mode == Mode::Insert {
            None
        } else {
            char_index_to_cell_update(
                &buffer,
                selection.to_char_index(&editor.cursor_direction.reverse()),
                theme.ui.primary_selection_secondary_cursor,
            )
        };

        let secondary_selection = secondary_selections.iter().flat_map(|secondary_selection| {
            range_to_cell_update(
                &buffer,
                secondary_selection.extended_range(),
                theme,
                StyleKey::UiSecondarySelection,
            )
        });
        let seconday_selection_anchors = secondary_selections.iter().flat_map(|selection| {
            selection.anchors().into_iter().flat_map(|range| {
                range_to_cell_update(&buffer, range, theme, StyleKey::UiSecondarySelectionAnchors)
            })
        });

        let secondary_selection_cursors = secondary_selections
            .iter()
            .filter_map(|secondary_selection| {
                Some(
                    [
                        char_index_to_cell_update(
                            &buffer,
                            secondary_selection.to_char_index(&editor.cursor_direction.reverse()),
                            theme.ui.secondary_selection_secondary_cursor,
                        ),
                        char_index_to_cell_update(
                            &buffer,
                            secondary_selection.to_char_index(&editor.cursor_direction),
                            theme.ui.secondary_selection_primary_cursor,
                        ),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>(),
                )
            })
            .flatten();

        let diagnostics = diagnostics
            .iter()
            .sorted_by(|a, b| a.severity.cmp(&b.severity))
            .rev()
            .filter_map(|diagnostic| {
                // We use `.ok()` to ignore diagnostics that are outside the buffer's range.
                let start = buffer.position_to_char(diagnostic.range.start).ok()?;
                let end = buffer.position_to_char(diagnostic.range.end).ok()?;
                let end = if start == end { end + 1 } else { end };
                let char_index_range = (start..end).into();

                let style_source = match diagnostic.severity {
                    Some(DiagnosticSeverity::ERROR) => StyleKey::DiagnosticsError,
                    Some(DiagnosticSeverity::WARNING) => StyleKey::DiagnosticsWarning,
                    Some(DiagnosticSeverity::INFORMATION) => StyleKey::DiagnosticsInformation,
                    Some(DiagnosticSeverity::HINT) => StyleKey::DiagnosticsHint,
                    _ => StyleKey::DiagnosticsDefault,
                };
                Some(range_to_cell_update(
                    &buffer,
                    char_index_range,
                    theme,
                    style_source,
                ))
            })
            .flatten();
        let jumps = editor
            .jumps()
            .into_iter()
            .enumerate()
            .filter_map(|(index, jump)| {
                let position = buffer
                    .char_to_position(jump.selection.to_char_index(&self.cursor_direction))
                    .ok()?;

                let style = if index % 2 == 0 {
                    theme.ui.jump_mark_even
                } else {
                    theme.ui.jump_mark_odd
                };

                Some(
                    CellUpdate::new(position)
                        .style(style)
                        .symbol(jump.character.to_string()),
                )
            });
        let extra_decorations = buffer
            .decorations()
            .iter()
            .flat_map(|decoration| {
                Some(range_to_cell_update(
                    &buffer,
                    decoration
                        .selection_range()
                        .to_char_index_range(&buffer)
                        .ok()?,
                    theme,
                    *decoration.style_key(),
                ))
            })
            .flatten()
            .collect_vec();
        let highlighted_spans = highlighted_spans
            .iter()
            .flat_map(|highlighted_span| {
                highlighted_span.byte_range.clone().filter_map(|byte| {
                    Some(
                        CellUpdate::new(buffer.byte_to_position(byte).ok()?)
                            .style(highlighted_span.style)
                            .source(highlighted_span.source),
                    )
                })
            })
            .collect_vec();

        let updates = vec![]
            .into_iter()
            .chain(highlighted_spans)
            .chain(extra_decorations)
            .chain(primary_selection_primary_cursor)
            .chain(possible_selections)
            .chain(primary_selection)
            .chain(secondary_selection)
            .chain(primary_selection_anchors)
            .chain(seconday_selection_anchors)
            .chain(bookmarks)
            .chain(diagnostics)
            .chain(jumps)
            .chain(primary_selection_secondary_cursor)
            .chain(secondary_selection_cursors);

        #[derive(Debug, Clone)]
        struct RenderLine {
            line_number: usize,
            content: String,
            wrapped: bool,
        }

        let render_lines = |grid: Grid, lines: Vec<RenderLine>| {
            lines.into_iter().enumerate().fold(
                grid,
                |grid,
                 (
                    line_index,
                    RenderLine {
                        line_number,
                        content: line,
                        wrapped,
                    },
                )| {
                    let background_color = if parent_lines_numbers.iter().contains(&line_number) {
                        Some(theme.ui.parent_lines_background)
                    } else {
                        None
                    };
                    let line_number_str = {
                        let line_number = if wrapped {
                            "↪".to_string()
                        } else {
                            (line_number + 1).to_string()
                        };
                        format!(
                            "{: >width$}",
                            line_number.to_string(),
                            width = max_line_number_len as usize
                        )
                    };
                    Grid::new(Dimension {
                        height,
                        width: max_line_number_len,
                    });
                    grid.set_row(
                        line_index,
                        Some(0),
                        Some(max_line_number_len as usize),
                        &line_number_str,
                        &theme
                            .ui
                            .line_number
                            .set_some_background_color(background_color),
                    )
                    .set_row(
                        line_index,
                        Some(max_line_number_len as usize),
                        Some((max_line_number_len + 1) as usize),
                        "│",
                        &theme
                            .ui
                            .line_number_separator
                            .set_some_background_color(background_color),
                    )
                    .set_row(
                        line_index,
                        Some((max_line_number_len + 1) as usize),
                        None,
                        &line.chars().take(width as usize).collect::<String>(),
                        &theme.ui.text.set_some_background_color(background_color),
                    )
                },
            )
        };
        let visible_lines_updates = updates
            .clone()
            .filter_map(|update| {
                let update = update.move_up((scroll_offset).into())?;

                let position = wrapped_lines.calibrate(update.position).ok()?;

                let position =
                    position.move_right(max_line_number_len + line_number_separator_width);

                Some(CellUpdate { position, ..update })
            })
            .collect::<Vec<_>>();
        let visible_render_lines = if lines.is_empty() {
            [RenderLine {
                line_number: 0,
                content: String::new(),
                wrapped: false,
            }]
            .to_vec()
        } else {
            lines
        };

        let visible_lines_grid = render_lines(visible_lines_grid, visible_render_lines)
            .apply_cell_updates(visible_lines_updates);

        let (hidden_parent_lines_grid, hidden_parent_lines_updates) = {
            let height = hidden_parent_lines.len() as u16;
            let hidden_parent_lines = hidden_parent_lines
                .iter()
                .map(|line| RenderLine {
                    line_number: line.line,
                    content: line.content.clone(),
                    wrapped: false,
                })
                .collect_vec();
            let updates = {
                let hidden_parent_lines_with_index =
                    hidden_parent_lines.iter().enumerate().collect_vec();
                updates
                    .filter_map(|update| {
                        if let Some((index, _)) = hidden_parent_lines_with_index
                            .iter()
                            .find(|(_, line)| &update.position.line == &line.line_number)
                        {
                            Some(
                                update
                                    .set_position_line(*index)
                                    .move_right(max_line_number_len + line_number_separator_width),
                            )
                        } else {
                            None
                        }
                    })
                    .collect_vec()
            };

            let grid = render_lines(
                Grid::new(Dimension {
                    width: editor.dimension().width,
                    height,
                }),
                hidden_parent_lines,
            );
            (grid, updates)
        };
        let hidden_parent_lines_grid =
            hidden_parent_lines_grid.apply_cell_updates(hidden_parent_lines_updates);

        let cursor_beyond_view_bottom =
            if let Some(cursor_position) = visible_lines_grid.get_cursor_position() {
                cursor_position
                    .line
                    .saturating_sub(height.saturating_sub(1).saturating_sub(top_offset) as usize)
            } else {
                0
            };
        let grid = {
            let visible_lines_grid = visible_lines_grid.clamp_top(cursor_beyond_view_bottom);
            let clamp_bottom_by = visible_lines_grid
                .dimension()
                .height
                .saturating_sub(height)
                .saturating_add(top_offset)
                .saturating_sub(cursor_beyond_view_bottom as u16);
            let bottom = visible_lines_grid.clamp_bottom(clamp_bottom_by);

            hidden_parent_lines_grid.merge_vertical(bottom)
        };
        let window_title_style = theme.ui.window_title;

        // NOTE: due to performance issue, we only highlight the content that are within view
        // This might result in some incorrectness, but that's a reasonable trade-off, because
        // highlighting the entire file becomes sluggish when the file has more than a thousand lines.

        let title_grid = Grid::new(Dimension {
            height: 1,
            width: editor.dimension().width,
        })
        .set_line(0, &self.title(context), &window_title_style);

        let grid = title_grid.merge_vertical(grid);
        let cursor_position = grid.get_cursor_position();

        GetGridResult {
            cursor: cursor_position.map(|position| {
                Cursor::new(
                    position,
                    if self.mode == Mode::Insert {
                        crossterm::cursor::SetCursorStyle::BlinkingBar
                    } else {
                        crossterm::cursor::SetCursorStyle::BlinkingBlock
                    },
                )
            }),
            grid,
        }
    }

    fn handle_paste_event(&mut self, content: String) -> anyhow::Result<Vec<Dispatch>> {
        self.insert(&content)
    }

    fn get_cursor_position(&self) -> anyhow::Result<Position> {
        self.buffer
            .borrow()
            .char_to_position(self.get_cursor_char_index())
    }

    fn scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    fn set_rectangle(&mut self, rectangle: Rectangle) {
        self.rectangle = rectangle;
        self.recalculate_scroll_offset();
    }

    fn rectangle(&self) -> &Rectangle {
        &self.rectangle
    }

    fn children(&self) -> Vec<Option<Rc<RefCell<dyn Component>>>> {
        vec![]
    }

    fn remove_child(&mut self, _component_id: ComponentId) {}

    fn handle_key_event(
        &mut self,
        context: &Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        self.handle_key_event(context, event)
    }

    fn handle_mouse_event(
        &mut self,
        mouse_event: crossterm::event::MouseEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        const SCROLL_HEIGHT: usize = 1;
        match mouse_event.kind {
            MouseEventKind::ScrollUp => {
                self.apply_scroll(Direction::Start, SCROLL_HEIGHT);
                Ok(vec![])
            }
            MouseEventKind::ScrollDown => {
                self.apply_scroll(Direction::End, SCROLL_HEIGHT);
                Ok(vec![])
            }
            MouseEventKind::Down(MouseButton::Left) => {
                Ok(vec![])

                // self
                // .set_cursor_position(mouse_event.row + window.scroll_offset(), mouse_event.column)
            }
            _ => Ok(vec![]),
        }
    }

    #[cfg(test)]
    fn handle_events(&mut self, events: &[event::KeyEvent]) -> anyhow::Result<Vec<Dispatch>> {
        let context = Context::default();
        Ok(events
            .iter()
            .map(|event| self.handle_key_event(&context, event.clone()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>())
    }

    fn handle_event(
        &mut self,
        context: &Context,
        event: event::event::Event,
    ) -> anyhow::Result<Vec<Dispatch>> {
        match event {
            event::event::Event::Key(event) => self.handle_key_event(context, event),
            event::event::Event::Paste(content) => self.handle_paste_event(content),
            event::event::Event::Mouse(event) => self.handle_mouse_event(event),
            _ => Ok(vec![]),
        }
    }

    fn descendants(&self) -> Vec<Rc<RefCell<dyn Component>>> {
        self.children()
            .into_iter()
            .flatten()
            .flat_map(|component| {
                std::iter::once(component.clone()).chain(component.borrow().descendants())
            })
            .collect::<Vec<_>>()
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
        }
    }
}

pub struct Editor {
    pub mode: Mode,

    pub selection_set: SelectionSet,

    pub jumps: Option<Vec<Jump>>,
    pub cursor_direction: Direction,

    /// This means the number of lines to be skipped from the top during rendering.
    /// 2 means the first line to be rendered on the screen if the 3rd line of the text.
    scroll_offset: u16,
    rectangle: Rectangle,

    buffer: Rc<RefCell<Buffer>>,
    title: Option<String>,
    id: ComponentId,
    pub current_view_alignment: Option<ViewAlignment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    Start,
    End,
}

impl Direction {
    pub fn reverse(&self) -> Self {
        match self {
            Direction::Start => Direction::End,
            Direction::End => Direction::Start,
        }
    }

    pub(crate) fn default() -> Direction {
        Direction::Start
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Movement {
    Next,
    Previous,
    Last,
    Current,
    Up,
    Down,
    First,
    /// 0-based
    Index(usize),
    Jump(CharIndexRange),
}

impl Editor {
    /// Returns (hidden_parent_lines, visible_parent_lines)
    pub fn get_parent_lines(&self) -> anyhow::Result<(Vec<Line>, Vec<Line>)> {
        let position = self.get_cursor_position()?;

        let parent_lines = self.buffer().get_parent_lines(position.line)?;
        Ok(parent_lines
            .into_iter()
            .partition(|line| line.line < self.scroll_offset as usize))
    }

    pub fn show_info(&mut self, info: Info) {
        self.set_decorations(info.decorations());
        self.set_content(info.content());
    }

    pub fn from_text(language: tree_sitter::Language, text: &str) -> Self {
        Self {
            selection_set: SelectionSet {
                primary: Selection::default(),
                secondary: vec![],
                mode: SelectionMode::Custom,
                filters: Filters::default(),
            },
            jumps: None,
            mode: Mode::Normal,
            cursor_direction: Direction::Start,
            scroll_offset: 0,
            rectangle: Rectangle::default(),
            buffer: Rc::new(RefCell::new(Buffer::new(language, text))),
            title: None,
            id: ComponentId::new(),
            current_view_alignment: None,
        }
    }

    pub fn from_buffer(buffer: Rc<RefCell<Buffer>>) -> Self {
        Self {
            selection_set: SelectionSet {
                primary: Selection::default(),
                secondary: vec![],
                mode: SelectionMode::Line,
                filters: Filters::default(),
            },
            jumps: None,
            mode: Mode::Normal,
            cursor_direction: Direction::Start,
            scroll_offset: 0,
            rectangle: Rectangle::default(),
            buffer,
            title: None,
            id: ComponentId::new(),
            current_view_alignment: None,
        }
    }

    pub fn current_line(&self) -> anyhow::Result<String> {
        let cursor = self.get_cursor_char_index();
        Ok(self
            .buffer
            .borrow()
            .get_line_by_char_index(cursor)?
            .to_string()
            .trim()
            .into())
    }

    pub fn get_current_word(&self) -> anyhow::Result<String> {
        let cursor = self.get_cursor_char_index();
        self.buffer.borrow().get_word_before_char_index(cursor)
    }

    pub fn select_kids(&mut self) -> anyhow::Result<Vec<Dispatch>> {
        let buffer = self.buffer.borrow().clone();

        Ok(self.update_selection_set(
            self.selection_set
                .select_kids(&buffer, &self.cursor_direction)?,
        ))
    }

    pub fn select_line(
        &mut self,
        movement: Movement,
        context: &Context,
    ) -> anyhow::Result<Vec<Dispatch>> {
        self.select(SelectionMode::Line, movement, context)
    }

    pub fn select_line_at(&mut self, line: usize) -> anyhow::Result<Vec<Dispatch>> {
        let start = self.buffer.borrow().line_to_char(line)?;
        let selection_set = SelectionSet {
            primary: Selection::new(
                (start
                    ..start
                        + self
                            .buffer
                            .borrow()
                            .get_line_by_char_index(start)?
                            .len_chars())
                    .into(),
            ),
            secondary: vec![],
            mode: SelectionMode::Line,
            filters: Filters::default(),
        };
        Ok(self.update_selection_set(selection_set))
    }

    pub fn reset(&mut self) {
        self.selection_set.escape_highlight_mode();
    }

    pub fn update_selection_set(&mut self, selection_set: SelectionSet) -> Vec<Dispatch> {
        let old_selection_set = std::mem::replace(&mut self.selection_set, selection_set.clone());
        self.recalculate_scroll_offset();
        self.buffer()
            .path()
            .map(|path| Dispatch::PushSelectionSet {
                new_selection_set: selection_set,
                old_selection_set,
                path,
            })
            .into_iter()
            .collect()
    }

    pub fn set_selection(&mut self, range: Range<Position>) -> anyhow::Result<Vec<Dispatch>> {
        let range = (self.buffer().position_to_char(range.start)?
            ..self.buffer().position_to_char(range.end)?)
            .into();

        let mode = if self.buffer().given_range_is_node(&range) {
            SelectionMode::SyntaxTree
        } else {
            SelectionMode::Custom
        };
        let primary = self.selection_set.primary.clone().set_range(range);
        let selection_set = SelectionSet {
            primary,
            secondary: vec![],
            mode,
            filters: Filters::default(),
        };
        Ok(self.update_selection_set(selection_set))
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

    pub fn align_cursor_to_bottom(&mut self) {
        self.scroll_offset = self.cursor_row().saturating_sub(
            self.rectangle
                .height
                .saturating_sub(1)
                .saturating_sub(WINDOW_TITLE_HEIGHT as u16),
        );
    }

    pub fn align_cursor_to_top(&mut self) {
        self.scroll_offset = self.cursor_row();
    }

    fn align_cursor_to_center(&mut self) {
        self.scroll_offset = self
            .cursor_row()
            .saturating_sub((self.rectangle.height as f64 / 2.0).ceil() as u16);
    }

    pub fn select(
        &mut self,
        selection_mode: SelectionMode,
        movement: Movement,
        context: &Context,
    ) -> anyhow::Result<Vec<Dispatch>> {
        //  There are a few selection modes where Current make sense.
        let direction = if self.selection_set.mode != selection_mode {
            Movement::Current
        } else {
            movement
        };

        let selection_set = self.get_selection_set(&selection_mode, direction, context)?;

        Ok(self.update_selection_set(selection_set))
    }

    fn jump_characters() -> Vec<char> {
        ('a'..='z').chain('A'..='Z').chain('0'..'9').collect_vec()
    }

    fn get_selection_mode_trait_object(
        &self,
        selection: &Selection,
        context: &Context,
    ) -> anyhow::Result<Box<dyn selection_mode::SelectionMode>> {
        self.selection_set.mode.to_selection_mode_trait_object(
            &self.buffer(),
            selection,
            &self.cursor_direction,
            context,
            &self.selection_set.filters,
        )
    }

    fn possible_selections_in_line_number_range(
        &self,
        selection: &Selection,
        context: &Context,
    ) -> anyhow::Result<Vec<ByteRange>> {
        let object = self.get_selection_mode_trait_object(selection, context)?;
        if self.selection_set.mode.is_contiguous() && self.selection_set.filters.is_empty() {
            return Ok(Vec::new());
        }

        let line_range = self.line_range();
        object.selections_in_line_number_range(
            &selection_mode::SelectionModeParams {
                context,
                buffer: &self.buffer(),
                current_selection: selection,
                cursor_direction: &self.cursor_direction,
                filters: &self.selection_set.filters,
            },
            line_range,
        )
    }

    fn jump_from_selection(
        &mut self,
        selection: &Selection,
        context: &Context,
    ) -> anyhow::Result<()> {
        let chars = Self::jump_characters();

        let object = self.get_selection_mode_trait_object(selection, context)?;

        let line_range = self.line_range();
        let jumps = object.jumps(
            selection_mode::SelectionModeParams {
                context,
                buffer: &self.buffer(),
                current_selection: selection,
                cursor_direction: &self.cursor_direction,
                filters: &self.selection_set.filters,
            },
            chars,
            line_range,
        )?;
        self.jumps = Some(jumps);

        Ok(())
    }

    pub fn jump(&mut self, context: &Context) -> anyhow::Result<()> {
        self.jump_from_selection(&self.selection_set.primary.clone(), context)
    }

    pub fn cut(&mut self) -> anyhow::Result<Vec<Dispatch>> {
        let edit_transaction = EditTransaction::from_action_groups({
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let current_range = selection.extended_range();
                    let copied_text = Some(self.buffer.borrow().slice(&current_range)?);
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit {
                                range: current_range,
                                new: Rope::new(),
                            }),
                            Action::Select(
                                Selection::new((current_range.start..current_range.start).into())
                                    .set_copied_text(copied_text),
                            ),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect()
        });
        // Set the clipboard content to the current selection
        // if there is only one cursor.
        let dispatch = if self.selection_set.secondary.is_empty() {
            Some(Dispatch::SetClipboardContent(
                self.buffer
                    .borrow()
                    .slice(&self.selection_set.primary.extended_range())?
                    .into(),
            ))
        } else {
            None
        };
        let dispatches = self
            .apply_edit_transaction(edit_transaction)
            .map(|dispatches| dispatches.into_iter().chain(dispatch).collect());
        self.enter_insert_mode(Direction::Start)?;
        dispatches
    }

    pub fn kill(&mut self, context: &Context) -> anyhow::Result<Vec<Dispatch>> {
        let edit_transaction = EditTransaction::from_action_groups({
            let buffer = self.buffer();
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let current_range = selection.extended_range();
                    // If the gap between the next selection and the current selection are only whitespaces, perform a "kill next" instead
                    let next_selection = Selection::get_selection_(
                        &buffer,
                        selection,
                        &self.selection_set.mode,
                        &Movement::Next,
                        &self.cursor_direction,
                        context,
                        &self.selection_set.filters,
                    )?
                    .selection;

                    let next_range = next_selection.extended_range();

                    let (delete_range, select_range) = {
                        let default = {
                            let start = current_range.start;
                            (current_range, (start..start + 1).into())
                        };
                        if current_range.end > next_range.start {
                            default
                        } else {
                            let inbetween_range: CharIndexRange =
                                (current_range.end..next_range.start).into();

                            let inbetween_text = buffer.slice(&inbetween_range)?.to_string();
                            if !inbetween_text.trim().is_empty()
                                && !self.selection_set.mode.is_contiguous()
                            {
                                default
                            } else {
                                let delete_range: CharIndexRange = (current_range.start
                                    ..next_selection.extended_range().start)
                                    .into();
                                (delete_range, {
                                    next_selection
                                        .extended_range()
                                        .shift_left(delete_range.len())
                                })
                            }
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

        self.apply_edit_transaction(edit_transaction)
    }

    pub fn copy(&mut self, context: &Context) -> anyhow::Result<Vec<Dispatch>> {
        self.selection_set.copy(&self.buffer.borrow(), context)
    }

    fn replace_current_selection_with<F>(&mut self, f: F) -> anyhow::Result<Vec<Dispatch>>
    where
        F: Fn(&Selection) -> Option<Rope>,
    {
        let edit_transactions = self.selection_set.map(|selection| {
            if let Some(copied_text) = f(selection) {
                let range = selection.extended_range();
                let start = range.start;
                EditTransaction::from_action_groups(
                    [ActionGroup::new(
                        [
                            Action::Edit(Edit {
                                range,
                                new: copied_text.clone(),
                            }),
                            Action::Select(
                                Selection::new({
                                    let start = start + copied_text.len_chars();
                                    (start..start).into()
                                })
                                .set_copied_text(Some(copied_text)),
                            ),
                        ]
                        .to_vec(),
                    )]
                    .to_vec(),
                )
            } else {
                EditTransaction::from_action_groups(vec![])
            }
        });
        let edit_transaction = EditTransaction::merge(edit_transactions);
        self.apply_edit_transaction(edit_transaction)
    }

    pub fn paste(&mut self, context: &Context) -> anyhow::Result<Vec<Dispatch>> {
        self.replace_current_selection_with(|selection| selection.copied_text(context))
    }

    pub fn replace(&mut self, context: &Context) -> anyhow::Result<Vec<Dispatch>> {
        let edit_transaction = EditTransaction::merge(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    if let Some(replacement) = &selection.copied_text(context) {
                        let replacement_text_len = replacement.len_chars();
                        let range = selection.extended_range();
                        let replaced_text = self.buffer.borrow().slice(&range)?;
                        Ok(EditTransaction::from_action_groups(
                            [ActionGroup::new(
                                [
                                    Action::Edit(Edit {
                                        range,
                                        new: replacement.clone(),
                                    }),
                                    Action::Select(
                                        Selection::new(
                                            (range.start..range.start + replacement_text_len)
                                                .into(),
                                        )
                                        .set_copied_text(Some(replaced_text)),
                                    ),
                                ]
                                .to_vec(),
                            )]
                            .to_vec(),
                        ))
                    } else {
                        Ok(EditTransaction::from_action_groups(vec![]))
                    }
                })
                .into_iter()
                .flatten()
                .collect(),
        );

        self.apply_edit_transaction(edit_transaction)
    }

    fn apply_edit_transaction(
        &mut self,
        edit_transaction: EditTransaction,
    ) -> anyhow::Result<Vec<Dispatch>> {
        let new_selection_set = self
            .buffer
            .borrow_mut()
            .apply_edit_transaction(&edit_transaction, self.selection_set.clone())?;

        self.selection_set = new_selection_set;

        self.recalculate_scroll_offset();

        Ok(self.get_document_did_change_dispatch())
    }

    pub fn get_document_did_change_dispatch(&mut self) -> Vec<Dispatch> {
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
    }

    pub fn enter_undo_tree_mode(&mut self) -> Vec<Dispatch> {
        self.mode = Mode::UndoTree;
        [self.show_undo_tree_dispatch()].to_vec()
    }

    pub fn show_undo_tree_dispatch(&self) -> Dispatch {
        Dispatch::ShowInfo {
            title: "Undo Tree History".to_string(),
            info: Info::new(self.buffer().display_history()),
        }
    }

    pub fn undo(&mut self) -> anyhow::Result<Vec<Dispatch>> {
        let selection_set = self.buffer.borrow_mut().undo()?;
        if let Some(selection_set) = selection_set {
            self.update_selection_set(selection_set);
        }
        Ok(self.get_document_did_change_dispatch())
    }

    pub fn redo(&mut self) -> anyhow::Result<Vec<Dispatch>> {
        let selection_set = self.buffer.borrow_mut().redo()?;
        if let Some(selection_set) = selection_set {
            self.update_selection_set(selection_set);
        }
        Ok(self.get_document_did_change_dispatch())
    }

    fn change_cursor_direction(&mut self) {
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
        context: &Context,
    ) -> anyhow::Result<SelectionSet> {
        self.selection_set.generate(
            &self.buffer.borrow(),
            mode,
            &movement,
            &self.cursor_direction,
            context,
        )
    }

    fn get_cursor_char_index(&self) -> CharIndex {
        self.selection_set
            .primary
            .to_char_index(&self.cursor_direction)
    }

    pub fn toggle_highlight_mode(&mut self) {
        self.selection_set.toggle_highlight_mode();
        self.recalculate_scroll_offset()
    }
    fn search_keymap(&self, scope: Scope) -> Keymap {
        Keymap::new(
            "s",
            "Search",
            Dispatch::ShowKeymapLegend(KeymapLegendConfig {
                title: Self::find_submenu_title("Search", scope),
                keymaps: [
                    ("a", "Ast Grep", SearchKind::AstGrep),
                    ("l", "Literal", SearchKind::Literal),
                    (
                        "shift+L",
                        "Literal (Case-sensitive)",
                        SearchKind::LiteralCaseSensitive,
                    ),
                    ("r", "Regex", SearchKind::Regex),
                    (
                        "shift+R",
                        "Regex (Case-sensitive)",
                        SearchKind::RegexCaseSensitive,
                    ),
                ]
                .into_iter()
                .map(|(key, description, kind)| {
                    Keymap::new(key, description, Dispatch::OpenSearchPrompt(kind, scope))
                })
                .chain(
                    self.current_selection()
                        .map(|search| {
                            Keymap::new(
                                "c",
                                "Current selection",
                                Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                                    SelectionMode::Find {
                                        search: Search {
                                            kind: SearchKind::RegexCaseSensitive,
                                            search: format!("\\b{search}\\b"),
                                        },
                                    },
                                )),
                            )
                        })
                        .ok(),
                )
                .collect_vec(),
                owner_id: self.id(),
            }),
        )
    }

    fn x_mode_keymap_legend_config(&self) -> anyhow::Result<KeymapLegendConfig> {
        Ok(KeymapLegendConfig {
            title: "X (Regex/Bracket/Quote)".to_string(),
            owner_id: self.id(),
            keymaps: [
                Keymap::new(
                    "e",
                    "Empty line",
                    Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                        SelectionMode::EmptyLine,
                    )),
                ),
                Keymap::new(
                    "n",
                    "Number",
                    Dispatch::ShowKeymapLegend(KeymapLegendConfig {
                        title: "Find number".to_string(),
                        owner_id: self.id(),
                        keymaps: [
                            ("b", "Binary", r"\b[01]+\b"),
                            ("f", "Float", r"[-+]?\d*\.\d+|\d+"),
                            ("h", "Hexadecimal", r"[0-9a-fA-F]+"),
                            ("i", "Integer", r"-?\d+"),
                            ("n", "Natural", r"\d+"),
                            ("o", "Octal", r"\b[0-7]+\b"),
                            ("s", "Scientific", r"[-+]?\d*\.?\d+[eE][-+]?\d+"),
                        ]
                        .into_iter()
                        .map(|(key, description, regex)| {
                            let search = Search {
                                search: regex.to_string(),
                                kind: SearchKind::Regex,
                            };
                            let dispatch = Dispatch::DispatchEditor(
                                DispatchEditor::SetSelectionMode(SelectionMode::Find { search }),
                            );
                            Keymap::new(key, description, dispatch)
                        })
                        .collect_vec(),
                    }),
                ),
                Keymap::new(
                    "o",
                    "One character",
                    Dispatch::DispatchEditor(DispatchEditor::FindOneChar),
                ),
            ]
            .into_iter()
            .collect_vec(),
        })
    }

    fn diagnostics_keymap(&self, scope: Scope) -> Keymap {
        let keymaps = [
            ("a", "Any", None),
            ("e", "Error", Some(DiagnosticSeverity::ERROR)),
            ("h", "Hint", Some(DiagnosticSeverity::HINT)),
            ("i", "Information", Some(DiagnosticSeverity::INFORMATION)),
            ("w", "Warning", Some(DiagnosticSeverity::WARNING)),
        ]
        .into_iter()
        .into_iter()
        .map(|(char, description, severity)| {
            Keymap::new(
                char,
                description,
                match scope {
                    Scope::Local => Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                        SelectionMode::Diagnostic(severity),
                    )),
                    Scope::Global => {
                        Dispatch::SetQuickfixList(QuickfixListType::LspDiagnostic(severity))
                    }
                },
            )
        })
        .collect_vec();
        Keymap::new(
            "d",
            "Diagnostics",
            Dispatch::ShowKeymapLegend(KeymapLegendConfig {
                title: Self::find_submenu_title("Diagnostic", scope),
                keymaps,
                owner_id: self.id(),
            }),
        )
    }

    fn find_local_keymap_legend_config(
        &self,
        context: &Context,
    ) -> anyhow::Result<KeymapLegendConfig> {
        let owner_id = self.id();
        let scope = Scope::Local;
        Ok(KeymapLegendConfig {
            title: Self::find_submenu_title("", scope),
            owner_id,
            keymaps: [
                Keymap::new(
                    "b",
                    "Bookmark",
                    Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                        SelectionMode::Bookmark,
                    )),
                ),
                Keymap::new(
                    "g",
                    "Git hunk",
                    Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                        SelectionMode::GitHunk,
                    )),
                ),
                Keymap::new(
                    "q",
                    "Quickfix list (current)",
                    Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                        SelectionMode::LocalQuickfix {
                            title: "LOCAL QUICKFIX".to_string(),
                        },
                    )),
                ),
            ]
            .into_iter()
            .chain(Some(self.search_keymap(scope)))
            .chain(Some(self.diagnostics_keymap(scope)))
            .chain(Some(self.lsp_keymap(scope)))
            .chain(context.last_search().map(|search| {
                Keymap::new(
                    "f",
                    "Enter Find Mode (using previous search)",
                    Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                        SelectionMode::Find { search },
                    )),
                )
            }))
            .collect_vec(),
        })
    }

    fn space_mode_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Space".to_string(),
            owner_id: self.id(),
            keymaps: vec![]
                .into_iter()
                .chain(
                    self.get_request_params()
                        .map(|params| {
                            [
                                Keymap::new(
                                    "e",
                                    "Explorer",
                                    Dispatch::ShowKeymapLegend(KeymapLegendConfig {
                                        title: "Space: Explorer".to_string(),
                                        keymaps: [Keymap::new(
                                            "r",
                                            "Reveal in Explorer",
                                            Dispatch::RevealInExplorer(params.path.clone()),
                                        )]
                                        .to_vec(),
                                        owner_id: self.id(),
                                    }),
                                ),
                                Keymap::new(
                                    "l",
                                    "LSP",
                                    Dispatch::ShowKeymapLegend(KeymapLegendConfig {
                                        title: "Space: LSP".to_string(),
                                        keymaps: [
                                            Keymap::new(
                                                "h",
                                                "Hover",
                                                Dispatch::RequestHover(params.clone()),
                                            ),
                                            Keymap::new(
                                                "r",
                                                "Rename",
                                                Dispatch::PrepareRename(params.clone()),
                                            ),
                                            Keymap::new(
                                                "a",
                                                "Code Actions",
                                                Dispatch::RequestCodeAction {
                                                    params,
                                                    diagnostics: context
                                                        .get_diagnostics(self.path())
                                                        .into_iter()
                                                        .filter_map(|diagnostic| {
                                                            if diagnostic.range.contains(
                                                                &self.get_cursor_position().ok()?,
                                                            ) {
                                                                diagnostic.original_value.clone()
                                                            } else {
                                                                None
                                                            }
                                                        })
                                                        .collect_vec(),
                                                },
                                            ),
                                        ]
                                        .to_vec(),
                                        owner_id: self.id(),
                                    }),
                                ),
                                Keymap::new(
                                    "t",
                                    "Transform",
                                    Dispatch::ShowKeymapLegend(
                                        self.transform_keymap_legend_config(),
                                    ),
                                ),
                                Keymap::new(
                                    "z",
                                    "Undo Tree",
                                    Dispatch::DispatchEditor(DispatchEditor::EnterUndoTreeMode),
                                ),
                            ]
                            .to_vec()
                        })
                        .unwrap_or_default(),
                )
                .collect_vec(),
        }
    }

    fn transform_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Transform".to_string(),
            owner_id: self.id(),
            keymaps: [
                ("a", "aLtErNaTiNg CaSe", Case::Toggle),
                ("c", "camelCase", Case::Camel),
                ("l", "lowercase", Case::Lower),
                ("k", "kebab-case", Case::Kebab),
                ("shift+K", "Upper-Kebab", Case::UpperKebab),
                ("p", "PascalCase", Case::Pascal),
                ("s", "snake_case", Case::Snake),
                ("m", "MARCO_CASE", Case::UpperSnake),
                ("t", "Title Case", Case::Title),
                ("u", "UPPERCASE", Case::Upper),
            ]
            .into_iter()
            .map(|(key, description, case)| {
                Keymap::new(
                    key,
                    description,
                    Dispatch::DispatchEditor(DispatchEditor::Transform(case)),
                )
            })
            .collect_vec(),
        }
    }

    pub fn apply_dispatch(
        &mut self,
        context: &Context,
        dispatch: DispatchEditor,
    ) -> anyhow::Result<Vec<Dispatch>> {
        match dispatch {
            DispatchEditor::AlignViewTop => self.align_cursor_to_top(),
            DispatchEditor::AlignViewCenter => self.align_cursor_to_center(),
            DispatchEditor::AlignViewBottom => self.align_cursor_to_bottom(),
            DispatchEditor::Transform(case) => return self.transform_selection(case),
            DispatchEditor::SetSelectionMode(selection_mode) => {
                return self.set_selection_mode(context, selection_mode);
            }

            DispatchEditor::FindOneChar => self.enter_single_character_mode(),

            DispatchEditor::MoveSelection(direction) => {
                return self.handle_movement(context, direction)
            }
            DispatchEditor::Copy => return self.copy(context),
            DispatchEditor::Paste => return self.paste(context),
            DispatchEditor::SelectAll => self.select_all(context)?,
            DispatchEditor::SetContent(content) => self.update_buffer(&content),
            DispatchEditor::Replace => return self.replace(context),
            DispatchEditor::Cut => return self.cut(),
            DispatchEditor::ToggleHighlightMode => self.toggle_highlight_mode(),
            DispatchEditor::EnterUndoTreeMode => return Ok(self.enter_undo_tree_mode()),
            DispatchEditor::EnterInsertMode(direction) => self.enter_insert_mode(direction)?,
            DispatchEditor::Kill => return self.kill(context),
            DispatchEditor::Insert(string) => return self.insert(&string),
            DispatchEditor::MatchLiteral(literal) => return self.match_literal(context, &literal),
            DispatchEditor::ToggleBookmark => self.toggle_bookmarks(),
            DispatchEditor::EnterInsideMode(kind) => {
                return self.set_selection_mode(context, SelectionMode::Inside(kind))
            }
            DispatchEditor::EnterNormalMode => self.enter_normal_mode()?,
            DispatchEditor::FilterPush(filter) => return self.filters_push(context, filter),
            DispatchEditor::CursorAddToAllSelections => {
                self.add_cursor_to_all_selections(context)?
            }
            DispatchEditor::FilterClear => self.filters_clear(),
            DispatchEditor::CursorKeepPrimaryOnly => self.cursor_keep_primary_only()?,
            DispatchEditor::Raise => return self.raise(context),
            DispatchEditor::Exchange(movement) => return self.exchange(&context, movement),
            DispatchEditor::EnterExchangeMode => self.enter_exchange_mode(),
        }
        Ok([].to_vec())
    }

    fn list_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "List".to_string(),
            owner_id: self.id(),
            keymaps: [
                ("g", "Git status", FilePickerKind::GitStatus),
                ("n", "Not git ignored files", FilePickerKind::NonGitIgnored),
                ("o", "Opened files", FilePickerKind::Opened),
            ]
            .into_iter()
            .map(|(key, description, kind)| {
                Keymap::new(key, description, Dispatch::OpenFilePicker(kind))
            })
            .collect_vec(),
        }
    }

    fn lsp_keymap(&self, scope: Scope) -> Keymap {
        let keymaps = self
            .get_request_params()
            .map(|params| {
                let params = params.set_kind(Some(scope));
                [
                    Keymap::new(
                        "c",
                        "Declarations",
                        Dispatch::RequestDeclarations(
                            params.clone().set_description("Declarations"),
                        ),
                    ),
                    Keymap::new(
                        "d",
                        "Definitions",
                        Dispatch::RequestDefinitions(params.clone().set_description("Definitions")),
                    ),
                    Keymap::new(
                        "i",
                        "Implementations",
                        Dispatch::RequestImplementations(
                            params.clone().set_description("Implementations"),
                        ),
                    ),
                    Keymap::new(
                        "r",
                        "References",
                        Dispatch::RequestReferences {
                            params: params.clone().set_description("References"),
                            include_declaration: false,
                        },
                    ),
                    Keymap::new(
                        "shift+R",
                        "References (include declaration)",
                        Dispatch::RequestReferences {
                            params: params
                                .clone()
                                .set_description("References (include declaration)"),
                            include_declaration: true,
                        },
                    ),
                    Keymap::new(
                        "t",
                        "Type Definitions",
                        Dispatch::RequestTypeDefinitions(
                            params.clone().set_description("Type Definitions"),
                        ),
                    ),
                    Keymap::new(
                        "s",
                        "Symbols",
                        Dispatch::RequestDocumentSymbols(params.set_description("Symbols")),
                    ),
                ]
                .into_iter()
                .collect_vec()
            })
            .unwrap_or_default();

        Keymap::new(
            "l",
            "LSP",
            Dispatch::ShowKeymapLegend(KeymapLegendConfig {
                title: Self::find_submenu_title("LSP", scope),
                keymaps,
                owner_id: self.id(),
            }),
        )
    }

    fn find_global_keymap_legend_config(&self) -> KeymapLegendConfig {
        let scope = Scope::Global;
        KeymapLegendConfig {
            title: Self::find_submenu_title("", scope),
            owner_id: self.id(),
            keymaps: []
                .into_iter()
                .chain(Some(self.search_keymap(scope)))
                .chain(Some(self.diagnostics_keymap(scope)))
                .chain(Some(self.lsp_keymap(scope)))
                .chain(Some(Keymap::new(
                    "g",
                    "Git Hunk",
                    Dispatch::GetRepoGitHunks,
                )))
                .chain(Some(Keymap::new(
                    "b",
                    "Bookmark",
                    Dispatch::SetQuickfixList(QuickfixListType::Bookmark),
                )))
                .collect_vec(),
        }
    }

    pub fn handle_key_event(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        match self.handle_universal_key(context, key_event)? {
            HandleEventResult::Ignored(key_event) => {
                if let Some(jumps) = self.jumps.take() {
                    self.handle_jump_mode(context, key_event, jumps)
                } else {
                    match &self.mode {
                        Mode::Normal => self.handle_normal_mode(context, key_event),
                        Mode::Insert => self.handle_insert_mode(context, key_event),
                        Mode::MultiCursor => self.handle_multi_cursor_mode(context, key_event),
                        Mode::FindOneChar => self.handle_find_one_char_mode(context, key_event),
                        Mode::Exchange => self.handle_exchange_mode(context, key_event),
                        Mode::UndoTree => self.handle_undo_tree_mode(context, key_event),
                    }
                }
            }
            HandleEventResult::Handled(dispatches) => Ok(dispatches),
        }
    }

    fn handle_universal_key(
        &mut self,
        context: &Context,
        event: KeyEvent,
    ) -> anyhow::Result<HandleEventResult> {
        match event {
            key!("left") => {
                self.selection_set.move_left(&self.cursor_direction);
                Ok(HandleEventResult::Handled(vec![]))
            }
            key!("right") => {
                self.selection_set.move_right(&self.cursor_direction);
                Ok(HandleEventResult::Handled(vec![]))
            }
            key!("ctrl+c") => {
                let dispatches = self.copy(context)?;
                Ok(HandleEventResult::Handled(dispatches))
            }
            key!("ctrl+d") => {
                self.scroll_page_down()?;
                Ok(HandleEventResult::Handled(vec![]))
            }
            key!("ctrl+l") => {
                self.switch_view_alignment();
                Ok(HandleEventResult::Handled(vec![]))
            }
            key!("ctrl+s") => {
                let dispatches = self.save()?;
                self.mode = Mode::Normal;
                Ok(HandleEventResult::Handled(dispatches))
            }
            key!("ctrl+u") => {
                self.scroll_page_up()?;
                Ok(HandleEventResult::Handled(vec![]))
            }
            key!("ctrl+x") => Ok(HandleEventResult::Handled(self.cut()?)),
            key!("ctrl+v") => Ok(HandleEventResult::Handled(self.paste(context)?)),
            key!("ctrl+y") => Ok(HandleEventResult::Handled(self.redo()?)),
            key!("ctrl+z") => Ok(HandleEventResult::Handled(self.undo()?)),
            _ => Ok(HandleEventResult::Ignored(event)),
        }
    }

    fn handle_jump_mode(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
        jumps: Vec<Jump>,
    ) -> anyhow::Result<Vec<Dispatch>> {
        match key_event {
            key!("esc") => {
                self.jumps = None;
                Ok(Vec::new())
            }
            key => {
                let KeyCode::Char(c) = key.code else {
                    return Ok(Vec::new());
                };
                let matching_jumps = jumps
                    .iter()
                    .filter(|jump| c == jump.character)
                    .collect_vec();
                match matching_jumps.split_first() {
                    None => Ok(Vec::new()),
                    Some((jump, [])) => self
                        .handle_movement(context, Movement::Jump(jump.selection.extended_range())),
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
                        Ok(Vec::new())
                    }
                }
            }
        }
    }

    /// Similar to Change in Vim, but does not copy the current selection
    pub fn change(&mut self) -> anyhow::Result<Vec<Dispatch>> {
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

        let dispatches = self.apply_edit_transaction(edit_transaction)?;
        self.enter_insert_mode(Direction::Start)?;
        Ok(dispatches)
    }

    pub fn insert(&mut self, s: &str) -> anyhow::Result<Vec<Dispatch>> {
        let edit_transaction =
            EditTransaction::from_action_groups(self.selection_set.map(|selection| {
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
                        Action::Select(
                            selection
                                .clone()
                                .set_range((range.start + s.len()..range.start + s.len()).into()),
                        ),
                    ]
                    .to_vec(),
                )
            }));

        self.apply_edit_transaction(edit_transaction)
    }

    fn handle_insert_mode(
        &mut self,
        context: &Context,
        event: KeyEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        match event {
            key!("esc") => self.enter_normal_mode()?,
            key!("backspace") => return self.backspace(),
            key!("enter") => return self.insert("\n"),
            key!("tab") => return self.insert("\t"),
            key!("ctrl+a") | key!("home") => self.home(context)?,
            key!("ctrl+e") | key!("end") => self.end(context)?,
            key!("alt+backspace") => return self.delete_word_backward(context),
            // key!("alt+left") => self.move_word_backward(),
            // key!("alt+right") => self.move_word_forward(),
            // key!("ctrl+u") => return self.delete_to_start_of_line(),
            // key!("ctrl+k") => return self.delete_to_end_of_line(),
            event => match event.code {
                KeyCode::Char(c) => return self.insert(&c.to_string()),
                _ => {}
            },
        };
        Ok(vec![])
    }

    pub fn get_request_params(&self) -> Option<RequestParams> {
        let component_id = self.id();
        let position = self.get_cursor_position().ok()?;
        self.path().map(|path| RequestParams {
            path,
            position,
            context: ResponseContext {
                component_id,
                scope: None,
                description: None,
            },
        })
    }

    pub fn set_selection_mode(
        &mut self,
        context: &Context,
        selection_mode: SelectionMode,
    ) -> anyhow::Result<Vec<Dispatch>> {
        self.filters_clear();
        self.move_selection_with_selection_mode_without_global_mode(
            context,
            Movement::Current,
            selection_mode,
        )
        .map(|dispatches| {
            Some(Dispatch::SetGlobalMode(None))
                .into_iter()
                .chain(dispatches)
                .collect::<Vec<_>>()
        })
    }

    fn move_selection_with_selection_mode(
        &mut self,
        context: &Context,
        movement: Movement,
        selection_mode: SelectionMode,
    ) -> anyhow::Result<Vec<Dispatch>> {
        if let Some(global_mode) = &context.mode() {
            match global_mode {
                GlobalMode::QuickfixListItem => Ok(vec![Dispatch::GotoQuickfixListItem(movement)]),
                GlobalMode::SelectionHistoryFile => {
                    Ok([Dispatch::GotoSelectionHistoryFile(movement)].to_vec())
                }
                GlobalMode::SelectionHistoryContiguous => {
                    Ok([Dispatch::GotoSelectionHistoryContiguous(movement)].to_vec())
                }
            }
        } else {
            self.move_selection_with_selection_mode_without_global_mode(
                context,
                movement,
                selection_mode,
            )
        }
    }

    pub fn handle_movement(
        &mut self,
        context: &Context,
        movement: Movement,
    ) -> anyhow::Result<Vec<Dispatch>> {
        match self.mode {
            Mode::Normal => self.move_selection_with_selection_mode(
                context,
                movement,
                self.selection_set.mode.clone(),
            ),
            Mode::Exchange => self.exchange(context, movement),
            Mode::UndoTree => self.navigate_undo_tree(movement),
            Mode::MultiCursor => self.add_cursor(context, &movement).map(|_| Vec::new()),
            _ => Ok(Vec::new()),
        }
    }

    pub fn toggle_bookmarks(&mut self) {
        let selections = self
            .selection_set
            .map(|selection| selection.extended_range());
        self.buffer_mut().save_bookmarks(selections)
    }

    fn handle_movement_key(
        &mut self,
        key_event: &KeyEvent,
        context: &Context,
    ) -> anyhow::Result<Option<Vec<Dispatch>>> {
        let move_selection = |movement: Movement| {
            Ok(Some(
                [Dispatch::DispatchEditor(DispatchEditor::MoveSelection(
                    movement,
                ))]
                .to_vec(),
            ))
        };
        match key_event {
            key!("d") => move_selection(Movement::Down),
            key!("j") => {
                self.jump(context)?;
                Ok(Some(Vec::new()))
            }
            key!("n") => move_selection(Movement::Next),
            key!("p") => move_selection(Movement::Previous),
            key!("u") => move_selection(Movement::Up),
            key!(".") => move_selection(Movement::Last),
            key!(",") => move_selection(Movement::First),
            _ => Ok(None),
        }
    }

    fn handle_normal_mode(
        &mut self,
        context: &Context,
        event: KeyEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        if let Some(dispatches) = self.handle_movement_key(&event, context)? {
            return Ok(dispatches);
        }
        if self.mode != Mode::Normal {
            self.mode = Mode::Normal
        }
        match event {
            key!("'") => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.list_mode_keymap_legend_config(),
                )]
                .to_vec())
            }
            key!(":") => return Ok([Dispatch::OpenCommandPrompt].to_vec()),
            key!("*") => self.select_all(context)?,

            key!("left") => return self.handle_movement(context, Movement::Previous),
            key!("shift+left") => return self.handle_movement(context, Movement::First),
            key!("right") => return self.handle_movement(context, Movement::Next),
            key!("shift+right") => return self.handle_movement(context, Movement::Last),
            key!("esc") => {
                self.reset();
                return Ok(vec![
                    Dispatch::CloseAllExceptMainPanel,
                    Dispatch::SetGlobalMode(None),
                ]);
            }
            // Objects
            key!("a") => self.enter_insert_mode(Direction::Start)?,
            key!("b") => self.toggle_bookmarks(),

            key!("c") => return self.set_selection_mode(context, SelectionMode::Character),
            // d = down
            key!("e") => self.enter_insert_mode(Direction::End)?,
            key!("f") => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.find_local_keymap_legend_config(context)?,
                )]
                .to_vec())
            }
            key!("g") => {
                return Ok(vec![Dispatch::ShowKeymapLegend(
                    self.find_global_keymap_legend_config(),
                )])
            }
            key!("h") => self.toggle_highlight_mode(),
            key!("i") => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.inside_mode_keymap_legend_config(),
                )]
                .to_vec())
            }

            // Initial

            // j = jump
            key!("k") => return self.kill(context),
            key!("shift+K") => return self.select_kids(),
            key!("l") => return self.set_selection_mode(context, SelectionMode::Line),
            key!("m") => self.mode = Mode::MultiCursor,
            key!("o") => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.omit_mode_keymap_legend_config(),
                )]
                .to_vec())
            }

            // p = previous
            key!("q") => {
                return Ok([Dispatch::SetGlobalMode(Some(GlobalMode::QuickfixListItem))].to_vec())
            }
            // r for rotate? more general than swapping/exchange, which does not warp back to first
            // selection
            key!("r") => return self.raise(context),
            key!("shift+R") => return self.replace(context),
            key!("s") => return self.set_selection_mode(context, SelectionMode::SyntaxTree),
            key!("t") => return self.set_selection_mode(context, SelectionMode::TopNode),
            // u = up
            // TODO: v = view (scroll line, scroll half page, scroll full page)
            key!("w") => return self.set_selection_mode(context, SelectionMode::Word),
            key!("x") => self.enter_exchange_mode(),
            key!("y") => {
                return Ok([Dispatch::SetGlobalMode(Some(
                    GlobalMode::SelectionHistoryFile,
                ))]
                .to_vec())
            }
            key!("z") => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.x_mode_keymap_legend_config()?,
                )]
                .to_vec())
            }
            key!("shift+X") => return self.exchange(context, Movement::Previous),
            // y = unused
            key!("backspace") => {
                return self.change();
            }
            key!("enter") => return self.open_new_line(),
            key!("%") => self.change_cursor_direction(),
            key!("(") | key!(")") => return self.enclose(Enclosure::RoundBracket),
            key!("[") | key!("]") => return self.enclose(Enclosure::SquareBracket),
            key!('{') | key!('}') => return self.enclose(Enclosure::CurlyBracket),
            key!('<') | key!('>') => return self.enclose(Enclosure::AngleBracket),

            key!("space") => {
                return Ok(vec![Dispatch::ShowKeymapLegend(
                    self.space_mode_keymap_legend_config(context),
                )])
            }
            key!("0") => return Ok([Dispatch::OpenMoveToIndexPrompt].to_vec()),
            _ => {
                log::info!("event: {:?}", event);
            }
        };
        Ok(vec![])
    }

    pub fn path(&self) -> Option<CanonicalizedPath> {
        self.editor().buffer().path()
    }

    pub fn enter_insert_mode(&mut self, direction: Direction) -> anyhow::Result<()> {
        self.selection_set =
            self.selection_set
                .apply(self.selection_set.mode.clone(), |selection| {
                    let range = selection.extended_range();
                    let char_index = match direction {
                        Direction::Start => range.start,
                        Direction::End => range.end,
                    };
                    Ok(selection.clone().set_range((char_index..char_index).into()))
                })?;
        self.mode = Mode::Insert;
        self.cursor_direction = Direction::Start;
        Ok(())
    }

    pub fn enter_normal_mode(&mut self) -> anyhow::Result<()> {
        if self.mode == Mode::Insert {
            // This is necessary for cursor to not overflow after exiting insert mode
            self.selection_set =
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
                    })?;
        }

        self.mode = Mode::Normal;

        Ok(())
    }

    #[cfg(test)]
    pub fn jump_chars(&self) -> Vec<char> {
        self.jumps()
            .into_iter()
            .map(|jump| jump.character)
            .collect_vec()
    }

    pub fn jumps(&self) -> Vec<&Jump> {
        self.jumps
            .as_ref()
            .map(|jumps| jumps.iter().collect())
            .unwrap_or_default()
    }

    // TODO: handle mouse click
    pub fn set_cursor_position(&mut self, row: u16, column: u16) -> anyhow::Result<Vec<Dispatch>> {
        let start = (self.buffer.borrow().line_to_char(row as usize)?) + column.into();
        let primary = self
            .selection_set
            .primary
            .clone()
            .set_range((start..start).into());
        Ok(self.update_selection_set(SelectionSet {
            mode: self.selection_set.mode.clone(),
            primary,
            ..self.selection_set.clone()
        }))
    }

    /// Get the selection that preserves the syntactic structure of the current selection.
    ///
    /// Returns a valid edit transaction if there is any, otherwise `Left(current_selection)`.
    fn get_valid_selection(
        &self,
        current_selection: &Selection,
        selection_mode: &SelectionMode,
        direction: &Movement,
        context: &Context,
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
            context,
            &self.selection_set.filters,
        )?
        .selection;

        if next_selection.eq(&current_selection) {
            return Ok(Either::Left(current_selection));
        }

        loop {
            let edit_transaction =
                get_actual_edit_transaction(&current_selection, &next_selection)?;
            edit_transaction.selections();
            let current_node = buffer.get_current_node(&current_selection, false)?;

            let new_buffer = {
                let mut new_buffer = self.buffer.borrow().clone();
                if let Err(_) =
                    new_buffer.apply_edit_transaction(&edit_transaction, self.selection_set.clone())
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
                    && next_nodes.iter().all(|next_node| {
                        current_node.kind_id() == next_node.kind_id()
                            && current_node.byte_range().len() == next_node.byte_range().len()
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
                context,
                &self.selection_set.filters,
            )?
            .selection;

            if next_selection.eq(&new_selection) {
                return Ok(Either::Left(current_selection));
            }

            next_selection = new_selection;
        }
    }

    /// Replace the next selection with the current selection without
    /// making the syntax tree invalid
    fn replace_faultlessly(
        &mut self,
        selection_mode: &SelectionMode,
        movement: Movement,
        context: &Context,
    ) -> anyhow::Result<Vec<Dispatch>> {
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
            self.get_valid_selection(
                selection,
                selection_mode,
                &movement,
                context,
                get_edit_transaction,
            )
        });

        self.apply_edit_transaction(EditTransaction::merge(
            edit_transactions
                .into_iter()
                .filter_map(|transaction| transaction.ok())
                .filter_map(|transaction| transaction.map_right(Some).right_or(None))
                .collect(),
        ))
    }

    pub fn exchange(
        &mut self,
        context: &Context,
        movement: Movement,
    ) -> anyhow::Result<Vec<Dispatch>> {
        let mode = self.selection_set.mode.clone();
        self.replace_faultlessly(&mode, movement, context)
    }

    pub fn add_cursor(&mut self, context: &Context, direction: &Movement) -> anyhow::Result<()> {
        self.selection_set.add_selection(
            &self.buffer.borrow(),
            direction,
            &self.cursor_direction,
            context,
        )?;
        self.recalculate_scroll_offset();
        Ok(())
    }

    #[cfg(test)]
    pub fn get_selected_texts(&self) -> Vec<String> {
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

    pub fn text(&self) -> String {
        let buffer = self.buffer.borrow().clone();
        buffer.rope().slice(0..buffer.len_chars()).to_string()
    }

    pub fn dimension(&self) -> Dimension {
        self.rectangle.dimension()
    }

    fn apply_scroll(&mut self, direction: Direction, scroll_height: usize) {
        self.scroll_offset = match direction {
            Direction::Start => self.scroll_offset.saturating_sub(scroll_height as u16),
            Direction::End => self.scroll_offset.saturating_add(scroll_height as u16),
        };
    }

    pub fn backspace(&mut self) -> anyhow::Result<Vec<Dispatch>> {
        let edit_transaction =
            EditTransaction::from_action_groups(self.selection_set.map(|selection| {
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
            }));

        self.apply_edit_transaction(edit_transaction)
    }

    pub fn delete_word_backward(
        &mut self,
        context: &Context,
    ) -> Result<Vec<Dispatch>, anyhow::Error> {
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
                            &SelectionMode::Word,
                            &movement,
                            &self.cursor_direction,
                            context,
                            &self.selection_set.filters,
                        )
                    };
                    let current_word = get_word(Movement::Current)?.selection;
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
    pub fn raise(&mut self, context: &Context) -> anyhow::Result<Vec<Dispatch>> {
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
                &Movement::Up,
                context,
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

    pub fn buffer(&self) -> Ref<Buffer> {
        self.buffer.borrow()
    }

    pub fn buffer_rc(&self) -> Rc<RefCell<Buffer>> {
        self.buffer.clone()
    }

    pub fn buffer_mut(&mut self) -> RefMut<Buffer> {
        self.buffer.borrow_mut()
    }

    fn update_buffer(&mut self, s: &str) {
        self.buffer.borrow_mut().update(s)
    }

    fn scroll(
        &mut self,
        direction: Direction,
        scroll_height: usize,
    ) -> anyhow::Result<Vec<Dispatch>> {
        let dispatch = self.update_selection_set(self.selection_set.apply(
            self.selection_set.mode.clone(),
            |selection| {
                let position = selection.extended_range().start.to_position(&self.buffer());
                let line = if direction == Direction::End {
                    position.line.saturating_add(scroll_height)
                } else {
                    position.line.saturating_sub(scroll_height)
                };
                let position = Position { line, ..position };
                let start = position.to_char_index(&self.buffer())?;
                Ok(selection.clone().set_range((start..start).into()))
            },
        )?);
        self.align_cursor_to_center();

        Ok(dispatch)
    }

    pub fn replace_previous_word(
        &mut self,
        completion: &str,
        context: &Context,
    ) -> anyhow::Result<Vec<Dispatch>> {
        // TODO: this algo is not correct, because SelectionMode::Word select small word, we need to change to Big Word
        let selection = self.get_selection_set(&SelectionMode::Word, Movement::Current, context)?;
        self.update_selection_set(selection);
        self.replace_current_selection_with(|_| Some(Rope::from_str(completion)))?;
        Ok(self.get_document_did_change_dispatch())
    }

    pub fn open_new_line(&mut self) -> anyhow::Result<Vec<Dispatch>> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| {
                    let buffer = self.buffer.borrow();
                    let cursor_index = selection.to_char_index(&self.cursor_direction);
                    let line_index = buffer.char_to_line(cursor_index).ok()?;
                    let line_start = buffer.line_to_char(line_index).ok()?;
                    let current_line = self
                        .buffer
                        .borrow()
                        .get_line_by_char_index(cursor_index)
                        .ok()?;
                    let leading_whitespaces = current_line
                        .chars()
                        .take_while(|c| c.is_whitespace())
                        .join("");
                    Some(ActionGroup::new(
                        [
                            Action::Edit(Edit {
                                range: {
                                    let start = line_start + current_line.len_chars();
                                    (start..start).into()
                                },
                                new: format!("{}\n", leading_whitespaces).into(),
                            }),
                            Action::Select(selection.clone().set_range({
                                let start = line_start
                                    + current_line.len_chars()
                                    + leading_whitespaces.len();
                                (start..start).into()
                            })),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect_vec(),
        );

        let dispatches = self.apply_edit_transaction(edit_transaction)?;
        self.enter_insert_mode(Direction::End)?;
        Ok(dispatches)
    }

    pub fn apply_positional_edits(
        &mut self,
        edits: Vec<PositionalEdit>,
    ) -> anyhow::Result<Vec<Dispatch>> {
        let edit_transaction = EditTransaction::from_action_groups(
            edits
                .into_iter()
                .enumerate()
                .filter_map(|(index, edit)| {
                    let range = edit.range.start.to_char_index(&self.buffer()).ok()?
                        ..edit.range.end.to_char_index(&self.buffer()).ok()?;
                    let next_text_len = edit.new_text.chars().count();

                    let action_edit = Action::Edit(Edit {
                        range: range.clone().into(),
                        new: edit.new_text.into(),
                    });

                    let action_select = Action::Select(Selection::new({
                        let end = range.start + next_text_len;
                        (end..end).into()
                    }));

                    Some(if index == 0 {
                        ActionGroup::new(vec![action_edit, action_select])
                    } else {
                        ActionGroup::new(vec![action_edit])
                    })
                })
                .collect(),
        );
        self.apply_edit_transaction(edit_transaction)
    }

    pub fn apply_positional_edit(&mut self, edit: PositionalEdit) -> anyhow::Result<Vec<Dispatch>> {
        self.apply_positional_edits(vec![edit])
    }

    pub fn save(&mut self) -> anyhow::Result<Vec<Dispatch>> {
        let Some(path) = self.buffer.borrow_mut().save(self.selection_set.clone())? else {
            return Ok(vec![]);
        };
        self.clamp()?;
        self.cursor_keep_primary_only()?;
        Ok(vec![Dispatch::DocumentDidSave { path }]
            .into_iter()
            .chain(self.get_document_did_change_dispatch())
            .collect())
    }

    /// Clamp everything that might be out of bound after the buffer content is modified elsewhere
    fn clamp(&mut self) -> anyhow::Result<()> {
        let len_chars = self.buffer().len_chars();
        self.selection_set = self.selection_set.clamp(CharIndex(len_chars))?;

        let len_lines = self.buffer().len_lines();
        self.scroll_offset = self.scroll_offset.clamp(0, len_lines as u16);

        Ok(())
    }

    fn enclose(&mut self, enclosure: Enclosure) -> anyhow::Result<Vec<Dispatch>> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let old = self.buffer().slice(&selection.extended_range())?;
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit {
                                range: selection.extended_range(),
                                new: format!(
                                    "{}{}{}",
                                    match enclosure {
                                        Enclosure::RoundBracket => "(",
                                        Enclosure::SquareBracket => "[",
                                        Enclosure::CurlyBracket => "{",
                                        Enclosure::AngleBracket => "<",
                                    },
                                    old,
                                    match enclosure {
                                        Enclosure::RoundBracket => ")",
                                        Enclosure::SquareBracket => "]",
                                        Enclosure::CurlyBracket => "}",
                                        Enclosure::AngleBracket => ">",
                                    }
                                )
                                .into(),
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

    fn transform_selection(&mut self, case: Case) -> anyhow::Result<Vec<Dispatch>> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let new: Rope = self
                        .buffer()
                        .slice(&selection.extended_range())?
                        .to_string()
                        .to_case(case)
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
                .flatten()
                .collect_vec(),
        );
        self.apply_edit_transaction(edit_transaction)
    }

    pub fn display_mode(&self) -> String {
        let selection_mode = self.selection_set.mode.display();
        let filters = self
            .selection_set
            .filters
            .display()
            .map(|display| format!("({})", display))
            .unwrap_or_default();
        let mode = match &self.mode {
            Mode::Normal => "MOVE",
            Mode::Insert => "INSERT",
            Mode::MultiCursor => "MULTI CURSOR",
            Mode::FindOneChar => "FIND ONE CHAR",
            Mode::Exchange => "EXCHANGE",
            Mode::UndoTree => "UNDO TREE",
        };
        let cursor_count = self.selection_set.len();
        let mode = format!("{}:{}{} x {}", mode, selection_mode, filters, cursor_count);
        if self.jumps.is_some() {
            format!("{} (JUMPING)", mode)
        } else {
            mode
        }
    }

    fn current_selection(&self) -> anyhow::Result<String> {
        Ok(self
            .buffer()
            .slice(&self.selection_set.primary.extended_range())?
            .into())
    }

    fn line_range(&self) -> Range<usize> {
        let start = self.scroll_offset;
        let end = (start as usize + self.rectangle.height as usize).min(self.buffer().len_lines());

        start as usize..end
    }

    fn handle_multi_cursor_mode(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> Result<Vec<Dispatch>, anyhow::Error> {
        match key_event {
            key!("esc") => self.enter_normal_mode(),
            key!("a") => self.add_cursor_to_all_selections(context),
            // todo: kill primary cursor does not work as expected, we need another editr cursor mode
            key!("k") => self.kill_primary_cursor(),
            key!("n") => self.add_cursor(context, &Movement::Next),
            key!("o") => self.cursor_keep_primary_only(),
            key!("p") => self.add_cursor(context, &Movement::Previous),
            other => return self.handle_normal_mode(context, other),
        }?;
        Ok(Vec::new())
    }

    fn kill_primary_cursor(&mut self) -> Result<(), anyhow::Error> {
        self.selection_set.delete_primary_cursor();
        Ok(())
    }

    pub fn add_cursor_to_all_selections(&mut self, context: &Context) -> Result<(), anyhow::Error> {
        self.selection_set
            .add_all(&self.buffer.borrow(), &self.cursor_direction, context)?;

        self.recalculate_scroll_offset();
        self.enter_normal_mode()?;
        Ok(())
    }

    pub fn cursor_keep_primary_only(&mut self) -> Result<(), anyhow::Error> {
        self.selection_set.only();
        self.enter_normal_mode()
    }

    fn enter_single_character_mode(&mut self) {
        self.mode = Mode::FindOneChar;
    }

    fn handle_find_one_char_mode(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> Result<Vec<Dispatch>, anyhow::Error> {
        match key_event.code {
            KeyCode::Char(c) => {
                self.enter_normal_mode()?;
                self.set_selection_mode(
                    context,
                    SelectionMode::Find {
                        search: Search {
                            search: c.to_string(),
                            kind: SearchKind::LiteralCaseSensitive,
                        },
                    },
                )
            }
            KeyCode::Esc => {
                self.enter_normal_mode()?;
                Ok(Vec::new())
            }
            _ => Ok(Vec::new()),
        }
    }

    pub fn set_decorations(&mut self, decorations: &Vec<super::suggestive_editor::Decoration>) {
        self.buffer.borrow_mut().set_decorations(decorations)
    }

    fn half_page_height(&self) -> usize {
        (self.dimension().height / 2) as usize
    }

    pub fn match_literal(
        &mut self,
        context: &Context,
        search: &str,
    ) -> anyhow::Result<Vec<Dispatch>> {
        self.set_selection_mode(
            context,
            SelectionMode::Find {
                search: Search {
                    kind: SearchKind::LiteralCaseSensitive,
                    search: search.to_string(),
                },
            },
        )
    }

    fn handle_exchange_mode(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> Result<Vec<Dispatch>, anyhow::Error> {
        match key_event {
            key!("esc") => {
                self.enter_normal_mode()?;
                Ok(Vec::new())
            }
            other => self.handle_normal_mode(context, other),
        }
    }

    pub fn home(&mut self, context: &Context) -> anyhow::Result<()> {
        self.select_line(Movement::Current, context)?;
        self.enter_insert_mode(Direction::Start)
    }

    pub fn end(&mut self, context: &Context) -> anyhow::Result<()> {
        self.select_line(Movement::Current, context)?;
        self.handle_movement(context, Movement::Last)?;
        self.enter_insert_mode(Direction::End)
    }

    fn select_all(&mut self, context: &Context) -> anyhow::Result<()> {
        self.move_selection_with_selection_mode_without_global_mode(
            context,
            Movement::First,
            self.selection_set.mode.clone(),
        )?;
        self.toggle_highlight_mode();
        self.move_selection_with_selection_mode_without_global_mode(
            context,
            Movement::Last,
            self.selection_set.mode.clone(),
        )?;
        Ok(())
    }

    fn move_selection_with_selection_mode_without_global_mode(
        &mut self,
        context: &Context,
        movement: Movement,
        selection_mode: SelectionMode,
    ) -> Result<Vec<Dispatch>, anyhow::Error> {
        let dispatches = self.select(selection_mode, movement, context)?;
        self.current_view_alignment = None;
        log::info!("dispatches = {:?}", dispatches);

        Ok(dispatches
            .into_iter()
            .chain(
                if let Some(info) = self
                    .selection_set
                    .map(|selection| selection.info())
                    .into_iter()
                    .flatten()
                    .reduce(Info::join)
                {
                    [Dispatch::ShowInfo {
                        title: "INFO".to_string(),
                        info,
                    }]
                    .to_vec()
                } else {
                    Vec::new()
                },
            )
            .collect())
    }

    pub fn scroll_page_down(&mut self) -> Result<Vec<Dispatch>, anyhow::Error> {
        self.scroll(Direction::End, self.half_page_height())
    }

    pub fn scroll_page_up(&mut self) -> Result<Vec<Dispatch>, anyhow::Error> {
        self.scroll(Direction::Start, self.half_page_height())
    }

    pub fn switch_view_alignment(&mut self) {
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

    fn handle_undo_tree_mode(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> Result<Vec<Dispatch>, anyhow::Error> {
        match key_event {
            key!("esc") => {
                self.enter_normal_mode()?;
                Ok(Vec::new())
            }
            other => self.handle_normal_mode(context, other),
        }
    }

    fn navigate_undo_tree(&mut self, movement: Movement) -> Result<Vec<Dispatch>, anyhow::Error> {
        let selection_set = self.buffer_mut().undo_tree_apply_movement(movement)?;
        if let Some(selection_set) = selection_set {
            self.update_selection_set(selection_set);
        }
        Ok(self.get_document_did_change_dispatch())
    }

    pub fn set_scroll_offset(&mut self, scroll_offset: u16) {
        self.scroll_offset = scroll_offset
    }

    pub(crate) fn set_language(&mut self, language: Language) -> Result<(), anyhow::Error> {
        self.buffer_mut().set_language(language)
    }

    fn render_area(&self) -> Dimension {
        let Dimension { height, width } = self.dimension();
        Dimension {
            height: height.saturating_sub(WINDOW_TITLE_HEIGHT as u16),
            width,
        }
    }

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

    fn inside_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Inside".to_string(),
            owner_id: self.id(),
            keymaps: [
                ("a", "Angular Bracket <>", InsideKind::AngularBrackets),
                ("b", "Back Quote ``", InsideKind::BackQuotes),
                ("c", "Curly Brace {}", InsideKind::CurlyBraces),
                ("d", "Double Quote \"\"", InsideKind::DoubleQuotes),
                ("p", "Parenthesis ()", InsideKind::Parentheses),
                ("q", "Single Quote ''", InsideKind::SingleQuotes),
                ("s", "Square Bracket []", InsideKind::SquareBrackets),
            ]
            .into_iter()
            .map(|(key, description, inside_kind)| {
                Keymap::new(
                    key,
                    description,
                    Dispatch::DispatchEditor(DispatchEditor::EnterInsideMode(inside_kind)),
                )
            })
            .chain(Some(Keymap::new(
                "o",
                "Other",
                Dispatch::OpenInsideOtherPromptOpen,
            )))
            .collect(),
        }
    }
    #[cfg(test)]
    pub(crate) fn handle_movements(
        &mut self,
        context: &Context,
        movements: &[Movement],
    ) -> anyhow::Result<()> {
        for movement in movements {
            self.handle_movement(context, *movement)?;
        }
        Ok(())
    }

    pub(crate) fn get_formatted_content(&self) -> Option<String> {
        self.buffer().get_formatted_content()
    }

    fn omit_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        let filter_mechanism_keymaps = |kind: FilterKind, target: FilterTarget| -> Dispatch {
            Dispatch::ShowKeymapLegend(KeymapLegendConfig {
                title: format!("Omit: {:?} {:?} matching", kind, target),
                owner_id: self.id(),
                keymaps: [
                    Keymap::new(
                        "l",
                        "Literal",
                        Dispatch::OpenOmitLiteralPrompt { kind, target },
                    ),
                    Keymap::new("r", "Regex", Dispatch::OpenOmitRegexPrompt { kind, target }),
                ]
                .to_vec(),
            })
        };
        let filter_target_keymaps = |kind: FilterKind| -> Dispatch {
            Dispatch::ShowKeymapLegend(KeymapLegendConfig {
                title: format!("Omit: {:?}", kind),
                owner_id: self.id(),
                keymaps: [
                    Keymap::new(
                        "c",
                        "Content",
                        filter_mechanism_keymaps(kind, FilterTarget::Content),
                    ),
                    Keymap::new(
                        "i",
                        "Info",
                        filter_mechanism_keymaps(kind, FilterTarget::Info),
                    ),
                ]
                .to_vec(),
            })
        };
        KeymapLegendConfig {
            title: "Omit".to_string(),
            owner_id: self.id(),
            keymaps: [
                Keymap::new(
                    "c",
                    "Clear",
                    Dispatch::DispatchEditor(DispatchEditor::FilterClear),
                ),
                Keymap::new("k", "keep", filter_target_keymaps(FilterKind::Keep)),
                Keymap::new("r", "remove", filter_target_keymaps(FilterKind::Remove)),
            ]
            .to_vec(),
        }
    }

    fn filters_push(
        &mut self,
        context: &Context,
        filter: Filter,
    ) -> Result<Vec<Dispatch>, anyhow::Error> {
        let selection_set = self.selection_set.clone().filter_push(filter);
        self.update_selection_set(selection_set);
        self.handle_movement(&context, Movement::Current)
    }

    #[cfg(test)]
    pub(crate) fn apply_dispatches(
        &mut self,
        context: &Context,
        dispatches: Vec<DispatchEditor>,
    ) -> anyhow::Result<()> {
        for dispatch in dispatches {
            self.apply_dispatch(context, dispatch)?;
        }
        Ok(())
    }

    fn filters_clear(&mut self) {
        let selection_set = self.selection_set.clone().filter_clear();
        self.update_selection_set(selection_set);
    }

    fn find_submenu_title(title: &str, scope: Scope) -> String {
        let scope = match scope {
            Scope::Local => "Local".to_string(),
            Scope::Global => "Global".to_string(),
        };
        format!("Find ({scope}): {title}")
    }

    fn find_local_submenu_title(arg: &str) -> String {
        format!("Find (this file): {arg}")
    }

    fn enter_exchange_mode(&mut self) {
        self.mode = Mode::Exchange
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum ViewAlignment {
    Top,
    Center,
    Bottom,
}

enum Enclosure {
    RoundBracket,
    SquareBracket,
    CurlyBracket,
    AngleBracket,
}

pub enum HandleEventResult {
    Handled(Vec<Dispatch>),
    Ignored(KeyEvent),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DispatchEditor {
    AlignViewTop,
    AlignViewCenter,
    AlignViewBottom,
    Transform(convert_case::Case),
    SetSelectionMode(SelectionMode),
    Exchange(Movement),
    FindOneChar,
    MoveSelection(Movement),
    Copy,
    Cut,
    Replace,
    Paste,
    SelectAll,
    SetContent(String),
    ToggleHighlightMode,
    EnterUndoTreeMode,
    EnterInsertMode(Direction),
    Kill,
    Insert(String),
    MatchLiteral(String),
    ToggleBookmark,
    EnterInsideMode(InsideKind),
    EnterNormalMode,
    EnterExchangeMode,
    FilterPush(Filter),
    FilterClear,
    CursorAddToAllSelections,
    CursorKeepPrimaryOnly,
    Raise,
}
