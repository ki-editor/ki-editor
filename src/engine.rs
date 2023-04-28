use std::ops::{Add, Range, Sub};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use itertools::Itertools;
use ropey::Rope;
use tree_sitter::{InputEdit, Node, Point, Tree};
use tree_sitter_traversal::{traverse, Order};

use crate::edit::{Edit, EditTransaction};

#[derive(PartialEq, Clone, Debug)]
pub struct Selection {
    pub mode: SelectionMode,
    pub start: CharIndex,
    pub end: CharIndex,
    pub node_id: Option<usize>,
}
impl Selection {
    fn to_char_index(&self, cursor_direction: &CursorDirection) -> CharIndex {
        // TODO(bug): when SelectionMode is Line and CursorDirection is End,
        // the cursor will be one line below the current selected line
        match cursor_direction {
            CursorDirection::Start => self.start,
            CursorDirection::End => self.end,
        }
    }

    fn from_two_char_indices(anchor: &CharIndex, get_cursor_char_index: &CharIndex) -> Selection {
        Selection {
            mode: SelectionMode::Custom,
            start: *anchor.min(get_cursor_char_index),
            end: *anchor.max(get_cursor_char_index),
            node_id: None,
        }
    }

    fn len(&self) -> usize {
        self.end.0.saturating_sub(self.start.0)
    }

    fn apply_offset(&self, change: isize) -> Selection {
        Selection {
            mode: self.mode,
            start: self.start.apply_offset(change),
            end: self.end.apply_offset(change),
            node_id: self.node_id,
        }
    }
}

impl Add<usize> for Selection {
    type Output = Selection;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            mode: self.mode,
            start: self.start + rhs,
            end: self.end + rhs,
            node_id: self.node_id,
        }
    }
}

impl Sub<usize> for Selection {
    type Output = Selection;

    fn sub(self, rhs: usize) -> Self::Output {
        Self {
            mode: self.mode,
            start: self.start - rhs,
            end: self.end - rhs,
            node_id: self.node_id,
        }
    }
}

impl Add<usize> for CharIndex {
    type Output = CharIndex;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0.saturating_add(rhs))
    }
}

impl Sub<usize> for CharIndex {
    type Output = CharIndex;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0.saturating_sub(rhs))
    }
}

#[derive(PartialEq, Clone, Debug, Copy, PartialOrd, Eq, Ord, Hash)]
pub struct CharIndex(pub usize);

impl CharIndex {
    pub fn to_point(self, rope: &Rope) -> Point {
        let line = self.to_line(rope);
        Point {
            row: line,
            column: rope
                .try_line_to_char(line)
                .map(|char_index| self.0.saturating_sub(char_index))
                .unwrap_or(0),
        }
    }

    pub fn to_line(self, rope: &Rope) -> usize {
        rope.try_char_to_line(self.0)
            .unwrap_or_else(|_| rope.len_lines())
    }

    pub fn to_byte(self, rope: &Rope) -> usize {
        rope.try_char_to_byte(self.0)
            .unwrap_or_else(|_| rope.len_bytes())
    }

    pub fn apply_offset(&self, change: isize) -> CharIndex {
        if change.is_positive() {
            *self + (change as usize)
        } else {
            *self - (change.abs() as usize)
        }
    }
}

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

pub struct State {
    pub text: Rope,
    pub mode: Mode,

    pub selection: Selection,

    pub cursor_direction: CursorDirection,
    pub tree: Tree,
    pub quit: bool,
    yanked_text: Option<Rope>,
    selection_history: Vec<Selection>,

    undo_edits: Vec<EditTransaction>,
    redo_edits: Vec<EditTransaction>,

    /// This indicates where the extended selection started
    ///
    /// Some = the selection is being extended
    /// None = the selection is not being extended
    extended_selection_anchor: Option<CharIndex>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SelectionMode {
    Line,
    Word,
    Alphabet,
    Custom,

    Token,

    NamedNode,
    ParentNode,
    SiblingNode,
}
impl SelectionMode {
    fn similar_to(&self, other: &SelectionMode) -> bool {
        self == other
        // || self.is_node() && other.is_node()
    }

    fn is_node(&self) -> bool {
        use SelectionMode::*;
        matches!(self, NamedNode | ParentNode | SiblingNode)
    }
}

pub enum CursorDirection {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Forward,
    Backward,
    Current,
}

impl State {
    pub fn new(text: Rope, tree: Tree) -> Self {
        Self {
            selection: Selection {
                mode: SelectionMode::Custom,
                start: CharIndex(0),
                end: CharIndex(0),
                node_id: None,
            },
            text,
            mode: Mode::Normal,
            cursor_direction: CursorDirection::Start,
            tree,
            yanked_text: None,
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
        if let Some(node) = self.get_nearest_node_under_cursor() {
            if let Some(parent) = node.parent() {
                let second_child = parent.child(1);
                let second_last_child = parent.child(parent.child_count() - 2).or(second_child);

                if let (Some(second_child), Some(second_last_child)) =
                    (second_child, second_last_child)
                {
                    self.update_selection(Selection {
                        start: CharIndex(second_child.start_byte()),
                        end: CharIndex(second_last_child.end_byte()),
                        node_id: None,
                        mode: SelectionMode::Custom,
                    });
                }
            }
        }
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

    fn select_word(&mut self) {
        todo!()
    }

    fn select_alphabet(&mut self, direction: Direction) {
        self.select(SelectionMode::Alphabet, direction);
    }

    fn select_backward(&mut self) {
        while let Some(selection) = self.selection_history.pop() {
            if selection != self.selection {
                self.selection = selection;
                break;
            }
        }
    }

    fn select_none(&mut self, direction: Direction) {
        self.select(SelectionMode::Custom, direction);
    }

    fn select_token(&mut self, direction: Direction) {
        self.select(SelectionMode::Token, direction);
    }

    fn update_selection(&mut self, selection: Selection) {
        self.selection = selection.clone();
        self.selection_history.push(selection);
    }

    fn select(&mut self, selection_mode: SelectionMode, direction: Direction) {
        let direction = if self.selection.mode.similar_to(&selection_mode) {
            direction
        } else {
            Direction::Current
        };
        log::info!("select: {:?} {:?}", selection_mode, direction);
        let selection = self.get_selection(&selection_mode, direction);
        if let Some(node_id) = selection.node_id {
            get_node_by_id(&self.tree, node_id).map(|node| {
                log::info!(
                    "node: {:?}",
                    node.utf8_text(&self.text.to_string().into_bytes()).unwrap()
                );
                log::info!("{}", node.to_sexp())
            });
        }
        self.update_selection(selection);
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
            let next_selection = Self::get_selection_(
                &self.text,
                &self.tree,
                &current_selection,
                &self.selection.mode,
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
        self.jump_from_selection(direction, &self.selection.clone());
    }

    pub fn get_current_selection(&self) -> Selection {
        if let Some(anchor) = self.extended_selection_anchor {
            return Selection::from_two_char_indices(&anchor, &self.get_cursor_char_index());
        }
        match &self.mode {
            Mode::Normal | Mode::Jump { .. } => self.selection.clone(),
            Mode::Insert => todo!(),
        }
    }

    fn delete_current_selection(&mut self) {
        let selection = self.get_current_selection();
        self.yank(&selection);
        self.edit(selection.start..selection.end, Rope::new());
        self.extended_selection_anchor = None;
        self.select(self.selection.mode, Direction::Current);
    }

    fn yank(&mut self, selection: &Selection) {
        self.yanked_text = self
            .text
            .get_slice(selection.start.0..selection.end.0)
            .map(|slice| slice.into());
        self.extended_selection_anchor = None;
    }

    fn yank_current_selection(&mut self) {
        self.yank(&self.get_current_selection());
    }

    fn paste(&mut self) {
        if let Some(yanked_text) = &self.yanked_text {
            let cursor_position = self.get_cursor_char_index();
            let yanked_text_len = yanked_text.len_chars();
            self.edit(cursor_position..cursor_position, yanked_text.clone());

            match (&self.cursor_direction, &self.mode) {
                (_, Mode::Normal) => {
                    self.selection = Selection::from_two_char_indices(
                        &cursor_position,
                        &(cursor_position + yanked_text_len),
                    )
                }

                (_, Mode::Insert) => {
                    let start = cursor_position + yanked_text_len;
                    self.selection = Selection {
                        start,
                        end: start,
                        mode: SelectionMode::Custom,
                        node_id: None,
                    }
                }
                _ => {}
            }
        }
    }

    fn replace(&mut self) {
        let replacement = self.yanked_text.take().unwrap_or_else(Rope::new);
        let selection = self.get_current_selection();
        let replacement_text_len = replacement.len_chars();
        self.yank(&selection);
        self.edit(selection.start..selection.end, replacement);
        self.selection = Selection {
            start: selection.start,
            end: selection.start + replacement_text_len,
            mode: SelectionMode::Custom,
            node_id: None,
        };
    }

    /// Replace the text in the given range with the given replacement.
    fn edit(&mut self, range: Range<CharIndex>, replacement: Rope) {
        self.apply_edit_transaction(
            EditHistoryKind::NewEdit,
            EditTransaction::from_edits(
                self.selection.clone(),
                vec![Edit {
                    start: range.start,
                    old: self.text.slice(range.start.0..range.end.0).into(),
                    new: replacement,
                }],
            ),
        )
    }

    fn apply_edit_transaction(
        &mut self,
        edit_history_kind: EditHistoryKind,
        edit_transaction: EditTransaction,
    ) {
        let inversed_edit_transaction = EditTransaction::from_edits(
            self.selection.clone(),
            edit_transaction
                .edits()
                .iter()
                .map(|edit| Edit {
                    start: edit.start,
                    old: edit.new.clone(),
                    new: edit.old.clone(),
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
        (self.tree, self.text) =
            apply_edit_transaction(self.tree.clone(), self.text.clone(), edit_transaction).unwrap();
    }

    fn undo(&mut self) {
        // TODO: undoing edit transaction does not work
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
        let selection = edit_transaction.selection.clone();
        self.apply_edit_transaction(edit_history_kind, edit_transaction);
        self.update_selection(selection)
    }

    fn change_cursor_direction(&mut self) {
        self.cursor_direction = match self.cursor_direction {
            CursorDirection::Start => CursorDirection::End,
            CursorDirection::End => CursorDirection::Start,
        };
    }

    fn get_nearest_node_under_cursor(&self) -> Option<Node> {
        let cursor_pos = self.get_cursor_char_index().to_byte(&self.text);

        get_nearest_node_after_byte(&self.tree, cursor_pos)
    }

    fn get_selection_(
        text: &Rope,
        tree: &Tree,
        current_selection: &Selection,
        mode: &SelectionMode,
        direction: &Direction,
        cursor_direction: &CursorDirection,
    ) -> Selection {
        let cursor_char_index = {
            let index = current_selection.to_char_index(cursor_direction);
            match cursor_direction {
                CursorDirection::Start => index,
                // Minus one so that selecting line backward works
                CursorDirection::End => index - 1,
            }
        };
        let cursor_byte = cursor_char_index.to_byte(&text);
        match mode {
            SelectionMode::NamedNode => match direction {
                Direction::Forward | Direction::Current => {
                    traverse(tree.root_node().walk(), Order::Pre)
                        .find(|node| node.start_byte() > cursor_byte && node.is_named())
                }
                Direction::Backward => ReverseTreeCursor::new(tree.root_node())
                    .tuple_windows()
                    .find(|(current, next)| {
                        next.start_byte() < current.start_byte()
                            && current.start_byte() < cursor_byte
                            && current.is_named()
                    })
                    .map(|(current, _)| current),
            }
            .map(|node| node_to_selection(node, *mode, text))
            .unwrap_or_else(|| current_selection.clone()),
            SelectionMode::Line => {
                let start = cursor_char_index.to_line(text);

                let start = CharIndex(
                    text.line_to_char(
                        match direction {
                            Direction::Forward => start.saturating_add(1),
                            Direction::Backward => start.saturating_sub(1),
                            Direction::Current => cursor_char_index.to_line(text),
                        }
                        .min(text.len_lines().saturating_sub(1)),
                    ),
                );
                let end = CharIndex(
                    start
                        .0
                        .saturating_add(text.line(start.to_line(text)).len_chars()),
                );
                Selection {
                    start,
                    end,
                    node_id: None,
                    mode: *mode,
                }
            }
            SelectionMode::Word => todo!(),
            SelectionMode::ParentNode => {
                let current_node = get_current_node(tree, cursor_byte, current_selection);

                fn get_node(node: Node, direction: Direction) -> Option<Node> {
                    match direction {
                        Direction::Current => Some(node),
                        Direction::Forward => node.parent(),

                        // Backward of ParentNode = ChildNode
                        Direction::Backward => node.named_child(0),
                    }
                }

                let node = {
                    if direction == &Direction::Current {
                        current_node
                    } else {
                        let mut node = get_node(current_node, *direction);

                        // This loop is to ensure we select the nearest parent that has a larger range than
                        // the current node
                        //
                        // This is necessary because sometimes the parent node can have the same range as
                        // the current node
                        while let Some(some_node) = node {
                            if some_node.range() != current_node.range() {
                                break;
                            }
                            node = get_node(some_node, *direction);
                        }
                        node.unwrap_or(current_node)
                    }
                };
                node_to_selection(node, *mode, text)
            }

            SelectionMode::SiblingNode => {
                let current_node = get_current_node(tree, cursor_byte, current_selection);
                let next_node = match direction {
                    Direction::Current => Some(current_node),
                    Direction::Forward => current_node.next_named_sibling(),
                    Direction::Backward => current_node.prev_named_sibling(),
                }
                .unwrap_or(current_node);
                node_to_selection(next_node, *mode, text)
            }
            SelectionMode::Token => {
                let current_selection_start_byte = current_selection.start.to_byte(text);
                let current_selection_end_byte = current_selection.end.to_byte(text);
                let selection = match direction {
                    Direction::Forward => get_next_token(tree, current_selection_end_byte, false),
                    Direction::Backward => {
                        get_prev_token(tree, current_selection_start_byte, false)
                    }
                    Direction::Current => get_next_token(tree, cursor_byte, false),
                }
                .unwrap_or_else(|| {
                    get_next_token(tree, cursor_byte, true).unwrap_or_else(|| tree.root_node())
                });
                node_to_selection(selection, *mode, text)
            }
            SelectionMode::Alphabet => match direction {
                Direction::Current => Selection {
                    start: cursor_char_index,
                    end: cursor_char_index + 1,
                    node_id: None,
                    mode: *mode,
                },
                Direction::Forward => Selection {
                    start: cursor_char_index + 1,
                    end: cursor_char_index + 2,
                    node_id: None,
                    mode: *mode,
                },
                Direction::Backward => Selection {
                    start: cursor_char_index - 1,
                    end: cursor_char_index,
                    node_id: None,
                    mode: *mode,
                },
            },
            SelectionMode::Custom => Selection {
                start: cursor_char_index,
                end: cursor_char_index,
                node_id: None,
                mode: *mode,
            },
        }
    }

    fn get_selection(&self, mode: &SelectionMode, direction: Direction) -> Selection {
        Self::get_selection_(
            &self.text,
            &self.tree,
            &self.selection,
            mode,
            &direction,
            &self.cursor_direction,
        )
    }

    pub fn get_cursor_point(&self) -> Point {
        self.get_cursor_char_index().to_point(&self.text)
    }

    fn get_cursor_char_index(&self) -> CharIndex {
        self.selection.to_char_index(&self.cursor_direction)
    }

    fn toggle_highlight_mode(&mut self) {
        if let Some(anchor) = self.extended_selection_anchor.take() {
            // Reverse the anchor with the current cursor position
            let cursor_index = self.get_cursor_char_index();
            self.extended_selection_anchor = Some(cursor_index);
            self.selection = Selection {
                start: anchor,
                end: anchor,
                node_id: None,
                mode: SelectionMode::Custom,
            };
            self.cursor_direction = if cursor_index > anchor {
                CursorDirection::Start
            } else {
                CursorDirection::End
            };
        } else {
            self.extended_selection_anchor = Some(self.get_cursor_char_index());
            self.cursor_direction = CursorDirection::End;
        }
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
        let cursor_char_index = self.get_cursor_char_index();
        match event.code {
            KeyCode::Left => {
                self.selection = Selection {
                    start: cursor_char_index - 1,
                    end: cursor_char_index - 1,
                    node_id: None,
                    mode: SelectionMode::Custom,
                };
                HandleKeyEventResult::Consumed
            }
            KeyCode::Right => {
                self.selection = Selection {
                    start: cursor_char_index + 1,
                    end: cursor_char_index + 1,
                    node_id: None,
                    mode: SelectionMode::Custom,
                };
                HandleKeyEventResult::Consumed
            }
            KeyCode::Char('a') if event.modifiers == KeyModifiers::CONTROL => {
                self.selection = Selection {
                    start: CharIndex(0),
                    end: CharIndex(self.text.len_chars()),
                    node_id: None,
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
                        .max_by(|a, b| a.selection.start.cmp(&b.selection.start))
                    {
                        self.jump_from_selection(Direction::Forward, &jump.selection.clone());
                    }
                }
                KeyCode::Char('J') => {
                    if let Some(jump) = jumps
                        .iter()
                        .min_by(|a, b| a.selection.end.cmp(&b.selection.end))
                    {
                        self.jump_from_selection(Direction::Backward, &jump.selection.clone());
                    }
                }
                KeyCode::Char(c) => {
                    let matching_jump = jumps.iter().find(|jump| c == jump.character);
                    if let Some(jump) = matching_jump {
                        self.update_selection(jump.selection.clone());
                        self.mode = Mode::Normal;
                    }
                }
                _ => {}
            },
            _ => unreachable!(),
        }
    }

    fn insert(&mut self, s: &str) {
        let Selection { start, end, .. } = self.selection;
        self.edit(start..end, s.into());
        self.selection = Selection {
            mode: SelectionMode::Custom,
            start: start + 1,
            end: end + 1,
            node_id: None,
        }
    }

    fn handle_insert_mode(&mut self, event: KeyEvent) {
        let Selection { start, end, .. } = self.selection;
        match event.code {
            KeyCode::Esc => self.enter_normal_mode(),
            KeyCode::Backspace => {
                self.edit(start - 1..end, "".into());
                self.selection = Selection {
                    start: start - 1,
                    end: end - 1,
                    node_id: None,
                    mode: SelectionMode::Custom,
                };
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
            KeyCode::Char('a') => self.select_alphabet(Direction::Forward),
            KeyCode::Char('A') => self.select_alphabet(Direction::Backward),
            KeyCode::Char('b') => self.select_backward(),
            KeyCode::Char('d') => self.delete_current_selection(),
            KeyCode::Char('h') => self.toggle_highlight_mode(),
            KeyCode::Char('i') => self.enter_insert_mode(),
            KeyCode::Char('j') => self.jump(Direction::Forward),
            KeyCode::Char('J') => self.jump(Direction::Backward),
            KeyCode::Char('k') => self.select_kids(),
            KeyCode::Char('l') => self.select_line(Direction::Forward),
            KeyCode::Char('L') => self.select_line(Direction::Backward),
            KeyCode::Char('n') => self.select_named_node(Direction::Forward),
            KeyCode::Char('N') => self.select_named_node(Direction::Backward),
            KeyCode::Char('o') => self.change_cursor_direction(),
            KeyCode::Char('s') => self.select_sibling(Direction::Forward),
            KeyCode::Char('S') => self.select_sibling(Direction::Backward),
            KeyCode::Char('t') => self.select_token(Direction::Forward),
            KeyCode::Char('T') => self.select_token(Direction::Backward),
            KeyCode::Char('w') => self.select_word(),
            KeyCode::Char('r') => self.replace(),
            KeyCode::Char('p') => self.select_parent(Direction::Forward),
            KeyCode::Char('P') => self.select_parent(Direction::Backward),
            KeyCode::Char('x') => self.exchange(Direction::Forward),
            KeyCode::Char('X') => self.exchange(Direction::Backward),
            KeyCode::Char('y') => self.yank_current_selection(),
            KeyCode::Char('0') => self.select_none(Direction::Forward),
            KeyCode::Esc => {
                self.extended_selection_anchor = None;
            }
            // Similar to Change in Vim
            KeyCode::Backspace => {
                let selection = self.get_current_selection();
                self.yank(&selection);
                self.edit(selection.start..selection.end, "".into());
                self.selection = Selection {
                    start: selection.start,
                    end: selection.start,
                    mode: SelectionMode::Custom,
                    node_id: None,
                };
                self.extended_selection_anchor = None;
                self.mode = Mode::Insert;
            }
            _ => {
                log::info!("event: {:?}", event);
                // todo!("Back to previous selection");
                // todo!("Search by node kind")
            }
        }
    }

    fn enter_insert_mode(&mut self) {
        let char_index = self.get_cursor_char_index();
        self.selection = Selection {
            start: char_index,
            end: char_index,
            node_id: None,
            mode: SelectionMode::Custom,
        };
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
        self.update_selection(Selection {
            mode: SelectionMode::Custom,
            start,
            end: start,
            node_id: None,
        })
    }

    /// Replace the next selection with the current selection without
    /// making the syntax tree invalid
    fn replace_faultlessly(&mut self, selection_mode: &SelectionMode, direction: Direction) {
        let current_selection = self.selection.clone();

        let text_at_current_selection = self
            .text
            .slice(current_selection.start.0..current_selection.end.0)
            .to_string();

        // Loop until the replacement does not result in errorneous node
        let mut next_selection = self.get_selection(&selection_mode, direction);

        loop {
            let edit_transaction = EditTransaction::from_edits(
                self.selection.clone(),
                vec![
                    Edit {
                        start: current_selection.start,
                        old: Rope::from_str(&text_at_current_selection),
                        new: Rope::from_str(
                            &self
                                .text
                                .slice(next_selection.start.0..next_selection.end.0)
                                .to_string(),
                        ),
                    },
                    Edit {
                        start: next_selection.start,
                        old: Rope::from_str(
                            &self
                                .text
                                .slice(next_selection.start.0..next_selection.end.0)
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
                            &(" ".to_string() + &text_at_current_selection + &" ".to_string()),
                        ),
                    },
                ],
            );

            let (tree, text) = apply_edit_transaction(
                self.tree.clone(),
                self.text.clone(),
                edit_transaction.clone(),
            )
            .unwrap();

            if let Some(node) = tree.root_node().descendant_for_byte_range(
                text.try_char_to_byte(edit_transaction.min_char_index().0)
                    .unwrap_or(0),
                text.try_char_to_byte(edit_transaction.max_char_index().0)
                    .unwrap_or(0),
            ) {
                let text_at_next_selection: Rope = self
                    .text
                    .slice(next_selection.start.0..next_selection.end.0)
                    .into();

                // Why don't we just use `tree.root_node().has_error()` instead?
                // Because I assume we want to be able to exchange even if some part of the tree
                // contains error
                if !text_at_next_selection.to_string().trim().is_empty()
                    && (!selection_mode.is_node() || !node.has_error())
                {
                    let edit_transaction = EditTransaction::from_edits(
                        self.selection.clone(),
                        vec![
                            Edit {
                                start: current_selection.start,
                                old: Rope::from_str(&text_at_current_selection),
                                new: Rope::from_str(
                                    &self
                                        .text
                                        .slice(next_selection.start.0..next_selection.end.0)
                                        .to_string(),
                                ),
                            },
                            Edit {
                                start: next_selection.start,
                                old: Rope::from_str(
                                    &self
                                        .text
                                        .slice(next_selection.start.0..next_selection.end.0)
                                        .to_string(),
                                ),
                                // This time without whitespace padding
                                new: Rope::from_str(&text_at_current_selection),
                            },
                        ],
                    );

                    self.apply_edit_transaction(EditHistoryKind::NewEdit, edit_transaction.clone());

                    let updated_selection = {
                        let edits = edit_transaction.edits();
                        let edit = match direction {
                            Direction::Forward | Direction::Current => edits.last(),
                            Direction::Backward => edits.first(),
                        }
                        .unwrap();

                        Selection {
                            start: edit.start,
                            end: edit.start + edit.new.len_chars(),
                            mode: *selection_mode,
                            node_id: None,
                        }
                    };
                    self.update_selection(updated_selection);
                    return;
                }
            }

            // Get the next selection

            let new_selection = Self::get_selection_(
                &self.text,
                &self.tree,
                &next_selection,
                selection_mode,
                &direction,
                &self.cursor_direction,
            );

            if next_selection.eq(&new_selection) {
                return;
            }

            next_selection = new_selection;
        }
    }

    fn exchange(&mut self, direction: Direction) {
        self.replace_faultlessly(&self.selection.mode.clone(), direction)
    }
}

fn get_prev_token(tree: &Tree, byte: usize, is_named: bool) -> Option<Node> {
    ReverseTreeCursor::new(tree.root_node()).find(|&node| {
        node.child_count() == 0 && (!is_named || node.is_named()) && node.start_byte() < byte
    })
}

fn get_next_token(tree: &Tree, byte: usize, is_named: bool) -> Option<Node> {
    traverse(tree.root_node().walk(), Order::Post).find(|&node| {
        node.child_count() == 0 && (!is_named || node.is_named()) && node.end_byte() > byte
    })
}

fn get_nearest_node_after_byte(tree: &Tree, byte: usize) -> Option<Node> {
    // Preorder is the main key here,
    // because preorder traversal walks the parent first
    traverse(tree.root_node().walk(), Order::Pre).find(|&node| node.start_byte() >= byte)
}

fn get_node_by_id(tree: &Tree, node_id: usize) -> Option<Node> {
    let result = traverse(tree.walk(), Order::Pre).find(|node| node.id() == node_id);
    result
}

fn node_to_selection(node: Node, mode: SelectionMode, text: &Rope) -> Selection {
    Selection {
        mode,
        start: CharIndex(text.byte_to_char(node.start_byte())),
        end: CharIndex(text.byte_to_char(node.end_byte())),
        node_id: Some(node.id()),
    }
}

struct ReverseTreeCursor<'a> {
    node: Node<'a>,
}

struct ParentTreeCursor<'a> {
    node: Node<'a>,
}

impl<'a> ReverseTreeCursor<'a> {
    fn new(node: Node<'a>) -> Self {
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

fn get_current_node<'a>(tree: &'a Tree, cursor_byte: usize, selection: &Selection) -> Node<'a> {
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

    log::info!("edit = {:?}", edit);

    log::info!(
        "Applying edit: {:?} -> {:?} (new end: {:?})",
        edit.start,
        edit.end(),
        new_end_char_index
    );

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
    tree = parser.parse(&text.to_string(), Some(&tree)).unwrap();
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
