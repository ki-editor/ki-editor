mod engine;

use log::LevelFilter;

use engine::{CharIndex, State};
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

const x = fibonacci(10);
console.log(x);
        ";
    handle_event(source_code)
}

fn render(code: &str, state: &State, stdout: &mut impl Write) {
    write!(stdout, "{}", clear::All).unwrap();

    let selection = &state.selection;
    let start_point = selection.start.0;
    let end_point = selection.end.0;
    state
        .source_code
        .chars()
        .enumerate()
        .for_each(|(index, c)| {
            let point = CharIndex(index).to_point(&state.source_code);

            write!(
                stdout,
                "{}",
                Goto((point.column + 1) as u16, (point.row + 1) as u16)
            )
            .unwrap();

            if start_point <= index && index < end_point {
                write!(stdout, "{}{}", color::Bg(color::LightGreen), c).unwrap();
            } else {
                write!(stdout, "{}{}", color::Bg(color::Reset), c).unwrap();
            };
        });
    write!(stdout, "{}", style::Reset).unwrap();

    let point = state.get_cursor_point();
    write!(
        stdout,
        "{}",
        Goto((point.column + 1) as u16, (point.row + 1) as u16,)
    )
    .unwrap();
}

fn handle_event(source_code: &str) {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_javascript::language())
        .unwrap();
    let tree = parser.parse(source_code, None).unwrap();
    let stdin = stdin();
    let mut stdout = stdout().into_raw_mode().unwrap();
    let mut state = State::new(source_code.into(), tree.root_node());
    render(&source_code, &state, &mut stdout);
    let root_node = tree.root_node();
    for c in stdin.keys() {
        match c.unwrap() {
            Key::Char('p') => {
                state.select_parent();
            }
            Key::Char('k') => {
                state.select_child();
            }
            Key::Char('s') => {
                state.select_sibling();
            }
            Key::Char('l') => state.select_line(),
            Key::Char('b') => state.select_backward(),
            Key::Char('o') => state.change_cursor_direction(),
            Key::Ctrl('c') => break,
            _ => {}
        }
        render(&source_code, &state, &mut stdout);
        stdout.flush().unwrap();
    }
}
