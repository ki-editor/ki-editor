use std::{collections::HashMap, io::stdout};

use crossterm::{
    cursor::MoveTo,
    event::{EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind},
    queue,
    style::{Color, Print, SetForegroundColor},
    terminal, ExecutableCommand,
};
use tree_sitter::Point;

use crate::{engine::Buffer, window::Window};

pub struct Screen {
    height: u16,
    width: u16,
    windows: Vec<Window>,
    focused_window_index: usize,
    buffers: HashMap<usize, Buffer>,
}

impl Screen {
    pub fn new() -> Screen {
        let (width, height) = terminal::size().unwrap();
        Screen {
            windows: vec![],
            height,
            width,
            focused_window_index: 0,
            buffers: HashMap::new(),
        }
    }

    pub fn run(&mut self, entry_buffer: Buffer) -> Result<(), anyhow::Error> {
        crossterm::terminal::enable_raw_mode()?;

        let buffer_id = self.add_buffer(entry_buffer);

        let mut stdout = stdout();
        self.add_window(Window::new(buffer_id));

        stdout.execute(EnableMouseCapture).unwrap();

        loop {
            // Generate layout
            let (rectangles, borders) =
                Rectangle::generate(self.windows.len(), self.width.into(), self.height.into());

            // Log windows length
            log::info!("{}", self.windows.len());

            // Render every window
            for (window, rectangle) in self.windows.iter_mut().zip(rectangles.into_iter()) {
                let buffer = self.buffers.get(&window.buffer_id()).unwrap();
                window.render(&buffer, &rectangle, &mut stdout)?;
                window.flush(&mut stdout);
            }

            // Render every border
            for border in borders {
                match border.direction {
                    BorderDirection::Horizontal => {
                        for i in 0..border.length {
                            // Set foreground color to black
                            queue!(stdout, SetForegroundColor(Color::Black))?;
                            queue!(
                                stdout,
                                MoveTo(
                                    border.start.column as u16 + i as u16,
                                    border.start.row as u16
                                ),
                                Print("─")
                            )?;
                        }
                    }
                    BorderDirection::Vertical => {
                        for i in 0..border.length {
                            // Set foreground color to black
                            queue!(stdout, SetForegroundColor(Color::Black))?;
                            queue!(
                                stdout,
                                MoveTo(
                                    border.start.column as u16,
                                    border.start.row as u16 + i as u16
                                ),
                                Print("│")
                            )?;
                        }
                    }
                }
                // border.render()?;
            }

            // Pass event to focused window
            let window = self.windows.get_mut(self.focused_window_index).unwrap();
            let buffer = self.buffers.get_mut(&window.buffer_id()).unwrap();
            let event = crossterm::event::read()?;

            match event {
                Event::Key(event) => match event.code {
                    KeyCode::Char('%') => {
                        let buffer_id = window.buffer_id().clone();
                        self.windows.push(Window::new(buffer_id));
                        self.focused_window_index = self.windows.len() - 1;
                        continue;
                    }
                    KeyCode::Char('q') if event.modifiers == KeyModifiers::CONTROL => {
                        // Remove current window
                        self.windows.remove(self.focused_window_index);
                        self.focused_window_index = self.focused_window_index.saturating_sub(1);

                        // TODO: remove this break
                        break;

                        continue;
                    }
                    _ => buffer.handle_key_event(event),
                },
                Event::Resize(columns, rows) => {
                    self.width = columns;
                    self.height = rows;
                }
                Event::Mouse(mouse_event) => {
                    const SCROLL_HEIGHT: isize = 1;
                    match mouse_event.kind {
                        MouseEventKind::ScrollUp => {
                            window.apply_scroll(-SCROLL_HEIGHT);
                        }
                        MouseEventKind::ScrollDown => {
                            window.apply_scroll(SCROLL_HEIGHT);
                        }
                        MouseEventKind::Down(MouseButton::Left) => buffer.set_cursor_position(
                            mouse_event.row + window.scroll_offset(),
                            mouse_event.column,
                        ),
                        _ => continue,
                    }
                }
                _ => {
                    log::info!("{:?}", event);

                    // Don't render for unknown events
                    continue;
                }
            }
        }
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    }

    fn add_buffer(&mut self, entry_buffer: Buffer) -> usize {
        // Look for a free buffer ID
        let mut buffer_ids = self.buffers.keys().cloned().collect::<Vec<_>>();
        buffer_ids.sort();

        let buffer_id = (0..*buffer_ids.iter().max().unwrap_or(&0))
            .find(|id| !buffer_ids.contains(id))
            .unwrap_or(0);

        self.buffers.insert(buffer_id, entry_buffer);
        buffer_id
    }

    fn add_window(&mut self, buffer_id: Window) {
        self.windows.push(buffer_id);
    }
}

#[derive(Debug, PartialEq, Eq)]
// A struct to represent a rectangle with origin, width and height
pub struct Rectangle {
    pub origin: Point,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, PartialEq, Eq)]
// A struct to represent a border with direction, start and length
struct Border {
    direction: BorderDirection,
    start: Point,
    length: usize,
}

#[derive(Debug, PartialEq, Eq)]
// An enum to represent the direction of a border (horizontal or vertical)
enum BorderDirection {
    Horizontal,
    Vertical,
}

impl Rectangle {
    // A method to split a rectangle into two smaller ones based on a fixed ratio of 0.5 and return a border between them
    fn split(&self, vertical: bool) -> (Rectangle, Rectangle, Border) {
        if vertical {
            // Split vertically
            let width1 = self.width / 2;
            let width2 = self.width - width1 - 1; // Corrected the width2 to leave space for the border
            let rectangle1 = Rectangle {
                width: width1,
                ..*self
            };
            let rectangle2 = Rectangle {
                origin: Point {
                    column: self.origin.column + width1 + 1,
                    ..self.origin
                },
                width: width2,
                ..*self
            };
            // Create a vertical border between the two rectangles
            let border = Border {
                direction: BorderDirection::Vertical,
                start: Point {
                    column: self.origin.column + width1,
                    row: self.origin.row,
                },
                length: self.height,
            };
            (rectangle1, rectangle2, border)
        } else {
            // Split horizontally
            let height1 = self.height / 2;
            let height2 = self.height - height1 - 1; // Corrected the height2 to leave space for the border
            let rectangle1 = Rectangle {
                height: height1,
                ..*self
            };
            let rectangle2 = Rectangle {
                origin: Point {
                    row: self.origin.row + height1 + 1,
                    ..self.origin
                },
                height: height2,
                ..*self
            };
            // Create a horizontal border between the two rectangles
            let border = Border {
                direction: BorderDirection::Horizontal,
                start: Point {
                    row: self.origin.row + height1,
                    column: self.origin.column,
                },
                length: self.width,
            };
            (rectangle1, rectangle2, border)
        }
    }

    // A method to generate a vector of rectangles and a vector of borders based on bspwm tiling algorithm
    fn generate(
        count: usize,
        screen_width: usize,
        screen_height: usize,
    ) -> (Vec<Rectangle>, Vec<Border>) {
        // Create an empty vector to store the rectangles
        let mut rectangles = Vec::new();

        // Create an empty vector to store the borders
        let mut borders = Vec::new();

        // Create a root rectangle that covers the whole screen
        let root = Rectangle {
            origin: Point { row: 0, column: 0 },
            width: screen_width,
            height: screen_height,
        };

        // Push the root rectangle to the vector
        rectangles.push(root);

        // Loop through the count and split the last rectangle in the vector
        for _ in 0..count - 1 {
            // Pop the last rectangle from the vector
            let last = rectangles.pop().unwrap();

            // Choose the direction to split based on the rectangle's height and width
            let cursor_width_to_cursor_height_ratio = 3;
            let vertical = last.width >= last.height * cursor_width_to_cursor_height_ratio;

            // Split the last rectangle into two smaller ones and get a border between them
            let (rectangle1, rectangle2, border) = last.split(vertical);

            // Push the two smaller rectangles to the vector
            rectangles.push(rectangle1);
            rectangles.push(rectangle2);

            // Push the border to the vector
            borders.push(border);
        }

        // Return the vector of rectangles and the vector of borders
        (rectangles, borders)
    }
}

#[cfg(test)]
mod test_rectangle {
    use tree_sitter::Point;

    use crate::screen::{Border, BorderDirection::*};

    use super::Rectangle;

    #[test]
    fn generate_height_larger_than_width() {
        let (rectangles, borders) = Rectangle::generate(4, 100, 50);
    }

    #[test]
    fn generate_same_height_and_width() {
        let (rectangles, borders) = Rectangle::generate(4, 100, 100);

        assert_eq!(rectangles.len(), 4);
        assert_eq!(borders.len(), 3);

        assert_eq!(
            borders,
            vec![
                Border {
                    direction: Vertical,
                    start: Point { row: 0, column: 50 },
                    length: 100
                },
                Border {
                    direction: Horizontal,
                    start: Point {
                        row: 50,
                        column: 51
                    },
                    length: 49
                },
                Border {
                    direction: Vertical,
                    start: Point {
                        row: 51,
                        column: 75
                    },
                    length: 49
                }
            ]
        );

        assert_eq!(
            rectangles,
            vec![
                Rectangle {
                    origin: Point { row: 0, column: 0 },
                    width: 50,
                    height: 100
                },
                Rectangle {
                    origin: Point { row: 0, column: 51 },
                    width: 49,
                    height: 50
                },
                Rectangle {
                    origin: Point {
                        row: 51,
                        column: 51
                    },
                    width: 24,
                    height: 49
                },
                Rectangle {
                    origin: Point {
                        row: 51,
                        column: 76
                    },
                    width: 24,
                    height: 49
                }
            ]
        );
    }
}
