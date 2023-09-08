use crate::{components::component::Cursor, grid::Grid, position::Position, screen::Dimension};

pub trait Frontend {
    fn get_terminal_dimension(&self) -> anyhow::Result<Dimension>;
    fn enter_alternate_screen(&mut self) -> anyhow::Result<()>;
    fn enable_mouse_capture(&mut self) -> anyhow::Result<()>;
    fn leave_alternate_screen(&mut self) -> anyhow::Result<()>;
    fn enable_raw_mode(&mut self) -> anyhow::Result<()>;
    fn disable_raw_mode(&mut self) -> anyhow::Result<()>;
    fn show_cursor(&mut self, cursor: &Cursor) -> anyhow::Result<()>;
    fn hide_cursor(&mut self) -> anyhow::Result<()>;
    fn clear_screen(&mut self) -> anyhow::Result<()>;
    fn render_grid(&mut self, grid: Grid) -> anyhow::Result<()>;
}
