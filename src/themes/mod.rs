pub mod from_vscode_theme_json;
pub mod vscode_dark;
pub(crate) mod vscode_light;
use std::collections::HashMap;

use itertools::Itertools;
use my_proc_macros::hex;
use once_cell::sync::OnceCell;
use strum::IntoEnumIterator as _;
pub(crate) use vscode_dark::VSCODE_DARK;
pub(crate) use vscode_light::VSCODE_LIGHT;

use crate::{grid::StyleKey, style::Style};

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct Theme {
    pub(crate) name: String,
    pub(crate) syntax: SyntaxStyles,
    pub(crate) ui: UiStyles,
    pub(crate) diagnostic: DiagnosticStyles,
    pub(crate) hunk_old_background: Color,
    pub(crate) hunk_new_background: Color,
    pub(crate) hunk_old_emphasized_background: Color,
    pub(crate) hunk_new_emphasized_background: Color,
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
            StyleKey::KeymapHint => self.ui.keymap_hint,
            StyleKey::KeymapArrow => self.ui.keymap_arrow,
            StyleKey::KeymapDescription => self.ui.keymap_description,
            StyleKey::KeymapKey => self.ui.keymap_key,
            StyleKey::UiFuzzyMatchedChar => self.ui.fuzzy_matched_char,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        VSCODE_LIGHT().clone()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct DiagnosticStyles {
    pub(crate) error: Style,
    pub(crate) warning: Style,
    pub(crate) information: Style,
    pub(crate) hint: Style,
    pub(crate) default: Style,
}

impl DiagnosticStyles {
    const fn default() -> Self {
        Self {
            error: Style::new().undercurl(hex!("#ff0000")),
            warning: Style::new().undercurl(hex!("#ffa500")),
            information: Style::new().undercurl(hex!("#007acc")),
            hint: Style::new().undercurl(hex!("#008000")),
            default: Style::new(),
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub(crate) struct UiStyles {
    pub(crate) fuzzy_matched_char: Style,
    pub(crate) global_title: Style,
    pub(crate) window_title: Style,
    pub(crate) parent_lines_background: Color,
    pub(crate) jump_mark_odd: Style,
    pub(crate) jump_mark_even: Style,
    pub(crate) text_foreground: Color,
    pub(crate) background_color: Color,
    pub(crate) primary_selection_background: Color,
    pub(crate) primary_selection_anchor_background: Color,
    pub(crate) primary_selection_secondary_cursor: Style,
    pub(crate) secondary_selection_background: Color,
    pub(crate) secondary_selection_anchor_background: Color,
    pub(crate) possible_selection_background: Color,
    pub(crate) secondary_selection_primary_cursor: Style,
    pub(crate) secondary_selection_secondary_cursor: Style,
    pub(crate) line_number: Style,
    pub(crate) line_number_separator: Style,
    pub(crate) bookmark: Style,
    pub(crate) keymap_key: Style,
    pub(crate) keymap_arrow: Style,
    pub(crate) keymap_hint: Style,
    pub(crate) keymap_description: Style,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub(crate) struct SyntaxStyles {
    map: once_cell::sync::OnceCell<HashMap<&'static str, Style>>,
    groups: Vec<(HighlightName, Style)>,
}
impl SyntaxStyles {
    pub fn new(groups: &[(HighlightName, Style)]) -> Self {
        Self {
            groups: groups.to_vec(),
            map: once_cell::sync::OnceCell::new(),
        }
    }
    fn map(&self) -> &HashMap<&'static str, Style> {
        self.map.get_or_init(|| {
            self.groups
                .iter()
                .map(|(key, style)| (key.into(), style.to_owned()))
                .collect()
        })
    }
    fn get_style(&self, highlight_group: &str) -> Option<Style> {
        let group = HighlightGroup::new(highlight_group);
        self.map()
            .get(group.full_name.as_str())
            .cloned()
            .or_else(|| self.get_style(&group.parent?))
    }
}

#[cfg(test)]
mod test_syntax_styles {
    use my_proc_macros::hex;

    use crate::style::fg;

    use super::HighlightName::*;
    use super::*;

    fn SYNTAX_STYLE() -> SyntaxStyles {
        SyntaxStyles::new(&[
            (String, fg(hex!("#267f99"))),
            (StringSpecial, fg(hex!("#e50000"))),
            (Variable, fg(hex!("#abcdef"))),
        ])
    }
    #[test]
    fn test_get_style() {
        assert_eq!(
            SYNTAX_STYLE().get_style("string").unwrap(),
            fg(hex!("#267f99"))
        );
        assert_eq!(
            SYNTAX_STYLE().get_style("string.special").unwrap(),
            fg(hex!("#e50000"))
        );
        assert_eq!(
            SYNTAX_STYLE().get_style("string.special.symbol").unwrap(),
            fg(hex!("#e50000"))
        );
        assert_eq!(
            SYNTAX_STYLE()
                .get_style("variable.parameter.builtin")
                .unwrap(),
            fg(hex!("#abcdef"))
        );
        assert_eq!(SYNTAX_STYLE().get_style("character"), None);
    }
}

pub(crate) struct HighlightGroup {
    full_name: String,
    parent: Option<String>,
}

impl HighlightGroup {
    fn new(group: &str) -> HighlightGroup {
        match group.split('.').collect_vec().split_last() {
            Some((_, parents)) if !parents.is_empty() => HighlightGroup {
                parent: Some(parents.join(".")),
                full_name: group.to_string(),
            },
            _ => HighlightGroup {
                parent: None,
                full_name: group.to_string(),
            },
        }
    }
}

/// Refer https://github.com/nvim-treesitter/nvim-treesitter/blob/23ba63028c6acca29be6462c0a291fc4a1b9eae8/CONTRIBUTING.md#highlights
///
/// The capture groups should tally with that of `nvim-treesitter` so that their
/// highlight queries can be used in this editor without modifications.
#[derive(
    strum_macros::EnumString,
    strum_macros::EnumIter,
    strum_macros::IntoStaticStr,
    Debug,
    PartialEq,
    Eq,
    Clone,
)]
pub enum HighlightName {
    #[strum(serialize = "ui.bar")]
    UiBar,
    #[strum(serialize = "ui")]
    Ui,
    #[strum(serialize = "syntax.keyword")]
    SyntaxKeyword,
    #[strum(serialize = "syntax.keyword.async")]
    SyntaxKeywordAsync,

    #[strum(serialize = "variable")]
    Variable,
    #[strum(serialize = "variable.builtin")]
    VariableBuiltin,
    #[strum(serialize = "variable.parameter")]
    VariableParameter,
    #[strum(serialize = "variable.parameter.builtin")]
    VariableParameterBuiltin,
    #[strum(serialize = "variable.member")]
    VariableMember,
    #[strum(serialize = "constant")]
    Constant,
    #[strum(serialize = "constant.builtin")]
    ConstantBuiltin,
    #[strum(serialize = "constant.macro")]
    ConstantMacro,
    #[strum(serialize = "module")]
    Module,
    #[strum(serialize = "module.builtin")]
    ModuleBuiltin,
    #[strum(serialize = "label")]
    Label,
    #[strum(serialize = "string")]
    String,
    #[strum(serialize = "string.documentation")]
    StringDocumentation,
    #[strum(serialize = "string.regexp")]
    StringRegexp,
    #[strum(serialize = "string.escape")]
    StringEscape,
    #[strum(serialize = "string.special")]
    StringSpecial,
    #[strum(serialize = "string.special.symbol")]
    StringSpecialSymbol,
    #[strum(serialize = "string.special.url")]
    StringSpecialUrl,
    #[strum(serialize = "string.special.path")]
    StringSpecialPath,
    #[strum(serialize = "character")]
    Character,
    #[strum(serialize = "character.special")]
    CharacterSpecial,
    #[strum(serialize = "boolean")]
    Boolean,
    #[strum(serialize = "number")]
    Number,
    #[strum(serialize = "number.float")]
    NumberFloat,
    #[strum(serialize = "type")]
    Type,
    #[strum(serialize = "type.builtin")]
    TypeBuiltin,
    #[strum(serialize = "type.definition")]
    TypeDefinition,
    #[strum(serialize = "attribute")]
    Attribute,
    #[strum(serialize = "attribute.builtin")]
    AttributeBuiltin,
    #[strum(serialize = "property")]
    Property,
    #[strum(serialize = "function")]
    Function,
    #[strum(serialize = "function.builtin")]
    FunctionBuiltin,
    #[strum(serialize = "function.call")]
    FunctionCall,
    #[strum(serialize = "function.macro")]
    FunctionMacro,
    #[strum(serialize = "function.method")]
    FunctionMethod,
    #[strum(serialize = "function.method.call")]
    FunctionMethodCall,
    #[strum(serialize = "constructor")]
    Constructor,
    #[strum(serialize = "operator")]
    Operator,
    #[strum(serialize = "keyword")]
    Keyword,
    #[strum(serialize = "keyword.coroutine")]
    KeywordCoroutine,
    #[strum(serialize = "keyword.function")]
    KeywordFunction,
    #[strum(serialize = "keyword.operator")]
    KeywordOperator,
    #[strum(serialize = "keyword.import")]
    KeywordImport,
    #[strum(serialize = "keyword.type")]
    KeywordType,
    #[strum(serialize = "keyword.modifier")]
    KeywordModifier,
    #[strum(serialize = "keyword.repeat")]
    KeywordRepeat,
    #[strum(serialize = "keyword.return")]
    KeywordReturn,
    #[strum(serialize = "keyword.debug")]
    KeywordDebug,
    #[strum(serialize = "keyword.exception")]
    KeywordException,
    #[strum(serialize = "keyword.conditional")]
    KeywordConditional,
    #[strum(serialize = "keyword.conditional.ternary")]
    KeywordConditionalTernary,
    #[strum(serialize = "keyword.directive")]
    KeywordDirective,
    #[strum(serialize = "keyword.directive.define")]
    KeywordDirectiveDefine,
    #[strum(serialize = "punctuation.delimiter")]
    PunctuationDelimiter,
    #[strum(serialize = "punctuation.bracket")]
    PunctuationBracket,
    #[strum(serialize = "punctuation.special")]
    PunctuationSpecial,
    #[strum(serialize = "comment")]
    Comment,
    #[strum(serialize = "comment.documentation")]
    CommentDocumentation,
    #[strum(serialize = "comment.error")]
    CommentError,
    #[strum(serialize = "comment.warning")]
    CommentWarning,
    #[strum(serialize = "comment.todo")]
    CommentTodo,
    #[strum(serialize = "comment.note")]
    CommentNote,
    #[strum(serialize = "markup.strong")]
    MarkupStrong,
    #[strum(serialize = "markup.italic")]
    MarkupItalic,
    #[strum(serialize = "markup.strikethrough")]
    MarkupStrikethrough,
    #[strum(serialize = "markup.underline")]
    MarkupUnderline,
    #[strum(serialize = "markup.heading")]
    MarkupHeading,
    #[strum(serialize = "markup.heading.1")]
    MarkupHeading1,
    #[strum(serialize = "markup.heading.2")]
    MarkupHeading2,
    #[strum(serialize = "markup.heading.3")]
    MarkupHeading3,
    #[strum(serialize = "markup.heading.4")]
    MarkupHeading4,
    #[strum(serialize = "markup.heading.5")]
    MarkupHeading5,
    #[strum(serialize = "markup.heading.6")]
    MarkupHeading6,
    #[strum(serialize = "markup.quote")]
    MarkupQuote,
    #[strum(serialize = "markup.math")]
    MarkupMath,
    #[strum(serialize = "markup.link")]
    MarkupLink,
    #[strum(serialize = "markup.link.label")]
    MarkupLinkLabel,
    #[strum(serialize = "markup.link.url")]
    MarkupLinkUrl,
    #[strum(serialize = "markup.raw")]
    MarkupRaw,
    #[strum(serialize = "markup.raw.block")]
    MarkupRawBlock,
    #[strum(serialize = "markup.list")]
    MarkupList,
    #[strum(serialize = "markup.list.checked")]
    MarkupListChecked,
    #[strum(serialize = "markup.list.unchecked")]
    MarkupListUnchecked,
    #[strum(serialize = "diff.plus")]
    DiffPlus,
    #[strum(serialize = "diff.minus")]
    DiffMinus,
    #[strum(serialize = "diff.delta")]
    DiffDelta,
    #[strum(serialize = "tag")]
    Tag,
    #[strum(serialize = "tag.builtin")]
    TagBuiltin,
    #[strum(serialize = "tag.attribute")]
    TagAttribute,
    #[strum(serialize = "tag.delimiter")]
    TagDelimiter,
}

pub fn highlight_names() -> &'static Vec<&'static str> {
    static INIT: once_cell::sync::OnceCell<Vec<&'static str>> = OnceCell::new();
    INIT.get_or_init(|| {
        HighlightName::iter()
            .map(|variant| variant.into())
            .collect_vec()
    })
}

/// This should be constructed using the `hex!` macro.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
pub(crate) struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub(crate) fn from_hex(hex: &str) -> anyhow::Result<Color> {
        let regex = lazy_regex::regex!(r"^#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3}|[A-Fa-f0-9]{8})$");
        if !regex.is_match(&hex) {
            return Err(anyhow::anyhow!("Invalid hex color: {}", hex));
        }
        let hex = &hex[1..];

        let r = u8::from_str_radix(&hex[0..2], 16)?;
        let g = u8::from_str_radix(&hex[2..4], 16)?;
        let b = u8::from_str_radix(&hex[4..6], 16)?;

        let alpha = if hex.len() == 8 {
            let alpha = u8::from_str_radix(&hex[6..8], 16)?;
            Some(alpha)
        } else {
            None
        };

        Ok(Color { r, g, b })
    }

    /// Refer https://docs.rs/colorsys/latest/src/colorsys/rgb/transform.rs.html#61
    /// Refer https://sl.bing.net/b69EKNHqrLw
    pub(crate) fn get_contrasting_color(&self) -> Color {
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

pub(crate) fn themes() -> Vec<Theme> {
    vec![VSCODE_DARK().clone(), VSCODE_LIGHT().clone()]
        .into_iter()
        .chain(
            from_vscode_theme_json::from_zed_theme(
                "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/one/one.json",
            )
            .unwrap(),
        ).chain(from_vscode_theme_json::from_zed_theme(
                "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/gruvbox/gruvbox.json",
            )
            .unwrap())
        .collect_vec()
}
