mod engine;

use crossterm::cursor::SetCursorStyle;
use crossterm::queue;
use crossterm::style::Print;
use log::LevelFilter;

use engine::{CharIndex, State};
use std::io::{stdout, Write};
use tree_sitter::Parser;

fn main() {
    simple_logging::log_to_file("my_log.txt", LevelFilter::Info).unwrap();
    let rust_source_code = r#"
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
                KeyCode::Char('w') => {
                    state.select_word();
                }
                KeyCode::Char('c') if event.modifiers == KeyModifiers::CONTROL => {
                    stdout.execute(Clear(ClearType::All)).unwrap();
                    break;
                }
            },
            _ => {}
        }
        render(&state, &mut stdout);
        stdout.flush().unwrap();
    }
    disable_raw_mode().unwrap();
}
        "#;
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
    handle_event(rust_source_code)
}

use crossterm::{
    cursor::MoveTo,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
    ExecutableCommand,
};

fn render<'a>(state: &State, stdout: &mut impl Write) {
    fn render<'a>(state: &State, stdout: &mut impl Write) -> Result<(), anyhow::Error> {
        match state.mode {
            Mode::Insert => {
                queue!(stdout, SetCursorStyle::BlinkingBar)?;
            }
            _ => {
                queue!(stdout, SetCursorStyle::SteadyBar)?;
            }
        }
        queue!(stdout, Clear(ClearType::All))?;

        let selection = &state.selection;
        let start_point = selection.start.0;
        let end_point = selection.end.0;

        let extended_selection = state.get_extended_selection();

        for (index, c) in state.source_code.chars().enumerate() {
            let point = CharIndex(index).to_point(&state.source_code);

            queue!(
                stdout,
                MoveTo(point.column as u16 + 1, point.row as u16 + 1)
            )?;
            if let Some(extended_selection) = extended_selection {
                // log::info!("extended_selection: {:?}", extended_selection);
                let x_start_point = extended_selection.start.0;
                let x_end_point = extended_selection.end.0;
                if start_point <= index
                    && index < end_point
                    && x_start_point <= index
                    && index < x_end_point
                {
                    queue!(stdout, SetBackgroundColor(Color::Green))?;
                } else if x_start_point <= index && index < x_end_point {
                    queue!(stdout, SetBackgroundColor(Color::Cyan))?;
                } else if start_point <= index && index < end_point {
                    queue!(stdout, SetBackgroundColor(Color::Yellow))?;
                } else {
                    queue!(stdout, SetBackgroundColor(Color::Reset))?;
                }
            } else if start_point <= index && index < end_point {
                queue!(stdout, SetBackgroundColor(Color::Yellow))?;
            } else {
                queue!(stdout, SetBackgroundColor(Color::Reset))?;
            }
            queue!(stdout, Print(c))?;
        }

        for (index, jump) in state.jumps().into_iter().enumerate() {
            let point = match state.cursor_direction {
                CursorDirection::Start => jump.selection.start,
                CursorDirection::End => jump.selection.end,
            }
            .to_point(&state.source_code);
            queue!(
                stdout,
                MoveTo(point.column as u16 + 1, point.row as u16 + 1)
            )?;
            // Background color: Odd index red, even index blue
            if index % 2 == 0 {
                queue!(stdout, SetBackgroundColor(Color::Red))?;
            } else {
                queue!(stdout, SetBackgroundColor(Color::Blue))?;
            }
            queue!(stdout, SetForegroundColor(Color::White))?;
            queue!(stdout, Print(jump.character))?;
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
    event::read,
    terminal::{disable_raw_mode, enable_raw_mode},
};

use crate::engine::{CursorDirection, Mode};

fn handle_event(source_code: &str) {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_rust::language()).unwrap();
    let tree = parser.parse(source_code, None).unwrap();
    let mut stdout = stdout();
    enable_raw_mode().unwrap();

    let mut state = State::new(source_code.into(), tree);
    render(&state, &mut stdout);
    loop {
        state.handle_event(read().unwrap());
        if state.quit {
            stdout.execute(Clear(ClearType::All)).unwrap();
            break;
        }
        render(&state, &mut stdout);
        stdout.flush().unwrap();
    }
    disable_raw_mode().unwrap();
}
