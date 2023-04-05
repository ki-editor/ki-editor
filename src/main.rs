mod engine;

use crossterm::event::KeyModifiers;
use log::LevelFilter;

use engine::{CharIndex, State};
use std::io::{stdout, Write};
use tree_sitter::Parser;

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

use crossterm::{
    cursor::MoveTo,
    style::{Color, ResetColor, SetBackgroundColor},
    terminal::{Clear, ClearType},
    ExecutableCommand,
};

fn render(state: &State, stdout: &mut impl Write) {
    stdout.execute(Clear(ClearType::All)).unwrap();

    let selection = &state.selection;
    let start_point = selection.start.0;
    let end_point = selection.end.0;
    state
        .source_code
        .chars()
        .enumerate()
        .for_each(|(index, c)| {
            let point = CharIndex(index).to_point(&state.source_code);

            stdout
                .execute(MoveTo(point.column as u16 + 1, point.row as u16 + 1))
                .unwrap();

            if start_point <= index && index < end_point {
                stdout.execute(SetBackgroundColor(Color::Green)).unwrap();
            } else {
                stdout.execute(ResetColor).unwrap();
            }
            write!(stdout, "{}", c).unwrap();
        });
    stdout.execute(ResetColor).unwrap();

    let point = state.get_cursor_point();
    stdout
        .execute(MoveTo(point.column as u16 + 1, point.row as u16 + 1))
        .unwrap();
}

use crossterm::{
    event::{read, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};

fn handle_event(source_code: &str) {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_javascript::language())
        .unwrap();
    let tree = parser.parse(source_code, None).unwrap();
    let mut stdout = stdout();
    enable_raw_mode().unwrap();
    let mut state = State::new(source_code.into(), tree.root_node());
    render(&state, &mut stdout);
    loop {
        match read().unwrap() {
            Event::Key(event) => match event.code {
                KeyCode::Char('p') => {
                    state.select_parent();
                }
                KeyCode::Char('k') => {
                    state.select_child();
                }
                KeyCode::Char('s') => {
                    state.select_sibling();
                }
                KeyCode::Char('l') => state.select_line(),
                KeyCode::Char('b') => state.select_backward(),
                KeyCode::Char('o') => state.change_cursor_direction(),
                KeyCode::Char('c') if event.modifiers == KeyModifiers::CONTROL => break,
                _ => {}
            },
            _ => {}
        }
        render(&state, &mut stdout);
        stdout.flush().unwrap();
    }
    disable_raw_mode().unwrap();
}
