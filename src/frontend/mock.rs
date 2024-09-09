#! #[cfg(test)]

use crate::{components::component::Cursor, screen::Screen};
#[derive(Clone, Default)]
pub(crate) struct MockFrontend {
    screen: Option<Screen>,
}

const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;
const DIMENSION: crate::app::Dimension = crate::app::Dimension {
    width: WIDTH,
    height: HEIGHT,
};

impl super::Frontend for MockFrontend {
    fn get_terminal_dimension(&self) -> anyhow::Result<crate::app::Dimension> {
        Ok(DIMENSION)
    }

    fn enter_alternate_screen(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn enable_mouse_capture(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn disable_mouse_capture(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn leave_alternate_screen(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn enable_raw_mode(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn disable_raw_mode(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn show_cursor(&mut self, _: &Cursor) -> anyhow::Result<()> {
        Ok(())
    }

    fn hide_cursor(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn clear_screen(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_screen(&mut self, grid: Screen) -> anyhow::Result<()> {
        self.screen = Some(grid);
        Ok(())
    }
}

impl MockFrontend {}
