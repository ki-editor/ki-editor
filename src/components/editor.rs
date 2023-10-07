use crate::{
    app::{FilePickerKind, RequestKind, RequestParams},
    buffer::Line,
    char_index_range::CharIndexRange,
    components::component::Cursor,
    context::{Context, GlobalMode, Search, SearchKind},
    grid::{CellUpdate, Style, StyleSource},
    lsp::process::ResponseContext,
    selection_mode, soft_wrap,
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
};

#[derive(PartialEq, Clone, Debug, Eq)]
pub enum Mode {
    Normal,
    Insert,
    MultiCursor,
    FindOneChar,
    ScrollLine,
    Exchange,
    UndoTree,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Jump {
    pub character: char,
    pub selection: Selection,
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

    fn set_content(&mut self, str: &str) -> Result<(), anyhow::Error> {
        self.update_buffer(str);
        self.clamp()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn set_title(&mut self, title: String) {
        self.title = title;
    }

    fn get_grid(&self, context: &mut Context) -> GetGridResult {
        let editor = self;
        let Dimension { height, width } = editor.dimension();
        let theme = context.theme();
        let diagnostics = context.get_diagnostics(self.path());

        let buffer = editor.buffer();
        let rope = buffer.rope();

        let len_lines = rope.len_lines().max(1) as u16;
        let max_line_number_len = len_lines.to_string().len() as u16;
        let line_number_separator_width = 1;
        let scroll_offset = editor.scroll_offset();
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
        let (hidden_parent_lines, visible_parent_lines) =
            self.get_parent_lines().unwrap_or_default();
        let parent_lines_numbers = visible_parent_lines
            .iter()
            .chain(hidden_parent_lines.iter())
            .map(|line| line.line)
            .collect_vec();

        let grid: Grid = Grid::new(Dimension { height, width });

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
                        line_number: line_number + scroll_offset as usize,
                        content: line,
                        wrapped: index > 0,
                    })
                    .collect_vec()
            })
            .take(height as usize)
            .collect::<Vec<_>>();

        let bookmarks = buffer.bookmarks().into_iter().flat_map(|bookmark| {
            range_to_cell_update(&buffer, bookmark, theme.ui.bookmark, StyleSource::Bookmark)
        });

        let secondary_selections = &editor.selection_set.secondary;

        fn range_to_cell_update(
            buffer: &Buffer,
            range: CharIndexRange,
            style: Style,
            source: StyleSource,
        ) -> Vec<CellUpdate> {
            range
                .iter()
                .filter_map(|char_index| {
                    let position = buffer.char_to_position(char_index).ok()?;
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
            Style::default().background_color(theme.ui.primary_selection_background),
            StyleSource::PrimarySelection,
        );

        let primary_selection_anchors = selection.anchors().into_iter().flat_map(|range| {
            range_to_cell_update(
                &buffer,
                range,
                Style::default().background_color(theme.ui.primary_selection_anchor_background),
                StyleSource::PrimarySelectionAnchors,
            )
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
                Style::default().background_color(theme.ui.secondary_selection_background),
                StyleSource::SecondarySelection,
            )
        });
        let seconday_selection_anchors = secondary_selections.iter().flat_map(|selection| {
            selection.anchors().into_iter().flat_map(|range| {
                range_to_cell_update(
                    &buffer,
                    range,
                    Style::default()
                        .background_color(theme.ui.secondary_selection_anchor_background),
                    StyleSource::SecondarySelectionAnchors,
                )
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
                let style = match diagnostic.severity {
                    Some(severity) => match severity {
                        DiagnosticSeverity::ERROR => theme.diagnostic.error,
                        DiagnosticSeverity::WARNING => theme.diagnostic.warning,
                        DiagnosticSeverity::INFORMATION => theme.diagnostic.info,
                        DiagnosticSeverity::HINT => theme.diagnostic.hint,
                        _ => theme.diagnostic.default,
                    },
                    None => theme.diagnostic.default,
                };
                Some(range_to_cell_update(
                    &buffer,
                    char_index_range,
                    style,
                    StyleSource::Diagnostics,
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
                    decoration.byte_range.to_char_index_range(&buffer).ok()?,
                    decoration.style_key.get_style(theme),
                    StyleSource::ExtraDecorations,
                ))
            })
            .flatten()
            .collect_vec();
        let updates = vec![]
            .into_iter()
            .chain(primary_selection_primary_cursor)
            .chain(bookmarks)
            .chain(primary_selection)
            .chain(primary_selection_anchors)
            .chain(secondary_selection)
            .chain(seconday_selection_anchors)
            .chain(diagnostics)
            .chain(jumps)
            .chain(primary_selection_secondary_cursor)
            .chain(secondary_selection_cursors)
            .chain(extra_decorations);

        #[derive(Clone)]
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
                let update = update.subtract_vertical_offset(scroll_offset.into())?;
                Some(CellUpdate {
                    position: wrapped_lines
                        .calibrate(update.position)
                        .ok()?
                        .move_down(hidden_parent_lines.len())
                        .move_right(max_line_number_len + line_number_separator_width),
                    ..update
                })
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
        let visible_lines_grid = render_lines(grid, visible_render_lines);

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

        let grid = {
            let bottom_height = height.saturating_sub(hidden_parent_lines_grid.dimension().height);

            let bottom = visible_lines_grid.clamp_bottom(bottom_height);

            hidden_parent_lines_grid.merge_vertical(bottom)
        };

        // NOTE: due to performance issue, we only highlight the content that are within view
        // This might result in some incorrectness, but that's a reasonable trade-off, because
        // highlighting the entire file becomes sluggish when the file has more than a thousand lines.
        let highlighted_spans = {
            let current_frame_content = hidden_parent_lines
                .into_iter()
                .map(|line| {
                    // Trim hidden parent line, because we do not wrapped their content
                    line.content
                        .chars()
                        .take(content_container_width)
                        .collect::<String>()
                })
                .chain(
                    visible_lines
                        .iter()
                        .map(|line| line.clone().trim_end().to_string()),
                )
                .collect_vec()
                .join("\n");
            let highlighted_spans = if let Some(language) = buffer.language() {
                context
                    .highlight(language, &current_frame_content)
                    .unwrap_or_default()
            } else {
                Default::default()
            };

            let buffer = Buffer::new(buffer.treesitter_language(), &current_frame_content);
            let wrapped_lines =
                soft_wrap::soft_wrap(&current_frame_content, content_container_width);
            highlighted_spans
                .0
                .iter()
                .flat_map(|highlighted_span| {
                    highlighted_span.byte_range.clone().filter_map(|byte| {
                        Some(
                            CellUpdate::new(
                                wrapped_lines
                                    .calibrate(buffer.byte_to_position(byte).ok()?)
                                    .ok()?
                                    .move_right(max_line_number_len + line_number_separator_width),
                            )
                            .style(highlighted_span.style)
                            .source(highlighted_span.source),
                        )
                    })
                })
                .collect_vec()
        };

        let grid = grid
            .apply_cell_updates(highlighted_spans)
            .apply_cell_updates(hidden_parent_lines_updates)
            .apply_cell_updates(visible_lines_updates);

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
                std::iter::once(component.clone())
                    .chain(component.borrow().descendants().into_iter())
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
            selection_history: self.selection_history.clone(),
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

    selection_history: Vec<SelectionSet>,

    /// Zero-based index.
    /// 2 means the first line to be rendered on the screen if the 3rd line of the text.
    scroll_offset: u16,
    rectangle: Rectangle,

    buffer: Rc<RefCell<Buffer>>,
    title: String,
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
    pub fn from_text(language: tree_sitter::Language, text: &str) -> Self {
        Self {
            selection_set: SelectionSet {
                primary: Selection::default(),
                secondary: vec![],
                mode: SelectionMode::Custom,
            },
            jumps: None,
            mode: Mode::Normal,
            cursor_direction: Direction::Start,
            selection_history: Vec::with_capacity(128),
            scroll_offset: 0,
            rectangle: Rectangle::default(),
            buffer: Rc::new(RefCell::new(Buffer::new(language, text))),
            title: String::new(),
            id: ComponentId::new(),
            current_view_alignment: None,
        }
    }

    pub fn from_buffer(buffer: Rc<RefCell<Buffer>>) -> Self {
        let title = buffer
            .borrow()
            .path()
            .map(|path| {
                let string = path.display_relative().unwrap_or_else(|_| path.display());
                format!(" {} {}", string, path.icon())
            })
            .unwrap_or_else(|| "<Untitled>".to_string());
        Self {
            selection_set: SelectionSet {
                primary: Selection::default(),
                secondary: vec![],
                mode: SelectionMode::Custom,
            },
            jumps: None,
            mode: Mode::Normal,
            cursor_direction: Direction::Start,
            selection_history: Vec::with_capacity(128),
            scroll_offset: 0,
            rectangle: Rectangle::default(),
            buffer,
            title,
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

    pub fn select_kids(&mut self) -> anyhow::Result<()> {
        let buffer = self.buffer.borrow().clone();
        self.update_selection_set(
            self.selection_set
                .select_kids(&buffer, &self.cursor_direction)?,
        );
        Ok(())
    }

    pub fn select_line(&mut self, movement: Movement, context: &Context) -> anyhow::Result<()> {
        self.select(SelectionMode::Line, movement, context)
    }

    pub fn select_line_at(&mut self, line: usize) -> anyhow::Result<()> {
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
        };
        self.update_selection_set(selection_set);
        Ok(())
    }

    fn select_backward(&mut self) {
        while let Some(selection_set) = self.selection_history.pop() {
            if selection_set != self.selection_set {
                self.selection_set = selection_set;
                self.recalculate_scroll_offset();
                break;
            }
        }
    }

    pub fn reset(&mut self) {
        self.selection_set.escape_highlight_mode();
    }

    fn update_selection_set(&mut self, selection_set: SelectionSet) {
        self.selection_set = selection_set.clone();
        self.selection_history.push(selection_set);
        self.recalculate_scroll_offset()
    }

    pub fn set_selection(&mut self, range: Range<Position>) -> anyhow::Result<()> {
        let range = (self.buffer().position_to_char(range.start)?
            ..self.buffer().position_to_char(range.end)?)
            .into();

        let mode = if self.buffer().given_range_is_node(&range) {
            SelectionMode::TopNode
        } else {
            SelectionMode::Custom
        };
        let primary = self.selection_set.primary.clone().set_range(range);
        let selection_set = SelectionSet {
            primary,
            secondary: vec![],
            mode,
        };
        self.update_selection_set(selection_set);
        Ok(())
    }

    fn cursor_row(&self) -> u16 {
        self.get_cursor_char_index()
            .to_position(&self.buffer.borrow())
            .line as u16
    }

    fn recalculate_scroll_offset(&mut self) {
        // Update scroll_offset if primary selection is out of view.
        let cursor_row = self.cursor_row();
        if cursor_row.saturating_sub(self.scroll_offset) > self.rectangle.height
            || cursor_row < self.scroll_offset
        {
            self.align_cursor_to_center();
        }
        self.current_view_alignment = None;
    }

    fn align_cursor_to_bottom(&mut self) {
        self.scroll_offset = self.cursor_row().saturating_sub(self.rectangle.height);
    }

    fn align_cursor_to_top(&mut self) {
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
    ) -> anyhow::Result<()> {
        //  There are a few selection modes where Current make sense.
        let direction = if self.selection_set.mode != selection_mode {
            Movement::Current
        } else {
            movement
        };

        let selection = self.get_selection_set(&selection_mode, direction, context)?;

        self.update_selection_set(selection);
        Ok(())
    }

    fn jump_characters() -> Vec<char> {
        ('a'..='z').chain('A'..='Z').chain('0'..'9').collect_vec()
    }

    fn jump_from_selection(
        &mut self,
        selection: &Selection,
        context: &Context,
    ) -> anyhow::Result<()> {
        let chars = Self::jump_characters();

        let object = self.selection_set.mode.to_selection_mode_trait_object(
            &self.buffer(),
            selection,
            context,
        )?;

        let line_range = self.line_range();
        let jumps = object.jumps(
            selection_mode::SelectionModeParams {
                context,
                buffer: &self.buffer(),
                current_selection: selection,
                cursor_direction: &self.cursor_direction,
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
        self.apply_edit_transaction(edit_transaction)
            .map(|dispatches| dispatches.into_iter().chain(dispatch).collect())
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
                    )?;

                    let next_range = next_selection.extended_range();

                    let (delete_range, select_range) = {
                        let default = (
                            current_range,
                            (current_range.start..current_range.start).into(),
                        );
                        if current_range.end > next_range.start {
                            default
                        } else {
                            let inbetween_range: CharIndexRange =
                                (current_range.end..next_range.start).into();

                            let inbetween_text = buffer.slice(&inbetween_range)?.to_string();
                            if !inbetween_text.trim().is_empty() {
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
            content: [self.buffer().display_history()].to_vec(),
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
    fn search_kinds_keymap() -> Vec<(&'static str, &'static str, SearchKind)> {
        [
            ("a", "Ast Grep", SearchKind::AstGrep),
            ("i", "Literal (Ignore case)", SearchKind::LiteralIgnoreCase),
            ("l", "Literal", SearchKind::Literal),
            ("x", "Regex", SearchKind::Regex),
        ]
        .to_vec()
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

    fn diagnostics_keymaps() -> Vec<(&'static str, &'static str, Option<DiagnosticSeverity>)> {
        [
            ("y", "Any (Diagnostic)", None),
            ("e", "Error (Diagnostic)", Some(DiagnosticSeverity::ERROR)),
            ("h", "Hint (Diagnostic)", Some(DiagnosticSeverity::HINT)),
            (
                "shift+I",
                "Information (Diagnostic)",
                Some(DiagnosticSeverity::INFORMATION),
            ),
            (
                "w",
                "Warning (Diagnostic)",
                Some(DiagnosticSeverity::WARNING),
            ),
        ]
        .into_iter()
        .collect_vec()
    }

    fn find_mode_keymap_legend_config(
        &self,
        context: &Context,
    ) -> anyhow::Result<KeymapLegendConfig> {
        let diagnostics_keymaps = Self::diagnostics_keymaps()
            .into_iter()
            .map(|(char, description, severity)| {
                Keymap::new(
                    char,
                    description,
                    Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                        SelectionMode::Diagnostic(severity),
                    )),
                )
            })
            .collect_vec();
        let open_search_prompt_keymaps = Self::search_kinds_keymap()
            .clone()
            .into_iter()
            .map(|(key, description, search_kind)| {
                Keymap::new(key, description, Dispatch::OpenSearchPrompt(search_kind))
            })
            .collect_vec();
        let find_current_selection_keymaps = KeymapLegendConfig {
            title: "Find current selection by".to_string(),
            keymaps: Self::search_kinds_keymap()
                .into_iter()
                .flat_map(|(key, description, search_kind)| -> anyhow::Result<_> {
                    Ok(Keymap::new(
                        key,
                        description,
                        Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                            SelectionMode::Find {
                                search: Search {
                                    kind: search_kind,
                                    search: self.current_selection()?,
                                },
                            },
                        )),
                    ))
                })
                .collect(),
            owner_id: self.id,
        };
        Ok(KeymapLegendConfig {
            title: "Find (current file)".to_string(),
            owner_id: self.id(),
            keymaps: [
                Keymap::new(
                    "c",
                    "Current selection",
                    Dispatch::ShowKeymapLegend(find_current_selection_keymaps),
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
            .chain(open_search_prompt_keymaps)
            .chain(diagnostics_keymaps)
            .chain(self.lsp_keymaps(RequestKind::Local))
            .chain(context.last_search().map(|search| {
                Keymap::new(
                    "f",
                    "Enter Find Mode (using previous search)",
                    Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                        SelectionMode::Find { search },
                    )),
                )
            }))
            .chain(self.get_request_params().map(|params| {
                Keymap::new(
                    "s",
                    "Symbols",
                    Dispatch::RequestDocumentSymbols(params.clone()),
                )
            }))
            .collect_vec(),
        })
    }

    fn space_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
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
                                    "Reveal in Explorer",
                                    Dispatch::RevealInExplorer(params.path.clone()),
                                ),
                                Keymap::new("h", "Hover", Dispatch::RequestHover(params.clone())),
                                Keymap::new("r", "Rename", Dispatch::PrepareRename(params.clone())),
                                Keymap::new(
                                    "a",
                                    "Code Actions",
                                    Dispatch::RequestCodeAction(params),
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
            DispatchEditor::SelectWholeFile => self.select_whole_file(),
            DispatchEditor::SetContent(content) => self.update_buffer(&content),
            DispatchEditor::Replace => return self.replace(context),
            DispatchEditor::Cut => return self.cut(),
            DispatchEditor::ToggleHighlightMode => self.toggle_highlight_mode(),
            DispatchEditor::EnterUndoTreeMode => return Ok(self.enter_undo_tree_mode()),
            DispatchEditor::EnterInsertMode(direction) => self.enter_insert_mode(direction)?,
            DispatchEditor::Kill => return self.kill(context),
            DispatchEditor::Insert(string) => return self.insert(&string),
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

    fn lsp_keymaps(&self, kind: RequestKind) -> Vec<Keymap> {
        self.get_request_params()
            .map(|params| {
                let params = params.set_kind(Some(kind));
                [
                    Keymap::new(
                        "d",
                        "Definitions",
                        Dispatch::RequestDefinitions(params.clone().set_description("Definitions")),
                    ),
                    Keymap::new(
                        "shift+D",
                        "Declarations",
                        Dispatch::RequestDeclarations(
                            params.clone().set_description("Declarations"),
                        ),
                    ),
                    Keymap::new(
                        "m",
                        "Implementations",
                        Dispatch::RequestImplementations(
                            params.clone().set_description("Implementations"),
                        ),
                    ),
                    Keymap::new(
                        "r",
                        "References",
                        Dispatch::RequestReferences(params.clone().set_description("References")),
                    ),
                    Keymap::new(
                        "t",
                        "Type Definitions",
                        Dispatch::RequestTypeDefinitions(
                            params.set_description("Type Definitions"),
                        ),
                    ),
                ]
                .into_iter()
                .collect_vec()
            })
            .unwrap_or_default()
    }

    fn global_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        let search_keymaps = Self::search_kinds_keymap()
            .into_iter()
            .map(|(key, description, search_kind)| {
                Keymap::new(
                    key,
                    description,
                    Dispatch::OpenGlobalSearchPrompt(search_kind),
                )
            })
            .chain(self.current_selection().ok().map(|selection| {
                Keymap::new(
                    "c",
                    "Current selection",
                    Dispatch::ShowKeymapLegend(KeymapLegendConfig {
                        title: "Find current selection (GLOBAL)".to_string(),
                        keymaps: Self::search_kinds_keymap()
                            .into_iter()
                            .map(|(key, description, search_kind)| {
                                Keymap::new(
                                    key,
                                    description,
                                    Dispatch::GlobalSearch(Search {
                                        kind: search_kind,
                                        search: selection.clone(),
                                    }),
                                )
                            })
                            .collect(),
                        owner_id: self.id,
                    }),
                )
            }))
            .collect_vec();
        let diagnostics_keymaps =
            Self::diagnostics_keymaps()
                .into_iter()
                .map(|(key, description, severity)| {
                    Keymap::new(
                        key,
                        description,
                        Dispatch::SetQuickfixList(QuickfixListType::LspDiagnostic(severity)),
                    )
                });
        KeymapLegendConfig {
            title: "Get (global)".to_string(),
            owner_id: self.id(),
            keymaps: search_keymaps
                .into_iter()
                .chain(diagnostics_keymaps)
                .chain(self.lsp_keymaps(RequestKind::Global))
                .chain(Some(Keymap::new(
                    "g",
                    "Git Hunk",
                    Dispatch::GetRepoGitHunks,
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
                        Mode::ScrollLine => self.handle_scroll_line_mode(context, key_event),
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
                let KeyCode::Char(c) = key.code else {return Ok(Vec::new())};
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
                request_kind: None,
                description: None,
            },
        })
    }

    pub fn set_selection_mode(
        &mut self,
        context: &Context,
        selection_mode: SelectionMode,
    ) -> anyhow::Result<Vec<Dispatch>> {
        self.move_selection_with_selection_mode_without_global_mode(
            context,
            Movement::Current,
            selection_mode,
        )
        .map(|dispatches| {
            Some(Dispatch::SetGlobalMode(None))
                .into_iter()
                .chain(dispatches.into_iter())
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
                GlobalMode::BufferNavigationHistory => {
                    Ok([Dispatch::GotoOpenedEditor(movement)].to_vec())
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

    pub fn save_bookmarks(&mut self) {
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
            key!("o") => Ok(Some(
                [Dispatch::ShowKeymapLegend(
                    self.other_movement_keymap_legend(),
                )]
                .to_vec(),
            )),
            key!("p") => move_selection(Movement::Previous),
            key!("u") => move_selection(Movement::Up),
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
            key!("*") => self.select_whole_file(),
            key!(":") => return Ok([Dispatch::OpenCommandPrompt].to_vec()),
            key!(",") => self.select_backward(),
            key!("left") => return self.handle_movement(context, Movement::Previous),
            key!("shift+left") => return self.handle_movement(context, Movement::First),
            key!("right") => return self.handle_movement(context, Movement::Next),
            key!("shift+right") => return self.handle_movement(context, Movement::Last),
            key!("esc") => {
                self.reset();
                return Ok(vec![Dispatch::CloseAllExceptMainPanel]);
            }
            // Objects
            key!("a") => self.enter_insert_mode(Direction::End)?,
            key!("b") => return self.set_selection_mode(context, SelectionMode::BottomNode),
            key!("ctrl+b") => self.save_bookmarks(),

            key!("c") => return self.set_selection_mode(context, SelectionMode::Character),
            // d = down
            key!("e") => self.mode = Mode::Exchange,
            key!("f") => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.find_mode_keymap_legend_config(context)?,
                )]
                .to_vec())
            }
            key!("g") => {
                return Ok(vec![Dispatch::ShowKeymapLegend(
                    self.global_mode_keymap_legend_config(),
                )])
            }
            key!("h") => self.toggle_highlight_mode(),

            key!("i") => self.enter_insert_mode(Direction::Start)?,
            // j = jump
            key!("k") => return self.kill(context),
            key!("shift+K") => self.select_kids()?,
            key!("l") => return self.set_selection_mode(context, SelectionMode::Line),
            key!("m") => self.mode = Mode::MultiCursor,

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
            key!("v") => {
                return Ok([Dispatch::SetGlobalMode(Some(
                    GlobalMode::BufferNavigationHistory,
                ))]
                .to_vec())
            }
            key!("w") => return self.set_selection_mode(context, SelectionMode::Word),
            key!("x") => {
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

            key!("alt+left") => return Ok(vec![Dispatch::GotoOpenedEditor(Movement::Previous)]),
            key!("alt+right") => return Ok(vec![Dispatch::GotoOpenedEditor(Movement::Next)]),
            key!("space") => {
                return Ok(vec![Dispatch::ShowKeymapLegend(
                    self.space_mode_keymap_legend_config(),
                )])
            }
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
    pub fn set_cursor_position(&mut self, row: u16, column: u16) -> anyhow::Result<()> {
        let start = (self.buffer.borrow().line_to_char(row as usize)?) + column.into();
        let primary = self
            .selection_set
            .primary
            .clone()
            .set_range((start..start).into());
        self.update_selection_set(SelectionSet {
            mode: self.selection_set.mode.clone(),
            primary,
            ..self.selection_set.clone()
        });
        Ok(())
    }

    /// Get the selection that will result in syntactically valid tree
    ///
    /// # Parameters
    /// ## `get_trial_edit_transaction`
    /// A function that returns an edit transaction based on the current and the
    /// next selection. This is used to check if the edit transaction will result in a
    /// syntactically valid tree.
    ///
    /// ## `get_actual_edit_transaction`
    /// Same as `get_trial_edit_transaction` but returns the actual edit transaction,
    /// which should not include any extra modifications such as white-space padding.
    ///
    /// # Returns
    /// Returns a valid edit transaction if there is any, otherwise `Left(current_selection)`.
    fn get_valid_selection(
        &self,
        current_selection: &Selection,
        selection_mode: &SelectionMode,
        direction: &Movement,
        context: &Context,
        get_trial_edit_transaction: impl Fn(
            /* current */ &Selection,
            /* next */ &Selection,
        ) -> anyhow::Result<EditTransaction>,
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
        )?;

        if next_selection.eq(&current_selection) {
            return Ok(Either::Left(current_selection));
        }

        loop {
            let edit_transaction = get_trial_edit_transaction(&current_selection, &next_selection)?;

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

            // Why don't we just use `tree.root_node().has_error()` instead?
            // Because I assume we want to be able to exchange even if some part of the tree
            // contains error
            if !selection_mode.is_node()
                || (!text_at_next_selection.to_string().trim().is_empty()
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
            )?;

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
        let get_trial_edit_transaction = |current_selection: &Selection,
                                          next_selection: &Selection|
         -> anyhow::Result<_> {
            let current_selection_range = current_selection.extended_range();
            let text_at_current_selection = buffer.slice(&current_selection_range)?;

            Ok(EditTransaction::from_action_groups(
                [
                    ActionGroup::new(
                        [Action::Edit(Edit {
                            range: current_selection_range,
                            new: buffer.slice(&next_selection.extended_range())?,
                        })]
                        .to_vec(),
                    ),
                    ActionGroup::new(
                        [Action::Edit(Edit {
                            range: next_selection.extended_range(),
                            // We need to add whitespace on both end of the replacement
                            //
                            // Otherwise we might get the following replacement in Rust:
                            // Assuming the selection is on `baz`, and the selection mode is `ParentNode`.
                            //
                            // Before:                              foo.bar(baz)
                            // Result (with whitespace padding):    baz
                            // Result (without padding):            foo.barbaz
                            new: Rope::from_str(
                                &(" ".to_string() + &text_at_current_selection.to_string() + " "),
                            ),
                        })]
                        .to_vec(),
                    ),
                ]
                .to_vec(),
            ))
        };

        let get_actual_edit_transaction = |current_selection: &Selection,
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
                                // This time without whitespace padding
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
                get_trial_edit_transaction,
                get_actual_edit_transaction,
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
                        )
                    };
                    let current_word = get_word(Movement::Current)?;
                    if current_word.extended_range().start <= start {
                        current_word
                    } else {
                        get_word(Movement::Previous)?
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
            let get_trial_edit_transaction =
                |current_selection: &Selection, other_selection: &Selection| -> anyhow::Result<_> {
                    let range = current_selection
                        .extended_range()
                        .start
                        .min(other_selection.extended_range().start)
                        ..current_selection
                            .extended_range()
                            .end
                            .max(other_selection.extended_range().end);

                    // Add whitespace padding
                    let new: Rope =
                        format!(" {} ", buffer.slice(&current_selection.extended_range())?).into();

                    Ok(EditTransaction::from_action_groups(vec![ActionGroup::new(
                        vec![Action::Edit(Edit {
                            range: range.into(),
                            new,
                        })],
                    )]))
                };
            let get_actual_edit_transaction =
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
                &SelectionMode::SyntaxTree,
                &Movement::Up,
                context,
                get_trial_edit_transaction,
                get_actual_edit_transaction,
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

    pub fn buffer_mut(&mut self) -> RefMut<Buffer> {
        self.buffer.borrow_mut()
    }

    fn update_buffer(&mut self, s: &str) {
        self.buffer.borrow_mut().update(s)
    }

    fn scroll(&mut self, direction: Direction, scroll_height: usize) -> anyhow::Result<()> {
        self.update_selection_set(self.selection_set.apply(
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

        Ok(())
    }

    pub fn replace_previous_word(
        &mut self,
        completion: &str,
        context: &Context,
    ) -> anyhow::Result<Vec<Dispatch>> {
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
        let Some(path) = self.buffer.borrow_mut().save(self.selection_set.clone())?  else {
            return Ok(vec![])
        };
        self.clamp()?;
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
        let mode = match &self.mode {
            Mode::Normal => {
                format!("NORMAL:{}", self.selection_set.mode.display())
            }
            Mode::Insert => "INSERT".to_string(),
            Mode::MultiCursor => "MULTI CURSOR".to_string(),
            Mode::FindOneChar => "FIND ONE CHAR".to_string(),
            Mode::ScrollLine => "SCROLL LINE".to_string(),
            Mode::Exchange => "EXCHANGE".to_string(),
            Mode::UndoTree => "UNDO TREE".to_string(),
        };
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
            key!("o") => self.only_current_cursor(),
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
        self.selection_set.add_all(&self.buffer.borrow(), context)?;
        self.recalculate_scroll_offset();
        self.enter_normal_mode()?;
        Ok(())
    }

    fn only_current_cursor(&mut self) -> Result<(), anyhow::Error> {
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
                            kind: SearchKind::Literal,
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

    pub fn add_decorations(&mut self, decorations: Vec<super::suggestive_editor::Decoration>) {
        self.buffer.borrow_mut().add_decorations(decorations)
    }

    fn handle_scroll_line_mode(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> Result<Vec<Dispatch>, anyhow::Error> {
        match key_event {
            key!("esc") => self.enter_normal_mode(),
            key!("n") => self.scroll(todo!(), 1),
            key!("p") => self.scroll(todo!(), 1),
            other => return self.handle_normal_mode(context, other),
        }?;
        Ok(Vec::new())
    }

    fn half_page_height(&self) -> usize {
        (self.dimension().height / 2) as usize
    }

    fn other_movement_keymap_legend(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Move".to_string(),
            owner_id: self.id(),
            keymaps: [
                Keymap::new(
                    "c",
                    "Current selection",
                    Dispatch::DispatchEditor(DispatchEditor::MoveSelection(Movement::Current)),
                ),
                Keymap::new(
                    "p",
                    "Previous most (first) selection",
                    Dispatch::DispatchEditor(DispatchEditor::MoveSelection(Movement::First)),
                ),
                Keymap::new("i", "To Index (1-based)", Dispatch::OpenMoveToIndexPrompt),
                Keymap::new(
                    "n",
                    "Next most (last) selection",
                    Dispatch::DispatchEditor(DispatchEditor::MoveSelection(Movement::Last)),
                ),
            ]
            .to_vec(),
        }
    }

    #[cfg(test)]
    pub fn match_literal(
        &mut self,
        context: &Context,
        search: &str,
    ) -> anyhow::Result<Vec<Dispatch>> {
        self.set_selection_mode(
            context,
            SelectionMode::Find {
                search: Search {
                    kind: SearchKind::Literal,
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
        self.enter_insert_mode(Direction::End)
    }

    fn select_whole_file(&mut self) {
        let selection_set = SelectionSet {
            primary: self
                .selection_set
                .primary
                .clone()
                .set_range((CharIndex(0)..CharIndex(self.buffer.borrow().len_chars())).into()),
            secondary: vec![],
            mode: SelectionMode::Custom,
        };
        self.update_selection_set(selection_set);
    }

    fn move_selection_with_selection_mode_without_global_mode(
        &mut self,
        context: &Context,
        movement: Movement,
        selection_mode: SelectionMode,
    ) -> Result<Vec<Dispatch>, anyhow::Error> {
        self.select(selection_mode, movement, context)?;

        let infos = self
            .selection_set
            .map(|selection| selection.info())
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        if infos.join("").is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![Dispatch::ShowInfo {
            title: "INFO".to_string(),
            content: infos,
        }])
    }

    pub fn scroll_page_down(&mut self) -> Result<(), anyhow::Error> {
        self.scroll(Direction::End, self.half_page_height())
    }

    pub fn scroll_page_up(&mut self) -> Result<(), anyhow::Error> {
        self.scroll(Direction::Start, self.half_page_height())
    }

    pub fn switch_view_alignment(&mut self) {
        let new_view_alignment = match self.current_view_alignment {
            None => ViewAlignment::Top,
            Some(ViewAlignment::Top) => ViewAlignment::Center,
            Some(ViewAlignment::Center) => ViewAlignment::Bottom,
            Some(ViewAlignment::Bottom) => ViewAlignment::Top,
        };
        self.current_view_alignment = Some(new_view_alignment);
        match new_view_alignment {
            ViewAlignment::Top => self.align_cursor_to_top(),
            ViewAlignment::Center => self.align_cursor_to_center(),
            ViewAlignment::Bottom => self.align_cursor_to_bottom(),
        }
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
        if let Some(selection_set) = match movement {
            Movement::Next => self.buffer_mut().redo()?,
            Movement::Previous => self.buffer_mut().undo()?,
            Movement::Up => self.buffer_mut().go_to_history_branch(Direction::End)?,
            Movement::Down => self.buffer_mut().go_to_history_branch(Direction::Start)?,
            _ => None,
        } {
            self.update_selection_set(selection_set)
        };
        Ok(self.get_document_did_change_dispatch())
    }

    pub fn set_scroll_offset(&mut self, scroll_offset: u16) {
        self.scroll_offset = scroll_offset
    }

    pub(crate) fn set_language(&mut self, language: Language) -> Result<(), anyhow::Error> {
        self.buffer_mut().set_language(language)
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
    FindOneChar,
    MoveSelection(Movement),
    Copy,
    Cut,
    Replace,
    Paste,
    SelectWholeFile,
    SetContent(String),
    ToggleHighlightMode,
    EnterUndoTreeMode,
    EnterInsertMode(Direction),
    Kill,
    Insert(String),
}
