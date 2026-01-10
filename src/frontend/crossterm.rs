use crate::{components::component::Cursor, screen::Screen};
use std::io::{self};

use super::{Frontend, MyWriter};

pub struct Crossterm {
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
    pub fn new() -> anyhow::Result<Crossterm> {
        Ok(Crossterm {
            stdout: Box::new(io::stdout()),
            previous_screen: Screen::default(),
        })
    }
}

use crossterm::{
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        KeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute, queue,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

impl Frontend for Crossterm {
    fn get_terminal_dimension(&self) -> anyhow::Result<crate::app::Dimension> {
        // First try to get dimensions from environment variables
        // This is particularly useful for VSCode integration where terminal access is limited
        if let (Ok(width_str), Ok(height_str)) = (
            std::env::var("KI_TERMINAL_WIDTH"),
            std::env::var("KI_TERMINAL_HEIGHT"),
        ) {
            if let (Ok(width), Ok(height)) =
                (width_str.parse::<usize>(), height_str.parse::<usize>())
            {
                log::debug!("Using terminal dimensions from environment: {width}x{height}");
                return Ok(crate::app::Dimension { width, height });
            }
        }

        // Fallback to crossterm terminal size detection
        match terminal::size() {
            Ok((width, height)) => {
                log::debug!("Detected terminal dimensions: {width}x{height}");
                Ok(crate::app::Dimension {
                    width: width as usize,
                    height: height as usize,
                })
            }
            Err(e) => {
                log::warn!("Failed to get terminal dimensions: {e}");
                // If terminal size detection fails, use reasonable defaults
                let width = 100;
                let height = 30;
                log::debug!("Using default dimensions: {width}x{height}");
                Ok(crate::app::Dimension { width, height })
            }
        }
    }

    fn enter_alternate_screen(&mut self) -> anyhow::Result<()> {
        self.stdout.execute(EnterAlternateScreen)?;
        self.stdout.execute(EnableBracketedPaste)?;

        // Enable [Kitty's Keyboard Protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/)
        // so that we can detect Key Release events
        // which is crucial for implementing momentary layers
        self.stdout.execute(PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
        ))?;
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
        // Check if we're in VSCode mode
        if let Ok(value) = std::env::var("KI_TERMINAL_WIDTH") {
            if !value.is_empty() {
                log::debug!("Skipping raw mode in VSCode integration mode");
                return Ok(());
            }
        }

        // Try to enable raw mode, but handle errors gracefully
        match crossterm::terminal::enable_raw_mode() {
            Ok(_) => {
                log::debug!("Raw mode enabled successfully");
                Ok(())
            }
            Err(e) => {
                if let Some(raw_code) = e.raw_os_error() {
                    if raw_code == 35 {
                        // Resource temporarily unavailable
                        log::warn!("Ignoring resource temporarily unavailable error when enabling raw mode");
                        return Ok(());
                    }
                }
                // For other errors, propagate them
                log::error!("Failed to enable raw mode: {e}");
                Err(anyhow::anyhow!("Failed to enable raw mode: {}", e))
            }
        }
    }

    fn disable_raw_mode(&mut self) -> anyhow::Result<()> {
        // Check if we're in VSCode mode
        if let Ok(value) = std::env::var("KI_TERMINAL_WIDTH") {
            if !value.is_empty() {
                log::debug!("Skipping disable raw mode in VSCode integration mode");
                return Ok(());
            }
        }

        // Try to disable raw mode, but handle errors gracefully
        match crossterm::terminal::disable_raw_mode() {
            Ok(_) => {
                log::debug!("Raw mode disabled successfully");
                Ok(())
            }
            Err(e) => {
                if let Some(raw_code) = e.raw_os_error() {
                    if raw_code == 35 {
                        // Resource temporarily unavailable
                        log::warn!("Ignoring resource temporarily unavailable error when disabling raw mode");
                        return Ok(());
                    }
                }
                // For other errors, log but don't fail
                log::warn!("Failed to disable raw mode: {e}");
                Ok(()) // Return OK to avoid crashing on exit
            }
        }
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
