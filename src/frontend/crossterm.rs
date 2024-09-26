use crate::{components::component::Cursor, screen::Screen};
use std::io::{self};

use super::{Frontend, MyWriter};

pub(crate) struct Crossterm {
    stdout: Box<dyn MyWriter>,
    /// Used for diffing to reduce unnecessary re-painting.
    previous_screen: Screen,
}

impl MyWriter for std::io::Stdout {
    #[cfg(test)]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Crossterm {
    pub(crate) fn new() -> anyhow::Result<Crossterm> {
        Ok(Crossterm {
            stdout: Box::new(io::stdout()),
            previous_screen: Screen::default(),
        })
    }
}

use crossterm::{
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute, queue,
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
        crossterm::terminal::enable_raw_mode()?; // This should be the issue
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

    fn writer(&mut self) -> &mut Box<dyn MyWriter> {
        &mut self.stdout
    }

    fn previous_screen(&mut self) -> Screen {
        std::mem::take(&mut self.previous_screen)
    }

    fn set_previous_screen(&mut self, previous_screen: Screen) {
        self.previous_screen = previous_screen
    }
}
