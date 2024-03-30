use crate::themes::Color;

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Style {
    pub foreground_color: Option<Color>,
    pub background_color: Option<Color>,
    pub undercurl: Option<Color>,
}

pub const fn fg(color: Color) -> Style {
    Style::new().foreground_color(color)
}

pub const fn bg(color: Color) -> Style {
    Style::new().background_color(color)
}

impl Style {
    pub const fn new() -> Style {
        Style {
            foreground_color: None,
            background_color: None,
            undercurl: None,
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

    pub const fn undercurl(self, color: Option<Color>) -> Style {
        Style {
            undercurl: color,
            ..self
        }
    }

    pub(crate) fn set_some_background_color(self, background_color: Option<Color>) -> Style {
        Style {
            background_color,
            ..self
        }
    }
}
