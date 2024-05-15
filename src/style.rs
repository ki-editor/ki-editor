use crate::{
    grid::{CellLine, CellLineStyle},
    themes::Color,
};

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub(crate) struct Style {
    pub(crate) foreground_color: Option<Color>,
    pub(crate) background_color: Option<Color>,
    pub(crate) line: Option<CellLine>,
    pub(crate) is_bold: bool,
}

pub const fn fg(color: Color) -> Style {
    Style::new().foreground_color(color)
}

impl Style {
    pub const fn new() -> Style {
        Style {
            foreground_color: None,
            background_color: None,
            line: None,
            is_bold: false,
        }
    }

    pub const fn foreground_color(self, color: Color) -> Style {
        Style {
            foreground_color: Some(color),
            ..self
        }
    }

    pub const fn background_color(self, color: Color) -> Style {
        Style {
            background_color: Some(color),
            ..self
        }
    }

    pub(crate) fn set_some_background_color(self, background_color: Option<Color>) -> Style {
        Style {
            background_color,
            ..self
        }
    }

    pub(crate) const fn line(self, line: Option<CellLine>) -> Style {
        Style { line, ..self }
    }

    pub(crate) const fn underline(self, color: Color) -> Style {
        self.line(Some(CellLine {
            color,
            style: CellLineStyle::Underline,
        }))
    }

    pub(crate) const fn undercurl(&self, color: Color) -> Style {
        self.line(Some(CellLine {
            color,
            style: CellLineStyle::Undercurl,
        }))
    }

    pub(crate) const fn bold(self) -> Style {
        Style {
            is_bold: true,
            ..self
        }
    }
}
