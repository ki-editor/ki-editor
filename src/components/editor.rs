use crate::{
    canonicalized_path::CanonicalizedPath, context::Context, grid::CellUpdate,
    screen::RequestParams, selection::RangeCharIndex,
};
use std::{
    cell::{Ref, RefCell, RefMut},
    ops::Range,
    rc::Rc,
};

use crossterm::{
    event::{KeyCode, MouseButton, MouseEventKind},
    style::Color,
};
use event::KeyEvent;
use itertools::{Either, Itertools};
use key_event_macro::key;
use lsp_types::DiagnosticSeverity;
use ropey::{Rope, RopeSlice};
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
    component::ComponentId,
    keymap_legend::{Keymap, KeymapLegendConfig},
};

#[derive(PartialEq, Clone, Debug)]
pub enum Mode {
    Normal,
    Insert,
    Jump { jumps: Vec<Jump> },
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
        match &self.mode {
            Mode::Normal => {
                format!(
                    "{} [NORMAL:{}]",
                    &self.title,
                    self.selection_set.mode.display()
                )
            }
            Mode::Insert => {
                format!("{} [INSERT]", &self.title)
            }
            Mode::Jump { .. } => {
                format!("{} [JUMP]", &self.title)
            }
        }
    }
    fn set_title(&mut self, title: String) {
        self.title = title;
    }
    fn get_grid(&self, diagnostics: &[Diagnostic]) -> Grid {
        let editor = self;
        let Dimension { height, width } = editor.dimension();
        let mut grid: Grid = Grid::new(Dimension { height, width });
        let selection = &editor.selection_set.primary;

        // If the buffer selection is updated less recently than the window's scroll offset,
        // use the window's scroll offset.

        let scroll_offset = editor.scroll_offset();
        let buffer = editor.buffer();
        let rope = buffer.rope();
        let lines = rope
            .lines()
            .enumerate()
            .skip(scroll_offset.into())
            .take(height as usize)
            .collect::<Vec<(_, RopeSlice)>>();

        let secondary_selections = &editor.selection_set.secondary;

        let diagnostics = diagnostics
            .iter()
            .sorted_by(|a, b| a.severity.cmp(&b.severity))
            .rev()
            .filter_map(|diagnostic| {
                // We use `.ok()` to ignore diagnostics that are outside the buffer's range.
                let start = buffer.position_to_char(diagnostic.range.start).ok()?;
                let end = buffer.position_to_char(diagnostic.range.end).ok()?;
                let end = if start == end { end + 1 } else { end };
                let char_index_range = start..end;
                Some((diagnostic, char_index_range))
            });
        for (line_index, line) in lines {
            for (column_index, c) in line.chars().take(width as usize).enumerate() {
                grid.rows[line_index - scroll_offset as usize][column_index] = Cell {
                    symbol: c.to_string(),
                    background_color: Color::White,
                    foreground_color: Color::Black,
                    undercurl: None,
                };
            }
        }

        let updates = vec![]
            .into_iter()
            //
            // Jumps
            //
            .chain(
                editor
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
                        let background_color = if index % 2 == 0 {
                            Color::Red
                        } else {
                            Color::Blue
                        };

                        Some(
                            CellUpdate::new(position)
                                .background_color(background_color)
                                .foreground_color(Color::White)
                                .symbol(jump.character.to_string()),
                        )
                    }),
            )
            //
            // Diagnostics
            //
            .chain(diagnostics.into_iter().flat_map(|(diagnostic, range)| {
                range.to_usize_range().filter_map(|char_index| {
                    let char_index = CharIndex(char_index);
                    let position = buffer.char_to_position(char_index).ok()?;

                    let undercurl_color = match diagnostic.severity {
                        Some(severity) => match severity {
                            DiagnosticSeverity::ERROR => Color::DarkRed,
                            DiagnosticSeverity::WARNING => Color::DarkMagenta,
                            DiagnosticSeverity::INFORMATION => Color::DarkBlue,
                            DiagnosticSeverity::HINT => Color::DarkGreen,
                            _ => Color::Black,
                        },
                        None => Color::White,
                    };
                    Some(CellUpdate::new(position).undercurl(Some(undercurl_color)))
                })
            }))
            .chain(
                // Syntax highlight
                buffer
                    .highlighted_spans()
                    .iter()
                    .flat_map(|highlighted_span| {
                        highlighted_span
                            .range
                            .to_usize_range()
                            .filter_map(|char_index| {
                                Some(
                                    CellUpdate::new(
                                        buffer.char_to_position(CharIndex(char_index)).ok()?,
                                    )
                                    .style(highlighted_span.style),
                                )
                            })
                    }),
            )
            .chain(
                // Primary selection
                selection
                    .extended_range()
                    .to_usize_range()
                    .filter_map(|char_index| {
                        Some(
                            CellUpdate::new(buffer.char_to_position(CharIndex(char_index)).ok()?)
                                .background_color(Color::Yellow),
                        )
                    }),
            )
            .chain(
                // Primary selection secondary cursor
                buffer
                    .char_to_position(selection.to_char_index(&editor.cursor_direction.reverse()))
                    .ok()
                    .map(|position| {
                        CellUpdate::new(position)
                            .background_color(Color::DarkGrey)
                            .foreground_color(Color::White)
                    }),
            )
            .chain(
                // Secondary selection
                secondary_selections.iter().flat_map(|secondary_selection| {
                    secondary_selection
                        .range
                        .to_usize_range()
                        .filter_map(|char_index| {
                            let char_index = CharIndex(char_index);
                            let position = buffer.char_to_position(char_index).ok()?;

                            Some(CellUpdate::new(position).background_color(Color::DarkYellow))
                        })
                }),
            )
            .chain(
                // Secondary selection cursors
                secondary_selections
                    .iter()
                    .filter_map(|secondary_selection| {
                        Some(
                            vec![
                                Some(
                                    CellUpdate::new(
                                        buffer
                                            .char_to_position(
                                                secondary_selection.to_char_index(
                                                    &editor.cursor_direction.reverse(),
                                                ),
                                            )
                                            .ok()?,
                                    )
                                    .background_color(Color::Black)
                                    .foreground_color(Color::White),
                                ),
                                Some(
                                    CellUpdate::new(
                                        buffer
                                            .char_to_position(
                                                secondary_selection
                                                    .to_char_index(&editor.cursor_direction),
                                            )
                                            .ok()?,
                                    )
                                    .background_color(Color::DarkGrey)
                                    .foreground_color(Color::White),
                                ),
                            ]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>(),
                        )
                    })
                    .flatten(),
            )
            .filter_map(|update| update.subtract_vertical_offset(scroll_offset.into()))
            .collect::<Vec<_>>();

        grid.apply_cell_updates(updates)
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

#[derive(Clone)]
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
    Forward,
    Backward,
    Current,
}

impl Editor {
    pub fn from_text(language: tree_sitter::Language, text: &str) -> Self {
        Self {
            selection_set: SelectionSet {
                primary: Selection {
                    range: CharIndex(0)..CharIndex(0),
                    copied_text: None,
                    initial_range: None,
                },
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
            .map(|path| path.display_relative())
            .unwrap_or_else(|| Ok("<Untitled>".to_string()))
            .unwrap_or_else(|_| "<Untitled>".to_string());
        Self {
            selection_set: SelectionSet {
                primary: Selection {
                    range: CharIndex(0)..CharIndex(0),
                    copied_text: None,
                    initial_range: None,
                },
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

    fn select_parent(&mut self, direction: Direction) -> anyhow::Result<()> {
        self.select(SelectionMode::ParentNode, direction)
    }

    fn select_kids(&mut self) -> anyhow::Result<()> {
        let buffer = self.buffer.borrow().clone();
        self.update_selection_set(
            self.selection_set
                .select_kids(&buffer, &self.cursor_direction)?,
        );
        Ok(())
    }

    fn select_sibling(&mut self, direction: Direction) -> anyhow::Result<()> {
        self.select(SelectionMode::SiblingNode, direction)
    }

    pub fn select_line(&mut self, direction: Direction) -> anyhow::Result<()> {
        self.select(SelectionMode::Line, direction)
    }

    pub fn select_line_at(&mut self, line: usize) -> anyhow::Result<()> {
        let start = self.buffer.borrow().line_to_char(line)?;
        let selection_set = SelectionSet {
            primary: Selection {
                range: start..start + self.buffer.borrow().get_line(start)?.len_chars(),
                copied_text: None,
                initial_range: None,
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
        search: &Option<String>,
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
        self.select(SelectionMode::NamedNode, direction)
    }

    fn select_character(&mut self, direction: Direction) -> anyhow::Result<()> {
        self.select(SelectionMode::Character, direction)
    }

    fn select_diagnostic(&mut self, direction: Direction) -> anyhow::Result<Vec<Dispatch>> {
        self.select(SelectionMode::Diagnostic, direction)?;
        if let Some(diagnostic) = self
            .buffer
            .borrow()
            .find_diagnostic(&self.selection_set.primary.range)
        {
            Ok(vec![Dispatch::ShowInfo {
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

    fn reset(&mut self) {
        self.select(SelectionMode::Custom, Direction::Current);
        self.selection_set.reset()
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
        let range = self.buffer().position_to_char(range.start)?
            ..self.buffer().position_to_char(range.end)?;

        let mode = if self.buffer().given_range_is_node(&range) {
            SelectionMode::NamedNode
        } else {
            SelectionMode::Custom
        };
        let selection_set = SelectionSet {
            primary: Selection {
                range,
                copied_text: self.selection_set.primary.copied_text.clone(),
                initial_range: None,
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
        let direction = match selection_mode {
            SelectionMode::Line
            | SelectionMode::Character
            | SelectionMode::Token
            | SelectionMode::Diagnostic
                if self.selection_set.mode != selection_mode =>
            {
                // TODO: Current only applies to a few selection mode
                //       we need to rethink how to solve this discrepancies
                Direction::Current
            }
            _ => direction,
        };
        let selection = self.get_selection_set(&selection_mode, direction)?;

        self.update_selection_set(selection);
        Ok(())
    }

    fn select_final(&mut self, direction: Direction) -> anyhow::Result<()> {
        let selection_mode = if self.selection_set.mode.is_node() {
            SelectionMode::SiblingNode
        } else {
            self.selection_set.mode.clone()
        };
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

    fn jump_from_selection(
        &mut self,
        direction: Direction,
        selection: &Selection,
    ) -> anyhow::Result<()> {
        let mut current_selection = selection.clone();
        let mut jumps = Vec::new();

        for char in ('a'..='z')
            .interleave('A'..='Z')
            .interleave('0'..='9')
            .chain(",.".chars())
            // 'j' and 'J' are reserved for subsequent jump.
            .filter(|c| c != &'j' && c != &'J')
        {
            let next_selection = Selection::get_selection_(
                &self.buffer.borrow(),
                &current_selection,
                &self.selection_set.mode,
                &direction,
                &self.cursor_direction,
            )?;

            if next_selection != current_selection {
                jumps.push(Jump {
                    character: char,
                    selection: next_selection.clone(),
                });
                current_selection = next_selection;
            } else {
                break;
            }
        }
        self.mode = Mode::Jump { jumps };
        Ok(())
    }

    fn jump(&mut self, direction: Direction) -> anyhow::Result<()> {
        self.jump_from_selection(direction, &self.selection_set.primary.clone())
    }

    fn cut(&mut self, context: &mut Context) -> Vec<Dispatch> {
        // Set the clipboard content to the current selection
        // if there is only one cursor.
        if self.selection_set.secondary.is_empty() {
            context.set_clipboard_content(
                self.buffer
                    .borrow()
                    .slice(&self.selection_set.primary.range)
                    .into(),
            )
        }
        let edit_transaction =
            EditTransaction::from_action_groups(self.selection_set.map(|selection| {
                let old_range = selection.extended_range();
                let old = self.buffer.borrow().slice(&old_range);
                ActionGroup::new(vec![
                    Action::Edit(Edit {
                        start: old_range.start,
                        old: old.clone(),
                        new: Rope::new(),
                    }),
                    Action::Select(Selection {
                        range: old_range.start..old_range.start,
                        copied_text: Some(old),
                        initial_range: None,
                    }),
                ])
            }));

        self.apply_edit_transaction(edit_transaction)
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
                let old = self.buffer.borrow().slice(&range);
                EditTransaction::from_action_groups(vec![ActionGroup::new(vec![
                    Action::Edit(Edit {
                        start,
                        old,
                        new: copied_text.clone(),
                    }),
                    Action::Select(Selection {
                        range: {
                            let start = start + copied_text.len_chars();
                            start..start
                        },
                        copied_text: Some(copied_text),
                        initial_range: None,
                    }),
                ])])
            } else {
                EditTransaction::from_action_groups(vec![])
            }
        });
        let edit_transaction = EditTransaction::merge(edit_transactions);
        self.apply_edit_transaction(edit_transaction)
    }

    fn paste(&mut self, context: &Context) -> Vec<Dispatch> {
        self.replace_current_selection_with(|selection| {
            selection
                .copied_text
                .clone()
                .or_else(|| context.get_clipboard_content().map(Rope::from))
        })
    }

    fn replace(&mut self) -> Vec<Dispatch> {
        let edit_transaction = EditTransaction::merge(self.selection_set.map(|selection| {
            if let Some(replacement) = &selection.copied_text {
                let replacement_text_len = replacement.len_chars();
                let replaced_text = self.buffer.borrow().slice(&selection.range);
                EditTransaction::from_action_groups(vec![ActionGroup::new(vec![
                    Action::Edit(Edit {
                        start: selection.range.start,
                        old: replaced_text.clone(),
                        new: replacement.clone(),
                    }),
                    Action::Select(Selection {
                        range: selection.range.start..selection.range.start + replacement_text_len,
                        copied_text: Some(replaced_text),
                        initial_range: None,
                    }),
                ])])
            } else {
                EditTransaction::from_action_groups(vec![])
            }
        }));
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

    pub fn get_document_did_change_dispatch(&self) -> Vec<Dispatch> {
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

    fn g_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Get",
            owner_id: self.id(),
            keymaps: vec![]
                .into_iter()
                .chain(
                    self.get_request_params()
                        .map(|params| {
                            vec![
                                Keymap::new(
                                    "d",
                                    "Definition(s)",
                                    Dispatch::RequestDefinitions(params.clone()),
                                ),
                                Keymap::new("r", "References", Dispatch::RequestReferences(params)),
                                Keymap::new(
                                    "e",
                                    "Errors",
                                    Dispatch::SetQuickfixList(QuickfixListType::LspDiagnostic),
                                ),
                            ]
                        })
                        .unwrap_or_default(),
                )
                .collect(),
        }
    }

    fn open_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Open",
            owner_id: self.id(),
            keymaps: vec![Keymap::new(
                "f",
                "Git tracked files",
                Dispatch::OpenFilePicker,
            )],
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
            },
            HandleEventResult::Handled(dispatches) => Ok(dispatches),
            _ => Ok(vec![]),
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
                        range: CharIndex(0)..CharIndex(self.buffer.borrow().len_chars()),
                        copied_text: self.selection_set.primary.copied_text.clone(),
                        initial_range: None,
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
            key!("ctrl+x") => Ok(HandleEventResult::Handled(self.cut(context))),
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
                key!('j') => {
                    if let Some(jump) = jumps
                        .iter()
                        .max_by(|a, b| a.selection.range.start.cmp(&b.selection.range.start))
                    {
                        self.jump_from_selection(Direction::Forward, &jump.selection.clone())?;
                    }
                }
                key!('J') => {
                    if let Some(jump) = jumps
                        .iter()
                        .min_by(|a, b| a.selection.range.end.cmp(&b.selection.range.end))
                    {
                        self.jump_from_selection(Direction::Backward, &jump.selection.clone())?;
                    }
                }
                key => {
                    let KeyCode::Char(c) = key.code else {return Ok(())};
                    let matching_jump = jumps.iter().find(|jump| c == jump.character);
                    if let Some(jump) = matching_jump {
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
                }
            },
            _ => unreachable!(),
        }
        Ok(())
    }

    /// Similar to Change in Vim, but does not copy the current selection
    fn change(&mut self) {
        let edit_transaction =
            EditTransaction::from_action_groups(self.selection_set.map(|selection| {
                let copied_text: Rope = self.buffer.borrow().slice(&selection.range);
                ActionGroup::new(vec![
                    Action::Edit(Edit {
                        start: selection.range.start,
                        old: copied_text,
                        new: Rope::new(),
                    }),
                    Action::Select(Selection {
                        range: selection.range.start..selection.range.start,
                        copied_text: selection.copied_text.clone(),
                        initial_range: None,
                    }),
                ])
            }));

        self.apply_edit_transaction(edit_transaction);
        self.enter_insert_mode(CursorDirection::Start);
    }

    fn insert(&mut self, s: &str) -> Vec<Dispatch> {
        let edit_transaction =
            EditTransaction::from_action_groups(self.selection_set.map(|selection| {
                ActionGroup::new(vec![
                    Action::Edit(Edit {
                        start: selection.to_char_index(&CursorDirection::End),
                        old: Rope::new(),
                        new: Rope::from_str(s),
                    }),
                    Action::Select(Selection {
                        range: selection.range.start + s.len()..selection.range.start + s.len(),
                        copied_text: selection.copied_text.clone(),
                        initial_range: None,
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

    fn handle_normal_mode(
        &mut self,
        context: &mut Context,
        event: KeyEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        match event {
            // Objects
            key!("a") => self.add_selection()?,
            key!("shift+A") => self.add_selection()?,
            key!("b") => self.select_backward(),
            key!("c") => self.select_character(Direction::Forward)?,
            key!("shift+C") => self.select_character(Direction::Backward)?,
            key!("d") => {
                return Ok(self.delete(Direction::Forward));
            }
            key!("shift+D") => return Ok(self.delete(Direction::Backward)),
            key!("e") => return self.select_diagnostic(Direction::Forward),
            key!("shift+E") => return self.select_diagnostic(Direction::Backward),
            key!("f") => self.select_final(Direction::Forward)?,
            key!("shift+F") => self.select_final(Direction::Backward)?,
            // f
            // TODO: f goes into file picker mode,
            // for example, pressing fg means select git tracked files
            // fc means changed files
            // fb means opened editor
            // F
            key!("g") => {
                return Ok(vec![Dispatch::ShowKeymapLegend(
                    self.g_mode_keymap_legend_config(),
                )])
            }
            key!("h") => self.toggle_highlight_mode(),
            // H
            key!("i") => self.enter_insert_mode(CursorDirection::End),
            key!("shift+I") => self.enter_insert_mode(CursorDirection::Start),
            // I
            key!("j") => self.jump(Direction::Forward)?,
            key!("shift+J") => self.jump(Direction::Backward)?,
            key!("k") => self.select_kids()?,
            key!("l") => self.select_line(Direction::Forward)?,
            key!("shift+L") => self.select_line(Direction::Backward)?,
            key!("m") => self.select_match(Direction::Forward, &context.last_search())?,
            key!("shift+M") => self.select_match(Direction::Backward, &context.last_search())?,
            key!("n") => self.select_named_node(Direction::Forward)?,
            key!("shift+N") => self.select_named_node(Direction::Backward)?,
            key!("o") => {
                return Ok(vec![Dispatch::ShowKeymapLegend(
                    self.open_mode_keymap_legend_config(),
                )])
            }
            // O
            key!("p") => self.select_parent(Direction::Forward)?,
            key!("shift+P") => self.select_parent(Direction::Backward)?,
            key!("q") => return Ok(vec![Dispatch::GotoQuickfixListItem(Direction::Forward)]),
            key!("shift+Q") => {
                return Ok(vec![Dispatch::GotoQuickfixListItem(Direction::Backward)])
            }
            key!("r") => return Ok(self.replace()),
            key!("shift+R") => {
                return Ok(self
                    .get_request_params()
                    .map(|params| vec![Dispatch::PrepareRename(params)])
                    .unwrap_or_default())
            }
            key!("s") => self.select_sibling(Direction::Forward)?,
            key!("shift+S") => self.select_sibling(Direction::Backward)?,
            key!("t") => self.select_token(Direction::Forward)?,
            key!("shift+T") => self.select_token(Direction::Backward)?,
            key!("u") => return Ok(self.upend(Direction::Forward)),
            key!("v") => {
                self.select_view(Direction::Forward)?;
            }
            key!("shift+V") => {
                self.select_view(Direction::Backward)?;
            }
            key!("w") => self.select_word(Direction::Forward)?,
            key!("shift+W") => self.select_word(Direction::Backward)?,
            key!("x") => return Ok(self.exchange(Direction::Forward)),
            key!("shift+X") => return Ok(self.exchange(Direction::Backward)),
            // y
            key!("z") => self.align_cursor_to_center(),
            key!("shift+Z") => self.align_cursor_to_top(),
            key!("0") => self.reset(),
            key!("backspace") => {
                self.change();
            }
            key!("enter") => return Ok(self.open_new_line()),
            key!(",") => {
                return Ok(self
                    .get_request_params()
                    .map(|params| vec![Dispatch::RequestCodeAction(params)])
                    .unwrap_or_default())
            }
            key!("?") => {
                self.editor_mut().set_mode(Mode::Normal);
                return Ok(self.request_hover());
            }
            key!("%") => self.change_cursor_direction(),
            key!("(") | key!(")") => return Ok(self.enclose(Enclosure::RoundBracket)),
            key!("[") | key!("]") => return Ok(self.enclose(Enclosure::SquareBracket)),
            key!('{') | key!('}') => return Ok(self.enclose(Enclosure::CurlyBracket)),
            key!('<') | key!('>') => return Ok(self.enclose(Enclosure::AngleBracket)),

            // TODO: - and = are temporarily assigned keys
            key!('-') => return Ok(vec![Dispatch::GotoOpenedEditor(Direction::Backward)]),
            key!('=') => return Ok(vec![Dispatch::GotoOpenedEditor(Direction::Forward)]),
            key!('*') => return self.match_current_selection(),
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
            selection.range = char_index..char_index
        });
        self.selection_set.mode = SelectionMode::Custom;
        self.mode = Mode::Insert;
        self.cursor_direction = CursorDirection::Start;
    }

    pub fn enter_normal_mode(&mut self) -> anyhow::Result<()> {
        self.mode = Mode::Normal;
        self.select(SelectionMode::Custom, Direction::Current)
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
                range: start..start,
                copied_text: self.selection_set.primary.copied_text.clone(),
                initial_range: self.selection_set.primary.initial_range.clone(),
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
        ) -> EditTransaction,
        get_actual_edit_transaction: impl Fn(
            /* current */ &Selection,
            /* next */ &Selection,
        ) -> EditTransaction,
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
            let edit_transaction = get_trial_edit_transaction(&current_selection, &next_selection);

            let new_buffer = {
                let mut new_buffer = self.buffer.borrow().clone();
                if let Err(_) =
                    new_buffer.apply_edit_transaction(&edit_transaction, self.selection_set.clone())
                {
                    continue;
                }
                new_buffer
            };

            let text_at_next_selection: Rope = buffer.slice(&next_selection.range);

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
                )));
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
            |current_selection: &Selection, next_selection: &Selection| {
                let current_selection_range = current_selection.extended_range();
                let text_at_current_selection = buffer.slice(&current_selection_range);

                EditTransaction::from_action_groups(vec![
                    ActionGroup::new(vec![Action::Edit(Edit {
                        start: current_selection_range.start,
                        old: text_at_current_selection.clone(),
                        new: buffer.slice(&next_selection.range),
                    })]),
                    ActionGroup::new(vec![Action::Edit(Edit {
                        start: next_selection.range.start,
                        old: buffer.slice(&next_selection.range),
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
                ])
            };

        let get_actual_edit_transaction =
            |current_selection: &Selection, next_selection: &Selection| {
                let current_selection_range = current_selection.extended_range();
                let text_at_current_selection: Rope = buffer.slice(&current_selection_range);
                let text_at_next_selection: Rope = buffer.slice(&next_selection.range);

                EditTransaction::from_action_groups(vec![
                    ActionGroup::new(vec![Action::Edit(Edit {
                        start: current_selection_range.start,
                        old: text_at_current_selection.clone(),
                        new: text_at_next_selection.clone(),
                    })]),
                    ActionGroup::new(vec![
                        Action::Edit(Edit {
                            start: next_selection.range.start,
                            old: text_at_next_selection,
                            // This time without whitespace padding
                            new: text_at_current_selection.clone(),
                        }),
                        Action::Select(Selection {
                            range: next_selection.range.start
                                ..(next_selection.range.start
                                    + text_at_current_selection.len_chars()),
                            copied_text: current_selection.copied_text.clone(),

                            // TODO: fix this, the initial_range should be updated as well
                            initial_range: current_selection.initial_range.clone(),
                        }),
                    ]),
                ])
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
        let selection_mode = if self.selection_set.mode.is_node() {
            SelectionMode::SiblingNode
        } else {
            self.selection_set.mode.clone()
        };
        self.replace_faultlessly(&selection_mode, direction)
    }

    fn add_selection(&mut self) -> anyhow::Result<()> {
        self.selection_set
            .add_selection(&self.buffer.borrow(), &self.cursor_direction)?;
        self.recalculate_scroll_offset();
        Ok(())
    }

    #[cfg(test)]
    pub fn get_selected_texts(&self) -> Vec<String> {
        let buffer = self.buffer.borrow();
        let mut selections = self.selection_set.map(|selection| {
            (
                selection.range.clone(),
                buffer.slice(&selection.extended_range()).to_string(),
            )
        });
        selections.sort_by(|a, b| a.0.start.0.cmp(&b.0.start.0));
        selections
            .into_iter()
            .map(|selection| selection.1)
            .collect()
    }

    #[cfg(test)]
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
                        start,
                        old: self.buffer.borrow().slice(&(start..selection.range.start)),
                        new: Rope::from(""),
                    }),
                    Action::Select(Selection {
                        range: start..start,
                        copied_text: selection.copied_text.clone(),
                        initial_range: selection.initial_range.clone(),
                    }),
                ])
            }));

        self.apply_edit_transaction(edit_transaction)
    }

    fn delete(&mut self, direction: Direction) -> Vec<Dispatch> {
        let buffer = self.buffer.borrow().clone();
        let mode = if self.selection_set.mode.is_node() {
            // If the selection is a node, the mode should be SiblingNode
            // because other node-based movement does not make sense for delete
            SelectionMode::SiblingNode
        } else {
            self.selection_set.mode.clone()
        };

        let edit_transactions = self.selection_set.map(|selection| {
            let get_trial_edit_transaction =
                |current_selection: &Selection, other_selection: &Selection| {
                    let range = current_selection
                        .range
                        .start
                        .min(other_selection.range.start)
                        ..current_selection.range.end.max(other_selection.range.end);

                    // Add whitespace padding
                    let new: Rope = format!(" {} ", buffer.slice(&current_selection.range)).into();

                    EditTransaction::from_action_groups(vec![ActionGroup::new(vec![Action::Edit(
                        Edit {
                            start: range.start,
                            old: buffer.slice(&range),
                            new,
                        },
                    )])])
                };
            let get_actual_edit_transaction =
                |current_selection: &Selection, other_selection: &Selection| {
                    let range = current_selection
                        .range
                        .start
                        .min(other_selection.range.start)
                        ..current_selection.range.end.max(other_selection.range.end);
                    let new: Rope = buffer.slice(&other_selection.range);

                    let new_len_chars = new.len_chars();
                    EditTransaction::from_action_groups(vec![ActionGroup::new(vec![
                        Action::Edit(Edit {
                            start: range.start,
                            old: buffer.slice(&range),
                            new,
                        }),
                        Action::Select(Selection {
                            range: range.start..(range.start + new_len_chars),
                            copied_text: current_selection.copied_text.clone(),
                            initial_range: current_selection.initial_range.clone(),
                        }),
                    ])])
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
                                start: selection.range.start,
                                old: buffer.slice(&selection.range),
                                new: Rope::from(""),
                            }),
                            Action::Select(Selection {
                                range: selection.range.start..selection.range.start,
                                copied_text: selection.copied_text.clone(),
                                initial_range: selection.initial_range.clone(),
                            }),
                        ])])
                    }
                })
                .collect(),
        );
        self.apply_edit_transaction(edit_transaction)
    }

    /// Replace the parent node of the current node with the current node
    fn upend(&mut self, direction: Direction) -> Vec<Dispatch> {
        let buffer = self.buffer.borrow().clone();
        let edit_transactions = self.selection_set.map(|selection| {
            let get_trial_edit_transaction =
                |current_selection: &Selection, other_selection: &Selection| {
                    let range = current_selection
                        .range
                        .start
                        .min(other_selection.range.start)
                        ..current_selection.range.end.max(other_selection.range.end);

                    // Add whitespace padding
                    let new: Rope = format!(" {} ", buffer.slice(&current_selection.range)).into();

                    EditTransaction::from_action_groups(vec![ActionGroup::new(vec![Action::Edit(
                        Edit {
                            start: range.start,
                            old: buffer.slice(&range),
                            new,
                        },
                    )])])
                };
            let get_actual_edit_transaction =
                |current_selection: &Selection, other_selection: &Selection| {
                    let range = current_selection
                        .range
                        .start
                        .min(other_selection.range.start)
                        ..current_selection.range.end.max(other_selection.range.end);
                    let new: Rope = buffer.slice(&current_selection.range);

                    let new_len_chars = new.len_chars();
                    EditTransaction::from_action_groups(vec![ActionGroup::new(vec![
                        Action::Edit(Edit {
                            start: range.start,
                            old: buffer.slice(&range),
                            new,
                        }),
                        Action::Select(Selection {
                            range: range.start..(range.start + new_len_chars),
                            copied_text: current_selection.copied_text.clone(),
                            initial_range: current_selection.initial_range.clone(),
                        }),
                    ])])
                };
            self.get_valid_selection(
                selection,
                &SelectionMode::ParentNode,
                &direction,
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
        self.scroll_offset = match direction {
            Direction::Forward => self
                .scroll_offset
                .saturating_add(self.rectangle.height)
                .min(self.buffer.borrow().len_lines() as u16),
            Direction::Backward => self.scroll_offset.saturating_sub(self.rectangle.height),
            Direction::Current => self.scroll_offset,
        };

        let char_index = self
            .buffer
            .borrow()
            .line_to_char(self.scroll_offset as usize)?;
        self.update_selection_set(SelectionSet {
            primary: Selection {
                range: char_index..char_index,
                copied_text: self.selection_set.primary.copied_text.clone(),
                initial_range: self.selection_set.primary.initial_range.clone(),
            },
            secondary: vec![],
            mode: SelectionMode::Custom,
        });
        self.align_cursor_to_center();
        Ok(())
    }

    pub fn reset_selection(&mut self) {
        self.selection_set = SelectionSet {
            primary: Selection {
                range: CharIndex(0)..CharIndex(0),
                copied_text: None,
                initial_range: None,
            },
            secondary: vec![],
            mode: SelectionMode::Line,
        };
    }

    pub fn replace_previous_word(&mut self, completion: &str) -> anyhow::Result<Vec<Dispatch>> {
        let selection = self.get_selection_set(&SelectionMode::Word, Direction::Backward)?;
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
                            start: line_start + current_line.len_chars(),
                            old: Rope::new(),
                            new: format!("{}\n", leading_whitespaces).into(),
                        }),
                        Action::Select(Selection {
                            range: {
                                let start = line_start
                                    + current_line.len_chars()
                                    + leading_whitespaces.len();
                                start..start
                            },
                            copied_text: selection.copied_text.clone(),
                            initial_range: selection.initial_range.clone(),
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
                        start: range.start,
                        old: self.buffer().slice(&range),
                        new: edit.new_text.into(),
                    });

                    let action_select = Action::Select(Selection {
                        range: {
                            let end = range.start + next_text_len;
                            end..end
                        },
                        copied_text: None,
                        initial_range: None,
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
        Ok(vec![Dispatch::DocumentDidSave { path }])
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
        let edit_transaction =
            EditTransaction::from_action_groups(self.selection_set.map(|selection| {
                let old = self.buffer().slice(&selection.extended_range());
                ActionGroup::new(vec![
                    Action::Edit(Edit {
                        start: selection.range.start,
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
                        old,
                    }),
                    Action::Select(Selection {
                        range: selection.range.start..selection.range.end + 2,
                        copied_text: selection.copied_text.clone(),
                        initial_range: selection.initial_range.clone(),
                    }),
                ])
            }));

        self.apply_edit_transaction(edit_transaction)
    }

    fn match_current_selection(&mut self) -> anyhow::Result<Vec<Dispatch>> {
        let content = self
            .buffer()
            .slice(&self.selection_set.primary.extended_range());

        if content.len_chars() == 0 {
            return Ok(vec![]);
        }

        self.select(
            SelectionMode::Match {
                search: content.clone().to_string(),
            },
            Direction::Current,
        )?;

        return Ok(vec![Dispatch::SetSearch {
            search: content.into(),
        }]);
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
    copied_text: Option<Rope>,
    initial_range: Option<Range<CharIndex>>,
) -> anyhow::Result<Selection> {
    Ok(Selection {
        range: buffer.byte_to_char(node.start_byte())?..buffer.byte_to_char(node.end_byte())?,
        copied_text,
        initial_range,
    })
}

pub enum HandleEventResult {
    Handled(Vec<Dispatch>),
    Ignored(KeyEvent),
}

#[cfg(test)]

mod test_editor {

    use crate::{
        components::{
            component::Component,
            editor::{CursorDirection, Mode},
        },
        context::Context,
        lsp::diagnostic::Diagnostic,
        position::Position,
        screen::Dispatch,
        selection::SelectionMode,
    };

    use super::{Direction, Editor};
    use key_event_macro::keys;
    use pretty_assertions::assert_eq;
    use tree_sitter_rust::language;

    #[test]
    fn select_character() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        editor.select_character(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["f"]);
        editor.select_character(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["n"]);

        editor.select_character(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["f"]);
        Ok(())
    }

    #[test]
    fn select_line() -> anyhow::Result<()> {
        // Multiline source code
        let mut editor = Editor::from_text(language(), "\nfn main() {\n\n\nlet x = 1;\n}\n");
        editor.select_line(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["\n"]);
        editor.select_line(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["fn main() {\n"]);
        editor.select_line(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["\n"]);
        editor.select_line(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["\n"]);
        editor.select_line(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["let x = 1;\n"]);
        editor.select_line(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["}\n"]);

        editor.select_line(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["let x = 1;\n"]);
        editor.select_line(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["\n"]);
        editor.select_line(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["\n"]);
        editor.select_line(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["fn main() {\n"]);
        editor.select_line(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["\n"]);
        Ok(())
    }

    #[test]
    fn select_word() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(
            language(),
            "fn main_fn() { let x = \"hello world lisp-y\"; }",
        );
        editor.select_word(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["fn"]);
        editor.select_word(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["main_fn"]);
        editor.select_word(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["let"]);
        editor.select_word(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["x"]);
        editor.select_word(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["hello"]);
        editor.select_word(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["world"]);
        editor.select_word(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["lisp"]);
        editor.select_word(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["y"]);

        editor.select_word(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["lisp"]);
        editor.select_word(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["world"]);
        editor.select_word(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["hello"]);
        editor.select_word(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["x"]);
        editor.select_word(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["let"]);
        editor.select_word(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["main_fn"]);
        editor.select_word(Direction::Backward)?;
        Ok(())
    }

    #[test]
    /// Should search using AST grep first whenever possible
    fn select_match_ast_grep() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = f(y); f(x); f( z ) }");
        let search = Some("f($EXPR)".to_string());

        editor.select_match(Direction::Forward, &search)?;
        assert_eq!(editor.get_selected_texts(), vec!["f(y)"]);
        editor.select_match(Direction::Forward, &search)?;
        assert_eq!(editor.get_selected_texts(), vec!["f(x)"]);
        editor.select_match(Direction::Forward, &search)?;
        assert_eq!(editor.get_selected_texts(), vec!["f( z )"]);

        editor.select_match(Direction::Backward, &search)?;
        assert_eq!(editor.get_selected_texts(), vec!["f(x)"]);
        editor.select_match(Direction::Backward, &search)?;
        assert_eq!(editor.get_selected_texts(), vec!["f(y)"]);
        Ok(())
    }

    #[test]
    /// Should search using string if AST grep matches nothing
    fn select_match_string() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = f(y); f(x); f( z ) }");
        let search = Some("f(".to_string());

        editor.select_match(Direction::Forward, &search)?;
        assert_eq!(editor.get_selected_texts(), vec!["f("]);
        editor.select_match(Direction::Forward, &search)?;
        assert_eq!(editor.get_selected_texts(), vec!["f("]);
        editor.select_match(Direction::Forward, &search)?;
        assert_eq!(editor.get_selected_texts(), vec!["f("]);

        editor.insert("hello");
        assert_eq!(
            editor.text(),
            "fn main() { let x = f(y); f(x); fhello( z ) }"
        );
        Ok(())
    }

    #[test]
    /// Should search using regex if literal strig search matches nothing
    fn select_match_regex() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = f(y); f(x); f( z ) }");
        let search = Some(r"f\([a-z]\)".to_string());

        editor.select_match(Direction::Forward, &search)?;
        assert_eq!(editor.get_selected_texts(), vec!["f(y)"]);
        editor.select_match(Direction::Forward, &search)?;
        assert_eq!(editor.get_selected_texts(), vec!["f(x)"]);
        editor.select_match(Direction::Forward, &search)?;
        assert_eq!(editor.get_selected_texts(), vec!["f(x)"]);

        Ok(())
    }

    #[test]
    fn select_token() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        editor.select_token(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["fn"]);
        editor.select_token(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["main"]);
        editor.select_token(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["("]);
        editor.select_token(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec![")"]);
        editor.select_token(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["{"]);
        editor.select_token(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["let"]);
        editor.select_token(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["x"]);

        editor.select_token(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["let"]);
        editor.select_token(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["{"]);
        editor.select_token(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec![")"]);
        editor.select_token(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["("]);
        editor.select_token(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["main"]);
        editor.select_token(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["fn"]);
        editor.select_token(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["fn"]);
        Ok(())
    }

    #[test]
    fn select_parent() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        // Move token to 1
        for _ in 0..9 {
            editor.select_token(Direction::Forward)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["1"]);

        editor.select_parent(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["let x = 1;"]);
        editor.select_parent(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["{ let x = 1; }"]);
        editor.select_parent(Direction::Forward)?;
        assert_eq!(
            editor.get_selected_texts(),
            vec!["fn main() { let x = 1; }"]
        );

        editor.select_parent(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["main"]);
        Ok(())
    }

    #[test]
    fn select_sibling() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize, y: Vec<A>) {}");
        // Move token to "x: usize"
        for _ in 0..3 {
            editor.select_named_node(Direction::Forward)?;
        }
        assert_eq!(editor.get_selected_texts(), vec!["x: usize"]);

        editor.select_sibling(Direction::Forward)?;
        editor.select_sibling(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["y: Vec<A>"]);
        editor.select_sibling(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["y: Vec<A>"]);

        editor.select_sibling(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["x: usize"]);
        editor.select_sibling(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["x: usize"]);
        Ok(())
    }

    #[test]
    /// Should select the most ancestral node if the node's child and its parents has the same range.
    fn select_sibling_2() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = X {a,b,c:d} }");

        // Select `a`
        for _ in 0..11 {
            editor.select_token(Direction::Forward)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["a"]);

        editor.select_sibling(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["b"]);

        editor.select_sibling(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["c:d"]);

        editor.select_sibling(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["b"]);

        editor.select_sibling(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["a"]);
        Ok(())
    }

    #[test]
    fn select_kids() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize, y: Vec<A>) {}");
        // Move token to "x"
        for _ in 0..4 {
            editor.select_token(Direction::Forward)?;
        }
        assert_eq!(editor.get_selected_texts(), vec!["x"]);

        editor.select_kids()?;
        assert_eq!(editor.get_selected_texts(), vec!["x: usize, y: Vec<A>"]);
        Ok(())
    }

    #[test]
    fn select_named_node() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize) { let x = 1; }");

        editor.select_named_node(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["main"]);
        editor.select_named_node(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["(x: usize)"]);
        editor.select_named_node(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["x: usize"]);
        editor.select_named_node(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["usize"]);
        editor.select_named_node(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["{ let x = 1; }"]);
        editor.select_named_node(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["let x = 1;"]);

        editor.select_named_node(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["{ let x = 1; }"]);
        editor.select_named_node(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["usize"]);
        editor.select_named_node(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["x: usize"]);
        editor.select_named_node(Direction::Backward)?;
        assert_eq!(editor.get_selected_texts(), vec!["(x: usize)"]);
        editor.select_named_node(Direction::Backward)?;
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
                    Dispatch::ShowInfo { content } => Some(content),
                    _ => None,
                })
                .unwrap();
            assert!(content.join("\n").contains(info))
        }

        let dispatches = editor.select_diagnostic(Direction::Forward)?;
        assert_dispatch_contains(
            dispatches,
            "[UNKNOWN]\nspongebob\n\n[RELATED INFORMATION]\n",
        );
        assert_eq!(editor.get_selected_texts(), vec!["f"]);

        let dispatches = editor.select_diagnostic(Direction::Forward)?;
        assert_dispatch_contains(dispatches, "sandy");
        assert_eq!(editor.get_selected_texts(), vec!["fn"]);

        let dispatches = editor.select_diagnostic(Direction::Forward)?;
        assert_dispatch_contains(dispatches, "patrick");
        assert_eq!(editor.get_selected_texts(), vec!["n "]);

        let dispatches = editor.select_diagnostic(Direction::Forward)?;
        assert_dispatch_contains(dispatches, "squidward");
        assert_eq!(editor.get_selected_texts(), vec![" m"]);

        let dispatches = editor.select_diagnostic(Direction::Forward)?;
        assert_dispatch_contains(dispatches, "squidward");
        assert_eq!(editor.get_selected_texts(), vec![" m"]);

        let dispatches = editor.select_diagnostic(Direction::Backward)?;
        assert_dispatch_contains(dispatches, "patrick");

        let dispatches = editor.select_diagnostic(Direction::Backward)?;
        assert_dispatch_contains(dispatches, "sandy");

        let dispatches = editor.select_diagnostic(Direction::Backward)?;
        assert_dispatch_contains(dispatches, "spongebob");
        Ok(())
    }

    #[test]
    fn select_named_node_from_line_mode() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize) { \n let x = 1; }");
        // Select the second line
        editor.select_line(Direction::Forward)?;
        editor.select_line(Direction::Forward)?;

        // Select next name node
        editor.select_named_node(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["let x = 1;"]);
        Ok(())
    }

    #[test]
    fn copy_replace() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        editor.select_token(Direction::Forward)?;
        let mut context = Context::default();
        editor.copy(&mut context);
        editor.select_token(Direction::Forward)?;
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
        editor.select_token(Direction::Forward)?;
        let mut context = Context::default();
        editor.copy(&mut context);
        editor.select_token(Direction::Forward)?;
        editor.paste(&context);
        assert_eq!(editor.text(), "fn fn() { let x = 1; }");
        assert_eq!(editor.get_selected_texts(), vec![""]);
        Ok(())
    }

    #[test]
    fn cut_paste() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = 1; }");
        let mut context = Context::default();
        editor.select_token(Direction::Forward)?;
        editor.cut(&mut context);
        assert_eq!(editor.text(), " main() { let x = 1; }");
        assert_eq!(editor.get_selected_texts(), vec![""]);

        editor.select_token(Direction::Forward)?;
        editor.paste(&context);

        assert_eq!(editor.text(), " fn() { let x = 1; }");
        assert_eq!(editor.get_selected_texts(), vec![""]);
        Ok(())
    }

    #[test]
    fn exchange_sibling() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main(x: usize, y: Vec<A>) {}");
        // Move token to "x: usize"
        for _ in 0..3 {
            editor.select_named_node(Direction::Forward)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["x: usize"]);

        editor.exchange(Direction::Forward);
        assert_eq!(editor.text(), "fn main(y: Vec<A>, x: usize) {}");

        editor.exchange(Direction::Backward);
        assert_eq!(editor.text(), "fn main(x: usize, y: Vec<A>) {}");
        Ok(())
    }

    #[test]
    fn exchange_sibling_2() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "use a;\nuse b;\nuse c;");

        // Select first statement
        editor.select_character(Direction::Forward)?;
        editor.select_character(Direction::Forward)?;

        editor.select_parent(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["use a;"]);

        editor.exchange(Direction::Forward);
        assert_eq!(editor.text(), "use b;\nuse a;\nuse c;");
        editor.exchange(Direction::Forward);
        assert_eq!(editor.text(), "use b;\nuse c;\nuse a;");
        Ok(())
    }

    #[test]
    fn upend() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn main() { let x = a.b(c()); }");
        // Move selection to "c()"
        for _ in 0..9 {
            editor.select_named_node(Direction::Forward)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["c()"]);

        editor.upend(Direction::Forward);
        assert_eq!(editor.text(), "fn main() { let x = c(); }");

        editor.upend(Direction::Forward);
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

        editor.select_line(Direction::Forward)?;
        editor.select_line(Direction::Forward)?;

        editor.exchange(Direction::Forward);
        assert_eq!(
            editor.text(),
            "
    let x = 1;
fn main() {
    let y = 2;
}"
        );

        editor.exchange(Direction::Backward);
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
        editor.select_character(Direction::Forward)?;

        editor.exchange(Direction::Forward);
        assert_eq!(editor.text(), "nf main() { let x = 1; }");
        editor.exchange(Direction::Forward);
        assert_eq!(editor.text(), "n fmain() { let x = 1; }");

        editor.exchange(Direction::Backward);
        assert_eq!(editor.text(), "nf main() { let x = 1; }");
        editor.exchange(Direction::Backward);

        assert_eq!(editor.text(), "fn main() { let x = 1; }");
        Ok(())
    }

    #[test]
    fn multi_insert() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "struct A(usize, char)");
        // Select 'usize'
        for _ in 0..3 {
            editor.select_named_node(Direction::Forward)?;
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
            editor.select_named_node(Direction::Forward)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["let x = S(a);"]);

        editor.add_selection()?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["let x = S(a);", "let y = S(b);"]
        );

        for _ in 0..4 {
            editor.select_named_node(Direction::Forward)?;
        }

        assert_eq!(editor.get_selected_texts(), vec!["a", "b"]);

        editor.upend(Direction::Forward);

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
        editor.select_token(Direction::Forward)?;
        editor.select_parent(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f(x:a,y:b){}"]);

        editor.add_selection()?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["fn f(x:a,y:b){}", "fn g(x:a,y:b){}"]
        );

        editor.select_named_node(Direction::Forward)?;
        editor.select_named_node(Direction::Forward)?;
        editor.select_named_node(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["x:a", "x:a"]);

        editor.exchange(Direction::Forward);
        assert_eq!(editor.text(), "fn f(y:b,x:a){} fn g(y:b,x:a){}");
        assert_eq!(editor.get_selected_texts(), vec!["x:a", "x:a"]);

        editor.exchange(Direction::Backward);
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
            editor.select_named_node(Direction::Forward)?;
        }

        assert_eq!(
            editor.get_selected_texts(),
            vec!["let x = S(spongebob_squarepants);"]
        );

        editor.add_selection()?;
        editor.select_named_node(Direction::Forward)?;
        editor.select_named_node(Direction::Forward)?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["S(spongebob_squarepants)", "S(b)"]
        );

        let mut context = Context::default();
        editor.cut(&mut context);
        editor.enter_insert_mode(CursorDirection::Start);

        editor.insert("Some(");
        editor.paste(&context);
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

        editor.select_token(Direction::Forward)?;
        editor.toggle_highlight_mode();
        editor.select_token(Direction::Forward)?;
        editor.select_token(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f("]);

        // Toggle the second time should inverse the initial_range
        editor.toggle_highlight_mode();

        editor.select_token(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["f("]);

        editor.reset();

        assert_eq!(editor.get_selected_texts(), vec![""]);

        editor.select_token(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["f"]);

        // After reset, expect highlight mode is turned off
        editor.select_token(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["("]);
        Ok(())
    }

    #[test]
    fn highlight_mode_cut() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        editor.select_token(Direction::Forward)?;
        editor.toggle_highlight_mode();
        editor.select_token(Direction::Forward)?;
        editor.select_token(Direction::Forward)?;
        editor.select_token(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f()"]);

        let mut context = Context::default();
        editor.cut(&mut context);

        assert_eq!(editor.text(), "{ let x = S(a); let y = S(b); }");

        editor.paste(&context);

        assert_eq!(editor.text(), "fn f(){ let x = S(a); let y = S(b); }");
        Ok(())
    }

    #[test]
    fn highlight_mode_copy() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        editor.select_token(Direction::Forward)?;
        editor.toggle_highlight_mode();
        editor.select_token(Direction::Forward)?;
        editor.select_token(Direction::Forward)?;
        editor.select_token(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f()"]);

        let mut context = Context::default();
        editor.copy(&mut context);

        editor.select_token(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["{"]);

        editor.paste(&context);

        assert_eq!(editor.text(), "fn f()fn f() let x = S(a); let y = S(b); }");
        Ok(())
    }

    #[test]
    fn highlight_mode_replace() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        editor.select_token(Direction::Forward)?;
        editor.toggle_highlight_mode();
        editor.select_token(Direction::Forward)?;
        editor.select_token(Direction::Forward)?;
        editor.select_token(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f()"]);

        let mut context = Context::default();
        editor.copy(&mut context);

        editor.select_named_node(Direction::Forward)?;

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
        editor.select_token(Direction::Forward)?;

        let mut context = Context::default();
        editor.copy(&mut context);

        assert_eq!(editor.get_selected_texts(), vec!["fn"]);

        editor.toggle_highlight_mode();
        editor.select_token(Direction::Forward)?;
        editor.select_token(Direction::Forward)?;
        editor.select_token(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f()"]);

        editor.paste(&context);

        assert_eq!(editor.text(), "fn{ let x = S(a); let y = S(b); }");
        Ok(())
    }

    #[test]
    #[ignore]
    fn highlight_mode_exchange_word() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        editor.select_word(Direction::Forward)?;
        editor.toggle_highlight_mode();
        editor.select_word(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f"]);

        editor.exchange(Direction::Forward);

        assert_eq!(editor.text(), "let(){ fn f x = S(a); let y = S(b); }");
        assert_eq!(editor.get_selected_texts(), vec!["fn f"]);

        editor.exchange(Direction::Forward);

        assert_eq!(editor.text(), "let(){ x fn f = S(a); let y = S(b); }");
        Ok(())
    }

    #[test]
    #[ignore]
    fn highlight_mode_exchange_sibling() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(){} fn g(){} fn h(){} fn i(){}");

        // select `fn f(){}`
        editor.select_token(Direction::Forward)?;
        editor.select_parent(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f(){}"]);

        editor.toggle_highlight_mode();
        editor.select_sibling(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn f(){} fn g(){}"]);

        editor.exchange(Direction::Forward);

        assert_eq!(editor.text(), "fn h(){} fn f(){} fn g(){} fn i(){}");

        editor.select_sibling(Direction::Forward)?;

        assert_eq!(
            editor.get_selected_texts(),
            vec!["fn f(){} fn g(){} fn i(){}"]
        );

        editor.exchange(Direction::Forward);

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

        editor.select_character(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["f"]);

        editor.delete(Direction::Forward);
        assert_eq!(editor.text(), "n f(){ let x = S(a); let y = S(b); }");

        editor.delete(Direction::Forward);
        assert_eq!(editor.text(), " f(){ let x = S(a); let y = S(b); }");

        editor.select_match(Direction::Forward, &Some("x".to_string()))?;
        editor.select_character(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["x"]);

        editor.delete(Direction::Backward);
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

        editor.select_line(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["fn f() {\n"]);

        editor.delete(Direction::Forward);
        assert_eq!(
            editor.text(),
            "
let x = S(a);

let y = S(b);
}"
            .trim()
        );

        editor.delete(Direction::Forward);
        assert_eq!(
            editor.text(),
            "
let y = S(b);
}"
        );
        assert_eq!(editor.get_selected_texts(), vec!["\n"]);

        editor.select_line(Direction::Forward)?;
        assert_eq!(editor.get_selected_texts(), vec!["let y = S(b);\n"]);
        editor.delete(Direction::Backward);
        assert_eq!(
            editor.text(),
            "
}"
        );

        editor.delete(Direction::Forward);
        assert_eq!(editor.text(), "}");

        editor.delete(Direction::Forward);
        assert_eq!(editor.text(), "");
        Ok(())
    }

    #[test]
    fn delete_sibling() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(x: a, y: b, z: c){}");
        // Select 'x: a'
        editor.select_named_node(Direction::Forward)?;
        editor.select_named_node(Direction::Forward)?;
        editor.select_named_node(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["x: a"]);

        editor.select_sibling(Direction::Current)?;
        editor.delete(Direction::Forward);

        assert_eq!(editor.text(), "fn f(y: b, z: c){}");

        editor.select_sibling(Direction::Forward)?;
        editor.delete(Direction::Backward);

        assert_eq!(editor.text(), "fn f(y: b){}");
        Ok(())
    }

    #[test]
    fn delete_token() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn f(x: a, y: b, z: c){}");
        // Select 'fn'
        editor.select_token(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn"]);

        editor.delete(Direction::Forward);

        assert_eq!(editor.text(), "f(x: a, y: b, z: c){}");

        editor.delete(Direction::Forward);

        assert_eq!(editor.text(), "(x: a, y: b, z: c){}");

        editor.select_token(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["x"]);

        editor.delete(Direction::Backward);

        assert_eq!(editor.text(), "(: a, y: b, z: c){}");
        Ok(())
    }

    #[test]
    fn paste_from_clipboard() {
        let mut editor = Editor::from_text(language(), "fn f(){ let x = S(a); let y = S(b); }");
        let mut context = Context::default();

        context.set_clipboard_content("let z = S(c);".to_string());

        editor.reset();

        editor.paste(&context);

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

        assert_eq!(editor.selection_set.mode, SelectionMode::NamedNode);

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
        editor.select_named_node(Direction::Forward)?;
        editor.select_named_node(Direction::Forward)?;
        editor.select_named_node(Direction::Forward)?;
        editor.select_named_node(Direction::Forward)?;

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
        editor.select_named_node(Direction::Forward)?;
        editor.select_named_node(Direction::Forward)?;
        editor.select_named_node(Direction::Forward)?;
        editor.select_named_node(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["x.y()"]);

        editor.handle_events(keys!(") } ] >")).unwrap();

        assert_eq!(editor.text(), "fn main() { <[{(x.y())}]> }");
        assert_eq!(editor.get_selected_texts(), vec!["<[{(x.y())}]>"]);
        Ok(())
    }

    #[test]
    fn select_final() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn\nmain()\n{ x.y() }");

        editor.select_line(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn\n"]);

        editor.select_final(Direction::Forward)?;

        assert_eq!(editor.get_selected_texts(), vec!["{ x.y() }"]);

        editor.select_final(Direction::Backward)?;

        assert_eq!(editor.get_selected_texts(), vec!["fn\n"]);
        Ok(())
    }

    #[test]
    fn match_current_selection() -> anyhow::Result<()> {
        let mut editor = Editor::from_text(language(), "fn\nmain()\n{ x.y(); x.y(); x.y(); }");

        // Select x.y()

        for _ in 0..4 {
            editor.select_named_node(Direction::Forward)?;
        }
        editor.select_parent(Direction::Backward)?;

        assert_eq!(editor.get_selected_texts(), vec!["x.y()"]);

        let dispatches = editor.match_current_selection()?;

        assert_eq!(
            dispatches,
            vec![Dispatch::SetSearch {
                search: "x.y()".to_string()
            }]
        );
        assert_eq!(
            editor.selection_set.mode,
            SelectionMode::Match {
                search: "x.y()".to_string()
            }
        );
        Ok(())
    }
}
