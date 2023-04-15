use std::ops::{Add, Range, Sub};

use itertools::Itertools;
use ropey::Rope;
use tree_sitter::{InputEdit, Node, Point, Tree};
use tree_sitter_traversal::{traverse, Order};

#[derive(PartialEq, Clone, Debug, Copy)]
pub struct Selection {
    pub start: CharIndex,
    pub end: CharIndex,
    pub node_id: Option<usize>,
}

impl Add<usize> for Selection {
    type Output = Selection;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
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

#[derive(PartialEq, Clone, Debug, Copy)]
pub struct CharIndex(pub usize);

impl CharIndex {
    pub fn to_point(&self, rope: &Rope) -> Point {
        let line = rope.char_to_line(self.0);
        Point {
            row: line,
            column: self.0.saturating_sub(rope.line_to_char(line)),
        }
    }
}

pub enum Mode {
    Normal,
    Insert,
    Extend { extended_selection: Selection },
}

pub struct State {
    pub selection: Selection,
    pub source_code: Rope,
    pub mode: Mode,
    pub selection_mode: SelectionMode,
    pub cursor_direction: CursorDirection,
    pub tree: Tree,
    yanked_text: Option<Rope>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SelectionMode {
    Line,
    NamedNode,
    Character,
    Word,
    Node,
    NamedToken,
    Token,
}
impl SelectionMode {
    fn similar_to(&self, other: &SelectionMode) -> bool {
        use SelectionMode::*;
        match (self, other) {
            (NamedToken, Token) => true,
            (Token, NamedToken) => true,
            (a, b) => a == b,
        }
    }
}

pub enum CursorDirection {
    Start,
    End,
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Forward,
    Backward,
    Current,
}

impl State {
    pub fn new(source_code: Rope, tree: Tree) -> Self {
        Self {
            selection: to_selection(tree.root_node(), &source_code),
            source_code,
            mode: Mode::Normal,
            selection_mode: SelectionMode::Line,
            cursor_direction: CursorDirection::Start,
            tree,
            yanked_text: None,
        }
    }
    pub fn select_ancestor(&mut self) {
        self.select_node(|node| node.parent());
    }

    pub fn select_kids(&mut self) {
        if let Some(node) = self.get_nearest_node_under_cursor() {
            if let Some(parent) = node.parent() {
                let second_child = parent.child(1);
                let second_last_child = parent.child(parent.child_count() - 2).or(second_child);
                match (second_child, second_last_child) {
                    (Some(second_child), Some(second_last_child)) => {
                        self.update_selection(
                            Direction::Forward,
                            Selection {
                                start: CharIndex(second_child.start_byte() as usize),
                                end: CharIndex(second_last_child.end_byte() as usize),
                                node_id: None,
                            },
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn select_sibling(&mut self) {
        self.select_node(|node| node.next_sibling());
    }

    pub fn select_line(&mut self) {
        self.select(SelectionMode::Line);
    }

    pub fn select_named_node(&mut self) {
        self.select(SelectionMode::NamedNode)
    }

    pub fn select_word(&mut self) {
        todo!()
    }

    pub fn select_charater(&mut self) {
        self.select(SelectionMode::Character);
    }

    pub fn select_backward(&mut self) {
        self.update_selection(
            Direction::Backward,
            self.get_selection(&self.selection_mode, Direction::Backward),
        )
    }

    pub fn select_named_token(&mut self) {
        self.select(SelectionMode::NamedToken);
    }

    pub fn select_token(&mut self) {
        self.select(SelectionMode::Token);
    }

    fn update_selection(&mut self, direction: Direction, selection: Selection) {
        self.selection = selection;
        if let Mode::Extend { extended_selection } = self.mode {
            let f = match direction {
                Direction::Forward => usize::max,
                Direction::Backward => usize::min,
                Direction::Current => usize::max,
            };
            self.mode = Mode::Extend {
                extended_selection: Selection {
                    start: CharIndex(selection.start.0.min(extended_selection.start.0)),
                    end: CharIndex(f(selection.end.0, extended_selection.end.0)),
                    node_id: None,
                },
            }
        }
    }

    fn select(&mut self, selection_mode: SelectionMode) {
        let direction = if self.selection_mode.similar_to(&selection_mode) {
            Direction::Forward
        } else {
            Direction::Current
        };
        self.update_selection(direction, self.get_selection(&selection_mode, direction));
        self.selection_mode = selection_mode;
    }

    fn get_input_edit(
        &self,
        start_char_index: CharIndex,
        old_end_char_index: CharIndex,
        new_end_char_index: CharIndex,
    ) -> InputEdit {
        InputEdit {
            start_byte: self.source_code.char_to_byte(start_char_index.0),
            old_end_byte: self.source_code.char_to_byte(old_end_char_index.0),
            new_end_byte: self.source_code.char_to_byte(new_end_char_index.0),
            start_position: start_char_index.to_point(&self.source_code),
            old_end_position: old_end_char_index.to_point(&self.source_code),
            new_end_position: new_end_char_index.to_point(&self.source_code),
        }
    }

    fn get_current_selection(&self) -> Selection {
        match self.mode {
            Mode::Normal => self.selection,
            Mode::Insert => todo!(),
            Mode::Extend { extended_selection } => extended_selection,
        }
    }

    pub fn delete_current_selection(&mut self) {
        let selection = self.get_current_selection();
        self.replace_with(selection.start.0..selection.end.0, Rope::new());
    }

    pub fn yank(&mut self) {
        let selection = self.get_current_selection();
        self.yanked_text = Some(
            self.source_code
                .slice(selection.start.0..selection.end.0)
                .into(),
        );
    }

    pub fn paste(&mut self) {
        if let Some(yanked_text) = &self.yanked_text {
            let cursor_position = self.get_cursor_char_index();

            self.tree.edit(&self.get_input_edit(
                cursor_position,
                cursor_position,
                cursor_position + yanked_text.len_chars(),
            ));

            let mut parser = tree_sitter::Parser::new();
            parser.set_language(self.tree.language()).unwrap();
            self.source_code
                .insert(cursor_position.0, yanked_text.to_string().as_str());
            self.tree = parser
                .parse(&self.source_code.to_string(), Some(&self.tree))
                .unwrap();

            if let CursorDirection::Start = self.cursor_direction {
                // TODO: what if we are in extend mode?
                self.selection = self.selection + yanked_text.len_chars()
            }
        }
    }

    pub fn replace(&mut self) {
        let yanked_text = self.yanked_text.take().unwrap_or_else(|| Rope::new());
        let selection = self.get_current_selection();
        self.replace_with(selection.start.0..selection.end.0, yanked_text);
    }

    fn replace_with(&mut self, range: Range<usize>, replacement: Rope) {
        self.tree.edit(&self.get_input_edit(
            CharIndex(range.start),
            CharIndex(range.end),
            CharIndex(range.start) + replacement.len_chars(),
        ));
        if !range.is_empty() {
            self.yanked_text = Some(self.source_code.slice(range.clone()).into());
        }
        self.source_code.remove(range.clone());
        self.source_code
            .insert(range.start, replacement.to_string().as_str());
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(self.tree.language()).unwrap();
        self.tree = parser
            .parse(&self.source_code.to_string(), Some(&self.tree))
            .unwrap();

        self.mode = Mode::Normal;
        self.selection_mode = SelectionMode::Character;

        // TODO: what if we are in extend mode?
        self.selection = Selection {
            start: CharIndex(range.start),
            end: CharIndex(range.start + replacement.len_chars()),
            node_id: None,
        };
    }

    fn get_next_token(&self, position: &CharIndex, is_named: bool) -> Option<Node> {
        for node in traverse(self.tree.root_node().walk(), Order::Post) {
            if node.child_count() == 0
                && (!is_named || node.is_named())
                && self.source_code.byte_to_char(node.end_byte()) > position.0
            {
                return Some(node);
            }
        }
        None
    }

    fn get_prev_token(&self, position: &CharIndex, is_named: bool) -> Option<Node> {
        for node in ReverseTreeCursor::new(self.tree.root_node()) {
            if node.child_count() == 0
                && (!is_named || node.is_named())
                && self.source_code.byte_to_char(node.start_byte()) < position.0
            {
                return Some(node);
            }
        }
        None
    }

    pub fn change_cursor_direction(&mut self) {
        self.cursor_direction = match self.cursor_direction {
            CursorDirection::Start => CursorDirection::End,
            CursorDirection::End => CursorDirection::Start,
        };
    }

    fn get_nearest_node_under_cursor(&self) -> Option<Node> {
        let cursor_pos = self.get_cursor_char_index();

        // Preorder is the main key here,
        // because preorder traversal walks the parent first
        for node in traverse(self.tree.root_node().walk(), Order::Pre) {
            if self.source_code.byte_to_char(node.start_byte()) >= cursor_pos.0 {
                return Some(node);
            }
        }
        None
    }

    fn get_current_node(&self) -> Option<Node> {
        match self.selection.node_id {
            Some(node_id) => self.get_node_by_id(node_id),
            None => None,
        }
    }

    fn get_node_by_id(&self, node_id: usize) -> Option<Node> {
        let result = traverse(self.tree.walk(), Order::Pre).find(|node| node.id() == node_id);
        result
    }

    fn select_node<F>(&mut self, f: F)
    where
        F: Fn(Node) -> Option<Node>,
    {
        if let Some(node) = self
            .get_current_node()
            .map(f)
            .unwrap_or_else(|| self.get_nearest_node_under_cursor())
        {
            let mode = SelectionMode::Node;
            self.update_selection(Direction::Current, to_selection(node, &self.source_code));
            self.selection_mode = mode;
        }
    }

    fn get_selection(&self, mode: &SelectionMode, direction: Direction) -> Selection {
        match mode {
            SelectionMode::NamedNode => {
                let cursor_byte_index = self
                    .source_code
                    .char_to_byte(self.get_cursor_char_index().0);
                return match direction {
                    Direction::Forward | Direction::Current => {
                        traverse(self.tree.root_node().walk(), Order::Pre)
                            .find(|node| node.start_byte() > cursor_byte_index && node.is_named())
                    }
                    Direction::Backward => ReverseTreeCursor::new(self.tree.root_node())
                        .tuple_windows()
                        .find(|(current, next)| {
                            next.start_byte() < current.start_byte()
                                && current.start_byte() < cursor_byte_index
                                && current.is_named()
                        })
                        .map(|(current, _)| current),
                }
                .map(|node| to_selection(node, &self.source_code))
                .unwrap_or_else(|| self.selection);
            }
            SelectionMode::Line => {
                let start = self.source_code.char_to_line(self.selection.start.0);
                let start = CharIndex(self.source_code.line_to_char(match direction {
                    Direction::Forward => start.saturating_add(1),
                    Direction::Backward => start.saturating_sub(1),
                    Direction::Current => self.get_cursor_point().row,
                }));
                let end = CharIndex(
                    start.0.saturating_add(
                        self.source_code
                            .line(self.source_code.char_to_line(start.0))
                            .len_chars(),
                    ),
                );
                Selection {
                    start: start.clone(),
                    end,
                    node_id: None,
                }
            }
            SelectionMode::Word => todo!(),
            SelectionMode::Node => match self.selection.node_id {
                Some(node_id) => {
                    let current_node = self
                        .get_node_by_id(node_id)
                        .or_else(|| self.get_nearest_node_under_cursor())
                        .unwrap_or_else(|| self.tree.root_node());

                    fn get_node<F>(node: Node, f: F) -> Option<Node>
                    where
                        F: Fn(Node) -> Option<Node>,
                    {
                        f(node)
                    }

                    let node = match direction {
                        Direction::Forward => get_node(current_node, |node| node.next_sibling()),
                        Direction::Backward => get_node(current_node, |node| node.prev_sibling()),
                        Direction::Current => get_node(current_node, |node| Some(node)),
                    }
                    .unwrap_or_else(|| current_node);

                    to_selection(node, &self.source_code)
                }
                _ => self
                    .get_nearest_node_under_cursor()
                    .map(|node| to_selection(node, &self.source_code))
                    .unwrap_or(self.selection.clone()),
            },
            SelectionMode::NamedToken => {
                let selection = match direction {
                    Direction::Forward => self.get_next_token(&self.selection.end, true),
                    Direction::Backward => self.get_prev_token(&self.selection.start, true),
                    Direction::Current => self.get_next_token(&self.get_cursor_char_index(), true),
                }
                .unwrap_or_else(|| {
                    self.get_nearest_node_under_cursor()
                        .unwrap_or_else(|| self.tree.root_node())
                });
                to_selection(selection, &self.source_code)
            }
            SelectionMode::Token => {
                let selection = match direction {
                    Direction::Forward => self.get_next_token(&self.selection.end, false),
                    Direction::Backward => self.get_prev_token(&self.selection.start, false),
                    Direction::Current => self.get_next_token(&self.get_cursor_char_index(), false),
                }
                .unwrap_or_else(|| {
                    self.get_nearest_node_under_cursor()
                        .unwrap_or_else(|| self.tree.root_node())
                });
                to_selection(selection, &self.source_code)
            }
            SelectionMode::Character => match direction {
                Direction::Forward => self.selection + 1,
                Direction::Backward => self.selection - 1,
                Direction::Current => {
                    let cursor = self.get_cursor_char_index();
                    Selection {
                        start: cursor,
                        end: cursor + 1,
                        node_id: None,
                    }
                }
            },
        }
    }

    pub fn get_cursor_point(&self) -> Point {
        self.get_cursor_char_index().to_point(&self.source_code)
    }

    fn get_cursor_char_index(&self) -> CharIndex {
        match self.cursor_direction {
            CursorDirection::Start => self.selection.start.clone(),
            CursorDirection::End => self.selection.end.clone(),
        }
    }

    pub fn toggle_extend_mode(&mut self) {
        self.mode = match self.mode {
            Mode::Extend { .. } => Mode::Normal,
            _ => Mode::Extend {
                extended_selection: self.selection,
            },
        };
    }
}

fn to_selection(node: Node, source_code: &Rope) -> Selection {
    Selection {
        start: CharIndex(source_code.byte_to_char(node.start_byte())),
        end: CharIndex(source_code.byte_to_char(node.end_byte())),
        node_id: Some(node.id()),
    }
}

struct ReverseTreeCursor<'a> {
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
