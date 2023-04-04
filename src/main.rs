use log::LevelFilter;
use std::io::{stdin, stdout, Write};
use termion::cursor::Goto;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, color, cursor, style};
use tree_sitter::{Node, Parser, Point};

fn main() {
    simple_logging::log_to_file("my_log.txt", LevelFilter::Info).unwrap();
    let source_code = "
function fibonacci(n) {
    if (n <= 0) {
        return 0;
    } else if (n === 1) {
        return 1;
    } else {
        return fibonacci(n - 1) + fibonacci(n - 2);
    }
}
        ";
    handle_event(source_code)
}

#[derive(PartialEq)]
struct Selection {
    start: Point,
    end: Point,
}

struct State {
    selection: Selection,
    source_code: String,
    selection_mode: SelectionMode,
    cursor_direction: CursorDirection,
}

enum SelectionMode {
    Line,
    Node,
    Word,
}

enum CursorDirection {
    Start,
    End,
}

enum Direction {
    Forward,
    Backward,
}

impl State {
    fn select_node(&mut self, node: Option<Node>) {
        if let Some(node) = node {
            self.selection = to_selection(node);
            self.selection_mode = SelectionMode::Node;
        }
    }

    fn get_line(&self, line_number: usize) -> Option<String> {
        let lines: Vec<&str> = self.source_code.lines().collect();
        match lines.get(line_number) {
            Some(line) => Some(line.to_string()),
            None => None,
        }
    }

    fn get_current_line(&self) -> Option<String> {
        self.get_line(self.selection.start.row)
    }

    fn select_line(&mut self, line_number: usize) {
        if let Some(current_line) = self.get_line(line_number) {
            log::info!("current_line: {}", current_line);
            log::info!("line_number: {}", line_number);
            log::info!("current_line.len(): {}", current_line.len());
            self.selection = Selection {
                start: Point {
                    row: line_number,
                    column: 0,
                },
                end: Point {
                    row: line_number,
                    column: current_line.len(),
                },
            };
            self.selection_mode = SelectionMode::Line;
        }
    }

    fn move_by_line(&mut self, direction: Direction) {
        let cursor_point = self.get_cursor_point();
        if matches!(self.selection_mode, SelectionMode::Line) {
            match direction {
                Direction::Forward => self.select_line(cursor_point.row.saturating_add(1)),
                Direction::Backward => self.select_line(cursor_point.row.saturating_sub(1)),
            }
        } else {
            self.select_line(cursor_point.row as usize);
        };
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
        let source_code = &self.source_code;
        let cursor_position = self.get_cursor_point();
        let lines: Vec<&str> = source_code.lines().collect();
        let current_line = self.get_current_line().unwrap();
        let words: Vec<&str> = current_line.split_whitespace().collect();
        let mut start_index = 0;
        for word in words {
            if cursor_position.column as usize >= start_index
                && (cursor_position.column as usize) < start_index + word.len()
            {
                return Selection {
                    start: Point {
                        row: cursor_position.row,
                        column: start_index,
                    },
                    end: Point {
                        row: cursor_position.row,
                        column: (start_index + word.len()),
                    },
                };
            }
            start_index += word.len() + 1;
        }
        Selection {
            start: Point { row: 0, column: 0 },
            end: Point { row: 0, column: 0 },
        }
    }

    fn get_cursor_point(&self) -> Point {
        match self.cursor_direction {
            CursorDirection::Start => self.selection.start,
            CursorDirection::End => self.selection.end,
        }
    }
}

fn to_selection(node: Node) -> Selection {
    Selection {
        start: node.start_position(),
        end: node.end_position(),
    }
}

fn handle_event(source_code: &str) {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_javascript::language())
        .unwrap();
    let tree = parser.parse(source_code, None).unwrap();
    let stdin = stdin();
    let mut stdout = stdout().into_raw_mode().unwrap();
    let mut state = State {
        selection: to_selection(tree.root_node()),
        source_code: source_code.to_string(),
        selection_mode: SelectionMode::Node,
        cursor_direction: CursorDirection::Start,
    };
    render(&source_code, &state, &mut stdout);
    let root_node = tree.root_node();
    for c in stdin.keys() {
        let node = root_node
            .descendant_for_point_range(state.selection.start, state.selection.end)
            .unwrap_or(root_node);
        match c.unwrap() {
            Key::Char('p') => {
                state.select_node(node.parent());
            }
            Key::Char('k') => {
                state.select_node(node.named_child(0));
            }
            Key::Char('s') => {
                state.select_node(node.next_named_sibling());
            }
            Key::Char('l') => state.move_by_line(Direction::Forward),
            Key::Char('L') => state.move_by_line(Direction::Backward),
            Key::Char('w') => {
                state.move_by_word(Direction::Forward);
            }
            Key::Char('W') => {
                state.move_by_word(Direction::Backward);
            }
            Key::Char('o') => {
                state.cursor_direction = match state.cursor_direction {
                    CursorDirection::Start => CursorDirection::End,
                    CursorDirection::End => CursorDirection::Start,
                };
            }
            Key::Ctrl('c') => break,
            _ => {}
        }
        render(&source_code, &state, &mut stdout);
        stdout.flush().unwrap();
    }
}

fn render(code: &str, state: &State, stdout: &mut impl Write) {
    write!(stdout, "{}", clear::All).unwrap();
    let lines: Vec<&str> = code.split('\n').collect();

    let selection = &state.selection;
    let start_point = selection.start;
    let end_point = selection.end;
    for (line_number, line) in lines.iter().enumerate() {
        write!(stdout, "{}", cursor::Goto(1, (line_number + 1) as u16)).unwrap();
        for (column_number, c) in line.char_indices() {
            if (line_number > start_point.row && line_number < end_point.row)
                || (line_number == start_point.row
                    && column_number >= start_point.column
                    && line_number < end_point.row)
                || (line_number == end_point.row
                    && column_number < end_point.column
                    && line_number > start_point.row)
                || (line_number == start_point.row
                    && column_number >= start_point.column
                    && line_number == end_point.row
                    && column_number < end_point.column)
            {
                write!(stdout, "{}{}", color::Bg(color::LightGreen), c).unwrap();
            } else {
                write!(stdout, "{}{}", color::Bg(color::Reset), c).unwrap();
            }
        }
        write!(stdout, "\n").unwrap();
    }

    write!(stdout, "{}", style::Reset).unwrap();

    let point = state.get_cursor_point();
    write!(
        stdout,
        "{}",
        Goto((point.column + 1) as u16, (point.row + 1) as u16,)
    )
    .unwrap();
}
