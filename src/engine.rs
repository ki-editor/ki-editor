use ropey::{Rope, RopeSlice};
use std::ops::Range;
use tree_sitter::{Node, Point};
use unicode_segmentation::UnicodeSegmentation;

#[derive(PartialEq)]
pub struct Selection {
    pub start: CharIndex,
    pub end: CharIndex,
}

#[derive(PartialEq, Clone, Debug)]
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

pub struct State<'a> {
    pub selection: Selection,
    pub source_code: Rope,
    selection_mode: SelectionMode,
    pub cursor_direction: CursorDirection,
    pub root_node: Node<'a>,
}

#[derive(Debug)]
enum SelectionMode {
    Line,
    Node,
    NamedToken,
    Word,
}

pub enum CursorDirection {
    Start,
    End,
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Forward,
    Backward,
}

impl<'a> State<'a> {
    pub fn new(source_code: Rope, root_node: Node<'a>) -> Self {
        Self {
            selection: to_selection(root_node, &source_code),
            source_code,
            selection_mode: SelectionMode::Line,
            cursor_direction: CursorDirection::Start,
            root_node,
        }
    }
    pub fn select_parent(&mut self) {
        self.select_node(|node| get_named_parent(node));
    }

    pub fn select_child(&mut self) {
        self.select_node(|node| node.named_child(0));
    }

    pub fn select_sibling(&mut self) {
        self.select_node(|node| node.next_named_sibling());
    }

    pub fn select_line(&mut self) {
        self.move_by_line(Direction::Forward);
    }

    pub fn select_word(&mut self) {
        todo!()
    }

    pub fn select_backward(&mut self) {
        match self.selection_mode {
            SelectionMode::Word => {
                todo!()
            }
            SelectionMode::Line => self.move_by_line(Direction::Backward),
            SelectionMode::Node => self.select_node(|node| node.prev_named_sibling()),
            SelectionMode::NamedToken => self.move_to_named_token(Direction::Backward),
        }
    }

    fn move_to_named_token(&mut self, direction: Direction) {
        self.select_node(|node| Self::named_token(node, direction));
        self.selection_mode = SelectionMode::NamedToken;
    }

    fn named_token(node: Node, direction: Direction) -> Option<Node> {
        if let Some(node) = match direction {
            Direction::Forward => node.next_named_sibling(),
            Direction::Backward => node.prev_named_sibling(),
        } {
            Self::get_named_token(node, direction)
        } else if let Some(node) = node.parent() {
            Self::named_token(node, direction)
        } else {
            None
        }
    }

    fn get_named_token(node: Node, direction: Direction) -> Option<Node> {
        let mut node = node;
        let node = match direction {
            Direction::Forward => {
                while let Some(child) = node.named_child(0) {
                    node = child
                }
                node
            }
            Direction::Backward => {
                fn goto_last_child(node: Node) -> Node {
                    if let Some(child) =
                        node.named_child(node.named_child_count().saturating_sub(1))
                    {
                        goto_last_child(child)
                    } else {
                        node
                    }
                }
                goto_last_child(node)
            }
        };
        if node.is_named() {
            Some(node)
        } else {
            Self::named_token(node, Direction::Forward)
        }
    }

    pub fn select_named_token(&mut self) {
        match self.selection_mode {
            SelectionMode::NamedToken => {
                self.move_to_named_token(Direction::Forward);
            }
            _ => {
                self.select_node(|node| Self::get_named_token(node, Direction::Forward));
                self.selection_mode = SelectionMode::NamedToken;
            }
        }
    }

    pub fn change_cursor_direction(&mut self) {
        self.cursor_direction = match self.cursor_direction {
            CursorDirection::Start => CursorDirection::End,
            CursorDirection::End => CursorDirection::Start,
        };
    }

    fn select_node<F>(&mut self, f: F)
    where
        F: Fn(Node) -> Option<Node>,
    {
        let cursor_pos = self.get_cursor_index();
        let (start, end) = match self.selection_mode {
            SelectionMode::Line | SelectionMode::Word => ((cursor_pos.0), (cursor_pos.0)),
            SelectionMode::Node | SelectionMode::NamedToken => (
                self.selection.start.0,
                self.selection.end.0.saturating_sub(1),
            ),
        };
        let current_node = self
            .root_node
            .descendant_for_byte_range(
                self.source_code.char_to_byte(start),
                self.source_code.char_to_byte(end),
            )
            .unwrap_or(self.root_node);
        log::info!("current_node_name = {}", current_node.kind());
        if let Some(node) = f(current_node) {
            self.selection = to_selection(node, &self.source_code);
            self.selection_mode = SelectionMode::Node;
        }
    }

    fn move_by_line(&mut self, direction: Direction) {
        fn get_selection(source_code: &Rope, start: CharIndex) -> Selection {
            let end = CharIndex(
                start
                    .0
                    .saturating_add(
                        source_code
                            .line(source_code.char_to_line(start.0))
                            .len_chars(),
                    )
                    .saturating_sub(1),
            );

            Selection {
                start: start.clone(),
                end,
            }
        }
        if matches!(self.selection_mode, SelectionMode::Line) {
            match direction {
                Direction::Forward => {
                    self.selection = get_selection(
                        &self.source_code,
                        CharIndex(
                            self.source_code.line_to_char(
                                self.source_code
                                    .char_to_line(self.selection.start.0)
                                    .saturating_add(1),
                            ),
                        ),
                    )
                }
                Direction::Backward => {
                    self.selection = get_selection(
                        &self.source_code,
                        CharIndex(
                            self.source_code.line_to_char(
                                self.source_code
                                    .char_to_line(self.selection.start.0)
                                    .saturating_sub(1),
                            ),
                        ),
                    )
                }
            }
        } else {
            let cursor_point = self.get_cursor_point();
            self.selection = get_selection(
                &self.source_code,
                CharIndex(self.source_code.line_to_char(cursor_point.row)),
            );
        };
        self.selection_mode = SelectionMode::Line;
    }

    pub fn get_cursor_point(&self) -> Point {
        self.get_cursor_index().to_point(&self.source_code)
    }

    fn get_cursor_index(&self) -> CharIndex {
        match self.cursor_direction {
            CursorDirection::Start => self.selection.start.clone(),
            CursorDirection::End => CharIndex(self.selection.end.0.saturating_sub(1)),
        }
    }

    pub(crate) fn select_token(&self) {
        todo!()
    }
}

fn get_named_parent(node: Node) -> Option<Node> {
    node.parent().and_then(|parent| {
        if parent.is_named() {
            Some(parent)
        } else {
            get_named_parent(parent)
        }
    })
}

fn to_selection(node: Node, source_code: &Rope) -> Selection {
    Selection {
        start: CharIndex(source_code.byte_to_char(node.start_byte())),
        end: CharIndex(source_code.byte_to_char(node.end_byte())),
    }
}
