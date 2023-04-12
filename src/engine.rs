use std::ops::{Add, Sub};

use ropey::Rope;
use tree_sitter::{Node, Point, Tree};
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

pub struct State {
    pub selection: Selection,
    pub source_code: Rope,
    pub selection_mode: SelectionMode,
    pub cursor_direction: CursorDirection,
    pub tree: Tree,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SelectionMode {
    Line,
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
            selection_mode: SelectionMode::Line,
            cursor_direction: CursorDirection::Start,
            tree,
        }
    }
    pub fn select_parent(&mut self) {
        self.select_node(|node| node.parent());
    }

    pub fn select_child(&mut self) {
        self.select_node(|node| node.child(0));
    }

    pub fn select_sibling(&mut self) {
        self.select_node(|node| node.next_sibling());
    }

    pub fn select_line(&mut self) {
        self.select(SelectionMode::Line);
    }

    pub fn select_word(&mut self) {
        todo!()
    }

    pub fn select_charater(&mut self) {
        self.select(SelectionMode::Character);
    }

    pub fn select_backward(&mut self) {
        self.selection = self.get_selection(&self.selection_mode, Direction::Backward);
    }

    pub fn select_named_token(&mut self) {
        self.select(SelectionMode::NamedToken);
    }

    pub fn select_token(&mut self) {
        self.select(SelectionMode::Token);
    }

    fn select(&mut self, selection_mode: SelectionMode) {
        if self.selection_mode.similar_to(&selection_mode) {
            self.selection = self.get_selection(&self.selection_mode, Direction::Forward);
        } else {
            self.selection_mode = selection_mode;
            self.selection = self.get_selection(&self.selection_mode, Direction::Current);
        }
    }

    pub fn delete_current_selection(&mut self) {
        let current_selection = self.selection.clone();
        self.tree.edit(&tree_sitter::InputEdit {
            start_byte: self.source_code.char_to_byte(current_selection.start.0),
            old_end_byte: self.source_code.char_to_byte(current_selection.end.0),
            new_end_byte: self.source_code.char_to_byte(current_selection.start.0),
            start_position: current_selection.start.to_point(&self.source_code),
            old_end_position: current_selection.end.to_point(&self.source_code),
            new_end_position: current_selection.start.to_point(&self.source_code),
        });
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(self.tree.language()).unwrap();
        self.source_code
            .remove(self.selection.start.0..self.selection.end.0);
        self.tree = parser
            .parse(&self.source_code.to_string(), Some(&self.tree))
            .unwrap();

        self.selection = self.get_selection(&SelectionMode::Character, Direction::Current);
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
        let mut prev_node = None;
        for node in traverse(self.tree.root_node().walk(), Order::Post) {
            if self.source_code.byte_to_char(node.end_byte()) > position.0 {
                return prev_node;
            }
            if node.child_count() == 0 && (!is_named || node.is_named()) {
                prev_node = Some(node)
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
        log::info!("get_node_by_id: {}", node_id);
        let result = traverse(self.tree.walk(), Order::Pre).find(|node| node.id() == node_id);
        log::info!("get_node_by_id result: {:?}", result);
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
            log::info!("select_node: {:?}", node);

            let mode = SelectionMode::Node;
            self.selection = to_selection(node, &self.source_code);
            self.selection_mode = mode;
        }
    }

    fn get_selection(&self, mode: &SelectionMode, direction: Direction) -> Selection {
        match mode {
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
}

fn to_selection(node: Node, source_code: &Rope) -> Selection {
    Selection {
        start: CharIndex(source_code.byte_to_char(node.start_byte())),
        end: CharIndex(source_code.byte_to_char(node.end_byte())),
        node_id: Some(node.id()),
    }
}
