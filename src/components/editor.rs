use crate::{
    buffer::Line,
    char_index_range::CharIndexRange,
    components::component::Cursor,
    context::{Context, GlobalMode, Search, SearchKind},
    grid::{CellUpdate, Style},
    screen::{FilePickerKind, RequestParams},
    selection_mode, soft_wrap,
};
use shared::canonicalized_path::CanonicalizedPath;
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
use my_proc_macros::{hex, key};
use ropey::Rope;

use crate::{
    buffer::Buffer,
    components::component::Component,
    edit::{Action, ActionGroup, Edit, EditTransaction},
    grid::{Cell, Grid},
    lsp::completion::PositionalEdit,
    position::Position,
    quickfix_list::QuickfixListType,
    rectangle::Rectangle,
    screen::{Dimension, Dispatch},
    selection::{CharIndex, Selection, SelectionMode, SelectionSet},
};

use super::{
    component::{ComponentId, GetGridResult},
    keymap_legend::{Keymap, KeymapLegendConfig},
};

#[derive(PartialEq, Clone, Debug)]
pub enum Mode {
    Normal,
    Insert,
    Kill,
    MultiCursor,
    FindOneChar,
    ScrollPage,
    ScrollLine,
    Exchange,
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

    fn get_grid(&self, context: &Context) -> GetGridResult {
        let editor = self;
        let Dimension { height, width } = editor.dimension();
        let theme = context.theme();
        let diagnostics = context.get_diagnostics(self.path());

        let buffer = editor.buffer();
        let rope = buffer.rope();
        let len_lines = rope.len_lines() as u16;
        let max_line_number_len = len_lines.to_string().len() as u16;
        let line_number_separator_width = 1;
        let width = width.saturating_sub(max_line_number_len + line_number_separator_width);
        let scroll_offset = editor.scroll_offset();
        let wrapped_lines = soft_wrap::soft_wrap(
            &rope
                .lines()
                .skip(scroll_offset as usize)
                .take(height as usize)
                .join(""),
            width as usize,
        );
        let format_line_number = |line_number: usize| {
            format!(
                "{: >width$}",
                line_number.to_string(),
                width = max_line_number_len as usize
            )
        };

        let line_numbers_grid = (0..height.min(len_lines.saturating_sub(scroll_offset))).fold(
            Grid::new(Dimension {
                height,
                width: max_line_number_len,
            }),
            |grid, index| {
                let line_number = index + scroll_offset + 1;
                let line_number_str = format_line_number(line_number as usize);

                if let Ok(position) = wrapped_lines.calibrate(Position {
                    line: (line_number.saturating_sub(scroll_offset).saturating_sub(1)) as usize,
                    column: 0,
                }) {
                    let line_index = position.line;
                    if line_index < height as usize {
                        return grid.set_line(line_index, &line_number_str, theme.ui.line_number);
                    }
                }
                grid
            },
        );
        let make_line_numbers_separator_grid = |height: u16| {
            (0..height).fold(
                Grid::new(Dimension {
                    height,
                    width: line_number_separator_width,
                }),
                |grid, index| grid.set_line(index as usize, "│", theme.ui.line_number_separator),
            )
        };

        let line_numbers_separator_grid = make_line_numbers_separator_grid(height);

        let mut grid: Grid = Grid::new(Dimension { height, width });
        let selection = &editor.selection_set.primary;

        // If the buffer selection is updated less recently than the window's scroll offset,
        // use the window's scroll offset.

        let lines = wrapped_lines
            .lines()
            .iter()
            .flat_map(|line| line.lines())
            .take(height as usize)
            .enumerate()
            .collect::<Vec<(_, String)>>();

        let bookmarks = buffer
            .bookmarks()
            .into_iter()
            .flat_map(|bookmark| range_to_cell_update(&buffer, bookmark, theme.ui.bookmark));

        let secondary_selections = &editor.selection_set.secondary;

        for (line_index, line) in lines {
            for (column_index, c) in line.chars().take(width as usize).enumerate() {
                grid.rows[line_index][column_index] = Cell {
                    symbol: c.to_string(),
                    background_color: theme.ui.text.background_color.unwrap_or(hex!("#ffffff")),
                    foreground_color: theme.ui.text.foreground_color.unwrap_or(hex!("#000000")),
                    undercurl: None,
                    is_cursor: false,
                };
            }
        }

        fn range_to_cell_update(
            buffer: &Buffer,
            range: CharIndexRange,
            style: Style,
        ) -> Vec<CellUpdate> {
            range
                .iter()
                .filter_map(|char_index| {
                    let position = buffer.char_to_position(char_index).ok()?;
                    Some(CellUpdate::new(position).style(style))
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
            theme.ui.primary_selection,
        );
        let primary_selection_anchors = selection.anchors().into_iter().flat_map(|range| {
            range_to_cell_update(&buffer, range, theme.ui.primary_selection_anchor)
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
                theme.ui.secondary_selection,
            )
        });
        let seconday_selection_anchors = secondary_selections.iter().flat_map(|selection| {
            selection.anchors().into_iter().flat_map(|range| {
                range_to_cell_update(&buffer, range, theme.ui.secondary_selection_anchor)
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

        let highlighted_spans = buffer.highlighted_spans();
        let (inbound_highlight_spans, outbound_highlight_spans): (Vec<_>, Vec<_>) =
            highlighted_spans
                .iter()
                .flat_map(|highlighted_span| {
                    highlighted_span.byte_range.clone().filter_map(|byte| {
                        Some(
                            CellUpdate::new(buffer.byte_to_position(byte).ok()?)
                                .style(highlighted_span.style),
                        )
                    })
                })
                .partition(|span| span.position.line >= scroll_offset as usize);

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
                Some(range_to_cell_update(&buffer, char_index_range, style))
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

                // Background color: Odd index red, even index blue
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
            .chain(inbound_highlight_spans)
            .chain(jumps)
            .chain(primary_selection_secondary_cursor)
            .chain(secondary_selection_cursors)
            .chain(extra_decorations)
            .filter_map(|update| {
                let update = update.subtract_vertical_offset(scroll_offset.into())?;
                Some(CellUpdate {
                    position: wrapped_lines.calibrate(update.position).ok()?,
                    ..update
                })
            })
            .collect::<Vec<_>>();

        let hidden_parent_lines_grid = {
            let hidden_parent_lines = self.get_hidden_parent_lines().unwrap_or_default();
            let height = hidden_parent_lines.len() as u16;
            let line_number_grid = hidden_parent_lines.iter().enumerate().fold(
                Grid::new(Dimension {
                    width: max_line_number_len,
                    height,
                }),
                |grid, (index, line)| {
                    grid.set_line(
                        index,
                        &format_line_number(line.line + 1),
                        theme
                            .ui
                            .line_number
                            .background_color(theme.ui.parent_lines_background),
                    )
                },
            );
            let line_numbers_separator_grid = make_line_numbers_separator_grid(height);
            let content_grid = hidden_parent_lines.into_iter().enumerate().fold(
                Grid::new(Dimension {
                    width: editor.dimension().width,
                    height,
                }),
                |grid, (index, line)| {
                    let updates = outbound_highlight_spans
                        .iter()
                        .filter(|update| update.position.line == line.line)
                        .map(|update| update.clone().set_position(update.position.set_line(index)))
                        .collect_vec();

                    grid.set_line(
                        index,
                        &line.content,
                        theme
                            .ui
                            .text
                            .background_color(theme.ui.parent_lines_background),
                    )
                    .apply_cell_updates(updates)
                },
            );
            line_number_grid
                .merge_horizontal(line_numbers_separator_grid)
                .merge_horizontal(content_grid)
        };

        let grid = {
            let bottom_height = height.saturating_sub(hidden_parent_lines_grid.dimension().height);

            let bottom = line_numbers_grid
                .merge_horizontal(line_numbers_separator_grid)
                .merge_horizontal(grid.apply_cell_updates(updates))
                .clamp_bottom(bottom_height);

            hidden_parent_lines_grid.merge_vertical(bottom)
        };

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
        const SCROLL_HEIGHT: isize = 1;
        match mouse_event.kind {
            MouseEventKind::ScrollUp => {
                self.apply_scroll(-SCROLL_HEIGHT);
                Ok(vec![])
            }
            MouseEventKind::ScrollDown => {
                self.apply_scroll(SCROLL_HEIGHT);
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
        }
    }
}

pub struct Editor {
    pub mode: Mode,

    pub selection_set: SelectionSet,

    pub jumps: Option<Vec<Jump>>,
    pub cursor_direction: CursorDirection,

    selection_history: Vec<SelectionSet>,

    /// Zero-based index.
    /// 2 means the first line to be rendered on the screen if the 3rd line of the text.
    scroll_offset: u16,
    rectangle: Rectangle,

    buffer: Rc<RefCell<Buffer>>,
    title: String,
    id: ComponentId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CursorDirection {
    Start,
    End,
}

impl CursorDirection {
    pub fn reverse(&self) -> Self {
        match self {
            CursorDirection::Start => CursorDirection::End,
            CursorDirection::End => CursorDirection::Start,
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
    pub fn get_hidden_parent_lines(&self) -> anyhow::Result<Vec<Line>> {
        let position = self.get_cursor_position()?;
        let parent_lines = self.buffer().get_parent_lines(position.line)?;
        Ok(parent_lines
            .into_iter()
            .filter(|line| line.line < self.scroll_offset as usize)
            .collect_vec())
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
            cursor_direction: CursorDirection::Start,
            selection_history: Vec::with_capacity(128),
            scroll_offset: 0,
            rectangle: Rectangle::default(),
            buffer: Rc::new(RefCell::new(Buffer::new(language, text))),
            title: String::new(),
            id: ComponentId::new(),
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
            cursor_direction: CursorDirection::Start,
            selection_history: Vec::with_capacity(128),
            scroll_offset: 0,
            rectangle: Rectangle::default(),
            buffer,
            title,
            id: ComponentId::new(),
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

    fn select_kids(&mut self) -> anyhow::Result<()> {
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

    pub fn select_match(
        &mut self,
        movement: Movement,
        search: &Option<Search>,
        context: &Context,
    ) -> anyhow::Result<()> {
        if let Some(search) = search {
            self.select(
                SelectionMode::Find {
                    search: search.clone(),
                },
                movement,
                context,
            )?;
        }
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

    fn reset(&mut self) {
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
            SelectionMode::OutermostNode
        } else {
            SelectionMode::Custom
        };
        let primary =
            Selection::new(range).set_copied_text(self.selection_set.primary.copied_text());
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
        if cursor_row.saturating_sub(self.scroll_offset)
            >= (self.rectangle.height.saturating_sub(2))
            || cursor_row < self.scroll_offset
        {
            self.align_cursor_to_center();
        }
    }

    fn align_cursor_to_bottom(&mut self) {
        self.scroll_offset = self.cursor_row() - (self.rectangle.height - 2);
    }

    fn align_cursor_to_top(&mut self) {
        self.scroll_offset = self.cursor_row();
    }

    fn align_cursor_to_center(&mut self) {
        self.scroll_offset = self
            .cursor_row()
            .saturating_sub((self.rectangle.height.saturating_sub(2)) / 2);
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
        ('a'..='z').collect_vec()
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

    fn jump(&mut self, context: &Context) -> anyhow::Result<()> {
        self.jump_from_selection(&self.selection_set.primary.clone(), context)
    }

    fn cut(&mut self) -> anyhow::Result<Vec<Dispatch>> {
        self.delete(true)
    }

    fn delete(&mut self, cut: bool) -> anyhow::Result<Vec<Dispatch>> {
        // Set the clipboard content to the current selection
        // if there is only one cursor.
        let dispatch = if cut && self.selection_set.secondary.is_empty() {
            Some(Dispatch::SetClipboardContent(
                self.buffer
                    .borrow()
                    .slice(&self.selection_set.primary.extended_range())?
                    .into(),
            ))
        } else {
            None
        };
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let old_range = selection.extended_range();
                    let copied_text = if cut {
                        Some(self.buffer.borrow().slice(&old_range)?)
                    } else {
                        selection.copied_text()
                    };
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit {
                                range: old_range,
                                new: Rope::new(),
                            }),
                            Action::Select(
                                Selection::new((old_range.start..old_range.start).into())
                                    .set_copied_text(copied_text),
                            ),
                        ]
                        .to_vec(),
                    ))
                })
                .into_iter()
                .flatten()
                .collect(),
        );

        self.apply_edit_transaction(edit_transaction)
            .map(|dispatches| dispatches.into_iter().chain(dispatch).collect())
    }

    fn copy(&mut self, context: &Context) -> anyhow::Result<Vec<Dispatch>> {
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

    fn paste(&mut self, context: &Context) -> anyhow::Result<Vec<Dispatch>> {
        self.replace_current_selection_with(|selection| {
            selection
                .copied_text()
                .or_else(|| context.get_clipboard_content().map(Rope::from))
        })
    }

    fn replace(&mut self) -> anyhow::Result<Vec<Dispatch>> {
        let edit_transaction = EditTransaction::merge(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    if let Some(replacement) = &selection.copied_text() {
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
        self.buffer
            .borrow_mut()
            .apply_edit_transaction(&edit_transaction, self.selection_set.clone())?;

        if let Some((head, tail)) = edit_transaction.selections().split_first() {
            self.selection_set = SelectionSet {
                primary: (*head).clone(),
                secondary: tail.iter().map(|selection| (*selection).clone()).collect(),
                mode: self.selection_set.mode.clone(),
            }
        }

        self.recalculate_scroll_offset();

        Ok(self.get_document_did_change_dispatch())
    }

    pub fn get_document_did_change_dispatch(&mut self) -> Vec<Dispatch> {
        vec![Dispatch::DocumentDidChange {
            component_id: self.id(),
            path: self.buffer().path(),
            content: self.buffer().rope().to_string(),
            language: self.buffer().language(),
        }]
    }

    fn undo(&mut self) -> anyhow::Result<Vec<Dispatch>> {
        let selection_set = self.buffer.borrow_mut().undo(self.selection_set.clone())?;
        if let Some(selection_set) = selection_set {
            self.update_selection_set(selection_set);
        }
        Ok(self.get_document_did_change_dispatch())
    }

    fn redo(&mut self) -> anyhow::Result<Vec<Dispatch>> {
        let selection_set = self.buffer.borrow_mut().redo(self.selection_set.clone())?;
        if let Some(selection_set) = selection_set {
            self.update_selection_set(selection_set);
        }
        Ok(self.get_document_did_change_dispatch())
    }

    fn change_cursor_direction(&mut self) {
        self.cursor_direction = match self.cursor_direction {
            CursorDirection::Start => CursorDirection::End,
            CursorDirection::End => CursorDirection::Start,
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

    fn toggle_highlight_mode(&mut self) {
        self.selection_set.toggle_highlight_mode();
        self.recalculate_scroll_offset()
    }

    fn find_mode_keymap_legend_config(
        &self,
        context: &Context,
    ) -> anyhow::Result<KeymapLegendConfig> {
        Ok(KeymapLegendConfig {
            title: "Find (current file)".to_string(),
            owner_id: self.id(),
            keymaps: [
                Keymap::new(
                    "a",
                    "AST Grep",
                    Dispatch::OpenSearchPrompt(SearchKind::AstGrep),
                ),
                Keymap::new(
                    "c",
                    "Current selection",
                    Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                        SelectionMode::Find {
                            search: Search {
                                kind: SearchKind::IgnoreCase,
                                search: self.current_selection()?,
                            },
                        },
                    )),
                ),
                Keymap::new(
                    "e",
                    "Empty line",
                    Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                        SelectionMode::EmptyLine,
                    )),
                ),
                Keymap::new(
                    "h",
                    "Git hunk",
                    Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                        SelectionMode::GitHunk,
                    )),
                ),
                Keymap::new(
                    "f",
                    "Enter Find Mode",
                    Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                        SelectionMode::Find {
                            search: context.last_search().clone().unwrap_or(Search {
                                kind: SearchKind::Literal,
                                search: "".to_string(),
                            }),
                        },
                    )),
                ),
                Keymap::new(
                    "i",
                    "Ignore case",
                    Dispatch::OpenSearchPrompt(SearchKind::IgnoreCase),
                ),
                Keymap::new(
                    "l",
                    "Literal",
                    Dispatch::OpenSearchPrompt(SearchKind::Literal),
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
                Keymap::new("r", "Regex", Dispatch::OpenSearchPrompt(SearchKind::Regex)),
                Keymap::new(
                    "w",
                    "Word",
                    Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(SelectionMode::Word)),
                ),
            ]
            .into_iter()
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

    fn view_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "View".to_string(),
            keymaps: [
                ("b", "Align view bottom", DispatchEditor::AlignViewBottom),
                ("c", "Align view center", DispatchEditor::AlignViewCenter),
                (
                    "l",
                    "Enter scroll line mode",
                    DispatchEditor::EnterScrollLineMode,
                ),
                (
                    "p",
                    "Enter scroll page mode",
                    DispatchEditor::EnterScrollPageMode,
                ),
                ("t", "Align view top", DispatchEditor::AlignViewTop),
            ]
            .into_iter()
            .map(|(key, description, dispatch)| {
                Keymap::new(key, description, Dispatch::DispatchEditor(dispatch))
            })
            .collect_vec(),
            owner_id: self.id(),
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
                return self.set_selection_mode(context, selection_mode)
            }

            DispatchEditor::FindOneChar => self.enter_single_character_mode(),
            DispatchEditor::EnterScrollPageMode => self.mode = Mode::ScrollPage,
            DispatchEditor::EnterScrollLineMode => self.mode = Mode::ScrollLine,

            DispatchEditor::MoveSelection(direction) => {
                return self.move_selection(context, direction)
            }
        }
        Ok([].to_vec())
    }

    fn diagnostic_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Diagnostic".to_string(),
            owner_id: self.id(),
            keymaps: [
                ("a", "Any", None),
                ("e", "Error", Some(DiagnosticSeverity::ERROR)),
                ("h", "Hint", Some(DiagnosticSeverity::HINT)),
                ("i", "Information", Some(DiagnosticSeverity::INFORMATION)),
                ("w", "Warning", Some(DiagnosticSeverity::WARNING)),
            ]
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
            .collect_vec(),
        }
    }

    fn g_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Get (global)".to_string(),
            owner_id: self.id(),
            keymaps: self
                .get_request_params()
                .map(|params| {
                    [
                        Keymap::new(
                            "d",
                            "Definition(s)",
                            Dispatch::RequestDefinitions(params.clone()),
                        ),
                        Keymap::new(
                            "shift+D",
                            "Declaration(s)",
                            Dispatch::RequestDeclarations(params.clone()),
                        ),
                        Keymap::new(
                            "e",
                            "Errors",
                            Dispatch::SetQuickfixList(QuickfixListType::LspDiagnostic),
                        ),
                        Keymap::new(
                            "i",
                            "Implementation(s)",
                            Dispatch::RequestImplementations(params.clone()),
                        ),
                        Keymap::new(
                            "r",
                            "References",
                            Dispatch::RequestReferences(params.clone()),
                        ),
                        Keymap::new(
                            "t",
                            "Type Definition(s)",
                            Dispatch::RequestTypeDefinitions(params),
                        ),
                    ]
                    .into_iter()
                    .chain(Some(Keymap::new(
                        "f",
                        "Find",
                        Dispatch::ShowKeymapLegend(KeymapLegendConfig {
                            title: "Find Global".to_string(),
                            owner_id: self.id(),
                            keymaps: [
                                Keymap::new(
                                    "a",
                                    "AST Grep",
                                    Dispatch::OpenGlobalSearchPrompt(SearchKind::AstGrep),
                                ),
                                Keymap::new(
                                    "i",
                                    "Ignore case",
                                    Dispatch::OpenGlobalSearchPrompt(SearchKind::IgnoreCase),
                                ),
                                Keymap::new(
                                    "l",
                                    "Literal",
                                    Dispatch::OpenGlobalSearchPrompt(SearchKind::Literal),
                                ),
                                Keymap::new(
                                    "r",
                                    "Regex",
                                    Dispatch::OpenGlobalSearchPrompt(SearchKind::Regex),
                                ),
                            ]
                            .into_iter()
                            .chain(self.current_selection().ok().map(|selection| {
                                Keymap::new(
                                    "c",
                                    "Current selection",
                                    Dispatch::GlobalSearch(Search {
                                        kind: SearchKind::IgnoreCase,
                                        search: selection,
                                    }),
                                )
                            }))
                            .collect_vec(),
                        }),
                    )))
                    .chain(
                        [
                            ("g", "Git status", FilePickerKind::GitStatus),
                            ("n", "Not git ignored files", FilePickerKind::NonGitIgnored),
                            ("o", "Opened files", FilePickerKind::Opened),
                        ]
                        .into_iter()
                        .map(|(key, description, kind)| {
                            Keymap::new(key, description, Dispatch::OpenFilePicker(kind))
                        }),
                    )
                    .collect_vec()
                })
                .unwrap_or_default(),
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
                    self.handle_jump_mode(key_event, jumps)
                } else {
                    match &self.mode {
                        Mode::Normal => self.handle_normal_mode(context, key_event),
                        Mode::Insert => self.handle_insert_mode(key_event),
                        Mode::Kill => self.handle_kill_mode(context, key_event),
                        Mode::MultiCursor => self.handle_multi_cursor_mode(context, key_event),
                        Mode::FindOneChar => self.handle_find_one_char_mode(context, key_event),
                        Mode::ScrollPage => self.handle_scroll_page_mode(context, key_event),
                        Mode::ScrollLine => self.handle_scroll_line_mode(context, key_event),
                        Mode::Exchange => self.handle_exchange_mode(context, key_event),
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
            key!("ctrl+a") => {
                let selection_set = SelectionSet {
                    primary: Selection::new(
                        (CharIndex(0)..CharIndex(self.buffer.borrow().len_chars())).into(),
                    )
                    .set_copied_text(self.selection_set.primary.copied_text()),
                    secondary: vec![],
                    mode: SelectionMode::Custom,
                };
                self.update_selection_set(selection_set);
                Ok(HandleEventResult::Handled(vec![]))
            }
            key!("ctrl+c") => {
                let dispatches = self.copy(context)?;
                Ok(HandleEventResult::Handled(dispatches))
            }
            key!("ctrl+s") => {
                let dispatches = self.save()?;
                self.mode = Mode::Normal;
                Ok(HandleEventResult::Handled(dispatches))
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
                    Some((jump, [])) => {
                        Ok([Dispatch::DispatchEditor(DispatchEditor::MoveSelection(
                            Movement::Jump(jump.selection.extended_range()),
                        ))]
                        .to_vec())
                    }
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
    fn change(&mut self) -> anyhow::Result<Vec<Dispatch>> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit {
                                range: selection.extended_range(),
                                new: Rope::new(),
                            }),
                            Action::Select(
                                Selection::new(
                                    (selection.extended_range().start
                                        ..selection.extended_range().start)
                                        .into(),
                                )
                                .set_copied_text(selection.copied_text()),
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
        self.enter_insert_mode(CursorDirection::Start)?;
        Ok(dispatches)
    }

    fn insert(&mut self, s: &str) -> anyhow::Result<Vec<Dispatch>> {
        let edit_transaction =
            EditTransaction::from_action_groups(self.selection_set.map(|selection| {
                let range = selection.extended_range();
                ActionGroup::new(
                    [
                        Action::Edit(Edit {
                            range: {
                                let start = selection.to_char_index(&CursorDirection::End);
                                (start..start).into()
                            },
                            new: Rope::from_str(s),
                        }),
                        Action::Select(
                            Selection::new((range.start + s.len()..range.start + s.len()).into())
                                .set_copied_text(selection.copied_text()),
                        ),
                    ]
                    .to_vec(),
                )
            }));

        self.apply_edit_transaction(edit_transaction)
    }

    fn handle_insert_mode(&mut self, event: KeyEvent) -> anyhow::Result<Vec<Dispatch>> {
        match event.code {
            KeyCode::Esc => self.enter_normal_mode()?,
            KeyCode::Backspace => return self.backspace(),
            KeyCode::Enter => return self.insert("\n"),
            KeyCode::Char(c) => return self.insert(&c.to_string()),
            KeyCode::Tab => return self.insert("\t"),
            _ => {}
        };
        Ok(vec![])
    }

    pub fn get_request_params(&self) -> Option<RequestParams> {
        let component_id = self.id();
        let position = self.get_cursor_position().ok()?;
        self.path().map(|path| RequestParams {
            component_id,
            path,
            position,
        })
    }

    fn set_selection_mode(
        &mut self,
        context: &Context,
        selection_mode: SelectionMode,
    ) -> anyhow::Result<Vec<Dispatch>> {
        self.move_selection_with_selection_mode(context, Movement::Current, selection_mode)
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
            self.select(selection_mode, movement, context)?;

            let infos = self
                .selection_set
                .map(|selection| selection.info())
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();

            if infos.is_empty() {
                return Ok(vec![]);
            }

            Ok(vec![Dispatch::ShowInfo {
                title: "INFO".to_string(),
                content: infos,
            }])
        }
    }

    fn move_selection(
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
            Mode::Exchange => self.exchange(movement, context),
            Mode::Kill => self.kill(movement, context),
            Mode::MultiCursor => self.add_cursor(&movement, context).map(|_| Vec::new()),
            _ => Ok(Vec::new()),
        }
    }

    fn save_bookmarks(&mut self) {
        let selections = self
            .selection_set
            .map(|selection| selection.extended_range());
        self.buffer_mut().save_bookmarks(selections)
    }

    fn handle_movement(
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
            key!("z") => Ok(Some(
                [Dispatch::ShowKeymapLegend(self.move_mode_keymap_legend())].to_vec(),
            )),
            _ => Ok(None),
        }
    }

    fn handle_normal_mode(
        &mut self,
        context: &Context,
        event: KeyEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        if let Some(dispatches) = self.handle_movement(&event, context)? {
            return Ok(dispatches);
        }
        if self.mode != Mode::Normal {
            self.mode = Mode::Normal
        }
        match event {
            key!(":") => return Ok([Dispatch::OpenCommandPrompt].to_vec()),
            key!(",") => self.select_backward(),
            key!("left") => return self.move_selection(context, Movement::Previous),
            key!("shift+left") => return self.move_selection(context, Movement::First),
            key!("right") => return self.move_selection(context, Movement::Next),
            key!("shift+right") => return self.move_selection(context, Movement::Last),
            key!("esc") => {
                self.reset();
                return Ok(vec![Dispatch::CloseAllExceptMainPanel]);
            }
            // Objects
            key!("a") => self.enter_insert_mode(CursorDirection::End)?,
            key!("ctrl+b") => self.save_bookmarks(),
            key!("b") => {
                return Ok([Dispatch::SetGlobalMode(Some(
                    GlobalMode::BufferNavigationHistory,
                ))]
                .to_vec())
            }

            key!("c") => return self.set_selection_mode(context, SelectionMode::Character),
            // d = down
            key!("e") => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.diagnostic_keymap_legend_config(),
                )]
                .to_vec())
            }
            key!("f") => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.find_mode_keymap_legend_config(context)?,
                )]
                .to_vec())
            }
            key!("g") => {
                return Ok(vec![Dispatch::ShowKeymapLegend(
                    self.g_mode_keymap_legend_config(),
                )])
            }
            key!("h") => self.toggle_highlight_mode(),

            key!("i") => self.enter_insert_mode(CursorDirection::Start)?,
            // j = jump
            key!("k") => self.mode = Mode::Kill,
            key!("shift+K") => self.select_kids()?,
            key!("l") => return self.set_selection_mode(context, SelectionMode::Line),
            key!("m") => self.mode = Mode::MultiCursor,
            key!("o") => return self.set_selection_mode(context, SelectionMode::OutermostNode),
            // p = previous
            key!("q") => {
                return Ok([Dispatch::SetGlobalMode(Some(GlobalMode::QuickfixListItem))].to_vec())
            }
            // r for rotate? more general than swapping/exchange, which does not warp back to first
            // selection
            key!("r") => return self.raise(context),
            key!("shift+R") => return self.replace(),
            key!("s") => return self.set_selection_mode(context, SelectionMode::SyntaxTree),
            key!("t") => return self.set_selection_mode(context, SelectionMode::Token),
            // u = up
            key!("v") => {
                return Ok(vec![Dispatch::ShowKeymapLegend(
                    self.view_mode_keymap_legend_config(),
                )]);
            }
            // wipe
            key!("w") => return self.delete(false),
            key!("x") => self.mode = Mode::Exchange,
            key!("shift+X") => return self.exchange(Movement::Previous, context),
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

    fn request_hover(&self) -> Vec<Dispatch> {
        let Some(path) = self.path() else { return vec![] };
        let Ok(position) = self.get_cursor_position() else { return vec![] };
        vec![Dispatch::RequestHover(RequestParams {
            component_id: self.id(),
            path,
            position,
        })]
    }

    pub fn enter_insert_mode(&mut self, direction: CursorDirection) -> anyhow::Result<()> {
        self.selection_set =
            self.selection_set
                .apply(self.selection_set.mode.clone(), |selection| {
                    let range = selection.extended_range();
                    let char_index = match direction {
                        CursorDirection::Start => range.start,
                        CursorDirection::End => range.end,
                    };
                    Ok(selection.clone().set_range((char_index..char_index).into()))
                })?;
        self.mode = Mode::Insert;
        self.cursor_direction = CursorDirection::Start;
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

    fn exchange(&mut self, movement: Movement, context: &Context) -> anyhow::Result<Vec<Dispatch>> {
        let mode = self.selection_set.mode.clone();
        self.replace_faultlessly(&mode, movement, context)
    }

    fn add_cursor(&mut self, direction: &Movement, context: &Context) -> anyhow::Result<()> {
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

    fn select_word(&mut self, movement: Movement, context: &Context) -> anyhow::Result<()> {
        self.select(SelectionMode::Word, movement, context)
    }

    pub fn dimension(&self) -> Dimension {
        self.rectangle.dimension()
    }

    fn apply_scroll(&mut self, scroll_height: isize) {
        self.scroll_offset = if scroll_height.is_positive() {
            self.scroll_offset.saturating_add(scroll_height as u16)
        } else {
            self.scroll_offset.saturating_sub(scroll_height as u16)
        };
    }

    fn backspace(&mut self) -> anyhow::Result<Vec<Dispatch>> {
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

    fn kill(&mut self, movement: Movement, context: &Context) -> anyhow::Result<Vec<Dispatch>> {
        let buffer = self.buffer.borrow().clone();
        let mode = self.selection_set.mode.clone();

        let edit_transactions = self.selection_set.map(|selection| -> anyhow::Result<_> {
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
                    let new: Rope = buffer.slice(&other_selection.extended_range())?;

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
                &mode,
                &movement,
                context,
                get_trial_edit_transaction,
                get_actual_edit_transaction,
            )
        });
        let edit_transaction = EditTransaction::merge(
            edit_transactions
                .into_iter()
                .filter_map(|edit_transaction| edit_transaction.ok())
                .map(|edit_transaction| match edit_transaction {
                    Either::Right(edit_transaction) => edit_transaction,

                    // If no `edit_transaction` is returned, it means that the selection
                    // does not has a next item in the given direction. In this case,
                    // we should just delete the selection and collapse the cursor.
                    Either::Left(selection) => EditTransaction::from_action_groups(
                        [ActionGroup::new(
                            [
                                Action::Edit(Edit {
                                    range: selection.extended_range(),
                                    new: Rope::from(""),
                                }),
                                Action::Select(
                                    selection.clone().set_range(
                                        (selection.extended_range().start
                                            ..selection.extended_range().start)
                                            .into(),
                                    ),
                                ),
                            ]
                            .to_vec(),
                        )]
                        .to_vec(),
                    ),
                })
                .collect(),
        );
        self.apply_edit_transaction(edit_transaction)
    }

    /// Replace the parent node of the current node with the current node
    fn raise(&mut self, context: &Context) -> anyhow::Result<Vec<Dispatch>> {
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
    fn scroll(&mut self, movement: Movement, scroll_height: isize) -> anyhow::Result<()> {
        self.update_selection_set(self.selection_set.apply(
            self.selection_set.mode.clone(),
            |selection| {
                let position = selection.extended_range().start.to_position(&self.buffer());
                let position = Position {
                    line: if movement == Movement::Next {
                        position.line.saturating_add(scroll_height as usize)
                    } else {
                        position.line.saturating_sub(scroll_height as usize)
                    },
                    ..position
                };
                let start = position.to_char_index(&self.buffer())?;
                Ok(selection.clone().set_range((start..start).into()))
            },
        )?);
        self.align_cursor_to_center();
        Ok(())
    }

    pub fn reset_selection(&mut self) {
        self.selection_set = SelectionSet {
            primary: Selection::default(),
            secondary: vec![],
            mode: SelectionMode::Line,
        };
    }

    pub fn replace_previous_word(
        &mut self,
        completion: &str,
        context: &Context,
    ) -> anyhow::Result<Vec<Dispatch>> {
        let selection = self.get_selection_set(&SelectionMode::Word, Movement::Current, context)?;
        self.update_selection_set(selection);
        self.replace_current_selection_with(|_| Some(Rope::from_str(completion)));
        Ok(self.get_document_did_change_dispatch())
    }

    fn open_new_line(&mut self) -> anyhow::Result<Vec<Dispatch>> {
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
        self.enter_insert_mode(CursorDirection::End)?;
        Ok(dispatches)
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
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

    fn match_current_selection(
        &mut self,
        kind: SearchKind,
        context: &Context,
    ) -> anyhow::Result<Vec<Dispatch>> {
        let content = self
            .buffer()
            .slice(&self.selection_set.primary.extended_range())?;

        if content.len_chars() == 0 {
            return Ok(vec![]);
        }

        let search = Search {
            search: content.to_string(),
            kind,
        };
        self.select(
            SelectionMode::Find {
                search: search.clone(),
            },
            Movement::Current,
            context,
        )?;

        Ok(vec![Dispatch::SetSearch(search)])
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
            Mode::Kill => "KILL".to_string(),
            Mode::MultiCursor => "MULTI CURSOR".to_string(),
            Mode::FindOneChar => "FIND ONE CHAR".to_string(),
            Mode::ScrollPage => "SCROLL PAGE".to_string(),
            Mode::ScrollLine => "SCROLL LINE".to_string(),
            Mode::Exchange => "EXCHANGE".to_string(),
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

    fn handle_kill_mode(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> Result<Vec<Dispatch>, anyhow::Error> {
        match key_event {
            key!("esc") => {
                self.enter_normal_mode()?;
                Ok(Vec::new())
            }
            key!("k") => {
                let dispatches = self.kill(Movement::Current, context)?;
                self.enter_normal_mode()?;
                Ok(dispatches)
            }
            key!("n") => self.kill(Movement::Next, context),
            key!("p") => self.kill(Movement::Previous, context),
            other => self.handle_normal_mode(context, other),
        }
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
            key!("n") => self.add_cursor(&Movement::Next, context),
            key!("o") => self.only_current_cursor(),
            key!("p") => self.add_cursor(&Movement::Previous, context),
            other => return self.handle_normal_mode(context, other),
        }?;
        Ok(Vec::new())
    }

    fn kill_primary_cursor(&mut self) -> Result<(), anyhow::Error> {
        self.selection_set.delete_primary_cursor();
        Ok(())
    }

    fn add_cursor_to_all_selections(&mut self, context: &Context) -> Result<(), anyhow::Error> {
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
            key!("n") => self.scroll(Movement::Next, 1),
            key!("p") => self.scroll(Movement::Previous, 1),
            other => return self.handle_normal_mode(context, other),
        }?;
        Ok(Vec::new())
    }

    fn handle_scroll_page_mode(
        &mut self,
        context: &Context,
        key_event: KeyEvent,
    ) -> Result<Vec<Dispatch>, anyhow::Error> {
        let height = (self.dimension().height / 2) as isize;
        match key_event {
            key!("esc") => {
                self.enter_normal_mode()?;
                Ok(Vec::new())
            }
            key!("n") => {
                self.scroll(Movement::Next, height)?;
                Ok(Vec::new())
            }
            key!("p") => {
                self.scroll(Movement::Previous, height)?;
                Ok(Vec::new())
            }
            other => self.handle_normal_mode(context, other),
        }
    }

    /// 0-based line number
    fn go_to_line_number(&mut self, line_number: usize) -> anyhow::Result<()> {
        let range = (self.buffer().line_to_char(line_number)?
            ..self.buffer().line_to_char(line_number)?)
            .into();
        self.selection_set = SelectionSet {
            primary: self.selection_set.primary.clone().set_range(range),
            secondary: Vec::new(),
            mode: self.selection_set.mode.clone(),
        };
        self.recalculate_scroll_offset();
        Ok(())
    }

    fn move_mode_keymap_legend(&self) -> KeymapLegendConfig {
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
                    "First selection",
                    Dispatch::DispatchEditor(DispatchEditor::MoveSelection(Movement::First)),
                ),
                Keymap::new("i", "To Index (1-based)", Dispatch::OpenMoveToIndexPrompt),
                Keymap::new(
                    "n",
                    "Last selection",
                    Dispatch::DispatchEditor(DispatchEditor::MoveSelection(Movement::Last)),
                ),
            ]
            .to_vec(),
        }
    }

    fn match_literal(&mut self, context: &Context, search: &str) -> anyhow::Result<Vec<Dispatch>> {
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
    EnterScrollPageMode,
    EnterScrollLineMode,
    MoveSelection(Movement),
}

#[cfg(test)]
mod test_editor {

    use crate::{
        components::{
            component::Component,
            editor::{CursorDirection, Mode},
        },
        context::{Context, Search, SearchKind},
        position::Position,
        screen::Dispatch,
        selection::SelectionMode,
    };

    use super::{Editor, Movement};
    use my_proc_macros::keys;
    use pretty_assertions::assert_eq;
    use tree_sitter_rust::language;

    #[test]
    fn select_character() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::Character)?;
        assert_eq!(editor.get_selected_texts(), vec!["f"]);
        editor.move_selection(&context, Movement::Next)?;
        assert_eq!(editor.get_selected_texts(), vec!["n"]);

        editor.move_selection(&context, Movement::Previous)?;
        assert_eq!(editor.get_selected_texts(), vec!["f"]);
        Ok(())
    }

    #[test]
    fn select_word() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "camelCase, snake_case, ALLCAPS: 123");
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::Word)?;
        assert_eq!(editor.get_selected_texts(), vec!["camel"]);
        editor.move_selection(&context, Movement::Next)?;
        assert_eq!(editor.get_selected_texts(), vec!["Case"]);
        editor.move_selection(&context, Movement::Next)?;
        assert_eq!(editor.get_selected_texts(), vec!["snake"]);
        editor.move_selection(&context, Movement::Next)?;
        assert_eq!(editor.get_selected_texts(), vec!["case"]);
        editor.move_selection(&context, Movement::Next)?;
        assert_eq!(editor.get_selected_texts(), vec!["ALLCAPS"]);
        editor.move_selection(&context, Movement::Next)?;
        assert_eq!(editor.get_selected_texts(), vec!["123"]);

        editor.move_selection(&context, Movement::Previous)?;
        assert_eq!(editor.get_selected_texts(), vec!["ALLCAPS"]);
        editor.move_selection(&context, Movement::Previous)?;
        assert_eq!(editor.get_selected_texts(), vec!["case"]);
        editor.move_selection(&context, Movement::Previous)?;
        assert_eq!(editor.get_selected_texts(), vec!["snake"]);
        editor.move_selection(&context, Movement::Previous)?;
        assert_eq!(editor.get_selected_texts(), vec!["Case"]);
        editor.move_selection(&context, Movement::Previous)?;
        assert_eq!(editor.get_selected_texts(), vec!["camel"]);
        Ok(())
    }

    #[test]
    fn select_kids() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize, y: Vec<A>) {}");
        let context = Context::new();

        editor.match_literal(&context, "x")?;
        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["x"]);

        editor.select_kids()?;
        assert_eq!(editor.get_selected_texts(), vec!["x: usize, y: Vec<A>"]);
        Ok(())
    }

    #[test]
    fn copy_replace() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::Token)?;
        let context = Context::default();
        editor.copy(&context)?;
        editor.move_selection(&context, Movement::Next)?;
        editor.replace()?;
        assert_eq!(editor.text(), "fn fn() { let x = 1; }");
        assert_eq!(editor.get_selected_texts(), vec!["fn"]);
        editor.replace()?;
        assert_eq!(editor.text(), "fn main() { let x = 1; }");
        assert_eq!(editor.get_selected_texts(), vec!["main"]);
        Ok(())
    }

    #[test]
    fn copy_paste() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        let context = Context::default();

        editor.set_selection_mode(&context, SelectionMode::Token)?;
        editor.copy(&context)?;
        editor.move_selection(&context, Movement::Next)?;
        editor.paste(&context)?;
        assert_eq!(editor.text(), "fn fn() { let x = 1; }");
        assert_eq!(editor.get_selected_texts(), vec![""]);
        Ok(())
    }

    #[test]
    fn cut_paste() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::Token)?;
        editor.cut()?;
        assert_eq!(editor.text(), " main() { let x = 1; }");
        assert_eq!(editor.get_selected_texts(), vec![""]);

        editor.move_selection(&context, Movement::Current)?;
        assert_eq!(editor.get_selected_texts(), vec!["main"]);

        editor.paste(&context)?;

        assert_eq!(editor.text(), " fn() { let x = 1; }");
        assert_eq!(editor.get_selected_texts(), vec![""]);
        Ok(())
    }

    #[test]
    fn exchange_sibling() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize, y: Vec<A>) {}");
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::OutermostNode)?;
        // Move token to "x: usize"
        for _ in 0..3 {
            editor.move_selection(&context, Movement::Next)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["x: usize"]);

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.exchange(Movement::Next, &context)?;
        assert_eq!(editor.text(), "fn main(y: Vec<A>, x: usize) {}");

        editor.exchange(Movement::Previous, &context)?;
        assert_eq!(editor.text(), "fn main(x: usize, y: Vec<A>) {}");
        Ok(())
    }

    #[test]
    fn exchange_sibling_2() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "use a;\nuse b;\nuse c;");
        let context = Context::default();

        // Select first statement
        editor.set_selection_mode(&context, SelectionMode::Token)?;
        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.move_selection(&context, Movement::Up)?;
        assert_eq!(editor.get_selected_texts(), vec!["use a;"]);

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.exchange(Movement::Next, &context)?;
        assert_eq!(editor.text(), "use b;\nuse a;\nuse c;");
        editor.exchange(Movement::Next, &context)?;
        assert_eq!(editor.text(), "use b;\nuse c;\nuse a;");
        Ok(())
    }

    #[test]
    fn raise() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = a.b(c()); }");
        let context = Context::default();
        // Move selection to "c()"
        editor.match_literal(&context, "c()")?;
        assert_eq!(editor.get_selected_texts(), vec!["c()"]);

        editor.raise(&context)?;
        assert_eq!(editor.text(), "fn main() { let x = c(); }");

        editor.raise(&context)?;
        assert_eq!(editor.text(), "fn main() { c() }");
        Ok(())
    }

    #[test]
    fn exchange_line() -> anyhow::Result<()> {
        // Multiline source code
        let mut editor = Editor::from_text(
            language(),
            "
fn main() {
    let x = 1;
    let y = 2;
}",
        );

        let context = Context::default();
        editor.select_line(Movement::Next, &context)?;
        editor.select_line(Movement::Next, &context)?;

        editor.exchange(Movement::Next, &context)?;
        assert_eq!(
            editor.text(),
            "
    let x = 1;
fn main() {
    let y = 2;
}"
        );

        editor.exchange(Movement::Previous, &context)?;
        assert_eq!(
            editor.text(),
            "
fn main() {
    let x = 1;
    let y = 2;
}"
        );
        Ok(())
    }

    #[test]
    fn exchange_character() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::Character)?;
        editor.exchange(Movement::Next, &context)?;
        assert_eq!(editor.text(), "nf main() { let x = 1; }");
        editor.exchange(Movement::Next, &context)?;
        assert_eq!(editor.text(), "n fmain() { let x = 1; }");

        editor.exchange(Movement::Previous, &context)?;
        assert_eq!(editor.text(), "nf main() { let x = 1; }");
        editor.exchange(Movement::Previous, &context)?;

        assert_eq!(editor.text(), "fn main() { let x = 1; }");
        Ok(())
    }

    #[test]
    fn multi_insert() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "struct A(usize, char)");
        let context = Context::default();

        // Select 'usize'
        editor.match_literal(&context, "usize")?;
        assert_eq!(editor.get_selected_texts(), vec!["usize"]);

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.add_cursor(&Movement::Next, &context)?;

        assert_eq!(editor.get_selected_texts(), vec!["usize", "char"]);
        editor.enter_insert_mode(CursorDirection::Start)?;
        editor.insert("pub ")?;

        assert_eq!(editor.text(), "struct A(pub usize, pub char)");

        editor.backspace()?;

        assert_eq!(editor.text(), "struct A(pubusize, pubchar)");
        assert_eq!(editor.get_selected_texts(), vec!["", ""]);
        Ok(())
    }

    #[test]
    fn multi_raise() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        let context = Context::default();

        // Select 'let x = S(a);'
        editor.match_literal(&context, "let x = S(a);")?;
        assert_eq!(editor.get_selected_texts(), vec!["let x = S(a);"]);

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.add_cursor(&Movement::Next, &context)?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["let x = S(a);", "let y = S(b);"]
        );

        editor.set_selection_mode(&context, SelectionMode::OutermostNode)?;
        for _ in 0..5 {
            editor.move_selection(&context, Movement::Next)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["a", "b"]);

        editor.raise(&context)?;

        assert_eq!(editor.text(), "fn f(){ let x = a; let y = b; }");

        editor.undo()?;

        assert_eq!(editor.text(), "fn f(){ let x = S(a); let y = S(b); }");
        assert_eq!(editor.get_selected_texts(), vec!["a", "b"]);

        editor.redo()?;

        assert_eq!(editor.text(), "fn f(){ let x = a; let y = b; }");
        assert_eq!(editor.get_selected_texts(), vec!["a", "b"]);
        Ok(())
    }

    #[test]
    fn multi_exchange_sibling() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(x:a,y:b){} fn g(x:a,y:b){}");
        let context = Context::default();

        // Select 'fn f(x:a,y:b){}'
        editor.match_literal(&context, "fn f(x:a,y:b){}")?;
        assert_eq!(editor.get_selected_texts(), vec!["fn f(x:a,y:b){}"]);

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.add_cursor(&Movement::Next, &context)?;
        assert_eq!(
            editor.get_selected_texts(),
            vec!["fn f(x:a,y:b){}", "fn g(x:a,y:b){}"]
        );

        editor.set_selection_mode(&context, SelectionMode::OutermostNode)?;
        for _ in 0..3 {
            editor.move_selection(&context, Movement::Next)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["x:a", "x:a"]);

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.exchange(Movement::Next, &context)?;
        assert_eq!(editor.text(), "fn f(y:b,x:a){} fn g(y:b,x:a){}");
        assert_eq!(editor.get_selected_texts(), vec!["x:a", "x:a"]);

        editor.exchange(Movement::Previous, &context)?;
        assert_eq!(editor.text(), "fn f(x:a,y:b){} fn g(x:a,y:b){}");
        Ok(())
    }

    #[test]
    fn multi_paste() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(
            language(),
            "fn f(){ let x = S(spongebob_squarepants); let y = S(b); }",
        );
        let context = Context::default();

        // Select 'let x = S(a)'
        editor.match_literal(&context, "let x = S(spongebob_squarepants);")?;
        assert_eq!(
            editor.get_selected_texts(),
            vec!["let x = S(spongebob_squarepants);"]
        );

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.add_cursor(&Movement::Next, &context)?;

        editor.set_selection_mode(&context, SelectionMode::OutermostNode)?;
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["S(spongebob_squarepants)", "S(b)"]
        );

        let context = Context::default();
        editor.cut()?;
        editor.enter_insert_mode(CursorDirection::Start)?;

        editor.insert("Some(")?;
        editor.paste(&context)?;
        editor.insert(")")?;

        assert_eq!(
            editor.text(),
            "fn f(){ let x = Some(S(spongebob_squarepants)); let y = Some(S(b)); }"
        );
        Ok(())
    }

    #[test]
    fn toggle_highlight_mode() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        let context = Context::default();

        editor.set_selection_mode(&context, SelectionMode::Token)?;
        editor.toggle_highlight_mode();
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f("]);

        // Toggle the second time should inverse the initial_range
        editor.toggle_highlight_mode();

        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["f("]);

        editor.reset();

        assert_eq!(editor.get_selected_texts(), vec!["f"]);

        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["("]);

        Ok(())
    }

    #[test]
    fn highlight_mode_cut() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::Token)?;
        editor.toggle_highlight_mode();
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f()"]);

        let context = Context::default();
        editor.cut()?;

        assert_eq!(editor.text(), "{ let x = S(a); let y = S(b); }");

        editor.paste(&context)?;

        assert_eq!(editor.text(), "fn f(){ let x = S(a); let y = S(b); }");
        Ok(())
    }

    #[test]
    fn highlight_mode_copy() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::Token)?;
        editor.toggle_highlight_mode();
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f()"]);

        let context = Context::default();
        editor.copy(&context)?;

        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["{"]);

        editor.paste(&context)?;

        assert_eq!(editor.text(), "fn f()fn f() let x = S(a); let y = S(b); }");
        Ok(())
    }

    #[test]
    fn highlight_mode_replace() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::Token)?;
        editor.toggle_highlight_mode();
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f()"]);

        let context = Context::default();
        editor.copy(&context)?;

        editor.set_selection_mode(&context, SelectionMode::OutermostNode)?;
        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["{ let x = S(a); let y = S(b); }"]
        );

        editor.replace()?;

        assert_eq!(editor.text(), "fn f()fn f()");
        Ok(())
    }

    #[test]
    fn highlight_mode_paste() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        let context = Context::default();

        editor.set_selection_mode(&context, SelectionMode::Token)?;
        let context = Context::default();
        editor.copy(&context)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn"]);

        editor.toggle_highlight_mode();
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Next)?;
        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f()"]);

        editor.paste(&context)?;

        assert_eq!(editor.text(), "fn{ let x = S(a); let y = S(b); }");
        Ok(())
    }

    #[test]
    fn open_new_line() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(
            language(),
            "
fn f() {
    let x = S(a);
}
"
            .trim(),
        );

        // Move to the second line
        editor.select_line_at(1)?;

        assert_eq!(editor.get_selected_texts(), vec!["    let x = S(a);\n"]);

        editor.open_new_line()?;

        assert_eq!(editor.mode, Mode::Insert);

        editor.insert("let y = S(b);")?;

        assert_eq!(
            editor.text(),
            "
fn f() {
    let x = S(a);
    let y = S(b);
}"
            .trim()
        );
        Ok(())
    }

    #[test]
    fn kill_character() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        let context = Context::default();

        editor.set_selection_mode(&context, SelectionMode::Character)?;
        assert_eq!(editor.get_selected_texts(), vec!["f"]);

        editor.kill(Movement::Next, &context)?;
        assert_eq!(editor.text(), "n f(){ let x = S(a); let y = S(b); }");

        editor.kill(Movement::Next, &context)?;
        assert_eq!(editor.text(), " f(){ let x = S(a); let y = S(b); }");

        editor.set_selection_mode(
            &context,
            SelectionMode::Find {
                search: Search {
                    search: "x".to_string(),
                    kind: SearchKind::Literal,
                },
            },
        )?;
        editor.move_selection(&context, Movement::Next)?;
        editor.set_selection_mode(&context, SelectionMode::Character)?;
        assert_eq!(editor.get_selected_texts(), vec!["x"]);

        editor.kill(Movement::Previous, &context)?;
        assert_eq!(editor.text(), " f(){ let  = S(a); let y = S(b); }");
        Ok(())
    }

    #[test]
    fn kill_line() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(
            language(),
            "
fn f() {
let x = S(a);

let y = S(b);
}"
            .trim(),
        );
        let context = Context::default();
        editor.set_selection_mode(&context, SelectionMode::Line)?;
        assert_eq!(editor.get_selected_texts(), vec!["fn f() {\n"]);

        editor.kill(Movement::Next, &context)?;
        assert_eq!(
            editor.text(),
            "
let x = S(a);

let y = S(b);
}"
            .trim()
        );

        editor.kill(Movement::Next, &context)?;
        assert_eq!(
            editor.text(),
            "
let y = S(b);
}"
        );
        assert_eq!(editor.get_selected_texts(), vec!["\n"]);

        editor.move_selection(&context, Movement::Next)?;
        assert_eq!(editor.get_selected_texts(), vec!["let y = S(b);\n"]);
        editor.kill(Movement::Previous, &context)?;
        assert_eq!(
            editor.text(),
            "
}"
        );

        editor.kill(Movement::Next, &context)?;
        assert_eq!(editor.text(), "}");

        Ok(())
    }

    #[test]
    fn kill_sibling() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(x: a, y: b, z: c){}");
        let context = Context::default();

        // Select 'x: a'
        editor.match_literal(&context, "x: a")?;
        assert_eq!(editor.get_selected_texts(), vec!["x: a"]);

        editor.set_selection_mode(&context, SelectionMode::SyntaxTree)?;
        editor.kill(Movement::Next, &context)?;

        assert_eq!(editor.text(), "fn f(y: b, z: c){}");

        editor.move_selection(&context, Movement::Next)?;
        editor.kill(Movement::Previous, &context)?;

        assert_eq!(editor.text(), "fn f(y: b){}");
        Ok(())
    }

    #[test]
    fn kill_token() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(x: a, y: b, z: c){}");
        let context = Context::default();
        // Select 'fn'
        editor.set_selection_mode(&context, SelectionMode::Token)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn"]);

        editor.kill(Movement::Next, &context)?;

        assert_eq!(editor.text(), "f(x: a, y: b, z: c){}");

        editor.kill(Movement::Next, &context)?;

        assert_eq!(editor.text(), "(x: a, y: b, z: c){}");

        editor.move_selection(&context, Movement::Next)?;

        assert_eq!(editor.get_selected_texts(), vec!["x"]);

        editor.kill(Movement::Previous, &context)?;

        assert_eq!(editor.text(), "(: a, y: b, z: c){}");
        Ok(())
    }

    #[test]
    fn paste_from_clipboard() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        let mut context = Context::default();

        context.set_clipboard_content("let z = S(c);".to_string());

        editor.reset();

        editor.paste(&context)?;

        assert_eq!(
            editor.text(),
            "let z = S(c);fn f(){ let x = S(a); let y = S(b); }"
        );
        Ok(())
    }

    #[test]
    fn enter_newline() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "");

        // Enter insert mode
        editor.enter_insert_mode(CursorDirection::Start)?;
        // Type in 'hello'
        editor.handle_events(keys!("h e l l o"))?;

        // Type in enter
        editor.handle_events(keys!("enter"))?;

        // Type in 'world'
        editor.handle_events(keys!("w o r l d"))?;

        // Expect the text to be 'hello\nworld'
        assert_eq!(editor.text(), "hello\nworld");

        // Move cursor left
        editor.handle_events(keys!("left"))?;

        // Type in enter
        editor.handle_events(keys!("enter"))?;

        // Expect the text to be 'hello\nworl\nd'
        assert_eq!(editor.text(), "hello\nworl\nd");
        Ok(())
    }

    #[test]
    fn set_selection() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() {}");

        // Select a range which highlights a node
        editor.set_selection(Position::new(0, 0)..Position::new(0, 2))?;

        assert_eq!(editor.selection_set.mode, SelectionMode::OutermostNode);

        // Select a range which does not highlights a node
        editor.set_selection(Position::new(0, 0)..Position::new(0, 1))?;

        assert_eq!(editor.selection_set.mode, SelectionMode::Custom);

        Ok(())
    }

    #[test]
    fn insert_mode_start() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() {}");
        let context = Context::default();

        // Select the first word
        editor.set_selection_mode(&context, SelectionMode::Word)?;

        // Enter insert mode
        editor.enter_insert_mode(CursorDirection::Start)?;

        // Type something
        editor.insert("hello")?;

        // Expect the text to be 'hellofn main() {}'
        assert_eq!(editor.text(), "hellofn main() {}");
        Ok(())
    }

    #[test]
    fn insert_mode_end() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() {}");
        let context = Context::default();

        // Select the first word
        editor.set_selection_mode(&context, SelectionMode::Word)?;

        // Enter insert mode
        editor.enter_insert_mode(CursorDirection::End)?;

        // Type something
        editor.insert("hello")?;

        // Expect the text to be 'fnhello main() {}'
        assert_eq!(editor.text(), "fnhello main() {}");
        Ok(())
    }

    #[test]
    fn enclose_left_bracket() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { x.y() }");
        let context = Context::default();

        // Select 'x.y()'
        editor.match_literal(&context, "x.y()")?;

        assert_eq!(editor.get_selected_texts(), vec!["x.y()"]);

        editor.handle_events(keys!("( { [ <")).unwrap();

        assert_eq!(editor.text(), "fn main() { <[{(x.y())}]> }");
        assert_eq!(editor.get_selected_texts(), vec!["<[{(x.y())}]>"]);
        Ok(())
    }

    #[test]
    fn enclose_right_bracket() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { x.y() }");
        let context = Context::default();

        // Select 'x.y()'
        editor.match_literal(&context, "x.y()")?;

        assert_eq!(editor.get_selected_texts(), vec!["x.y()"]);

        editor.handle_events(keys!(") } ] >")).unwrap();

        assert_eq!(editor.text(), "fn main() { <[{(x.y())}]> }");
        assert_eq!(editor.get_selected_texts(), vec!["<[{(x.y())}]>"]);
        Ok(())
    }

    #[test]
    fn match_current_selection() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn\nmain()\n{ x.y(); x.y(); x.y(); }");
        let context = Context::default();

        // Select x.y()
        editor.match_literal(&context, "x.y()")?;

        assert_eq!(editor.get_selected_texts(), vec!["x.y()"]);

        let dispatches = editor.match_current_selection(SearchKind::Literal, &context)?;

        let search = Search {
            search: "x.y()".to_string(),
            kind: SearchKind::Literal,
        };
        assert_eq!(dispatches, vec![Dispatch::SetSearch(search.clone())]);
        assert_eq!(editor.selection_set.mode, SelectionMode::Find { search });
        Ok(())
    }

    #[test]
    fn enter_normal_mode_should_highlight_one_character() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn\nmain()\n{ x.y(); x.y(); x.y(); }");
        let context = Context::default();
        // Select x.y()
        editor.match_literal(&context, "x.y()")?;

        editor.enter_insert_mode(CursorDirection::End)?;
        editor.enter_normal_mode()?;
        assert_eq!(editor.get_selected_texts(), vec![")"]);
        Ok(())
    }
}
