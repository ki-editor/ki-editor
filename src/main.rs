mod engine;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{EnableMouseCapture, MouseButton, MouseEventKind};
use crossterm::queue;
use crossterm::style::Print;
use crossterm::{cursor::SetCursorStyle, event::Event, terminal};
use log::LevelFilter;

use engine::{CharIndex, State};
use ropey::RopeSlice;
use std::io::{stdout, Write};
use tree_sitter::{Parser, Point};

fn main() {
    simple_logging::log_to_file("my_log.txt", LevelFilter::Info).unwrap();
    let rust_source_code = r#"fn handle_event(source_code: &str) {
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

use crossterm::{
    event::read,
    terminal::{disable_raw_mode, enable_raw_mode},
};

use crate::engine::{CursorDirection, Mode};

fn handle_event(source_code: &str) {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_rust::language()).unwrap();
    let tree = parser.parse(source_code, None).unwrap();
    enable_raw_mode().unwrap();

    let mut state = State::new(source_code.into(), tree);
    let (columns, rows) = terminal::size().unwrap();
    let mut view = View {
        scroll_offset: 0,
        columns,
        rows,
        stdout: stdout(),
    };

    view.stdout.execute(EnableMouseCapture).unwrap();
    view.render(&state).unwrap();
    loop {
        let event = read().unwrap();
        match event {
            Event::Key(event) => state.handle_key_event(event),
            Event::Resize(columns, rows) => {
                view.set_columns(columns);
                view.set_row(rows)
            }
            Event::Mouse(mouse_event) => match mouse_event.kind {
                MouseEventKind::ScrollUp => {
                    view.scroll_offset = view.scroll_offset.saturating_sub(1)
                }
                MouseEventKind::ScrollDown => {
                    view.scroll_offset = view.scroll_offset.saturating_add(1)
                }
                MouseEventKind::Down(MouseButton::Left) => state
                    .set_cursor_position(mouse_event.row + view.scroll_offset, mouse_event.column),
                _ => {}
            },
            _ => {
                log::info!("{:?}", event)
            }
        }
        if state.quit {
            view.stdout.execute(Clear(ClearType::All)).unwrap();
            break;
        }
        view.render(&state).unwrap();
        view.stdout.flush().unwrap();
    }
    disable_raw_mode().unwrap();
}

struct View {
    /// Zero-based index.
    /// 2 means the first line to be rendered on the screen if the 3rd line of the text.
    scroll_offset: u16,

    rows: u16,
    columns: u16,
    stdout: std::io::Stdout,
}

impl View {
    fn set_columns(&mut self, columns: u16) {
        self.columns = columns;
    }

    fn set_row(&mut self, rows: u16) {
        self.rows = rows;
    }

    fn move_cursor(&mut self, point: Point) -> Result<(), anyhow::Error> {
        // Hide the cursor if the point is out of view
        if point.row as u16 - self.scroll_offset >= self.rows {
            queue!(self.stdout, Hide)?;
        } else {
            queue!(self.stdout, Show)?;
            queue!(
                self.stdout,
                MoveTo(
                    point.column as u16,
                    point.row as u16 - self.scroll_offset as u16
                )
            )?;
        }
        Ok(())
    }

    fn render(&mut self, state: &State) -> Result<(), anyhow::Error> {
        match state.mode {
            Mode::Insert => {
                queue!(self.stdout, SetCursorStyle::BlinkingBar)?;
            }
            _ => {
                queue!(self.stdout, SetCursorStyle::SteadyBar)?;
            }
        }
        queue!(self.stdout, Clear(ClearType::All))?;

        let selection = &state.selection;
        let start_char_index = selection.start.0;
        let end_char_index = selection.end.0;

        let extended_selection = state.get_extended_selection();

        let lines = state
            .text
            .lines()
            .enumerate()
            .skip(self.scroll_offset.into())
            .take(self.rows as usize - 1)
            .collect::<Vec<(_, RopeSlice)>>();

        for (line_index, line) in lines {
            let line_start_char_index = CharIndex(state.text.line_to_char(line_index));
            for (local_char_index, c) in line.chars().enumerate() {
                let char_index = line_start_char_index + local_char_index;

                self.move_cursor(char_index.to_point(&state.text))?;

                let char_index = char_index.0;
                if let Some(ref extended_selection) = extended_selection {
                    let x_start_point = extended_selection.start.0;
                    let x_end_point = extended_selection.end.0;
                    if start_char_index <= char_index
                        && char_index < end_char_index
                        && x_start_point <= char_index
                        && char_index < x_end_point
                    {
                        queue!(self.stdout, SetBackgroundColor(Color::Green))?;
                    } else if x_start_point <= char_index && char_index < x_end_point {
                        queue!(self.stdout, SetBackgroundColor(Color::Cyan))?;
                    } else if start_char_index <= char_index && char_index < end_char_index {
                        queue!(self.stdout, SetBackgroundColor(Color::Yellow))?;
                    } else {
                        queue!(self.stdout, SetBackgroundColor(Color::Reset))?;
                    }
                } else if start_char_index <= char_index && char_index < end_char_index {
                    queue!(self.stdout, SetBackgroundColor(Color::Yellow))?;
                } else {
                    queue!(self.stdout, SetBackgroundColor(Color::Reset))?;
                }
                queue!(self.stdout, Print(c))?;
            }
        }

        for (index, jump) in state.jumps().into_iter().enumerate() {
            let point = match state.cursor_direction {
                CursorDirection::Start => jump.selection.start,
                CursorDirection::End => jump.selection.end,
            }
            .to_point(&state.text);
            self.move_cursor(point)?;
            // Background color: Odd index red, even index blue
            if index % 2 == 0 {
                queue!(self.stdout, SetBackgroundColor(Color::Red))?;
            } else {
                queue!(self.stdout, SetBackgroundColor(Color::Blue))?;
            }
            queue!(self.stdout, SetForegroundColor(Color::White))?;
            queue!(self.stdout, Print(jump.character))?;
        }

        queue!(self.stdout, ResetColor)?;

        let point = state.get_cursor_point();
        self.move_cursor(point)?;
        Ok(())
    }
}
