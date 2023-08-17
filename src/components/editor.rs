use crate::{
    canonicalized_path::CanonicalizedPath,
    char_index_range::CharIndexRange,
    context::{Context, GlobalMode, Search, SearchKind},
    grid::{CellUpdate, Style},
    screen::{FilePickerKind, RequestParams},
    selection_mode, soft_wrap,
    themes::Theme,
};
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
use tree_sitter::Node;

use crate::{
    buffer::Buffer,
    components::component::Component,
    edit::{Action, ActionGroup, Edit, EditTransaction},
    grid::{Cell, Grid},
    lsp::{completion::PositionalEdit, diagnostic::Diagnostic},
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
    Jump { jumps: Vec<Jump> },
    Kill,
    AddCursor,
    FindOneChar,
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

    fn set_content(&mut self, str: &str) {
        self.update_buffer(str)
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn set_title(&mut self, title: String) {
        self.title = title;
    }

    fn get_grid(&self, theme: &Theme, diagnostics: &[Diagnostic]) -> GetGridResult {
        let editor = self;
        let Dimension { height, width } = editor.dimension();

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

        let line_numbers_grid = (0..height.min(len_lines.saturating_sub(scroll_offset))).fold(
            Grid::new(Dimension {
                height,
                width: max_line_number_len,
            }),
            |grid, index| {
                let line_number = index + scroll_offset + 1;
                let line_number_str = format!(
                    "{: >width$}",
                    line_number.to_string(),
                    width = max_line_number_len as usize
                );

                if let Ok(position) = wrapped_lines.calibrate(Position {
                    line: (line_number
                        .saturating_sub(scroll_offset as u16)
                        .saturating_sub(1)) as usize,
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

        let line_numbers_separator_grid = (0..height).fold(
            Grid::new(Dimension {
                height,
                width: line_number_separator_width,
            }),
            |grid, index| grid.set_line(index as usize, "│", theme.ui.line_number_separator),
        );

        let mut grid: Grid = Grid::new(Dimension { height, width });
        let selection = &editor.selection_set.primary;

        // If the buffer selection is updated less recently than the window's scroll offset,
        // use the window's scroll offset.

        let lines = wrapped_lines
            .lines()
            .into_iter()
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

        let primary_selection_secondary_cursor = char_index_to_cell_update(
            &buffer,
            selection.to_char_index(&editor.cursor_direction.reverse()),
            theme.ui.primary_selection_secondary_cursor,
        );

        let secondary_selection = secondary_selections.iter().flat_map(|secondary_selection| {
            range_to_cell_update(
                &buffer,
                secondary_selection.extended_range(),
                theme.ui.secondary_selection,
            )
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
        let syntax_highlight = highlighted_spans.iter().flat_map(|highlighted_span| {
            highlighted_span
                .byte_range
                .clone()
                .into_iter()
                .filter_map(|byte| {
                    Some(
                        CellUpdate::new(buffer.byte_to_position(byte).ok()?)
                            .style(highlighted_span.style),
                    )
                })
        });

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
                    .char_to_position(match editor.cursor_direction {
                        CursorDirection::Start => jump.selection.range.start,
                        CursorDirection::End => jump.selection.range.start,
                    })
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
            .into_iter()
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
            .chain(bookmarks)
            .chain(primary_selection)
            .chain(secondary_selection)
            .chain(diagnostics)
            .chain(syntax_highlight)
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

        let left_width =
            line_numbers_grid.dimension().width + line_numbers_separator_grid.dimension().width;

        let cursor_position = self
            .get_cursor_position()
            .ok()
            .map(|position| position.move_up(scroll_offset as usize));

        GetGridResult {
            cursor_position: {
                let cursor_position = cursor_position
                    .and_then(|position| {
                        // Need to move the cursor left by one to account for
                        // the insert mode cursor position at the last column of the current line
                        // which exceeds the columns of the current line by one in
                        // insert mode
                        let column_non_zero = position.column > 0;
                        let position = if column_non_zero {
                            position.move_left(1)
                        } else {
                            position
                        };
                        let position = wrapped_lines.calibrate(position).ok()?;
                        Some(if column_non_zero {
                            // Move the cursor right by one to account for the
                            // move left by one above
                            position.move_right(1)
                        } else {
                            position
                        })
                    })
                    .map(|position| position.move_right(left_width as u16));
                cursor_position
            },
            grid: line_numbers_grid
                .merge_horizontal(line_numbers_separator_grid)
                .merge_horizontal(grid.apply_cell_updates(updates)),
        }
    }

    fn handle_paste_event(&mut self, content: String) -> anyhow::Result<Vec<Dispatch>> {
        Ok(self.insert(&content))
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
        context: &mut Context,
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
        let mut context = Context::default();
        Ok(events
            .iter()
            .map(|event| self.handle_key_event(&mut context, event.clone()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>())
    }

    fn handle_event(
        &mut self,
        context: &mut Context,
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
pub enum Direction {
    Right,
    Left,
    RightMost,
    Current,
    Up,
    Down,
    LeftMost,
}

impl Editor {
    pub fn from_text(language: tree_sitter::Language, text: &str) -> Self {
        Self {
            selection_set: SelectionSet {
                primary: Selection::default(),
                secondary: vec![],
                mode: SelectionMode::Custom,
            },
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
                path.display_relative()
                    .unwrap_or_else(|_| "<Untitled>".to_string())
            })
            .unwrap_or_else(|| "<Untitled>".to_string());
        Self {
            selection_set: SelectionSet {
                primary: Selection::default(),
                secondary: vec![],
                mode: SelectionMode::Custom,
            },
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
            .get_line(cursor)?
            .to_string()
            .trim()
            .into())
    }

    pub fn get_current_word(&self) -> anyhow::Result<String> {
        let cursor = self.get_cursor_char_index();
        self.buffer.borrow().get_word_before_char_index(cursor)
    }

    fn select_syntax_tree(&mut self, direction: Direction) -> anyhow::Result<()> {
        self.select(SelectionMode::SyntaxTree, direction)
    }

    fn select_kids(&mut self) -> anyhow::Result<()> {
        let buffer = self.buffer.borrow().clone();
        self.update_selection_set(
            self.selection_set
                .select_kids(&buffer, &self.cursor_direction)?,
        );
        Ok(())
    }

    pub fn select_line(&mut self, direction: Direction) -> anyhow::Result<()> {
        self.select(SelectionMode::Line, direction)
    }

    pub fn select_line_at(&mut self, line: usize) -> anyhow::Result<()> {
        let start = self.buffer.borrow().line_to_char(line)?;
        let selection_set = SelectionSet {
            primary: Selection {
                range: (start..start + self.buffer.borrow().get_line(start)?.len_chars()).into(),
                copied_text: None,
                initial_range: None,
                info: None,
            },
            secondary: vec![],
            mode: SelectionMode::Line,
        };
        self.update_selection_set(selection_set);
        Ok(())
    }

    pub fn select_match(
        &mut self,
        direction: Direction,
        search: &Option<Search>,
    ) -> anyhow::Result<()> {
        if let Some(search) = search {
            self.select(
                SelectionMode::Match {
                    search: search.clone(),
                },
                direction,
            )?;
        }
        Ok(())
    }

    fn select_named_node(&mut self, direction: Direction) -> anyhow::Result<()> {
        self.select(SelectionMode::LargestNode, direction)
    }

    fn select_character(&mut self, direction: Direction) -> anyhow::Result<()> {
        self.select(SelectionMode::Character, direction)
    }

    fn select_diagnostic(&mut self, direction: Direction) -> anyhow::Result<Vec<Dispatch>> {
        self.select(
            SelectionMode::Diagnostic(Some(DiagnosticSeverity::ERROR)),
            direction,
        )?;
        if let Some(diagnostic) = self
            .buffer
            .borrow()
            .find_diagnostic(&self.selection_set.primary.range)
        {
            Ok(vec![Dispatch::ShowInfo {
                title: "Diagnostic".to_string(),
                content: vec![diagnostic.message()],
            }])
        } else {
            Ok(vec![])
        }
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

    fn reset(&mut self) -> anyhow::Result<()> {
        self.select(self.selection_set.mode.clone(), Direction::Current)?;
        self.selection_set.only();
        Ok(())
    }

    fn select_token(&mut self, direction: Direction) -> anyhow::Result<()> {
        self.select(SelectionMode::Token, direction)
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
            SelectionMode::LargestNode
        } else {
            SelectionMode::Custom
        };
        let selection_set = SelectionSet {
            primary: Selection {
                range,
                copied_text: self.selection_set.primary.copied_text.clone(),
                initial_range: None,
                info: None,
            },
            secondary: vec![],
            mode,
        };
        self.update_selection_set(selection_set);
        Ok(())
    }

    fn cursor_row(&self) -> u16 {
        self.get_cursor_char_index()
            .to_position(self.buffer.borrow().rope())
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
        direction: Direction,
    ) -> anyhow::Result<()> {
        //  There are a few selection modes where Current make sense.
        let direction = if self.selection_set.mode != selection_mode {
            Direction::Current
        } else {
            direction
        };

        let selection = self.get_selection_set(&selection_mode, direction)?;

        self.update_selection_set(selection);
        Ok(())
    }

    /// TODO: this should also show diagnostics
    fn select_final(&mut self, direction: Direction) -> anyhow::Result<()> {
        let selection_mode = self.selection_set.mode.clone();
        fn get_final_selection(
            buffer: &Buffer,
            selection: &Selection,
            mode: &SelectionMode,
            direction: &Direction,
        ) -> anyhow::Result<Selection> {
            let next_selection = Selection::get_selection_(
                buffer,
                selection,
                mode,
                direction,
                &CursorDirection::Start,
            )?;
            if next_selection == *selection {
                Ok(selection.clone())
            } else {
                get_final_selection(buffer, &next_selection, mode, direction)
            }
        }

        let selection_set = self
            .selection_set
            .apply(selection_mode.clone(), |selection| {
                get_final_selection(
                    &self.buffer.borrow(),
                    selection,
                    &selection_mode,
                    &direction,
                )
            })?;

        self.update_selection_set(selection_set);
        Ok(())
    }
    fn jump_characters() -> Vec<char> {
        ('a'..='z').collect_vec()
    }

    fn jump_from_selection(&mut self, selection: &Selection) -> anyhow::Result<()> {
        let chars = Self::jump_characters();

        let object = self
            .selection_set
            .mode
            .to_selection_mode_trait_object(&self.buffer(), selection)?;

        let line_range = self.line_range();
        let jumps = object.jumps(
            selection_mode::SelectionModeParams {
                buffer: &self.buffer(),
                current_selection: selection,
                cursor_direction: &self.cursor_direction,
            },
            chars,
            line_range,
        )?;
        self.mode = Mode::Jump { jumps };

        Ok(())
    }

    fn jump(&mut self) -> anyhow::Result<()> {
        self.jump_from_selection(&self.selection_set.primary.clone())
    }

    fn cut(&mut self, context: &mut Context) -> anyhow::Result<Vec<Dispatch>> {
        // Set the clipboard content to the current selection
        // if there is only one cursor.
        if self.selection_set.secondary.is_empty() {
            context.set_clipboard_content(
                self.buffer
                    .borrow()
                    .slice(&self.selection_set.primary.range)?
                    .into(),
            )
        }
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let old_range = selection.extended_range();
                    let old = self.buffer.borrow().slice(&old_range)?;
                    Ok(ActionGroup::new(vec![
                        Action::Edit(Edit {
                            range: old_range,
                            new: Rope::new(),
                        }),
                        Action::Select(Selection {
                            range: (old_range.start..old_range.start).into(),
                            copied_text: Some(old),
                            initial_range: None,
                            info: None,
                        }),
                    ]))
                })
                .into_iter()
                .flatten()
                .collect(),
        );

        Ok(self.apply_edit_transaction(edit_transaction))
    }

    fn copy(&mut self, context: &mut Context) {
        self.selection_set.copy(&self.buffer.borrow(), context);
    }

    fn replace_current_selection_with<F>(&mut self, f: F) -> Vec<Dispatch>
    where
        F: Fn(&Selection) -> Option<Rope>,
    {
        let edit_transactions = self.selection_set.map(|selection| {
            if let Some(copied_text) = f(selection) {
                let range = selection.extended_range();
                let start = range.start;
                EditTransaction::from_action_groups(vec![ActionGroup::new(vec![
                    Action::Edit(Edit {
                        range,
                        new: copied_text.clone(),
                    }),
                    Action::Select(Selection {
                        range: {
                            let start = start + copied_text.len_chars();
                            (start..start).into()
                        },
                        copied_text: Some(copied_text),
                        initial_range: None,
                        info: None,
                    }),
                ])])
            } else {
                EditTransaction::from_action_groups(vec![])
            }
        });
        let edit_transaction = EditTransaction::merge(edit_transactions);
        self.apply_edit_transaction(edit_transaction)
    }

    fn paste(&mut self, context: &mut Context) -> Vec<Dispatch> {
        self.replace_current_selection_with(|selection| {
            selection
                .copied_text
                .clone()
                .or_else(|| context.get_clipboard_content().map(Rope::from))
        })
    }

    fn replace(&mut self) -> Vec<Dispatch> {
        let edit_transaction = EditTransaction::merge(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    if let Some(replacement) = &selection.copied_text {
                        let replacement_text_len = replacement.len_chars();
                        let replaced_text = self.buffer.borrow().slice(&selection.range)?;
                        Ok(EditTransaction::from_action_groups(vec![ActionGroup::new(
                            vec![
                                Action::Edit(Edit {
                                    range: selection.range,
                                    new: replacement.clone(),
                                }),
                                Action::Select(Selection {
                                    range: (selection.range.start
                                        ..selection.range.start + replacement_text_len)
                                        .into(),
                                    copied_text: Some(replaced_text),
                                    initial_range: None,
                                    info: None,
                                }),
                            ],
                        )]))
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

    fn apply_edit_transaction(&mut self, edit_transaction: EditTransaction) -> Vec<Dispatch> {
        self.buffer
            .borrow_mut()
            .apply_edit_transaction(&edit_transaction, self.selection_set.clone())
            .unwrap();

        if let Some((head, tail)) = edit_transaction.selections().split_first() {
            self.selection_set = SelectionSet {
                primary: (*head).clone(),
                secondary: tail.iter().map(|selection| (*selection).clone()).collect(),
                mode: self.selection_set.mode.clone(),
            }
        }

        self.recalculate_scroll_offset();

        self.get_document_did_change_dispatch()
    }

    pub fn get_document_did_change_dispatch(&mut self) -> Vec<Dispatch> {
        if let Some(path) = self.buffer().path() {
            vec![Dispatch::DocumentDidChange {
                path,
                content: self.buffer().rope().to_string(),
            }]
        } else {
            vec![]
        }
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
        direction: Direction,
    ) -> anyhow::Result<SelectionSet> {
        self.selection_set.generate(
            &self.buffer.borrow(),
            mode,
            &direction,
            &self.cursor_direction,
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

    fn find_mode_keymap_legend_config(&self) -> anyhow::Result<KeymapLegendConfig> {
        Ok(KeymapLegendConfig {
            title: "Find by",
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
                        SelectionMode::Match {
                            search: Search {
                                kind: SearchKind::IgnoreCase,
                                search: self.current_selection()?,
                            },
                        },
                    )),
                ),
                Keymap::new(
                    "g",
                    "Global",
                    Dispatch::ShowKeymapLegend(KeymapLegendConfig {
                        title: "Find Global",
                        owner_id: self.id(),
                        keymaps: [
                            Keymap::new(
                                "a",
                                "AST Grep",
                                Dispatch::OpenGlobalSearchPrompt(SearchKind::AstGrep),
                            ),
                            Keymap::new(
                                "c",
                                "Current selection",
                                Dispatch::GlobalSearch(Search {
                                    kind: SearchKind::IgnoreCase,
                                    search: self.current_selection()?,
                                }),
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
                        .to_vec(),
                    }),
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
                        title: "Find number",
                        owner_id: self.id(),
                        keymaps: [
                            ("f", "Float", r"[-+]?\d*\.\d+|\d+"),
                            ("i", "Integer", r"-?\d+"),
                            ("n", "Natural", r"\d+"),
                            ("s", "Scientific", r"[-+]?\d*\.?\d+[eE][-+]?\d+"),
                        ]
                        .into_iter()
                        .map(|(key, description, regex)| {
                            let search = Search {
                                search: regex.to_string(),
                                kind: SearchKind::Regex,
                            };
                            let dispatch = Dispatch::DispatchEditor(
                                DispatchEditor::SetSelectionMode(SelectionMode::Match { search }),
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
            ]
            .to_vec(),
        })
    }

    fn space_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Space",
            owner_id: self.id(),
            keymaps: vec![]
                .into_iter()
                .chain(
                    self.get_request_params()
                        .map(|params| {
                            [
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
            title: "Transform",
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
            title: "View",
            keymaps: [
                ("b", "Align view bottom", DispatchEditor::AlignViewBottom),
                ("c", "Align view center", DispatchEditor::AlignViewCenter),
                ("d", "Scroll down", DispatchEditor::ScrollDown),
                ("t", "Align view top", DispatchEditor::AlignViewTop),
                ("u", "Scroll up", DispatchEditor::ScrollUp),
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
        context: &mut Context,
        dispatch: DispatchEditor,
    ) -> anyhow::Result<Vec<Dispatch>> {
        match dispatch {
            DispatchEditor::ScrollUp => self.select_view(Direction::Left)?,
            DispatchEditor::ScrollDown => self.select_view(Direction::Right)?,
            DispatchEditor::AlignViewTop => self.align_cursor_to_top(),
            DispatchEditor::AlignViewCenter => self.align_cursor_to_center(),
            DispatchEditor::AlignViewBottom => self.align_cursor_to_bottom(),
            DispatchEditor::Transform(case) => return Ok(self.transform_selection(case)),
            DispatchEditor::SetSelectionMode(selection_mode) => {
                return self.set_selection_mode(context, selection_mode)
            }
            DispatchEditor::EnterInsertMode(cursor_direction) => {
                self.enter_insert_mode(cursor_direction)
            }
            DispatchEditor::FindOneChar => self.enter_single_character_mode(),
        }
        Ok([].to_vec())
    }

    fn diagnostic_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Diagnostic",
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

    fn insert_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Insert",
            owner_id: self.id(),
            keymaps: [
                Keymap::new(
                    "n",
                    "End of selection",
                    Dispatch::DispatchEditor(DispatchEditor::EnterInsertMode(CursorDirection::End)),
                ),
                Keymap::new(
                    "p",
                    "Opening of selection",
                    Dispatch::DispatchEditor(DispatchEditor::EnterInsertMode(
                        CursorDirection::Start,
                    )),
                ),
            ]
            .to_vec(),
        }
    }

    fn g_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Get",
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
                            "s",
                            "Symbols",
                            Dispatch::RequestDocumentSymbols(params.clone()),
                        ),
                        Keymap::new(
                            "t",
                            "Type Definition(s)",
                            Dispatch::RequestTypeDefinitions(params),
                        ),
                    ]
                    .into_iter()
                    .chain(
                        [
                            ("g", "Git status", FilePickerKind::GitStatus),
                            ("n", "Not git ignored", FilePickerKind::NonGitIgnored),
                            ("o", "Opened", FilePickerKind::Opened),
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
        context: &mut Context,
        key_event: KeyEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        match self.handle_universal_key(context, key_event)? {
            HandleEventResult::Ignored(key_event) => match &self.mode {
                Mode::Normal => self.handle_normal_mode(context, key_event),
                Mode::Insert => self.handle_insert_mode(key_event),
                Mode::Jump { .. } => {
                    self.handle_jump_mode(key_event)?;
                    Ok(vec![])
                }
                Mode::Kill => self.handle_kill_mode(key_event),
                Mode::AddCursor => {
                    self.handle_add_cursor_mode(key_event)?;
                    Ok(Vec::new())
                }
                Mode::FindOneChar => self.handle_find_one_char_mode(context, key_event),
            },
            HandleEventResult::Handled(dispatches) => Ok(dispatches),
        }
    }

    fn handle_universal_key(
        &mut self,
        context: &mut Context,
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
                    primary: Selection {
                        range: (CharIndex(0)..CharIndex(self.buffer.borrow().len_chars())).into(),
                        copied_text: self.selection_set.primary.copied_text.clone(),
                        initial_range: None,
                        info: None,
                    },
                    secondary: vec![],
                    mode: SelectionMode::Custom,
                };
                self.update_selection_set(selection_set);
                Ok(HandleEventResult::Handled(vec![]))
            }
            key!("ctrl+c") => {
                self.copy(context);
                Ok(HandleEventResult::Handled(vec![]))
            }
            key!("ctrl+s") => {
                let dispatches = self.save()?;
                self.mode = Mode::Normal;
                Ok(HandleEventResult::Handled(dispatches))
            }
            key!("ctrl+x") => Ok(HandleEventResult::Handled(self.cut(context)?)),
            key!("ctrl+v") => Ok(HandleEventResult::Handled(self.paste(context))),
            key!("ctrl+y") => Ok(HandleEventResult::Handled(self.redo()?)),
            key!("ctrl+z") => Ok(HandleEventResult::Handled(self.undo()?)),
            _ => Ok(HandleEventResult::Ignored(event)),
        }
    }

    fn handle_jump_mode(&mut self, key_event: KeyEvent) -> anyhow::Result<()> {
        match self.mode {
            Mode::Jump { ref jumps, .. } => match key_event {
                key!("esc") => {
                    self.mode = Mode::Normal;
                }
                key => {
                    let KeyCode::Char(c) = key.code else {return Ok(())};
                    let matching_jumps = jumps
                        .iter()
                        .filter(|jump| c == jump.character)
                        .collect_vec();
                    match matching_jumps.split_first() {
                        None => {}
                        Some((jump, [])) => {
                            self.update_selection_set(SelectionSet {
                                primary: Selection {
                                    copied_text: self.selection_set.primary.copied_text.clone(),
                                    ..jump.selection.clone()
                                },
                                secondary: vec![],
                                mode: self.selection_set.mode.clone(),
                            });
                            self.mode = Mode::Normal;
                        }
                        Some(_) => {
                            self.mode = Mode::Jump {
                                jumps: matching_jumps
                                    .into_iter()
                                    .zip(Self::jump_characters().into_iter().cycle())
                                    .map(|(jump, character)| Jump {
                                        character,
                                        ..jump.clone()
                                    })
                                    .collect_vec(),
                            }
                        }
                    }
                }
            },
            _ => unreachable!(),
        }
        Ok(())
    }

    /// Similar to Change in Vim, but does not copy the current selection
    fn change(&mut self) {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let copied_text: Rope = self.buffer.borrow().slice(&selection.range)?;
                    Ok(ActionGroup::new(vec![
                        Action::Edit(Edit {
                            range: selection.range,
                            new: Rope::new(),
                        }),
                        Action::Select(Selection {
                            range: (selection.range.start..selection.range.start).into(),
                            copied_text: selection.copied_text.clone(),
                            initial_range: None,
                            info: None,
                        }),
                    ]))
                })
                .into_iter()
                .flatten()
                .collect(),
        );

        self.apply_edit_transaction(edit_transaction);
        self.enter_insert_mode(CursorDirection::Start);
    }

    fn insert(&mut self, s: &str) -> Vec<Dispatch> {
        let edit_transaction =
            EditTransaction::from_action_groups(self.selection_set.map(|selection| {
                ActionGroup::new(vec![
                    Action::Edit(Edit {
                        range: {
                            let start = selection.to_char_index(&CursorDirection::End);
                            (start..start).into()
                        },
                        new: Rope::from_str(s),
                    }),
                    Action::Select(Selection {
                        range: (selection.range.start + s.len()..selection.range.start + s.len())
                            .into(),
                        copied_text: selection.copied_text.clone(),
                        initial_range: None,
                        info: None,
                    }),
                ])
            }));

        self.apply_edit_transaction(edit_transaction)
    }

    fn handle_insert_mode(&mut self, event: KeyEvent) -> anyhow::Result<Vec<Dispatch>> {
        match event.code {
            KeyCode::Esc => self.enter_normal_mode()?,
            KeyCode::Backspace => return Ok(self.backspace()),
            KeyCode::Enter => return Ok(self.insert("\n")),
            KeyCode::Char(c) => return Ok(self.insert(&c.to_string())),
            KeyCode::Tab => return Ok(self.insert("\t")),
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
        context: &mut Context,
        selection_mode: SelectionMode,
    ) -> anyhow::Result<Vec<Dispatch>> {
        context.mode = None;
        self.select_direction_mode(context, Direction::Current, selection_mode)
    }

    fn select_direction_mode(
        &mut self,
        context: &mut Context,
        direction: Direction,
        selection_mode: SelectionMode,
    ) -> anyhow::Result<Vec<Dispatch>> {
        if let Some(global_mode) = &context.mode {
            match global_mode {
                GlobalMode::QuickfixListItem => Ok(vec![Dispatch::GotoQuickfixListItem(direction)]),
            }
        } else {
            self.select(selection_mode, direction)?;

            let infos = self
                .selection_set
                .map(|selection| selection.info.clone())
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

    fn select_direction(
        &mut self,
        context: &mut Context,
        direction: Direction,
    ) -> anyhow::Result<Vec<Dispatch>> {
        self.select_direction_mode(context, direction, self.selection_set.mode.clone())
    }

    fn save_bookmarks(&mut self) {
        let selections = self
            .selection_set
            .map(|selection| selection.extended_range());
        self.buffer_mut().save_bookmarks(selections)
    }

    fn handle_normal_mode(
        &mut self,
        context: &mut Context,
        event: KeyEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        match event {
            key!(",") => self.select_backward(),
            key!("up") => return self.select_direction(context, Direction::Up),
            key!("down") => return self.select_direction(context, Direction::Down),
            key!("left") => return self.select_direction(context, Direction::Left),
            key!("shift+left") => return self.select_direction(context, Direction::LeftMost),
            key!("right") => return self.select_direction(context, Direction::Right),
            key!("shift+right") => return self.select_direction(context, Direction::RightMost),
            key!("esc") => {
                return Ok(vec![Dispatch::CloseAllExceptMainPanel]);
            }
            // Objects
            key!("a") => self.mode = Mode::AddCursor,
            key!("ctrl+b") => self.save_bookmarks(),
            key!("b") => return self.set_selection_mode(context, SelectionMode::Bookmark),

            key!("c") => return self.set_selection_mode(context, SelectionMode::Character),
            key!("d") => return self.select_direction(context, Direction::Down),

            key!("e") => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.diagnostic_keymap_legend_config(),
                )]
                .to_vec())
            }
            key!("f") => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.find_mode_keymap_legend_config()?,
                )]
                .to_vec())
            }
            key!("g") => {
                return Ok(vec![Dispatch::ShowKeymapLegend(
                    self.g_mode_keymap_legend_config(),
                )])
            }
            key!("h") => return self.set_selection_mode(context, SelectionMode::GitHunk),
            key!("h") => self.toggle_highlight_mode(),
            // H
            key!("i") => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.insert_mode_keymap_legend_config(),
                )]
                .to_vec())
            }
            key!("j") => self.jump()?,

            key!("k") => self.mode = Mode::Kill,
            // TODO: rebind
            key!("k") => self.select_kids()?,
            key!("l") => return self.set_selection_mode(context, SelectionMode::Line),
            key!("m") => {
                return self.set_selection_mode(
                    context,
                    SelectionMode::Match {
                        search: context.last_search().clone().unwrap_or(Search {
                            kind: SearchKind::Literal,
                            search: "".to_string(),
                        }),
                    },
                )
            }
            key!("n") => return self.select_direction(context, Direction::Right),
            key!("shift+N") => return self.select_direction(context, Direction::RightMost),

            // TODO: rebind
            key!("o") => return self.set_selection_mode(context, SelectionMode::LargestNode),
            // O
            key!("p") => {
                return self.select_direction(context, Direction::Left);
            }
            key!("shift+P") => return self.select_direction(context, Direction::LeftMost),
            key!("q") => {
                context.mode = Some(GlobalMode::QuickfixListItem);
            }
            // r for rotate? more general than swapping/exchange, which does not warp back to first
            // selection
            key!("r") => return Ok(self.raise()),
            key!("r") => return Ok(self.replace()),
            key!("s") => return self.set_selection_mode(context, SelectionMode::SyntaxTree),
            key!("t") => return self.set_selection_mode(context, SelectionMode::Token),

            key!("u") => return self.select_direction(context, Direction::Up),
            key!("v") => {
                return Ok(vec![Dispatch::ShowKeymapLegend(
                    self.view_mode_keymap_legend_config(),
                )]);
            }
            key!("w") => return self.set_selection_mode(context, SelectionMode::Word),
            key!("x") => return Ok(self.exchange(Direction::Right)),
            key!("shift+X") => return Ok(self.exchange(Direction::Left)),
            // y
            // z
            key!("backspace") => {
                self.change();
            }
            key!("enter") => return Ok(self.open_new_line()),
            key!("%") => self.change_cursor_direction(),
            key!("(") | key!(")") => return Ok(self.enclose(Enclosure::RoundBracket)),
            key!("[") | key!("]") => return Ok(self.enclose(Enclosure::SquareBracket)),
            key!('{') | key!('}') => return Ok(self.enclose(Enclosure::CurlyBracket)),
            key!('<') | key!('>') => return Ok(self.enclose(Enclosure::AngleBracket)),

            key!("alt+left") => return Ok(vec![Dispatch::GotoOpenedEditor(Direction::Left)]),
            key!("alt+right") => return Ok(vec![Dispatch::GotoOpenedEditor(Direction::Right)]),
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

    fn path(&self) -> Option<CanonicalizedPath> {
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

    pub fn enter_insert_mode(&mut self, direction: CursorDirection) {
        self.selection_set.apply_mut(|selection| {
            let char_index = match direction {
                CursorDirection::Start => selection.range.start,
                CursorDirection::End => selection.range.end,
            };
            selection.range = (char_index..char_index).into()
        });
        self.mode = Mode::Insert;
        self.cursor_direction = CursorDirection::Start;
    }

    pub fn enter_normal_mode(&mut self) -> anyhow::Result<()> {
        if self.mode == Mode::Insert {
            self.selection_set =
                self.selection_set
                    .apply(self.selection_set.mode.clone(), |selection| {
                        let range = {
                            if let Ok(position) =
                                self.buffer().char_to_position(selection.range.start)
                            {
                                let start =
                                    selection.range.start - if position.column > 0 { 1 } else { 0 };
                                (start..start).into()
                            } else {
                                selection.range
                            }
                        };
                        Ok(Selection {
                            range,
                            ..selection.clone()
                        })
                    })?;
        }
        self.mode = Mode::Normal;

        Ok(())
    }

    pub fn jumps(&self) -> Vec<&Jump> {
        match self.mode {
            Mode::Jump { ref jumps } => jumps.iter().collect(),
            _ => vec![],
        }
    }

    // TODO: handle mouse click
    pub fn set_cursor_position(&mut self, row: u16, column: u16) -> anyhow::Result<()> {
        let start = (self.buffer.borrow().line_to_char(row as usize)?) + column.into();
        self.update_selection_set(SelectionSet {
            mode: self.selection_set.mode.clone(),
            primary: Selection {
                range: (start..start).into(),
                copied_text: self.selection_set.primary.copied_text.clone(),
                initial_range: self.selection_set.primary.initial_range.clone(),
                info: self.selection_set.primary.info.clone(),
            },
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
        direction: &Direction,
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

            let text_at_next_selection: Rope = buffer.slice(&next_selection.range)?;

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
        direction: Direction,
    ) -> Vec<Dispatch> {
        let buffer = self.buffer.borrow().clone();
        let get_trial_edit_transaction =
            |current_selection: &Selection, next_selection: &Selection| -> anyhow::Result<_> {
                let current_selection_range = current_selection.extended_range();
                let text_at_current_selection = buffer.slice(&current_selection_range)?;

                Ok(EditTransaction::from_action_groups(vec![
                    ActionGroup::new(vec![Action::Edit(Edit {
                        range: current_selection_range,
                        new: buffer.slice(&next_selection.range)?,
                    })]),
                    ActionGroup::new(vec![Action::Edit(Edit {
                        range: next_selection.range,
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
                    })]),
                ]))
            };

        let get_actual_edit_transaction = |current_selection: &Selection,
                                           next_selection: &Selection|
         -> anyhow::Result<_> {
            let current_selection_range = current_selection.extended_range();
            let text_at_current_selection: Rope = buffer.slice(&current_selection_range)?;
            let text_at_next_selection: Rope = buffer.slice(&next_selection.range)?;

            Ok(EditTransaction::from_action_groups(vec![
                ActionGroup::new(vec![Action::Edit(Edit {
                    range: current_selection_range,
                    new: text_at_next_selection.clone(),
                })]),
                ActionGroup::new(vec![
                    Action::Edit(Edit {
                        range: next_selection.range,
                        // This time without whitespace padding
                        new: text_at_current_selection.clone(),
                    }),
                    Action::Select(Selection {
                        range: (next_selection.range.start
                            ..(next_selection.range.start + text_at_current_selection.len_chars()))
                            .into(),
                        copied_text: current_selection.copied_text.clone(),

                        // TODO: fix this, the initial_range should be updated as well
                        initial_range: current_selection.initial_range.clone(),
                        info: current_selection.info.clone(),
                    }),
                ]),
            ]))
        };

        let edit_transactions = self.selection_set.map(|selection| {
            self.get_valid_selection(
                selection,
                selection_mode,
                &direction,
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

    fn exchange(&mut self, direction: Direction) -> Vec<Dispatch> {
        let mode = self.selection_set.mode.clone();
        self.replace_faultlessly(&mode, direction)
    }

    fn add_selection(&mut self, direction: &Direction) -> anyhow::Result<()> {
        self.selection_set.add_selection(
            &self.buffer.borrow(),
            direction,
            &self.cursor_direction,
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
                    selection.range.clone(),
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

    fn select_word(&mut self, direction: Direction) -> anyhow::Result<()> {
        self.select(SelectionMode::Word, direction)
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

    fn backspace(&mut self) -> Vec<Dispatch> {
        let edit_transaction =
            EditTransaction::from_action_groups(self.selection_set.map(|selection| {
                let start = CharIndex(selection.range.start.0.saturating_sub(1));
                ActionGroup::new(vec![
                    Action::Edit(Edit {
                        range: (start..selection.range.start).into(),
                        new: Rope::from(""),
                    }),
                    Action::Select(Selection {
                        range: (start..start).into(),
                        ..selection.clone()
                    }),
                ])
            }));

        self.apply_edit_transaction(edit_transaction)
    }

    fn kill(&mut self, direction: Direction) -> Vec<Dispatch> {
        let buffer = self.buffer.borrow().clone();
        let mode = self.selection_set.mode.clone();

        let edit_transactions = self.selection_set.map(|selection| -> anyhow::Result<_> {
            let get_trial_edit_transaction =
                |current_selection: &Selection, other_selection: &Selection| -> anyhow::Result<_> {
                    let range = current_selection
                        .range
                        .start
                        .min(other_selection.range.start)
                        ..current_selection.range.end.max(other_selection.range.end);

                    // Add whitespace padding
                    let new: Rope = format!(" {} ", buffer.slice(&current_selection.range)?).into();

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
                        .range
                        .start
                        .min(other_selection.range.start)
                        ..current_selection.range.end.max(other_selection.range.end);
                    let new: Rope = buffer.slice(&other_selection.range)?;

                    let new_len_chars = new.len_chars();
                    Ok(EditTransaction::from_action_groups(vec![ActionGroup::new(
                        vec![
                            Action::Edit(Edit {
                                range: range.clone().into(),
                                new,
                            }),
                            Action::Select(Selection {
                                range: (range.start..(range.start + new_len_chars)).into(),
                                ..current_selection.clone()
                            }),
                        ],
                    )]))
                };
            self.get_valid_selection(
                selection,
                &mode,
                &direction,
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
                    Either::Left(selection) => {
                        EditTransaction::from_action_groups(vec![ActionGroup::new(vec![
                            Action::Edit(Edit {
                                range: selection.range,
                                new: Rope::from(""),
                            }),
                            Action::Select(Selection {
                                range: (selection.range.start..selection.range.start).into(),
                                ..selection.clone()
                            }),
                        ])])
                    }
                })
                .collect(),
        );
        self.apply_edit_transaction(edit_transaction)
    }

    /// Replace the parent node of the current node with the current node
    fn raise(&mut self) -> Vec<Dispatch> {
        let buffer = self.buffer.borrow().clone();
        let edit_transactions = self.selection_set.map(|selection| {
            let get_trial_edit_transaction =
                |current_selection: &Selection, other_selection: &Selection| -> anyhow::Result<_> {
                    let range = current_selection
                        .range
                        .start
                        .min(other_selection.range.start)
                        ..current_selection.range.end.max(other_selection.range.end);

                    // Add whitespace padding
                    let new: Rope = format!(" {} ", buffer.slice(&current_selection.range)?).into();

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
                        .range
                        .start
                        .min(other_selection.range.start)
                        ..current_selection.range.end.max(other_selection.range.end);
                    let new: Rope = buffer.slice(&current_selection.range)?;

                    let new_len_chars = new.len_chars();
                    Ok(EditTransaction::from_action_groups(vec![ActionGroup::new(
                        vec![
                            Action::Edit(Edit {
                                range: range.clone().into(),
                                new,
                            }),
                            Action::Select(Selection {
                                range: (range.start..(range.start + new_len_chars)).into(),
                                ..current_selection.clone()
                            }),
                        ],
                    )]))
                };
            self.get_valid_selection(
                selection,
                &SelectionMode::SyntaxTree,
                &Direction::Up,
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
    fn select_view(&mut self, direction: Direction) -> anyhow::Result<()> {
        let scroll_height = self.dimension().height / 2;
        self.update_selection_set(self.selection_set.apply(
            self.selection_set.mode.clone(),
            |selection| {
                let position = selection.range.start.to_position(self.buffer().rope());
                let position = Position {
                    line: if direction == Direction::Right {
                        position.line.saturating_add(scroll_height as usize)
                    } else {
                        position.line.saturating_sub(scroll_height as usize)
                    },
                    ..position
                };
                let start = position.to_char_index(&self.buffer())?;
                Ok(Selection {
                    range: (start..start).into(),
                    ..selection.clone()
                })
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

    pub fn replace_previous_word(&mut self, completion: &str) -> anyhow::Result<Vec<Dispatch>> {
        let selection = self.get_selection_set(&SelectionMode::Word, Direction::Left)?;
        self.update_selection_set(selection);
        self.replace_current_selection_with(|_| Some(Rope::from_str(completion)));
        Ok(self.get_document_did_change_dispatch())
    }

    fn open_new_line(&mut self) -> Vec<Dispatch> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| {
                    let buffer = self.buffer.borrow();
                    let cursor_index = selection.to_char_index(&self.cursor_direction);
                    let line_index = buffer.char_to_line(cursor_index).ok()?;
                    let line_start = buffer.line_to_char(line_index).ok()?;
                    let current_line = self.buffer.borrow().get_line(cursor_index).ok()?;
                    let leading_whitespaces = current_line
                        .chars()
                        .take_while(|c| c.is_whitespace())
                        .join("");
                    Some(ActionGroup::new(vec![
                        Action::Edit(Edit {
                            range: {
                                let start = line_start + current_line.len_chars();
                                (start..start).into()
                            },
                            new: format!("{}\n", leading_whitespaces).into(),
                        }),
                        Action::Select(Selection {
                            range: {
                                let start = line_start
                                    + current_line.len_chars()
                                    + leading_whitespaces.len();
                                (start..start).into()
                            },
                            ..selection.clone()
                        }),
                    ]))
                })
                .into_iter()
                .flatten()
                .collect_vec(),
        );

        let dispatches = self.apply_edit_transaction(edit_transaction);
        self.enter_insert_mode(CursorDirection::End);
        dispatches
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    pub fn set_diagnostics(&mut self, diagnostics: Vec<Diagnostic>) {
        self.buffer.borrow_mut().set_diagnostics(diagnostics)
    }

    pub fn apply_positional_edits(&mut self, edits: Vec<PositionalEdit>) -> Vec<Dispatch> {
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

                    let action_select = Action::Select(Selection {
                        range: {
                            let end = range.start + next_text_len;
                            (end..end).into()
                        },
                        ..Default::default()
                    });

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

    pub fn apply_positional_edit(&mut self, edit: PositionalEdit) -> Vec<Dispatch> {
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

    fn enclose(&mut self, enclosure: Enclosure) -> Vec<Dispatch> {
        let edit_transaction = EditTransaction::from_action_groups(
            self.selection_set
                .map(|selection| -> anyhow::Result<_> {
                    let old = self.buffer().slice(&selection.extended_range())?;
                    Ok(ActionGroup::new(vec![
                        Action::Edit(Edit {
                            range: selection.range,
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
                        Action::Select(Selection {
                            range: (selection.range.start..selection.range.end + 2).into(),
                            ..selection.clone()
                        }),
                    ]))
                })
                .into_iter()
                .flatten()
                .collect_vec(),
        );

        self.apply_edit_transaction(edit_transaction)
    }

    fn match_current_selection(&mut self, kind: SearchKind) -> anyhow::Result<Vec<Dispatch>> {
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
            SelectionMode::Match {
                search: search.clone(),
            },
            Direction::Current,
        )?;

        Ok(vec![Dispatch::SetSearch(search)])
    }

    fn transform_selection(&mut self, case: Case) -> Vec<Dispatch> {
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
                    Ok(ActionGroup::new(
                        [
                            Action::Edit(Edit {
                                range: selection.range,
                                new,
                            }),
                            Action::Select(Selection {
                                range: (selection.range.start
                                    ..selection.range.start + new_char_count)
                                    .into(),
                                ..selection.clone()
                            }),
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
        match &self.mode {
            Mode::Normal => {
                format!("NORMAL:{}", self.selection_set.mode.display())
            }
            Mode::Insert => "INSERT".to_string(),
            Mode::Jump { .. } => "JUMP".to_string(),
            Mode::Kill => "KILL".to_string(),
            Mode::AddCursor => "ADD CURSOR".to_string(),
            Mode::FindOneChar => "FIND ONE CHAR".to_string(),
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
        start as usize..(start as usize + self.rectangle.height as usize)
    }

    fn handle_kill_mode(&mut self, key_event: KeyEvent) -> Result<Vec<Dispatch>, anyhow::Error> {
        match key_event {
            key!("esc") => {
                self.enter_normal_mode()?;
                Ok(Vec::new())
            }
            key!("k") => {
                let dispatches = self.kill(Direction::Current);
                self.enter_normal_mode()?;
                Ok(dispatches)
            }
            key!("n") => Ok(self.kill(Direction::Right)),
            key!("p") => Ok(self.kill(Direction::Left)),
            _ => Ok(Vec::new()),
        }
    }

    fn handle_add_cursor_mode(&mut self, key_event: KeyEvent) -> Result<(), anyhow::Error> {
        match key_event {
            key!("esc") => self.enter_normal_mode(),
            key!("a") => self.add_cursor_to_all_selections(),
            // todo: delete primary cursor does not work as expected, we need another editr cursor mode
            key!("d") => self.delete_primary_cursor(),
            key!("n") => self.add_selection(&Direction::Right),
            key!("o") => self.only_current_cursor(),
            key!("p") => self.add_selection(&Direction::Left),
            _ => Ok(()),
        }
    }

    fn delete_primary_cursor(&mut self) -> Result<(), anyhow::Error> {
        self.selection_set.delete_primary_cursor();
        Ok(())
    }

    fn add_cursor_to_all_selections(&mut self) -> Result<(), anyhow::Error> {
        self.selection_set.add_all(&self.buffer.borrow())?;
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
        context: &mut Context,
        key_event: KeyEvent,
    ) -> Result<Vec<Dispatch>, anyhow::Error> {
        match key_event.code {
            KeyCode::Char(c) => {
                self.enter_normal_mode()?;
                self.set_selection_mode(
                    context,
                    SelectionMode::Match {
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
}

enum Enclosure {
    RoundBracket,
    SquareBracket,
    CurlyBracket,
    AngleBracket,
}

pub fn node_to_selection(
    node: Node,
    buffer: &Buffer,
    current_selection: &Selection,
) -> anyhow::Result<Selection> {
    Ok(Selection {
        range: (buffer.byte_to_char(node.start_byte())?..buffer.byte_to_char(node.end_byte())?)
            .into(),
        ..current_selection.clone()
    })
}

pub enum HandleEventResult {
    Handled(Vec<Dispatch>),
    Ignored(KeyEvent),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DispatchEditor {
    ScrollUp,
    ScrollDown,
    AlignViewTop,
    AlignViewCenter,
    AlignViewBottom,
    Transform(convert_case::Case),
    SetSelectionMode(SelectionMode),
    EnterInsertMode(CursorDirection),
    FindOneChar,
}

#[cfg(test)]
mod test_editor {

    use crate::{
        components::{
            component::Component,
            editor::{CursorDirection, Mode},
        },
        context::{Context, Search, SearchKind},
        lsp::diagnostic::Diagnostic,
        position::Position,
        screen::Dispatch,
        selection::SelectionMode,
    };

    use super::{Direction, Editor};
    use my_proc_macros::keys;
    use pretty_assertions::assert_eq;
    use tree_sitter_rust::language;

    #[test]
    fn select_character() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        let mut context = Context::default();
        editor.set_selection_mode(&mut context, SelectionMode::Character)?;
        assert_eq!(editor.get_selected_texts(), vec!["f"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["n"]);

        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["f"]);
        Ok(())
    }

    #[test]
    fn select_line() -> anyhow::Result<()> {
        // Multiline source code
        let mut editor = Editor::from_text(language(), "\nfn main() {\n\n\nlet x = 1;\n}\n");
        let mut context = Context::default();
        editor.set_selection_mode(&mut context, SelectionMode::Line)?;
        assert_eq!(editor.get_selected_texts(), vec![""]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["fn main() {"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec![""]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec![""]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["let x = 1;"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["}"]);

        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["let x = 1;"]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec![""]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec![""]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["fn main() {"]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec![""]);
        Ok(())
    }

    #[test]
    fn select_word() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "camelCase, snake_case, ALLCAPS: 123");
        let mut context = Context::default();
        editor.set_selection_mode(&mut context, SelectionMode::Word)?;
        assert_eq!(editor.get_selected_texts(), vec!["camel"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["Case"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["snake"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["case"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["ALLCAPS"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["123"]);

        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["ALLCAPS"]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["case"]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["snake"]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["Case"]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["camel"]);
        Ok(())
    }

    #[test]
    fn select_match_ast_grep() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = f(y); f(x); f( z ) }");
        let mut context = Context::default();
        let search = Search {
            search: "f($EXPR)".to_string(),
            kind: SearchKind::AstGrep,
        };

        editor.set_selection_mode(&mut context, SelectionMode::Match { search })?;
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["f(y)"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["f(x)"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["f( z )"]);

        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["f(x)"]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["f(y)"]);
        Ok(())
    }

    #[test]
    fn select_match_string() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = f(y); f(x); f( z ) }");
        let mut context = Context::default();
        let search = Search {
            search: "f(".to_string(),
            kind: SearchKind::Literal,
        };

        editor.set_selection_mode(&mut context, SelectionMode::Match { search })?;
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["f("]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["f("]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["f("]);

        editor.insert("hello");
        assert_eq!(
            editor.text(),
            "fn main() { let x = f(y); f(x); fhello( z ) }"
        );
        Ok(())
    }

    #[test]
    fn select_match_regex() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = f(y); f(x); f( z ) }");
        let mut context = Context::default();
        let search = Search {
            search: r"f\([a-z]\)".to_string(),
            kind: SearchKind::Regex,
        };

        editor.set_selection_mode(&mut context, SelectionMode::Match { search })?;
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["f(y)"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["f(x)"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["f(x)"]);

        Ok(())
    }

    #[test]
    fn select_token() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        let mut context = Context::default();
        editor.set_selection_mode(&mut context, SelectionMode::Token)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["main"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["("]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec![")"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["{"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["let"]);
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["x"]);

        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["let"]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["{"]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec![")"]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["("]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["main"]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["fn"]);
        editor.select_direction(&mut context, Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["fn"]);
        Ok(())
    }

    #[test]
    fn select_parent() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        let mut context = Context::default();
        // Move token to 1
        let search = Search {
            search: "1".to_string(),
            kind: SearchKind::Literal,
        };
        editor.set_selection_mode(&mut context, SelectionMode::Match { search })?;
        editor.select_direction(&mut context, Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["1"]);

        editor.set_selection_mode(&mut context, SelectionMode::SyntaxTree)?;
        editor.select_direction(&mut context, Direction::Up)?;
        assert_eq!(editor.get_selected_texts(), vec!["let x = 1;"]);
        editor.select_direction(&mut context, Direction::Up)?;
        assert_eq!(editor.get_selected_texts(), vec!["{ let x = 1; }"]);
        editor.select_direction(&mut context, Direction::Up)?;
        assert_eq!(
            editor.get_selected_texts(),
            vec!["fn main() { let x = 1; }"]
        );

        editor.select_direction(&mut context, Direction::Down)?;
        assert_eq!(editor.get_selected_texts(), vec!["main"]);
        Ok(())
    }

    #[test]
    fn select_syntax_tree() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize, y: Vec<A>) {}");
        let mut context = Context::default();
        let search = Search {
            search: "x: usize".to_string(),
            kind: SearchKind::Literal,
        };
        // Move token to "x: usize"
        editor.set_selection_mode(&mut context, SelectionMode::Match { search })?;
        editor.select_direction(&mut context, Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["x: usize"]);

        editor.select_syntax_tree(Direction::Right)?;
        editor.select_syntax_tree(Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["y: Vec<A>"]);
        editor.select_syntax_tree(Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["y: Vec<A>"]);

        editor.select_syntax_tree(Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["x: usize"]);
        editor.select_syntax_tree(Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["x: usize"]);
        Ok(())
    }

    #[test]
    /// Should select the most ancestral node if the node's child and its parents has the same range.
    fn select_syntax_tree_2() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = X {a,b,c:d} }");

        // Select `a`
        for _ in 0..11 {
            editor.select_token(Direction::Right)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["a"]);

        editor.select_syntax_tree(Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["b"]);

        editor.select_syntax_tree(Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["c:d"]);

        editor.select_syntax_tree(Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["b"]);

        editor.select_syntax_tree(Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["a"]);
        Ok(())
    }

    #[test]
    fn select_kids() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize, y: Vec<A>) {}");
        // Move token to "x"
        for _ in 0..4 {
            editor.select_token(Direction::Right)?;
        }
        assert_eq!(editor.get_selected_texts(), vec!["x"]);

        editor.select_kids()?;
        assert_eq!(editor.get_selected_texts(), vec!["x: usize, y: Vec<A>"]);
        Ok(())
    }

    #[test]
    fn select_named_node() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize) { let x = 1; }");

        editor.select_named_node(Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["main"]);
        editor.select_named_node(Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["(x: usize)"]);
        editor.select_named_node(Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["x: usize"]);
        editor.select_named_node(Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["usize"]);
        editor.select_named_node(Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["{ let x = 1; }"]);
        editor.select_named_node(Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["let x = 1;"]);

        editor.select_named_node(Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["{ let x = 1; }"]);
        editor.select_named_node(Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["usize"]);
        editor.select_named_node(Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["x: usize"]);
        editor.select_named_node(Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["(x: usize)"]);
        editor.select_named_node(Direction::Left)?;
        assert_eq!(editor.get_selected_texts(), vec!["main"]);
        Ok(())
    }

    #[test]
    fn select_diagnostic() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize) {\n  let x = 1; }");

        // We should have diagnostics with the following combinations:
        // 1. No intersection
        // 2. Intersected
        // 3. Subset

        // 'spongebob' and 'patrick' are not intersected
        // 'spongebob' and 'squidward' are intersected
        editor.set_diagnostics(vec![
            Diagnostic::new(
                Position { line: 0, column: 0 }..Position { line: 0, column: 1 },
                "spongebob".to_string(),
            ),
            Diagnostic::new(
                Position { line: 0, column: 0 }..Position { line: 0, column: 2 },
                "sandy".to_string(),
            ),
            Diagnostic::new(
                Position { line: 0, column: 1 }..Position { line: 0, column: 3 },
                "patrick".to_string(),
            ),
            Diagnostic::new(
                Position { line: 0, column: 2 }..Position { line: 0, column: 4 },
                "squidward".to_string(),
            ),
        ]);

        fn assert_dispatch_contains(dispatches: Vec<Dispatch>, info: &str) {
            let content = dispatches
                .into_iter()
                .find_map(|dispatch| match dispatch {
                    Dispatch::ShowInfo { content, .. } => Some(content),
                    _ => None,
                })
                .unwrap();
            assert!(content.join("\n").contains(info))
        }

        let dispatches = editor.select_diagnostic(Direction::Right)?;
        assert_dispatch_contains(
            dispatches,
            "[UNKNOWN]\nspongebob\n\n[RELATED INFORMATION]\n",
        );
        assert_eq!(editor.get_selected_texts(), vec!["f"]);

        let dispatches = editor.select_diagnostic(Direction::Right)?;
        assert_dispatch_contains(dispatches, "sandy");
        assert_eq!(editor.get_selected_texts(), vec!["fn"]);

        let dispatches = editor.select_diagnostic(Direction::Right)?;
        assert_dispatch_contains(dispatches, "patrick");
        assert_eq!(editor.get_selected_texts(), vec!["n "]);

        let dispatches = editor.select_diagnostic(Direction::Right)?;
        assert_dispatch_contains(dispatches, "squidward");
        assert_eq!(editor.get_selected_texts(), vec![" m"]);

        let dispatches = editor.select_diagnostic(Direction::Right)?;
        assert_dispatch_contains(dispatches, "squidward");
        assert_eq!(editor.get_selected_texts(), vec![" m"]);

        let dispatches = editor.select_diagnostic(Direction::Left)?;
        assert_dispatch_contains(dispatches, "patrick");

        let dispatches = editor.select_diagnostic(Direction::Left)?;
        assert_dispatch_contains(dispatches, "sandy");

        let dispatches = editor.select_diagnostic(Direction::Left)?;
        assert_dispatch_contains(dispatches, "spongebob");
        Ok(())
    }

    #[test]
    fn select_named_node_from_line_mode() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize) { \n let x = 1; }");
        // Select the second line
        editor.select_line(Direction::Right)?;
        editor.select_line(Direction::Right)?;

        // Select next name node
        editor.select_named_node(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["let x = 1;"]);
        Ok(())
    }

    #[test]
    fn copy_replace() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        editor.select_token(Direction::Right)?;
        let mut context = Context::default();
        editor.copy(&mut context);
        editor.select_token(Direction::Right)?;
        editor.replace();
        assert_eq!(editor.text(), "fn fn() { let x = 1; }");
        assert_eq!(editor.get_selected_texts(), vec!["fn"]);
        editor.replace();
        assert_eq!(editor.text(), "fn main() { let x = 1; }");
        assert_eq!(editor.get_selected_texts(), vec!["main"]);
        Ok(())
    }

    #[test]
    fn copy_paste() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        editor.select_token(Direction::Right)?;
        let mut context = Context::default();
        editor.copy(&mut context);
        editor.select_token(Direction::Right)?;
        editor.paste(&mut context);
        assert_eq!(editor.text(), "fn fn() { let x = 1; }");
        assert_eq!(editor.get_selected_texts(), vec![""]);
        Ok(())
    }

    #[test]
    fn cut_paste() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        let mut context = Context::default();
        editor.select_token(Direction::Right)?;
        editor.cut(&mut context);
        assert_eq!(editor.text(), " main() { let x = 1; }");
        assert_eq!(editor.get_selected_texts(), vec![""]);

        editor.select_token(Direction::Right)?;
        editor.paste(&mut context);

        assert_eq!(editor.text(), " fn() { let x = 1; }");
        assert_eq!(editor.get_selected_texts(), vec![""]);
        Ok(())
    }

    #[test]
    fn exchange_sibling() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize, y: Vec<A>) {}");
        // Move token to "x: usize"
        for _ in 0..3 {
            editor.select_named_node(Direction::Right)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["x: usize"]);

        editor.exchange(Direction::Right);
        assert_eq!(editor.text(), "fn main(y: Vec<A>, x: usize) {}");

        editor.exchange(Direction::Left);
        assert_eq!(editor.text(), "fn main(x: usize, y: Vec<A>) {}");
        Ok(())
    }

    #[test]
    fn exchange_sibling_2() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "use a;\nuse b;\nuse c;");

        // Select first statement
        editor.select_character(Direction::Right)?;
        editor.select_character(Direction::Right)?;

        editor.select_syntax_tree(Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["use a;"]);

        editor.exchange(Direction::Right);
        assert_eq!(editor.text(), "use b;\nuse a;\nuse c;");
        editor.exchange(Direction::Right);
        assert_eq!(editor.text(), "use b;\nuse c;\nuse a;");
        Ok(())
    }

    #[test]
    fn upend() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = a.b(c()); }");
        // Move selection to "c()"
        for _ in 0..9 {
            editor.select_named_node(Direction::Right)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["c()"]);

        editor.raise();
        assert_eq!(editor.text(), "fn main() { let x = c(); }");

        editor.raise();
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

        editor.select_line(Direction::Right)?;
        editor.select_line(Direction::Right)?;

        editor.exchange(Direction::Right);
        assert_eq!(
            editor.text(),
            "
    let x = 1;
fn main() {
    let y = 2;
}"
        );

        editor.exchange(Direction::Left);
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
        editor.select_character(Direction::Right)?;

        editor.exchange(Direction::Right);
        assert_eq!(editor.text(), "nf main() { let x = 1; }");
        editor.exchange(Direction::Right);
        assert_eq!(editor.text(), "n fmain() { let x = 1; }");

        editor.exchange(Direction::Left);
        assert_eq!(editor.text(), "nf main() { let x = 1; }");
        editor.exchange(Direction::Left);

        assert_eq!(editor.text(), "fn main() { let x = 1; }");
        Ok(())
    }

    #[test]
    fn multi_insert() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "struct A(usize, char)");
        // Select 'usize'
        for _ in 0..3 {
            editor.select_named_node(Direction::Right)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["usize"]);

        editor.add_selection()?;
        assert_eq!(editor.get_selected_texts(), vec!["usize", "char"]);
        editor.enter_insert_mode(CursorDirection::Start);
        editor.insert("pub ");

        assert_eq!(editor.text(), "struct A(pub usize, pub char)");

        editor.backspace();

        assert_eq!(editor.text(), "struct A(pubusize, pubchar)");
        assert_eq!(editor.get_selected_texts(), vec!["", ""]);
        Ok(())
    }

    #[test]
    fn multi_upend() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        // Select 'let x = S(a)'
        for _ in 0..4 {
            editor.select_named_node(Direction::Right)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["let x = S(a);"]);

        editor.add_selection()?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["let x = S(a);", "let y = S(b);"]
        );

        for _ in 0..4 {
            editor.select_named_node(Direction::Right)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["a", "b"]);

        editor.raise();

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
        // Select 'fn f(x:a,y:b){}'
        editor.select_token(Direction::Right)?;
        editor.select_syntax_tree(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f(x:a,y:b){}"]);

        editor.add_selection()?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["fn f(x:a,y:b){}", "fn g(x:a,y:b){}"]
        );

        editor.select_named_node(Direction::Right)?;
        editor.select_named_node(Direction::Right)?;
        editor.select_named_node(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["x:a", "x:a"]);

        editor.exchange(Direction::Right);
        assert_eq!(editor.text(), "fn f(y:b,x:a){} fn g(y:b,x:a){}");
        assert_eq!(editor.get_selected_texts(), vec!["x:a", "x:a"]);

        editor.exchange(Direction::Left);
        assert_eq!(editor.text(), "fn f(x:a,y:b){} fn g(x:a,y:b){}");
        Ok(())
    }

    #[test]
    fn multi_paste() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(
            language(),
            "fn f(){ let x = S(spongebob_squarepants); let y = S(b); }",
        );

        // Select 'let x = S(a)'
        for _ in 0..4 {
            editor.select_named_node(Direction::Right)?;
        }

        assert_eq!(
            editor.get_selected_texts(),
            vec!["let x = S(spongebob_squarepants);"]
        );

        editor.add_selection()?;
        editor.select_named_node(Direction::Right)?;
        editor.select_named_node(Direction::Right)?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["S(spongebob_squarepants)", "S(b)"]
        );

        let mut context = Context::default();
        editor.cut(&mut context);
        editor.enter_insert_mode(CursorDirection::Start);

        editor.insert("Some(");
        editor.paste(&mut context);
        editor.insert(")");

        assert_eq!(
            editor.text(),
            "fn f(){ let x = Some(S(spongebob_squarepants)); let y = Some(S(b)); }"
        );
        Ok(())
    }

    #[test]
    fn toggle_highlight_mode() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");

        editor.select_token(Direction::Right)?;
        editor.toggle_highlight_mode();
        editor.select_token(Direction::Right)?;
        editor.select_token(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f("]);

        // Toggle the second time should inverse the initial_range
        editor.toggle_highlight_mode();

        editor.select_token(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["f("]);

        editor.reset();

        assert_eq!(editor.get_selected_texts(), vec![""]);

        editor.select_token(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["f"]);

        // After reset, expect highlight mode is turned off
        editor.select_token(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["("]);
        Ok(())
    }

    #[test]
    fn highlight_mode_cut() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        editor.select_token(Direction::Right)?;
        editor.toggle_highlight_mode();
        editor.select_token(Direction::Right)?;
        editor.select_token(Direction::Right)?;
        editor.select_token(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f()"]);

        let mut context = Context::default();
        editor.cut(&mut context);

        assert_eq!(editor.text(), "{ let x = S(a); let y = S(b); }");

        editor.paste(&mut context);

        assert_eq!(editor.text(), "fn f(){ let x = S(a); let y = S(b); }");
        Ok(())
    }

    #[test]
    fn highlight_mode_copy() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        editor.select_token(Direction::Right)?;
        editor.toggle_highlight_mode();
        editor.select_token(Direction::Right)?;
        editor.select_token(Direction::Right)?;
        editor.select_token(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f()"]);

        let mut context = Context::default();
        editor.copy(&mut context);

        editor.select_token(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["{"]);

        editor.paste(&mut context);

        assert_eq!(editor.text(), "fn f()fn f() let x = S(a); let y = S(b); }");
        Ok(())
    }

    #[test]
    fn highlight_mode_replace() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        editor.select_token(Direction::Right)?;
        editor.toggle_highlight_mode();
        editor.select_token(Direction::Right)?;
        editor.select_token(Direction::Right)?;
        editor.select_token(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f()"]);

        let mut context = Context::default();
        editor.copy(&mut context);

        editor.select_named_node(Direction::Right)?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["{ let x = S(a); let y = S(b); }"]
        );

        editor.replace();

        assert_eq!(editor.text(), "fn f()fn f()");
        Ok(())
    }

    #[test]
    fn highlight_mode_paste() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        editor.select_token(Direction::Right)?;

        let mut context = Context::default();
        editor.copy(&mut context);

        assert_eq!(editor.get_selected_texts(), vec!["fn"]);

        editor.toggle_highlight_mode();
        editor.select_token(Direction::Right)?;
        editor.select_token(Direction::Right)?;
        editor.select_token(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f()"]);

        editor.paste(&mut context);

        assert_eq!(editor.text(), "fn{ let x = S(a); let y = S(b); }");
        Ok(())
    }

    #[test]
    #[ignore]
    fn highlight_mode_exchange_word() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        editor.select_word(Direction::Right)?;
        editor.toggle_highlight_mode();
        editor.select_word(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f"]);

        editor.exchange(Direction::Right);

        assert_eq!(editor.text(), "let(){ fn f x = S(a); let y = S(b); }");
        assert_eq!(editor.get_selected_texts(), vec!["fn f"]);

        editor.exchange(Direction::Right);

        assert_eq!(editor.text(), "let(){ x fn f = S(a); let y = S(b); }");
        Ok(())
    }

    #[test]
    #[ignore]
    fn highlight_mode_exchange_sibling() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){} fn g(){} fn h(){} fn i(){}");

        // select `fn f(){}`
        editor.select_token(Direction::Right)?;
        editor.select_syntax_tree(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f(){}"]);

        editor.toggle_highlight_mode();
        editor.select_syntax_tree(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f(){} fn g(){}"]);

        editor.exchange(Direction::Right);

        assert_eq!(editor.text(), "fn h(){} fn f(){} fn g(){} fn i(){}");

        editor.select_syntax_tree(Direction::Right)?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["fn f(){} fn g(){} fn i(){}"]
        );

        editor.exchange(Direction::Right);

        assert_eq!(editor.text(), "fn h(){} fn i(){} fn f(){} fn g(){}");
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

        editor.open_new_line();

        assert_eq!(editor.mode, Mode::Insert);

        editor.insert("let y = S(b);");

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
    fn delete_character() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        let mut context = Context::default();

        editor.select_character(Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["f"]);

        editor.kill(Direction::Right);
        assert_eq!(editor.text(), "n f(){ let x = S(a); let y = S(b); }");

        editor.kill(Direction::Right);
        assert_eq!(editor.text(), " f(){ let x = S(a); let y = S(b); }");

        editor.set_selection_mode(
            &mut context,
            SelectionMode::Match {
                search: Search {
                    search: "x".to_string(),
                    kind: SearchKind::Literal,
                },
            },
        )?;
        editor.select_direction(&mut context, Direction::Right)?;
        editor.set_selection_mode(&mut context, SelectionMode::Character)?;
        assert_eq!(editor.get_selected_texts(), vec!["x"]);

        editor.kill(Direction::Left);
        assert_eq!(editor.text(), " f(){ let  = S(a); let y = S(b); }");
        Ok(())
    }

    #[test]
    fn delete_line() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(
            language(),
            "
fn f() {
let x = S(a);

let y = S(b);
}"
            .trim(),
        );

        editor.select_line(Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["fn f() {\n"]);

        editor.kill(Direction::Right);
        assert_eq!(
            editor.text(),
            "
let x = S(a);

let y = S(b);
}"
            .trim()
        );

        editor.kill(Direction::Right);
        assert_eq!(
            editor.text(),
            "
let y = S(b);
}"
        );
        assert_eq!(editor.get_selected_texts(), vec!["\n"]);

        editor.select_line(Direction::Right)?;
        assert_eq!(editor.get_selected_texts(), vec!["let y = S(b);\n"]);
        editor.kill(Direction::Left);
        assert_eq!(
            editor.text(),
            "
}"
        );

        editor.kill(Direction::Right);
        assert_eq!(editor.text(), "}");

        editor.kill(Direction::Right);
        assert_eq!(editor.text(), "");
        Ok(())
    }

    #[test]
    fn delete_sibling() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(x: a, y: b, z: c){}");
        // Select 'x: a'
        editor.select_named_node(Direction::Right)?;
        editor.select_named_node(Direction::Right)?;
        editor.select_named_node(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["x: a"]);

        editor.select_syntax_tree(Direction::Current)?;
        editor.kill(Direction::Right);

        assert_eq!(editor.text(), "fn f(y: b, z: c){}");

        editor.select_syntax_tree(Direction::Right)?;
        editor.kill(Direction::Left);

        assert_eq!(editor.text(), "fn f(y: b){}");
        Ok(())
    }

    #[test]
    fn delete_token() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(x: a, y: b, z: c){}");
        // Select 'fn'
        editor.select_token(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn"]);

        editor.kill(Direction::Right);

        assert_eq!(editor.text(), "f(x: a, y: b, z: c){}");

        editor.kill(Direction::Right);

        assert_eq!(editor.text(), "(x: a, y: b, z: c){}");

        editor.select_token(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["x"]);

        editor.kill(Direction::Left);

        assert_eq!(editor.text(), "(: a, y: b, z: c){}");
        Ok(())
    }

    #[test]
    fn paste_from_clipboard() {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        let mut context = Context::default();

        context.set_clipboard_content("let z = S(c);".to_string());

        editor.reset();

        editor.paste(&mut context);

        assert_eq!(
            editor.text(),
            "let z = S(c);fn f(){ let x = S(a); let y = S(b); }"
        );
    }

    #[test]
    fn enter_newline() {
        let mut editor = Editor::from_text(language(), "");

        // Enter insert mode
        editor.handle_events(keys!("i")).unwrap();

        // Type in 'hello'
        editor.handle_events(keys!("h e l l o")).unwrap();

        // Type in enter
        editor.handle_events(keys!("enter")).unwrap();

        // Type in 'world'
        editor.handle_events(keys!("w o r l d")).unwrap();

        // Expect the text to be 'hello\nworld'
        assert_eq!(editor.text(), "hello\nworld");

        // Move cursor left
        editor.handle_events(keys!("left")).unwrap();

        // Type in enter
        editor.handle_events(keys!("enter")).unwrap();

        // Expect the text to be 'hello\nworl\nd'
        assert_eq!(editor.text(), "hello\nworl\nd");
    }

    #[test]
    fn set_selection() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() {}");

        // Select a range which highlights a node
        editor.set_selection(Position::new(0, 0)..Position::new(0, 2))?;

        assert_eq!(editor.selection_set.mode, SelectionMode::LargestNode);

        // Select a range which does not highlights a node
        editor.set_selection(Position::new(0, 0)..Position::new(0, 1))?;

        assert_eq!(editor.selection_set.mode, SelectionMode::Custom);

        Ok(())
    }

    #[test]
    fn insert_mode_start() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() {}");

        // Select the first word
        editor.select_word(Direction::Current)?;

        // Enter insert mode
        editor.enter_insert_mode(CursorDirection::Start);

        // Type something
        editor.insert("hello");

        // Expect the text to be 'hellofn main() {}'
        assert_eq!(editor.text(), "hellofn main() {}");
        Ok(())
    }

    #[test]
    fn insert_mode_end() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() {}");

        // Select the first word
        editor.select_word(Direction::Current)?;

        // Enter insert mode
        editor.enter_insert_mode(CursorDirection::End);

        // Type something
        editor.insert("hello");

        // Expect the text to be 'fnhello main() {}'
        assert_eq!(editor.text(), "fnhello main() {}");
        Ok(())
    }

    #[test]
    fn enclose_left_bracket() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { x.y() }");

        // Select 'x.y()'
        editor.select_named_node(Direction::Right)?;
        editor.select_named_node(Direction::Right)?;
        editor.select_named_node(Direction::Right)?;
        editor.select_named_node(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["x.y()"]);

        editor.handle_events(keys!("( { [ <")).unwrap();

        assert_eq!(editor.text(), "fn main() { <[{(x.y())}]> }");
        assert_eq!(editor.get_selected_texts(), vec!["<[{(x.y())}]>"]);
        Ok(())
    }

    #[test]
    fn enclose_right_bracket() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { x.y() }");

        // Select 'x.y()'
        editor.select_named_node(Direction::Right)?;
        editor.select_named_node(Direction::Right)?;
        editor.select_named_node(Direction::Right)?;
        editor.select_named_node(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["x.y()"]);

        editor.handle_events(keys!(") } ] >")).unwrap();

        assert_eq!(editor.text(), "fn main() { <[{(x.y())}]> }");
        assert_eq!(editor.get_selected_texts(), vec!["<[{(x.y())}]>"]);
        Ok(())
    }

    #[test]
    fn select_final() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn\nmain()\n{ x.y() }");

        editor.select_line(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn\n"]);

        editor.select_final(Direction::Right)?;

        assert_eq!(editor.get_selected_texts(), vec!["{ x.y() }"]);

        editor.select_final(Direction::Left)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn\n"]);
        Ok(())
    }

    #[test]
    fn match_current_selection() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn\nmain()\n{ x.y(); x.y(); x.y(); }");

        // Select x.y()

        for _ in 0..4 {
            editor.select_named_node(Direction::Right)?;
        }
        editor.select_syntax_tree(Direction::Left)?;

        assert_eq!(editor.get_selected_texts(), vec!["x.y()"]);

        let dispatches = editor.match_current_selection(SearchKind::Literal)?;

        let search = Search {
            search: "x.y()".to_string(),
            kind: SearchKind::Literal,
        };
        assert_eq!(dispatches, vec![Dispatch::SetSearch(search.clone())]);
        assert_eq!(editor.selection_set.mode, SelectionMode::Match { search });
        Ok(())
    }
}
