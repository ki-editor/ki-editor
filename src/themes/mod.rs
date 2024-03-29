pub mod vscode_light;
use std::collections::HashMap;

use itertools::Itertools;
pub use vscode_light::VSCODE_LIGHT;

use crate::{grid::StyleKey, style::Style};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub syntax: SyntaxStyles,
    pub ui: UiStyles,
    pub diagnostic: DiagnosticStyles,
    pub hunk_old_background: Color,
    pub hunk_new_background: Color,
    pub hunk_old_emphasized_background: Color,
    pub hunk_new_emphasized_background: Color,
}
impl Theme {
    pub(crate) fn get_style(&self, source: &StyleKey) -> Style {
        match source {
            StyleKey::UiBookmark => self.ui.bookmark,
            StyleKey::UiPrimarySelection => {
                Style::new().background_color(self.ui.primary_selection_background)
            }
            StyleKey::UiPrimarySelectionAnchors => {
                Style::new().background_color(self.ui.primary_selection_anchor_background)
            }
            StyleKey::UiSecondarySelection => {
                Style::new().background_color(self.ui.secondary_selection_background)
            }
            StyleKey::UiSecondarySelectionAnchors => {
                Style::new().background_color(self.ui.secondary_selection_anchor_background)
            }
            StyleKey::UiPossibleSelection => {
                Style::new().background_color(self.ui.possible_selection_background)
            }
            StyleKey::DiagnosticsHint => self.diagnostic.hint,
            StyleKey::DiagnosticsError => self.diagnostic.error,
            StyleKey::DiagnosticsWarning => self.diagnostic.warning,
            StyleKey::DiagnosticsInformation => self.diagnostic.information,
            StyleKey::DiagnosticsDefault => self.diagnostic.default,

            StyleKey::HunkOld => Style::new().background_color(self.hunk_old_background),
            StyleKey::HunkNew => Style::new().background_color(self.hunk_new_background),
            StyleKey::HunkOldEmphasized => {
                Style::new().background_color(self.hunk_old_emphasized_background)
            }
            StyleKey::HunkNewEmphasized => {
                Style::new().background_color(self.hunk_new_emphasized_background)
            }

            StyleKey::Syntax(highlight_group) => {
                self.syntax.get_style(highlight_group).unwrap_or_default()
            }
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        VSCODE_LIGHT
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DiagnosticStyles {
    pub error: Style,
    pub warning: Style,
    pub information: Style,
    pub hint: Style,
    pub default: Style,
}

#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct UiStyles {
    pub global_title: Style,
    pub window_title: Style,
    pub parent_lines_background: Color,
    pub jump_mark_odd: Style,
    pub jump_mark_even: Style,
    pub text: Style,
    pub primary_selection_background: Color,
    pub primary_selection_anchor_background: Color,
    pub primary_selection_secondary_cursor: Style,
    pub secondary_selection_background: Color,
    pub secondary_selection_anchor_background: Color,
    pub possible_selection_background: Color,
    pub secondary_selection_primary_cursor: Style,
    pub secondary_selection_secondary_cursor: Style,
    pub line_number: Style,
    pub line_number_separator: Style,
    pub bookmark: Style,
}

/// Refer https://github.com/nvim-treesitter/nvim-treesitter/blob/23ba63028c6acca29be6462c0a291fc4a1b9eae8/CONTRIBUTING.md#highlights
/// The capture groups should tally with that of `nvim-treesitter` so that their highlight queries can be used in
/// this editor without modifications.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct SyntaxStyles {
    map: once_cell::sync::OnceCell<HashMap<String, Style>>,
    groups: &'static [(&'static str, Style)],
}
impl SyntaxStyles {
    pub const fn new(groups: &'static [(&'static str, Style)]) -> Self {
        Self {
            groups,
            map: once_cell::sync::OnceCell::new(),
        }
    }
    fn map(&self) -> &HashMap<String, Style> {
        self.map.get_or_init(|| {
            self.groups
                .into_iter()
                .map(|(key, style)| {
                    if !HIGHLIGHT_NAMES.contains(key) {
                        panic!("Invalid highlight group: {}", key)
                    }
                    (key.to_string(), style.to_owned())
                })
                .collect()
        })
    }
    fn get_style(&self, highlight_group: &str) -> Option<Style> {
        let group = HighlightGroup::new(highlight_group);
        self.map()
            .get(&group.full_name)
            .cloned()
            .or_else(|| self.get_style(&group.parent?))
    }
}

#[cfg(test)]
mod test_syntax_styles {
    use my_proc_macros::hex;

    use crate::style::fg;

    use super::*;

    const SYNTAX_STYLE: SyntaxStyles = SyntaxStyles::new(&[
        ("string", fg(hex!("#267f99"))),
        ("string.special", fg(hex!("#e50000"))),
        ("variable", fg(hex!("#abcdef"))),
    ]);
    #[test]
    fn test_get_style() {
        assert_eq!(
            SYNTAX_STYLE.get_style("string").unwrap(),
            fg(hex!("#267f99"))
        );
        assert_eq!(
            SYNTAX_STYLE.get_style("string.special").unwrap(),
            fg(hex!("#e50000"))
        );
        assert_eq!(
            SYNTAX_STYLE.get_style("string.special.symbol").unwrap(),
            fg(hex!("#e50000"))
        );
        assert_eq!(
            SYNTAX_STYLE
                .get_style("variable.parameter.builtin")
                .unwrap(),
            fg(hex!("#abcdef"))
        );
    }
}

pub struct HighlightGroup {
    full_name: String,
    parent: Option<String>,
}

impl HighlightGroup {
    fn new(group: &str) -> HighlightGroup {
        match group.split(".").collect_vec().split_last() {
            Some((_, parent)) => HighlightGroup {
                parent: Some(parent.join(".")),
                full_name: group.to_string(),
            },
            None => HighlightGroup {
                parent: None,
                full_name: group.to_string(),
            },
        }
    }
}

pub const HIGHLIGHT_NAMES: &[&str] = &[
    "variable",
    "variable.builtin",
    "variable.parameter",
    "variable.parameter.builtin",
    "variable.member",
    "constant",
    "constant.builtin",
    "constant.macro",
    "module",
    "module.builtin",
    "label",
    "string",
    "string.documentation",
    "string.regexp",
    "string.escape",
    "string.special",
    "string.special.symbol",
    "string.special.url",
    "string.special.path",
    "character",
    "character.special",
    "boolean",
    "number",
    "number.float",
    "type",
    "type.builtin",
    "type.definition",
    "attribute",
    "attribute.builtin",
    "property",
    "function",
    "function.builtin",
    "function.call",
    "function.macro",
    "function.method",
    "function.method.call",
    "constructor",
    "operator",
    "keyword",
    "keyword.coroutine",
    "keyword.function",
    "keyword.operator",
    "keyword.import",
    "keyword.type",
    "keyword.modifier",
    "keyword.repeat",
    "keyword.return",
    "keyword.debug",
    "keyword.exception",
    "keyword.conditional",
    "keyword.conditional.ternary",
    "keyword.directive",
    "keyword.directive.define",
    "punctuation.delimiter",
    "punctuation.bracket",
    "punctuation.special",
    "comment",
    "comment.documentation",
    "comment.error",
    "comment.warning",
    "comment.todo",
    "comment.note",
    "markup.strong",
    "markup.italic",
    "markup.strikethrough",
    "markup.underline",
    "markup.heading",
    "markup.heading.1",
    "markup.heading.2",
    "markup.heading.3",
    "markup.heading.4",
    "markup.heading.5",
    "markup.heading.6",
    "markup.quote",
    "markup.math",
    "markup.link",
    "markup.link.label",
    "markup.link.url",
    "markup.raw",
    "markup.raw.block",
    "markup.list",
    "markup.list.checked",
    "markup.list.unchecked",
    "diff.plus",
    "diff.minus",
    "diff.delta",
    "tag",
    "tag.builtin",
    "tag.attribute",
    "tag.delimiter",
];

/// This should be constructed using the `hex!` macro.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub(crate) fn from_hex(hex: String) -> anyhow::Result<Color> {
        let regex = lazy_regex::regex!(r"^#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3})$");
        if !regex.is_match(&hex) {
            return Err(anyhow::anyhow!("Invalid hex color: {}", hex));
        }
        let hex = &hex[1..];

        let r = u8::from_str_radix(&hex[0..2], 16)?;
        let g = u8::from_str_radix(&hex[2..4], 16)?;
        let b = u8::from_str_radix(&hex[4..6], 16)?;

        Ok(Color { r, g, b })
    }

    /// Refer https://docs.rs/colorsys/latest/src/colorsys/rgb/transform.rs.html#61
    /// Refer https://sl.bing.net/b69EKNHqrLw
    pub fn get_contrasting_color(&self) -> Color {
        let Color { r, g, b } = self;
        // Calculate the luminance of the color
        let luminance = (0.299 * (*r as f64) + 0.587 * (*g as f64) + 0.114 * (*b as f64)) / 255.0;
        // Return black for bright colors, white for dark colors
        if luminance > 0.5 {
            Color { r: 0, g: 0, b: 0 }
        } else {
            Color {
                r: 255,
                g: 255,
                b: 255,
            }
        }
    }
}

impl From<Color> for crossterm::style::Color {
    fn from(val: Color) -> Self {
        crossterm::style::Color::Rgb {
            r: val.r,
            g: val.g,
            b: val.b,
        }
    }
}
