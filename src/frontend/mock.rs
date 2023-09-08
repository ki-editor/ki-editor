#! #[cfg(test)]

use crate::{components::component::Cursor, grid::Grid};
#[derive(Clone)]
pub struct MockFrontend {
    grid: Option<Grid>,
}

const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;
const DIMENSION: crate::screen::Dimension = crate::screen::Dimension {
    width: WIDTH,
    height: HEIGHT,
};

impl super::frontend::Frontend for MockFrontend {
    fn get_terminal_dimension(&self) -> anyhow::Result<crate::screen::Dimension> {
        Ok(DIMENSION)
    }

    fn enter_alternate_screen(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn enable_mouse_capture(&mut self) -> anyhow::Result<()> {
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

    fn render_grid(&mut self, grid: Grid) -> anyhow::Result<()> {
        self.grid = Some(grid);
        Ok(())
    }
}

impl MockFrontend {
    pub fn new() -> Self {
        Self { grid: None }
    }
    pub fn content(&self) -> String {
        self.grid
            .as_ref()
            .map(|grid| grid.content())
            .unwrap_or_default()
    }
}
