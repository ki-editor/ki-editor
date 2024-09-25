use crate::{components::component::Cursor, screen::Screen};
use std::any::Any;
use std::io::{self, Write};

use super::Frontend;

pub(crate) struct Crossterm {
    stdout: Box<dyn MyWriter>,
    /// Used for diffing to reduce unnecessary re-painting.
    previous_screen: Screen,
}

impl Crossterm {
    #[cfg(test)]
    pub(crate) fn string_content(&self) -> Option<String> {
        self.stdout
            .as_any()
            .downcast_ref::<StringWriter>()
            .map(|writer| writer.get_string())
    }
}

#[cfg(test)]
struct StringWriter {
    buffer: Vec<u8>,
}
trait MyWriter: Write + Any {
    #[cfg(test)]
    fn as_any(&self) -> &dyn Any;
}

#[cfg(test)]
impl MyWriter for StringWriter {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl MyWriter for std::io::Stdout {
    #[cfg(test)]
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
impl StringWriter {
    fn new() -> Self {
        StringWriter { buffer: Vec::new() }
    }

    fn get_string(&self) -> String {
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

pub(crate) enum StdoutKind {
    #[cfg(test)]
    String,
    Io,
}

impl Crossterm {
    pub(crate) fn new(stdout_kind: StdoutKind) -> anyhow::Result<Crossterm> {
        let output: Box<dyn MyWriter> = match stdout_kind {
            #[cfg(test)]
            StdoutKind::String => Box::new(StringWriter::new()),
            StdoutKind::Io => Box::new(io::stdout()),
        };
        Ok(Crossterm {
            stdout: output,
            previous_screen: Screen::default(),
        })
    }
}

use crossterm::{
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute, queue,
    style::{
        Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
        SetUnderlineColor,
    },
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

impl Frontend for Crossterm {
    fn get_terminal_dimension(&self) -> anyhow::Result<crate::app::Dimension> {
        let (width, height) = terminal::size()?;
        Ok(crate::app::Dimension { width, height })
    }
    fn enter_alternate_screen(&mut self) -> anyhow::Result<()> {
        self.stdout.execute(EnterAlternateScreen)?;
        self.stdout.execute(EnableBracketedPaste)?;
        Ok(())
    }

    fn enable_mouse_capture(&mut self) -> anyhow::Result<()> {
        self.stdout.execute(EnableMouseCapture)?;
        Ok(())
    }

    fn disable_mouse_capture(&mut self) -> anyhow::Result<()> {
        self.stdout.execute(DisableMouseCapture)?;
        Ok(())
    }

    fn leave_alternate_screen(&mut self) -> anyhow::Result<()> {
        self.stdout.execute(LeaveAlternateScreen)?;
        self.stdout.execute(DisableBracketedPaste)?;
        Ok(())
    }

    fn enable_raw_mode(&mut self) -> anyhow::Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        Ok(())
    }

    fn disable_raw_mode(&mut self) -> anyhow::Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    }

    fn show_cursor(&mut self, cursor: &Cursor) -> anyhow::Result<()> {
        let style: SetCursorStyle = cursor.style().into();
        queue!(self.stdout, Show, style)?;
        execute!(
            self.stdout,
            MoveTo(
                cursor.position().column as u16,
                cursor.position().line as u16
            )
        )?;
        Ok(())
    }

    fn hide_cursor(&mut self) -> anyhow::Result<()> {
        queue!(self.stdout, Hide)?;
        Ok(())
    }

    fn clear_screen(&mut self) -> anyhow::Result<()> {
        queue!(self.stdout, Clear(ClearType::All))?;
        Ok(())
    }

    fn render_screen(&mut self, mut screen: Screen) -> anyhow::Result<()> {
        let cells = {
            // Only perform diff if the dimension is the same
            let diff = if self.previous_screen.dimension() == screen.dimension() {
                screen.diff(&mut self.previous_screen)
            } else {
                self.clear_screen()?;
                screen.get_positioned_cells()
            };
            self.previous_screen = screen;

            diff
        };
        for cell in cells {
            queue!(
                self.stdout,
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
