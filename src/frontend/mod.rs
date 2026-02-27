pub mod crossterm;
#[cfg(test)]
pub mod mock;
mod render;

use std::any::Any;
use std::io::Write;
#[cfg(test)]
use std::io::{self};

use crate::{app::Dimension, components::component::Cursor, screen::Screen};
use itertools::Itertools;

pub trait Frontend {
    fn get_terminal_dimension(&self) -> anyhow::Result<Dimension>;
    fn enter_alternate_screen(&mut self) -> anyhow::Result<()>;
    fn enable_mouse_capture(&mut self) -> anyhow::Result<()>;
    fn disable_mouse_capture(&mut self) -> anyhow::Result<()>;
    fn leave_alternate_screen(&mut self) -> anyhow::Result<()>;
    fn enable_raw_mode(&mut self) -> anyhow::Result<()>;
    fn disable_raw_mode(&mut self) -> anyhow::Result<()>;
    fn show_cursor(&mut self, cursor: &Cursor) -> anyhow::Result<()>;
    fn hide_cursor(&mut self) -> anyhow::Result<()>;
    fn clear_screen(&mut self) -> anyhow::Result<()>;
    fn writer(&mut self) -> &mut Box<dyn MyWriter>;
    fn previous_screen(&mut self) -> Screen;
    fn set_previous_screen(&mut self, previous_screen: Screen);
    fn set_clipboard_with_osc52(&mut self, content: &str) -> anyhow::Result<()>;
    fn render_screen(&mut self, mut screen: Screen) -> anyhow::Result<()> {
        let cells = {
            // Only perform diff if the dimension is the same
            let mut previous_screen = self.previous_screen();
            let diff = if previous_screen.dimension() == screen.dimension() {
                screen.diff(&mut previous_screen)
            } else {
                self.clear_screen()?;
                screen.get_positioned_cells()
            };
            self.set_previous_screen(screen);

            diff
        };

        debug_assert_eq!(
            cells,
            cells
                .clone()
                .into_iter()
                .sorted_by_key(|cell| (cell.position.line, -(cell.position.column as isize)))
                .collect_vec(),
            "Cells should be sorted in reverse order by column to ensure proper rendering of
 multi-width characters in terminal displays"
        );
        render::render_cells(self.writer(), cells)
    }
}

pub trait MyWriter: Write + Any {
    #[cfg(test)]
    fn as_any(&self) -> &dyn Any;
}

#[cfg(test)]
impl MyWriter for StringWriter {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
impl MyWriter for NullWriter {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct StringWriter {
    buffer: Vec<u8>,
}

#[cfg(test)]
impl StringWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_string(&self) -> String {
        String::from_utf8(self.buffer.clone()).unwrap_or_default()
    }
}

#[cfg(test)]
impl Write for StringWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
pub struct NullWriter;

#[cfg(test)]
impl Write for NullWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Ok(0)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
