use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use itertools::Itertools;
use ropey::Rope;
use tree_sitter::{InputEdit, Node, Parser, Point, Tree};
use tree_sitter_traversal::{traverse, Order};

use crate::{
    edit::{Action, ActionGroup, Edit, EditTransaction},
    selection::{CharIndex, Selection, SelectionMode, SelectionSet},
};

pub enum Mode {
    Normal,
    Insert,
    Jump { jumps: Vec<Jump> },
}

#[derive(Clone)]
pub struct Jump {
    pub character: char,
    pub selection: Selection,
}

pub struct Buffer {
    pub text: Rope,
    pub mode: Mode,

    pub selection_set: SelectionSet,

    pub cursor_direction: CursorDirection,
    pub tree: Tree,
    pub quit: bool,
    selection_history: Vec<SelectionSet>,

    undo_edits: Vec<EditTransaction>,
    redo_edits: Vec<EditTransaction>,

    /// This indicates where the extended selection started
    ///
    /// Some = the selection is being extended
    /// None = the selection is not being extended
    extended_selection_anchor: Option<CharIndex>,
}

pub enum CursorDirection {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
    Current,
}

impl Buffer {
    pub fn new(language: tree_sitter::Language, text: &str) -> Self {
        Self {
            selection_set: SelectionSet {
                primary: Selection {
                    range: CharIndex(0)..CharIndex(0),
                    node_id: None,
                    yanked_text: None,
                },
                secondary: vec![],
                mode: SelectionMode::Custom,
            },
            text: Rope::from_str(text),
            mode: Mode::Normal,
            cursor_direction: CursorDirection::Start,
            tree: {
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();
                parser.parse(text.to_string(), None).unwrap()
            },
            quit: false,
            selection_history: Vec::with_capacity(128),
            undo_edits: Vec::new(),
            redo_edits: Vec::new(),
            extended_selection_anchor: None,
        }
    }

    fn select_parent(&mut self, direction: Direction) {
        self.select(SelectionMode::ParentNode, direction);
    }

    fn select_kids(&mut self) {
        self.update_selection_set(self.selection_set.select_kids(
            &self.text,
            &self.tree,
            &self.cursor_direction,
        ));
    }

    fn select_sibling(&mut self, direction: Direction) {
        self.select(SelectionMode::SiblingNode, direction);
    }

    fn select_line(&mut self, direction: Direction) {
        self.select(SelectionMode::Line, direction);
    }

    fn select_named_node(&mut self, direction: Direction) {
        self.select(SelectionMode::NamedNode, direction);
    }

    fn select_character(&mut self, direction: Direction) {
        self.select(SelectionMode::Character, direction);
    }

    fn select_backward(&mut self) {
        while let Some(selection_set) = self.selection_history.pop() {
            if selection_set != self.selection_set {
                self.selection_set = selection_set;
                break;
            }
        }
    }

    fn reset(&mut self) {
        self.select(SelectionMode::Custom, Direction::Current);
        self.extended_selection_anchor = None;
        self.selection_set.reset()
    }

    fn select_token(&mut self, direction: Direction) {
        self.select(SelectionMode::Token, direction);
    }

    fn update_selection_set(&mut self, selection_set: SelectionSet) {
        self.selection_set = selection_set.clone();
        self.selection_history.push(selection_set);
    }

    fn select(&mut self, selection_mode: SelectionMode, direction: Direction) {
        let direction = if self.selection_set.mode.similar_to(&selection_mode) {
            direction
        } else {
            Direction::Current
        };
        let selection = self.get_selection_set(&selection_mode, direction);

        self.update_selection_set(selection);
    }

    fn jump_from_selection(&mut self, direction: Direction, selection: &Selection) {
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
                &self.text,
                &self.tree,
                &current_selection,
                &self.selection_set.mode,
                &direction,
                &self.cursor_direction,
            );

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
    }

    fn jump(&mut self, direction: Direction) {
        self.jump_from_selection(direction, &self.selection_set.primary.clone());
    }

    fn delete_current_selection(&mut self) {
        self.yank_current_selection();
        let edit_transaction = self.selection_set.replace(
            |selection| {
                self.text
                    .slice(selection.range.start.0..selection.range.end.0)
                    .into()
            },
            |_| Rope::new(),
            |edit| edit.start..edit.start,
        );
        self.apply_edit_transaction(EditHistoryKind::NewEdit, edit_transaction);
    }

    fn yank_current_selection(&mut self) {
        self.selection_set.yank(&self.text);
    }

    fn paste(&mut self) {
        let edit_transactions = self.selection_set.map(|selection| {
            if let Some(yanked_text) = &selection.yanked_text {
                let start = selection.to_char_index(&self.cursor_direction);
                EditTransaction::from_action_groups(
                    self.selection_set.clone(),
                    vec![ActionGroup::new(vec![
                        Action::Edit(Edit {
                            start,
                            old: Rope::new(),
                            new: yanked_text.clone(),
                        }),
                        Action::Select(Selection {
                            range: start..(start + yanked_text.len_chars()),
                            node_id: None,
                            yanked_text: Some(yanked_text.clone()),
                        }),
                    ])],
                )
            } else {
                EditTransaction::from_action_groups(self.selection_set.clone(), vec![])
            }
        });
        let edit_transaction =
            EditTransaction::merge(self.selection_set.clone(), edit_transactions);
        self.apply_edit_transaction(EditHistoryKind::NewEdit, edit_transaction);
    }

    fn replace(&mut self) {
        let edit_transaction = EditTransaction::merge(
            self.selection_set.clone(),
            self.selection_set.map(|selection| {
                if let Some(replacement) = &selection.yanked_text {
                    let replacement_text_len = replacement.len_chars();
                    let replaced_text = self
                        .text
                        .slice(selection.range.start.0..selection.range.end.0)
                        .into();
                    EditTransaction::from_action_groups(
                        self.selection_set.clone(),
                        vec![ActionGroup::new(vec![
                            Action::Edit(Edit {
                                start: selection.range.start,
                                old: self
                                    .text
                                    .slice(selection.range.start.0..selection.range.end.0)
                                    .into(),
                                new: replacement.clone(),
                            }),
                            Action::Select(Selection {
                                range: selection.range.start
                                    ..selection.range.start + replacement_text_len,
                                yanked_text: Some(replaced_text),
                                node_id: None,
                            }),
                        ])],
                    )
                } else {
                    EditTransaction::from_action_groups(self.selection_set.clone(), vec![])
                }
            }),
        );
        self.apply_edit_transaction(EditHistoryKind::NewEdit, edit_transaction);
    }

    fn apply_edit_transaction(
        &mut self,
        edit_history_kind: EditHistoryKind,
        edit_transaction: EditTransaction,
    ) {
        let inversed_edit_transaction = EditTransaction::from_action_groups(
            self.selection_set.clone(),
            edit_transaction
                .edits()
                .iter()
                .map(|edit| {
                    ActionGroup::new(vec![Action::Edit(Edit {
                        start: edit.start,
                        old: edit.new.clone(),
                        new: edit.old.clone(),
                    })])
                })
                .collect_vec(),
        );

        match edit_history_kind {
            EditHistoryKind::NewEdit => {
                self.redo_edits.clear();
                self.undo_edits.push(inversed_edit_transaction);
            }
            EditHistoryKind::Undo => {
                self.redo_edits.push(inversed_edit_transaction);
            }
            EditHistoryKind::Redo => {
                self.undo_edits.push(inversed_edit_transaction);
            }
        }

        if let Some((head, tail)) = edit_transaction.selections().split_first() {
            self.selection_set = SelectionSet {
                primary: (*head).clone(),
                secondary: tail
                    .into_iter()
                    .map(|selection| (*selection).clone())
                    .collect(),
                mode: self.selection_set.mode,
            }
        }

        (self.tree, self.text) =
            apply_edit_transaction(self.tree.clone(), self.text.clone(), edit_transaction).unwrap();
    }

    fn undo(&mut self) {
        if let Some(edit) = self.undo_edits.pop() {
            self.revert_change(edit, EditHistoryKind::Undo);
        } else {
            log::info!("Nothing else to be undone")
        }
    }

    fn redo(&mut self) {
        if let Some(edit) = self.redo_edits.pop() {
            self.revert_change(edit, EditHistoryKind::Redo);
        } else {
            log::info!("Nothing else to be redone")
        }
    }

    fn revert_change(
        &mut self,
        edit_transaction: EditTransaction,
        edit_history_kind: EditHistoryKind,
    ) {
        let selection = edit_transaction.selection_set.clone();
        self.apply_edit_transaction(edit_history_kind, edit_transaction);
        self.update_selection_set(selection)
    }

    fn change_cursor_direction(&mut self) {
        self.cursor_direction = match self.cursor_direction {
            CursorDirection::Start => CursorDirection::End,
            CursorDirection::End => CursorDirection::Start,
        };
    }

    fn get_selection_set(&self, mode: &SelectionMode, direction: Direction) -> SelectionSet {
        self.selection_set.generate(
            &self.text,
            &self.tree,
            mode,
            &direction,
            &self.cursor_direction,
        )
    }

    pub fn get_cursor_point(&self) -> Point {
        self.get_cursor_char_index().to_point(&self.text)
    }

    fn get_cursor_char_index(&self) -> CharIndex {
        self.selection_set
            .primary
            .to_char_index(&self.cursor_direction)
    }

    fn toggle_highlight_mode(&mut self) {
        todo!()
        // if let Some(anchor) = self.extended_selection_anchor.take() {
        //     // Reverse the anchor with the current cursor position
        //     let cursor_index = self.get_cursor_char_index();
        //     self.extended_selection_anchor = Some(cursor_index);
        //     self.selection_set = Selection {
        //         range: anchor..anchor,
        //         node_id: None,
        //         mode: SelectionMode::Custom,
        //     };
        //     self.cursor_direction = if cursor_index > anchor {
        //         CursorDirection::Start
        //     } else {
        //         CursorDirection::End
        //     };
        // } else {
        //     self.extended_selection_anchor = Some(self.get_cursor_char_index());
        //     self.cursor_direction = CursorDirection::End;
        // }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        if let HandleKeyEventResult::Unconsumed(key_event) = self.handle_universal_key(key_event) {
            match &self.mode {
                Mode::Normal => self.handle_normal_mode(key_event),
                Mode::Insert => self.handle_insert_mode(key_event),
                Mode::Jump { .. } => self.handle_jump_mode(key_event),
            }
        }
    }

    fn handle_universal_key(&mut self, event: KeyEvent) -> HandleKeyEventResult {
        match event.code {
            KeyCode::Left => {
                self.selection_set.move_left(&self.cursor_direction);
                HandleKeyEventResult::Consumed
            }
            KeyCode::Right => {
                self.selection_set.move_right(&self.cursor_direction);
                HandleKeyEventResult::Consumed
            }
            KeyCode::Char('a') if event.modifiers == KeyModifiers::CONTROL => {
                self.selection_set = SelectionSet {
                    primary: Selection {
                        range: CharIndex(0)..CharIndex(self.text.len_chars()),
                        node_id: None,
                        yanked_text: self.selection_set.primary.yanked_text.clone(),
                    },
                    secondary: vec![],
                    mode: SelectionMode::Custom,
                };
                HandleKeyEventResult::Consumed
            }
            KeyCode::Char('q') if event.modifiers == KeyModifiers::CONTROL => {
                self.quit = true;
                HandleKeyEventResult::Consumed
            }
            KeyCode::Char('v') if event.modifiers == KeyModifiers::CONTROL => {
                self.paste();
                HandleKeyEventResult::Consumed
            }
            KeyCode::Char('y') if event.modifiers == KeyModifiers::CONTROL => {
                self.redo();
                HandleKeyEventResult::Consumed
            }
            KeyCode::Char('z') if event.modifiers == KeyModifiers::CONTROL => {
                self.undo();
                HandleKeyEventResult::Consumed
            }
            // Others include:
            // - ^t for new tab
            // - ^s for saving
            // - ^z for undo
            // - ^y for redo
            // - ^f for find
            // - ^a for select all
            // - ^q for closing current window
            _ => HandleKeyEventResult::Unconsumed(event),
        }
    }

    fn handle_jump_mode(&mut self, key_event: KeyEvent) {
        match self.mode {
            Mode::Jump { ref jumps, .. } => match key_event.code {
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                }
                KeyCode::Char('j') => {
                    if let Some(jump) = jumps
                        .iter()
                        .max_by(|a, b| a.selection.range.start.cmp(&b.selection.range.start))
                    {
                        self.jump_from_selection(Direction::Forward, &jump.selection.clone());
                    }
                }
                KeyCode::Char('J') => {
                    if let Some(jump) = jumps
                        .iter()
                        .min_by(|a, b| a.selection.range.end.cmp(&b.selection.range.end))
                    {
                        self.jump_from_selection(Direction::Backward, &jump.selection.clone());
                    }
                }
                KeyCode::Char(c) => {
                    let matching_jump = jumps.iter().find(|jump| c == jump.character);
                    if let Some(jump) = matching_jump {
                        self.update_selection_set(SelectionSet {
                            primary: Selection {
                                yanked_text: self.selection_set.primary.yanked_text.clone(),
                                ..jump.selection.clone()
                            },
                            secondary: vec![],
                            mode: self.selection_set.mode,
                        });
                        self.mode = Mode::Normal;
                    }
                }
                _ => {}
            },
            _ => unreachable!(),
        }
    }

    fn insert(&mut self, s: &str) {
        let edit_transaction = self.selection_set.replace(
            |_| Rope::new(),
            |_| Rope::from_str(s),
            |edit| edit.start + 1..edit.start + 1,
        );
        self.apply_edit_transaction(EditHistoryKind::NewEdit, edit_transaction);
    }

    fn handle_insert_mode(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Esc => self.enter_normal_mode(),
            KeyCode::Backspace => {
                self.select(SelectionMode::Character, Direction::Current);
                self.select(SelectionMode::Character, Direction::Backward);
                let edit_transaction = self.selection_set.replace(
                    |edit| self.text.slice(edit.range.start.0..edit.range.end.0).into(),
                    |_| Rope::new(),
                    |edit| edit.start..edit.start,
                );
                self.apply_edit_transaction(EditHistoryKind::NewEdit, edit_transaction);
            }
            KeyCode::Enter => self.insert("\n"),
            KeyCode::Char(c) => self.insert(&c.to_string()),
            KeyCode::Tab => self.insert("\t"),
            _ => {}
        }
    }

    fn handle_normal_mode(&mut self, event: KeyEvent) {
        match event.code {
            // Objects
            KeyCode::Char('a') => self.add_selection(),
            KeyCode::Char('A') => self.add_selection(),
            KeyCode::Char('b') => self.select_backward(),
            KeyCode::Char('c') => self.select_character(Direction::Forward),
            KeyCode::Char('d') => self.delete_current_selection(),
            KeyCode::Char('f') => self.move_selection(Direction::Forward),
            KeyCode::Char('F') => self.move_selection(Direction::Backward),
            KeyCode::Char('h') => self.toggle_highlight_mode(),
            KeyCode::Char('i') => self.enter_insert_mode(),
            KeyCode::Char('j') => self.jump(Direction::Forward),
            KeyCode::Char('J') => self.jump(Direction::Backward),
            KeyCode::Char('k') => self.select_kids(),
            KeyCode::Char('l') => self.select_line(Direction::Forward),
            KeyCode::Char('n') => self.select_named_node(Direction::Forward),
            KeyCode::Char('o') => self.change_cursor_direction(),
            KeyCode::Char('s') => self.select_sibling(Direction::Forward),
            KeyCode::Char('t') => self.select_token(Direction::Forward),
            KeyCode::Char('r') => self.replace(),
            KeyCode::Char('p') => self.select_parent(Direction::Forward),
            KeyCode::Char('x') => self.exchange(Direction::Forward),
            KeyCode::Char('X') => self.exchange(Direction::Backward),
            KeyCode::Char('y') => self.yank_current_selection(),
            KeyCode::Char('0') => self.reset(),
            KeyCode::Esc => {
                self.extended_selection_anchor = None;
            }
            // Similar to Change in Vim
            KeyCode::Backspace => {
                self.yank_current_selection();
                let edit_transaction = EditTransaction::merge(
                    self.selection_set.clone(),
                    self.selection_set.map(|selection| {
                        EditTransaction::from_action_groups(
                            self.selection_set.clone(),
                            vec![ActionGroup::new(vec![
                                Action::Edit(Edit {
                                    start: selection.range.start,
                                    old: self
                                        .text
                                        .slice(selection.range.start.0..selection.range.end.0)
                                        .into(),
                                    new: Rope::new(),
                                }),
                                Action::Select(Selection {
                                    range: selection.range.start..selection.range.start,
                                    ..selection.clone()
                                }),
                            ])],
                        )
                    }),
                );
                self.apply_edit_transaction(EditHistoryKind::NewEdit, edit_transaction);
                self.enter_insert_mode()
            }
            _ => {
                log::info!("event: {:?}", event);
                // todo!("Back to previous selection");
                // todo!("Search by node kind")
            }
        }
    }

    fn enter_insert_mode(&mut self) {
        self.selection_set.apply_mut(|selection| {
            let char_index = selection.to_char_index(&self.cursor_direction);
            selection.range = char_index..char_index
        });
        self.selection_set.mode = SelectionMode::Custom;
        self.extended_selection_anchor = None;
        self.mode = Mode::Insert;
        self.cursor_direction = CursorDirection::Start;
    }

    fn enter_normal_mode(&mut self) {
        self.mode = Mode::Normal;
        self.select(SelectionMode::Custom, Direction::Current);
    }

    pub fn jumps(&self) -> Vec<&Jump> {
        match self.mode {
            Mode::Jump { ref jumps } => jumps.iter().collect(),
            _ => vec![],
        }
    }

    pub fn get_extended_selection(&self) -> Option<Selection> {
        self.extended_selection_anchor
            .map(|anchor| Selection::from_two_char_indices(&anchor, &self.get_cursor_char_index()))
    }

    pub fn set_cursor_position(&mut self, row: u16, column: u16) {
        let start = CharIndex(self.text.line_to_char(row as usize)) + column.into();
        self.update_selection_set(SelectionSet {
            mode: self.selection_set.mode,
            primary: Selection {
                range: start..start,
                node_id: None,
                yanked_text: self.selection_set.primary.yanked_text.clone(),
            },
            ..self.selection_set.clone()
        })
    }

    /// Replace the next selection with the current selection without
    /// making the syntax tree invalid
    fn replace_faultlessly(&mut self, selection_mode: &SelectionMode, direction: Direction) {
        fn get_next_selection(
            current_selection_set: &SelectionSet,
            selection: &Selection,
            rope: &Rope,
            tree: &Tree,
            selection_mode: &SelectionMode,
            direction: &Direction,
            cursor_direction: &CursorDirection,
        ) -> EditTransaction {
            let current_selection = selection.clone();

            let text_at_current_selection: Rope = rope
                .slice(current_selection.range.start.0..current_selection.range.end.0)
                .into();

            // Loop until the replacement does not result in errorneous node
            let mut next_selection = Selection::get_selection_(
                rope,
                tree,
                &current_selection,
                selection_mode,
                &direction,
                &cursor_direction,
            );

            loop {
                let edit_transaction = EditTransaction::from_action_groups(
                    current_selection_set.clone(),
                    vec![
                        ActionGroup::new(vec![Action::Edit(Edit {
                            start: current_selection.range.start,
                            old: text_at_current_selection.clone(),
                            new: Rope::from_str(
                                &rope
                                    .slice(next_selection.range.start.0..next_selection.range.end.0)
                                    .to_string(),
                            ),
                        })]),
                        ActionGroup::new(vec![Action::Edit(Edit {
                            start: next_selection.range.start,
                            old: Rope::from_str(
                                &rope
                                    .slice(next_selection.range.start.0..next_selection.range.end.0)
                                    .to_string(),
                            ),
                            // We need to add whitespace on both end of the replacement
                            //
                            // Otherwise we might get the following replacement in Rust:
                            // Assuming the selection is on `baz`, and the selection mode is `ParentNode`.
                            //
                            // Before:                              foo.bar(baz)
                            // Result (with whitespace padding):    baz
                            // Result (without padding):            foo.barbaz
                            new: Rope::from_str(
                                &(" ".to_string()
                                    + &text_at_current_selection.to_string()
                                    + &" ".to_string()),
                            ),
                        })]),
                    ],
                );

                let (new_tree, new_rope) =
                    apply_edit_transaction(tree.clone(), rope.clone(), edit_transaction.clone())
                        .unwrap();

                if let Some(node) = new_tree.root_node().descendant_for_byte_range(
                    new_rope
                        .try_char_to_byte(edit_transaction.min_char_index().0)
                        .unwrap_or(0),
                    new_rope
                        .try_char_to_byte(edit_transaction.max_char_index().0)
                        .unwrap_or(0),
                ) {
                    let text_at_next_selection: Rope = rope
                        .slice(next_selection.range.start.0..next_selection.range.end.0)
                        .into();

                    // Why don't we just use `tree.root_node().has_error()` instead?
                    // Because I assume we want to be able to exchange even if some part of the tree
                    // contains error
                    if !text_at_next_selection.to_string().trim().is_empty()
                        && (!selection_mode.is_node() || !node.has_error())
                    {
                        // Log text_at_current_selection
                        log::info!(
                            "\nReplaced\n:'{}'\nwith:\n'{}'",
                            text_at_next_selection,
                            text_at_current_selection
                        );
                        let edit_transaction = EditTransaction::from_action_groups(
                            current_selection_set.clone(),
                            vec![
                                ActionGroup::new(vec![Action::Edit(Edit {
                                    start: current_selection.range.start,
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
                                            ..CharIndex(
                                                next_selection.range.start.0
                                                    + text_at_current_selection.len_chars(),
                                            ),
                                        node_id: None,
                                        yanked_text: current_selection.yanked_text,
                                    }),
                                ]),
                            ],
                        );

                        return edit_transaction;
                    }
                }

                // Get the next selection

                let new_selection = Selection::get_selection_(
                    &rope,
                    &tree,
                    &next_selection,
                    selection_mode,
                    &direction,
                    &cursor_direction,
                );

                if next_selection.eq(&new_selection) {
                    return EditTransaction::from_action_groups(
                        current_selection_set.clone(),
                        vec![],
                    );
                }

                next_selection = new_selection;
            }
        }

        let edit_transactions = self.selection_set.map(|selection| {
            get_next_selection(
                &self.selection_set,
                &selection,
                &self.text,
                &self.tree,
                &selection_mode,
                &direction,
                &self.cursor_direction,
            )
        });

        // TODO: merge should take a vector of ActionGroups
        // Because each edit in the same ActionGroups should not offset each other
        // This is crucial, because each set of edits from `get_next_selection` is already normalized
        self.apply_edit_transaction(
            EditHistoryKind::NewEdit,
            EditTransaction::merge(self.selection_set.clone(), edit_transactions),
        )
    }

    fn exchange(&mut self, direction: Direction) {
        self.replace_faultlessly(&self.selection_set.mode.clone(), direction)
    }

    fn move_selection(&mut self, direction: Direction) {
        let selection = self.get_selection_set(&self.selection_set.mode, direction);

        self.update_selection_set(selection);
    }

    fn add_selection(&mut self) {
        self.selection_set
            .add_selection(&self.text, &self.tree, &self.cursor_direction)
    }

    #[cfg(test)]
    fn get_selected_texts(&self) -> Vec<&str> {
        self.selection_set.map(|selection| {
            self.text
                .slice(selection.range.start.0..selection.range.end.0)
                .as_str()
                .unwrap()
        })
    }

    #[cfg(test)]
    fn get_text(&self) -> &str {
        self.text.slice(0..self.text.len_chars()).as_str().unwrap()
    }
}

pub fn get_prev_token(tree: &Tree, byte: usize, is_named: bool) -> Option<Node> {
    ReverseTreeCursor::new(tree.root_node()).find(|&node| {
        node.child_count() == 0 && (!is_named || node.is_named()) && node.start_byte() < byte
    })
}

pub fn get_next_token(tree: &Tree, byte: usize, is_named: bool) -> Option<Node> {
    traverse(tree.root_node().walk(), Order::Post).find(|&node| {
        node.child_count() == 0 && (!is_named || node.is_named()) && node.end_byte() > byte
    })
}

pub fn get_nearest_node_after_byte(tree: &Tree, byte: usize) -> Option<Node> {
    // Preorder is the main key here,
    // because preorder traversal walks the parent first
    traverse(tree.root_node().walk(), Order::Pre).find(|&node| node.start_byte() >= byte)
}

fn get_node_by_id(tree: &Tree, node_id: usize) -> Option<Node> {
    let result = traverse(tree.walk(), Order::Pre).find(|node| node.id() == node_id);
    result
}

pub fn node_to_selection(
    node: Node,
    mode: SelectionMode,
    text: &Rope,
    yanked_text: Option<Rope>,
) -> Selection {
    Selection {
        range: CharIndex(text.byte_to_char(node.start_byte()))
            ..CharIndex(text.byte_to_char(node.end_byte())),
        node_id: Some(node.id()),
        yanked_text,
    }
}

pub struct ReverseTreeCursor<'a> {
    node: Node<'a>,
}

struct ParentTreeCursor<'a> {
    node: Node<'a>,
}

impl<'a> ReverseTreeCursor<'a> {
    pub fn new(node: Node<'a>) -> Self {
        Self {
            node: go_to_last_descendant(node),
        }
    }
}

fn go_to_last_descendant(node: Node) -> Node {
    let mut node = node;
    loop {
        if let Some(sibling) = node.next_sibling() {
            node = sibling
        } else if let Some(child) = node.child(node.child_count().saturating_sub(1)) {
            node = child;
        } else {
            return node;
        }
    }
}

impl<'a> Iterator for ReverseTreeCursor<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self
            .node
            .prev_sibling()
            .map(|node| node.child(0).map(go_to_last_descendant).unwrap_or(node))
            .or_else(|| self.node.parent());
        if let Some(next) = next {
            self.node = next;
            Some(self.node)
        } else {
            None
        }
    }
}

impl<'a> Iterator for ParentTreeCursor<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.node.parent();
        if let Some(next) = next {
            self.node = next;
            Some(self.node)
        } else {
            None
        }
    }
}

pub fn get_current_node<'a>(tree: &'a Tree, cursor_byte: usize, selection: &Selection) -> Node<'a> {
    if let Some(node_id) = selection.node_id {
        get_node_by_id(tree, node_id)
    } else {
        get_nearest_node_after_byte(tree, cursor_byte)
    }
    .unwrap_or_else(|| tree.root_node())
}

fn apply_edit_transaction(
    tree: Tree,
    text: Rope,
    edit_transaction: EditTransaction,
) -> Result<(Tree, Rope), anyhow::Error> {
    edit_transaction
        .edits()
        .into_iter()
        .fold(Ok((tree, text)), |result, edit| match result {
            Err(err) => Err(err),
            Ok((tree, text)) => apply_edit(tree, text, edit),
        })
}

fn apply_edit(mut tree: Tree, mut text: Rope, edit: &Edit) -> Result<(Tree, Rope), anyhow::Error> {
    let start_char_index = edit.start;
    let old_end_char_index = edit.end();
    let new_end_char_index = edit.start + edit.new.len_chars();

    let start_byte = start_char_index.to_byte(&text);
    let old_end_byte = old_end_char_index.to_byte(&text);
    let start_position = start_char_index.to_point(&text);
    let old_end_position = old_end_char_index.to_point(&text);

    text.try_remove(edit.start.0..edit.end().0)?;
    text.try_insert(edit.start.0, edit.new.to_string().as_str())?;

    let new_end_byte = new_end_char_index.to_byte(&text);
    let new_end_position = new_end_char_index.to_point(&text);

    let mut parser = tree_sitter::Parser::new();
    parser.set_language(tree.language()).unwrap();
    tree.edit(&InputEdit {
        start_byte,
        old_end_byte,
        new_end_byte,
        start_position,
        old_end_position,
        new_end_position,
    });
    let tree = parser.parse(&text.to_string(), Some(&tree)).unwrap();
    Ok((tree, text))
}

enum HandleKeyEventResult {
    Consumed,
    Unconsumed(KeyEvent),
}

enum EditHistoryKind {
    Undo,
    Redo,
    NewEdit,
}

#[cfg(test)]
mod test_engine {
    use super::{Buffer, Direction};
    use tree_sitter_rust::language;

    #[test]
    fn select_character() {
        let mut buffer = Buffer::new(language(), "fn main() { let x = 1; }");
        buffer.select_character(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["f"]);
        buffer.select_character(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["n"]);

        buffer.select_character(Direction::Backward);
        assert_eq!(buffer.get_selected_texts(), vec!["f"]);
    }

    #[test]
    fn select_line() {
        // Multiline source code
        let mut buffer = Buffer::new(
            language(),
            "
fn main() {
    let x = 1;
}
"
            .trim(),
        );
        buffer.select_line(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["fn main() {\n"]);
        buffer.select_line(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["    let x = 1;\n"]);

        buffer.select_line(Direction::Backward);
        assert_eq!(buffer.get_selected_texts(), vec!["fn main() {\n"]);
    }

    #[test]
    fn select_token() {
        let mut buffer = Buffer::new(language(), "fn main() { let x = 1; }");
        buffer.select_token(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["fn"]);
        buffer.select_token(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["main"]);
        buffer.select_token(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["("]);
        buffer.select_token(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec![")"]);
        buffer.select_token(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["{"]);
        buffer.select_token(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["let"]);
        buffer.select_token(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["x"]);

        buffer.select_token(Direction::Backward);
        assert_eq!(buffer.get_selected_texts(), vec!["let"]);
    }

    #[test]
    fn select_parent() {
        let mut buffer = Buffer::new(language(), "fn main() { let x = 1; }");
        // Move token to 1
        for _ in 0..9 {
            buffer.select_token(Direction::Forward);
        }

        assert_eq!(buffer.get_selected_texts(), vec!["1"]);

        buffer.select_parent(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["1"]);
        buffer.select_parent(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["let x = 1;"]);
        buffer.select_parent(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["{ let x = 1; }"]);
        buffer.select_parent(Direction::Forward);
        assert_eq!(
            buffer.get_selected_texts(),
            vec!["fn main() { let x = 1; }"]
        );

        buffer.select_parent(Direction::Backward);
        assert_eq!(buffer.get_selected_texts(), vec!["main"]);
    }

    #[test]
    fn select_sibling() {
        let mut buffer = Buffer::new(language(), "fn main(x: usize, y: Vec<A>) {}");
        // Move token to "x: usize"
        for _ in 0..4 {
            buffer.select_token(Direction::Forward);
        }
        buffer.select_parent(Direction::Forward);
        buffer.select_parent(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["x: usize"]);

        buffer.select_sibling(Direction::Forward);
        buffer.select_sibling(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["y: Vec<A>"]);
        buffer.select_sibling(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["y: Vec<A>"]);

        buffer.select_sibling(Direction::Backward);
        assert_eq!(buffer.get_selected_texts(), vec!["x: usize"]);
        buffer.select_sibling(Direction::Backward);
        assert_eq!(buffer.get_selected_texts(), vec!["x: usize"]);
    }

    #[test]
    fn select_kids() {
        let mut buffer = Buffer::new(language(), "fn main(x: usize, y: Vec<A>) {}");
        // Move token to "x"
        for _ in 0..4 {
            buffer.select_token(Direction::Forward);
        }
        assert_eq!(buffer.get_selected_texts(), vec!["x"]);

        buffer.select_kids();
        assert_eq!(buffer.get_selected_texts(), vec!["x: usize, y: Vec<A>"]);
    }

    #[test]
    fn select_named_node() {
        let mut buffer = Buffer::new(language(), "fn main(x: usize) { let x = 1; }");

        buffer.select_named_node(Direction::Forward);
        assert_eq!(
            buffer.get_selected_texts(),
            vec!["fn main(x: usize) { let x = 1; }"]
        );
        buffer.select_named_node(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["main"]);
        buffer.select_named_node(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["(x: usize)"]);
        buffer.select_named_node(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["x: usize"]);
        buffer.select_named_node(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["usize"]);
        buffer.select_named_node(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["{ let x = 1; }"]);
        buffer.select_named_node(Direction::Forward);
        assert_eq!(buffer.get_selected_texts(), vec!["let x = 1;"]);

        buffer.select_named_node(Direction::Backward);
        assert_eq!(buffer.get_selected_texts(), vec!["{ let x = 1; }"]);
        buffer.select_named_node(Direction::Backward);
        assert_eq!(buffer.get_selected_texts(), vec!["usize"]);
        buffer.select_named_node(Direction::Backward);
        assert_eq!(buffer.get_selected_texts(), vec!["x: usize"]);
        buffer.select_named_node(Direction::Backward);
        assert_eq!(buffer.get_selected_texts(), vec!["(x: usize)"]);
        buffer.select_named_node(Direction::Backward);
        assert_eq!(buffer.get_selected_texts(), vec!["main"]);
    }

    #[test]
    fn yank_replace() {
        let mut buffer = Buffer::new(language(), "fn main() { let x = 1; }");
        buffer.select_token(Direction::Forward);
        buffer.yank_current_selection();
        buffer.select_token(Direction::Forward);
        buffer.replace();
        assert_eq!(buffer.get_text(), "fn fn() { let x = 1; }");
        assert_eq!(buffer.get_selected_texts(), vec!["fn"]);
        buffer.replace();
        assert_eq!(buffer.get_text(), "fn main() { let x = 1; }");
        assert_eq!(buffer.get_selected_texts(), vec!["main"]);
    }

    #[test]
    fn yank_paste() {
        let mut buffer = Buffer::new(language(), "fn main() { let x = 1; }");
        buffer.select_token(Direction::Forward);
        buffer.yank_current_selection();
        buffer.select_token(Direction::Forward);
        buffer.paste();
        assert_eq!(buffer.get_text(), "fn fnmain() { let x = 1; }");
        assert_eq!(buffer.get_selected_texts(), vec!["fn"]);
    }

    #[test]
    fn exchange_sibling() {
        let mut buffer = Buffer::new(language(), "fn main(x: usize, y: Vec<A>) {}");
        // Move token to "x: usize"
        for _ in 0..4 {
            buffer.select_token(Direction::Forward);
        }
        buffer.select_parent(Direction::Forward);
        buffer.select_parent(Direction::Forward);

        buffer.select_sibling(Direction::Forward);
        buffer.exchange(Direction::Forward);
        assert_eq!(buffer.get_text(), "fn main(y: Vec<A>, x: usize) {}");

        buffer.exchange(Direction::Backward);
        assert_eq!(buffer.get_text(), "fn main(x: usize, y: Vec<A>) {}");
    }

    #[test]
    fn exchange_parent() {
        let mut buffer = Buffer::new(language(), "fn main() { let x = a.b(c()); }");
        // Move selection to "c()"
        for _ in 0..10 {
            buffer.select_named_node(Direction::Forward);
        }

        assert_eq!(buffer.get_selected_texts(), vec!["c()"]);

        buffer.select_parent(Direction::Forward);

        buffer.exchange(Direction::Forward);
        assert_eq!(buffer.get_text(), "fn main() { let x = c(); }");

        buffer.exchange(Direction::Forward);
        assert_eq!(buffer.get_text(), "fn main() { c() }");
    }

    #[test]
    fn exchange_line() {
        // Multiline source code
        let mut buffer = Buffer::new(
            language(),
            "
fn main() {
    let x = 1;
    let y = 2;
}",
        );

        buffer.select_line(Direction::Forward);
        buffer.select_line(Direction::Forward);

        buffer.exchange(Direction::Forward);
        assert_eq!(
            buffer.get_text(),
            "
    let x = 1;
fn main() {
    let y = 2;
}"
        );

        buffer.exchange(Direction::Backward);
        assert_eq!(
            buffer.get_text(),
            "
fn main() {
    let x = 1;
    let y = 2;
}"
        );
    }

    #[test]
    fn exchange_character() {
        let mut buffer = Buffer::new(language(), "fn main() { let x = 1; }");
        buffer.select_character(Direction::Forward);

        buffer.exchange(Direction::Forward);
        assert_eq!(buffer.get_text(), "nf main() { let x = 1; }");
        buffer.exchange(Direction::Forward);
        assert_eq!(buffer.get_text(), "nm fain() { let x = 1; }");

        buffer.exchange(Direction::Backward);
        assert_eq!(buffer.get_text(), "nf main() { let x = 1; }");
        buffer.exchange(Direction::Backward);
        assert_eq!(buffer.get_text(), "fn main() { let x = 1; }");
    }

    #[test]
    fn multi_insert() {
        let mut buffer = Buffer::new(language(), "struct A(usize, char)");
        // Select 'usize'
        for _ in 0..4 {
            buffer.select_named_node(Direction::Forward);
        }

        assert_eq!(buffer.get_selected_texts(), vec!["usize"]);

        buffer.select_sibling(Direction::Forward);
        buffer.add_selection();
        assert_eq!(buffer.get_selected_texts(), vec!["usize", "char"]);
        buffer.enter_insert_mode();
        buffer.insert("pub ");

        assert_eq!(buffer.get_text(), "struct A(pub usize, pub char)");
    }

    #[test]
    fn multi_exchange_parent() {
        let mut buffer = Buffer::new(language(), "fn f(){ let x = S(a); let y = S(b); }");
        // Select 'let x = S(a)'
        for _ in 0..5 {
            buffer.select_named_node(Direction::Forward);
        }

        assert_eq!(buffer.get_selected_texts(), vec!["let x = S(a);"]);

        buffer.select_sibling(Direction::Forward);
        buffer.add_selection();

        assert_eq!(
            buffer.get_selected_texts(),
            vec!["let x = S(a);", "let y = S(b);"]
        );

        for _ in 0..5 {
            buffer.select_named_node(Direction::Forward);
        }

        assert_eq!(buffer.get_selected_texts(), vec!["a", "b"]);

        buffer.select_parent(Direction::Forward);
        buffer.exchange(Direction::Forward);

        assert_eq!(buffer.get_text(), "fn f(){ let x = a; let y = b; }");

        buffer.undo();

        assert_eq!(buffer.get_text(), "fn f(){ let x = S(a); let y = S(b); }");
        assert_eq!(buffer.get_selected_texts(), vec!["a", "b"]);

        buffer.redo();

        assert_eq!(buffer.get_text(), "fn f(){ let x = a; let y = b; }");
        assert_eq!(buffer.get_selected_texts(), vec!["a", "b"]);
    }

    #[test]
    fn multi_exchange_sibling() {
        let mut buffer = Buffer::new(language(), "fn f(x:a,y:b){} fn g(x:a,y:b){}");
        // Select 'fn f(x:a,y:b){}'
        buffer.select_token(Direction::Forward);
        buffer.select_parent(Direction::Forward);
        buffer.select_parent(Direction::Forward);

        assert_eq!(buffer.get_selected_texts(), vec!["fn f(x:a,y:b){}"]);

        buffer.select_sibling(Direction::Forward);
        buffer.add_selection();

        assert_eq!(
            buffer.get_selected_texts(),
            vec!["fn f(x:a,y:b){}", "fn g(x:a,y:b){}"]
        );

        buffer.select_named_node(Direction::Forward);
        buffer.select_named_node(Direction::Forward);
        buffer.select_named_node(Direction::Forward);
        buffer.select_named_node(Direction::Forward);

        assert_eq!(buffer.get_selected_texts(), vec!["x:a", "x:a"]);

        buffer.select_sibling(Direction::Forward);

        buffer.exchange(Direction::Forward);
        assert_eq!(buffer.get_text(), "fn f(y:b,x:a){} fn g(y:b,x:a){}");
        assert_eq!(buffer.get_selected_texts(), vec!["x:a", "x:a"]);

        buffer.exchange(Direction::Backward);
        assert_eq!(buffer.get_text(), "fn f(x:a,y:b){} fn g(x:a,y:b){}");
    }

    #[test]
    fn multi_paste() {
        todo!()
    }
}
