mod engine;

use crossterm::cursor::SetCursorStyle;
use crossterm::queue;
use crossterm::{event::KeyModifiers, style::Print};
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
        return fibonacci(n - 1) + fibonacci(n - 2, a, lol);
    }
}

const x = <div height='24' width='24'>hello world</div>

f(yo, waw)

const x = [{a: 1, b: 2}, {c: 1}, {d: 1}]

/* Hello world
 This is a comment */
const y = `who lives in a pineapple under the sea? ${answer + `${answer + 2} hello`}`
const x = fibonacci(10);
console.log(x);

    const interval = setInterval(() => {
           fetchData()
 }, 60 * 1000)

 import { test_displayRelatedProjectUnit } from './project/test-display-related-project-units'

        ";
    handle_event(source_code)
}

use crossterm::{
    cursor::MoveTo,
    style::{Color, ResetColor, SetBackgroundColor},
    terminal::{Clear, ClearType},
    ExecutableCommand,
};

fn render<'a>(state: &State, stdout: &mut impl Write) {
    fn render<'a>(state: &State, stdout: &mut impl Write) -> Result<(), anyhow::Error> {
        queue!(stdout, Clear(ClearType::All))?;

        let selection = &state.selection;
        let start_point = selection.start.0;
        let end_point = selection.end.0;

        for (index, c) in state.source_code.chars().enumerate() {
            let point = CharIndex(index).to_point(&state.source_code);

            queue!(
                stdout,
                MoveTo(point.column as u16 + 1, point.row as u16 + 1)
            )?;

            if start_point <= index && index < end_point {
                queue!(stdout, SetBackgroundColor(Color::Green))?;
            } else {
                queue!(stdout, SetBackgroundColor(Color::Reset))?;
            }
            queue!(stdout, Print(c))?;
        }
        queue!(stdout, ResetColor)?;

        let point = state.get_cursor_point();
        queue!(
            stdout,
            MoveTo(point.column as u16 + 1, point.row as u16 + 1)
        )?;
        Ok(())
    }
    render(state, stdout).unwrap();
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

    stdout.execute(SetCursorStyle::BlinkingBar).unwrap();
    let mut state = State::new(source_code.into(), tree);
    render(&state, &mut stdout);
    loop {
        match read().unwrap() {
            Event::Key(event) => match event.code {
                // Objects
                KeyCode::Char('a') => {
                    state.select_parent();
                }
                KeyCode::Char('k') => {
                    state.select_child();
                }
                KeyCode::Char('s') => {
                    state.select_sibling();
                }
                KeyCode::Char('w') => {
                    state.select_word();
                }
                KeyCode::Char('t') => state.select_token(),
                KeyCode::Char('n') => state.select_named_token(),
                KeyCode::Char('l') => state.select_line(),
                KeyCode::Char('b') => state.select_backward(),
                KeyCode::Char('o') => state.change_cursor_direction(),
                // Actions
                KeyCode::Char('d') => state.delete_current_selection(),
                KeyCode::Char('p') => state.paste(),
                KeyCode::Char('y') => state.yank(),
                KeyCode::Char('r') => state.replace(),
                KeyCode::Char('c') if event.modifiers == KeyModifiers::CONTROL => {
                    stdout.execute(Clear(ClearType::All)).unwrap();
                    break;
                }
                KeyCode::Char('c') => state.select_charater(),

                _ => {
                    // todo!("Back to previous selection");
                    // todo!("select all children");
                    // todo!("with this we can do select first and last children")
                    // todo!("Search by node kind")
                }
            },
            _ => {}
        }
        render(&state, &mut stdout);
        stdout.flush().unwrap();
    }
    disable_raw_mode().unwrap();
}
