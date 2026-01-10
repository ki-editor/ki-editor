use crate::{
    grid::{CellLine, CellLineStyle},
    themes::Color,
};

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord)]
pub struct Style {
    pub foreground_color: Option<Color>,
    pub background_color: Option<Color>,
    pub line: Option<CellLine>,
    pub is_bold: bool,
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

    pub const fn same_background_foreground(color: Color) -> Style {
        Style {
            foreground_color: Some(color),
            background_color: Some(color),
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

    pub fn set_some_background_color(self, background_color: Option<Color>) -> Style {
        Style {
            background_color,
            ..self
        }
    }

    pub fn set_some_foreground_color(self, foreground_color: Option<Color>) -> Style {
        Style {
            foreground_color,
            ..self
        }
    }

    pub const fn line(self, line: Option<CellLine>) -> Style {
        Style { line, ..self }
    }

    pub const fn underline(self, color: Color) -> Style {
        self.line(Some(CellLine {
            color,
            style: CellLineStyle::Underline,
        }))
    }

    pub const fn undercurl(&self, color: Color) -> Style {
        self.line(Some(CellLine {
            color,
            style: CellLineStyle::Undercurl,
        }))
    }
}
