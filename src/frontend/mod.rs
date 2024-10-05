pub(crate) mod crossterm;
#[cfg(test)]
pub(crate) mod mock;

use std::any::Any;
use std::io::Write;
#[cfg(test)]
use std::io::{self};

use crate::{app::Dimension, components::component::Cursor, screen::Screen};
use ::crossterm::{
    cursor::MoveTo,
    queue,
    style::{
        Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
        SetUnderlineColor,
    },
};
use itertools::Itertools;

pub(crate) trait Frontend {
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
        for cell in cells {
            queue!(
                self.writer(),
                MoveTo(cell.position.column as u16, cell.position.line as u16),
                SetAttribute(if cell.cell.is_bold {
                    Attribute::Bold
                } else {
                    Attribute::NoBold
                }),
                SetUnderlineColor(
                    cell.cell
                        .line
                        .map(|line| line.color.into())
                        .unwrap_or(Color::Reset),
                ),
                SetAttribute(
                    cell.cell
                        .line
                        .map(|line| match line.style {
                            crate::grid::CellLineStyle::Undercurl => Attribute::Undercurled,
                            crate::grid::CellLineStyle::Underline => Attribute::Underlined,
                        })
                        .unwrap_or(Attribute::NoUnderline),
                ),
                SetBackgroundColor(cell.cell.background_color.into()),
                SetForegroundColor(cell.cell.foreground_color.into()),
                Print(reveal(&cell.cell.symbol)),
                SetAttribute(Attribute::Reset),
            )?;
        }
        Ok(())
    }
}
/// Convert invisible character to visible character
fn reveal(s: &str) -> String {
    match s {
        "\n" => " ".to_string(),
        "\t" => " ".to_string(),
        _ => s.into(),
    }
}

pub(crate) trait MyWriter: Write + Any {
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
pub(crate) struct StringWriter {
    buffer: Vec<u8>,
}

#[cfg(test)]
impl StringWriter {
    pub(crate) fn new() -> Self {
        StringWriter { buffer: Vec::new() }
    }

    pub(crate) fn get_string(&self) -> String {
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
pub(crate) struct NullWriter;

#[cfg(test)]
impl Write for NullWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Ok(0)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
