use ropey::Rope;
use tree_sitter::{Node, Point};

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

enum SelectionMode {
    Line,
    Node,
    Word,
}

pub enum CursorDirection {
    Start,
    End,
}

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
        self.select_node(|node| node.parent());
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

    pub fn select_backward(&mut self) {
        match self.selection_mode {
            SelectionMode::Word => {
                self.move_by_word(Direction::Backward);
            }
            SelectionMode::Line => self.move_by_line(Direction::Backward),
            SelectionMode::Node => self.select_node(|node| node.prev_named_sibling()),
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
        let current_node = self
            .root_node
            .named_descendant_for_byte_range(
                self.source_code.char_to_byte(self.selection.start.0),
                self.source_code.char_to_byte(self.selection.end.0),
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
                start.0.saturating_add(
                    source_code
                        .line(source_code.char_to_line(start.0))
                        .len_chars(),
                ),
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

    fn move_by_word(&mut self, direction: Direction) {
        if matches!(self.selection_mode, SelectionMode::Word) {
            // match direction {}
        } else {
            self.select_current_word()
        }
    }

    fn select_current_word(&mut self) {
        self.selection = self.get_current_word_selection()
    }

    fn get_current_word_selection(&self) -> Selection {
        todo!()
    }

    pub fn get_cursor_point(&self) -> Point {
        match self.cursor_direction {
            CursorDirection::Start => &self.selection.start,
            CursorDirection::End => &self.selection.end,
        }
        .to_point(&self.source_code)
    }
}
fn to_selection(node: Node, source_code: &Rope) -> Selection {
    Selection {
        start: CharIndex(source_code.byte_to_char(node.start_byte())),
        end: CharIndex(source_code.byte_to_char(node.end_byte())),
    }
}
