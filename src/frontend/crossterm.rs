use crate::grid::Grid;

use super::frontend::Frontend;

pub struct Crossterm {
    stdout: std::io::Stdout,
    /// Used for diffing to reduce unnecessary re-painting.
    previous_grid: Option<Grid>,
}

impl Crossterm {
    pub fn new() -> Crossterm {
        Crossterm {
            stdout: std::io::stdout(),
            previous_grid: None,
        }
    }
}

use crossterm::{
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    event::EnableMouseCapture,
    execute, queue,
    style::{
        Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
        SetUnderlineColor,
    },
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

impl Frontend for Crossterm {
    fn get_terminal_dimension(&self) -> anyhow::Result<crate::screen::Dimension> {
        let (width, height) = terminal::size()?;
        Ok(crate::screen::Dimension { width, height })
    }
    fn enter_alternate_screen(&mut self) -> anyhow::Result<()> {
        self.stdout.execute(EnterAlternateScreen)?;
        Ok(())
    }

    fn enable_mouse_capture(&mut self) -> anyhow::Result<()> {
        self.stdout.execute(EnableMouseCapture)?;
        Ok(())
    }

    fn leave_alternate_screen(&mut self) -> anyhow::Result<()> {
        self.stdout.execute(LeaveAlternateScreen)?;
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

    fn show_cursor(&mut self, position: &crate::position::Position) -> anyhow::Result<()> {
        queue!(self.stdout, Show, SetCursorStyle::BlinkingBlock)?;
        execute!(
            self.stdout,
            MoveTo(position.column as u16, position.line as u16)
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

    fn render_grid(&mut self, grid: Grid) -> anyhow::Result<()> {
        let cells = {
            let diff = match self.previous_grid {
                // Only perform diff if the dimension is the same
                Some(ref previous_grid) if previous_grid.dimension() == grid.dimension() => {
                    previous_grid.diff(&grid)
                }
                _ => {
                    self.clear_screen()?;
                    grid.to_positioned_cells()
                }
            };

            self.previous_grid = Some(grid);

            diff
        };
        for cell in cells {
            queue!(
                self.stdout,
                MoveTo(cell.position.column as u16, cell.position.line as u16),
                SetUnderlineColor(cell.cell.undercurl.unwrap_or(Color::Reset)),
                SetAttribute(if cell.cell.undercurl.is_some() {
                    Attribute::Undercurled
                } else {
                    Attribute::NoUnderline
                }),
                SetBackgroundColor(cell.cell.background_color),
                SetForegroundColor(cell.cell.foreground_color),
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
        _ => s.into(),
    }
}
