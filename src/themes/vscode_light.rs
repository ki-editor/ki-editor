use super::Theme;
use crate::{grid::Style, themes::ThemeStyles};
use crossterm::style::Color;
use lazy_static::lazy_static;

fn theme() -> Theme {
    Theme {
        name: "vscode-light",
        styles: ThemeStyles {
            keyword: Some(Style::new().foreground_color(Color::Blue)),
            variable: Some(Style::new().foreground_color(Color::DarkBlue)),
            function: Some(Style::new().foreground_color(Color::DarkMagenta)),
            type_: Some(Style::new().foreground_color(Color::DarkGreen)),
            ..Default::default()
        },
    }
}

lazy_static! {
    pub static ref VSCODE_LIGHT: Theme = theme();
}
