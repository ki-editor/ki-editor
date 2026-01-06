#! #[cfg(test)]

use crate::{components::component::Cursor, screen::Screen};

use super::{MyWriter, StringWriter};

pub(crate) struct MockFrontend {
    /// Used for diffing to reduce unnecessary re-painting.
    previous_screen: Screen,
    writer: Box<dyn MyWriter>,
}

const WIDTH: usize = 80;
const HEIGHT: usize = 24;
const DIMENSION: crate::app::Dimension = crate::app::Dimension {
    width: WIDTH,
    height: HEIGHT,
};

impl MockFrontend {
    pub(crate) fn new(writer: Box<dyn MyWriter>) -> Self {
        Self {
            previous_screen: Default::default(),
            writer,
        }
    }
}

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

    fn previous_screen(&mut self) -> Screen {
        std::mem::take(&mut self.previous_screen)
    }

    fn set_previous_screen(&mut self, previous_screen: Screen) {
        self.previous_screen = previous_screen
    }

    fn writer(&mut self) -> &mut Box<dyn MyWriter> {
        &mut self.writer
    }
}

#[cfg(test)]
impl MockFrontend {
    pub(crate) fn string_content(&self) -> Option<String> {
        self.writer
            .as_any()
            .downcast_ref::<StringWriter>()
            .map(|writer| writer.get_string())
    }
}
