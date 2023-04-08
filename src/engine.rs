use ropey::Rope;
use tree_sitter::{Node, Point, Tree};
use tree_sitter_traversal::{traverse, Order};

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

pub struct State {
    pub selection: Selection,
    pub source_code: Rope,
    selection_mode: SelectionMode,
    pub cursor_direction: CursorDirection,
    pub tree: Tree,
}

#[derive(Debug)]
enum SelectionMode {
    Line,
    Node,
    NamedToken,
    Token,
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
        self.select_node(|node| node.named_child(0));
    }

    pub fn select_sibling(&mut self) {
        self.select_node(|node| node.next_sibling());
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
            SelectionMode::NamedToken => self.move_to_prev_token(true),
            SelectionMode::Token => self.move_to_prev_token(false),
        }
    }

    pub fn select_named_token(&mut self) {
        self.select_token_(true)
    }

    pub fn select_token(&mut self) {
        self.select_token_(false)
    }

    fn select_token_(&mut self, is_named: bool) {
        let position = match self.selection_mode {
            SelectionMode::NamedToken if is_named => self.selection.end.clone(),
            SelectionMode::Token if !is_named => self.selection.end.clone(),
            _ => self.get_cursor_char_index(),
        };
        self.move_to_next_token(position, is_named)
    }

    fn move_to_next_token(&mut self, position: CharIndex, is_named: bool) {
        for node in traverse(self.tree.root_node().walk(), Order::Post) {
            if node.child_count() == 0
                && (!is_named || node.is_named())
                && self.source_code.byte_to_char(node.end_byte()) > position.0.saturating_add(1)
            {
                self.selection = to_selection(node, &self.source_code);
                self.selection_mode = if is_named {
                    SelectionMode::NamedToken
                } else {
                    SelectionMode::Token
                };
                return;
            }
        }
    }

    fn move_to_prev_token(&mut self, is_named: bool) {
        let mut prev_node = None;
        for node in traverse(self.tree.root_node().walk(), Order::Post) {
            if self.source_code.byte_to_char(node.end_byte()) > self.selection.start.0 {
                break;
            }
            if node.child_count() == 0 && (!is_named || node.is_named()) {
                prev_node = Some(node)
            }
        }
        if let Some(node) = prev_node {
            self.selection = to_selection(node, &self.source_code);
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
        let cursor_pos = self.get_cursor_char_index();
        let (start, end) = match self.selection_mode {
            SelectionMode::Line | SelectionMode::Word => ((cursor_pos.0), (cursor_pos.0)),
            SelectionMode::Node | SelectionMode::NamedToken | SelectionMode::Token => (
                self.selection.start.0,
                self.selection.end.0.saturating_sub(1),
            ),
        };
        let current_node = self
            .tree
            .root_node()
            .descendant_for_byte_range(
                self.source_code.char_to_byte(start),
                self.source_code.char_to_byte(end),
            )
            .unwrap_or(self.tree.root_node());
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
        self.get_cursor_char_index().to_point(&self.source_code)
    }

    fn get_cursor_char_index(&self) -> CharIndex {
        match self.cursor_direction {
            CursorDirection::Start => self.selection.start.clone(),
            CursorDirection::End => CharIndex(self.selection.end.0.saturating_sub(1)),
        }
    }
}

fn to_selection(node: Node, source_code: &Rope) -> Selection {
    Selection {
        start: CharIndex(source_code.byte_to_char(node.start_byte())),
        end: CharIndex(source_code.byte_to_char(node.end_byte())),
    }
}
