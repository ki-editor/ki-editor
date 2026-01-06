pub mod from_zed_theme;
pub(crate) mod theme_descriptor;
pub(crate) mod vscode_dark;
pub(crate) mod vscode_light;
use std::collections::HashMap;

use itertools::Itertools;
use my_proc_macros::hex;
use once_cell::sync::OnceCell;
use strum::IntoEnumIterator as _;
pub(crate) use vscode_dark::vscode_dark;
pub(crate) use vscode_light::vscode_light;

use crate::{env::parse_env, grid::StyleKey, style::Style};

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct Theme {
    pub(crate) name: String,
    pub(crate) syntax: SyntaxStyles,
    pub(crate) ui: UiStyles,
    pub(crate) diagnostic: DiagnosticStyles,
    pub(crate) hunk: HunkStyles,
    pub(crate) git_gutter: GitGutterStyles,
}

pub(crate) fn from_name(name: &str) -> Result<Theme, String> {
    let descriptors = crate::themes::theme_descriptor::all();
    descriptors
        .iter()
        .find(|descriptor| descriptor.name() == name)
        .map(|descriptor| descriptor.to_theme())
        .ok_or_else(|| {
            let valid_themes: Vec<_> = descriptors.iter().map(|d| d.name()).collect();
            format!("'{name}' is not a valid theme. Available: {valid_themes:?}")
        })
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct HunkStyles {
    pub(crate) old_background: Color,
    pub(crate) new_background: Color,
    pub(crate) old_emphasized_background: Color,
    pub(crate) new_emphasized_background: Color,
}

impl HunkStyles {
    fn dark() -> Self {
        Self {
            new_background: hex!("#383D2C"),
            old_background: hex!("#47221F"),
            old_emphasized_background: hex!("#682520"),
            new_emphasized_background: hex!("#4E5A32"),
        }
    }

    fn light() -> Self {
        Self {
            new_background: hex!("#EBFEED"),
            old_background: hex!("#FCECEA"),
            old_emphasized_background: hex!("#F9D8D6"),
            new_emphasized_background: hex!("#BAF0C0"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct GitGutterStyles {
    pub(crate) insertion: Color,
    pub(crate) deletion: Color,
    pub(crate) replacement: Color,
}

impl GitGutterStyles {
    pub(crate) fn new() -> Self {
        Self {
            insertion: hex!("#BAF0C0"),
            deletion: hex!("#F9D8D6"),
            replacement: hex!("#f0e68c"),
        }
    }
}

impl Theme {
    pub(crate) fn get_style(&self, source: &StyleKey) -> Style {
        match source {
            StyleKey::UiMark => self.ui.mark,
            StyleKey::UiPrimarySelection => {
                Style::new().background_color(self.ui.primary_selection_background)
            }
            StyleKey::UiCursorLineNumber => {
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
            StyleKey::UiIncrementalSearchMatch => {
                Style::new().background_color(self.ui.incremental_search_match_background)
            }
            StyleKey::DiagnosticsHint => self.diagnostic.hint,
            StyleKey::DiagnosticsError => self.diagnostic.error,
            StyleKey::DiagnosticsWarning => self.diagnostic.warning,
            StyleKey::DiagnosticsInformation => self.diagnostic.info,
            StyleKey::DiagnosticsDefault => self.diagnostic.default,
            StyleKey::HunkOld => Style::new().background_color(self.hunk.old_background),
            StyleKey::HunkNew => Style::new().background_color(self.hunk.new_background),
            StyleKey::HunkOldEmphasized => {
                Style::new().background_color(self.hunk.old_emphasized_background)
            }
            StyleKey::HunkNewEmphasized => {
                Style::new().background_color(self.hunk.new_emphasized_background)
            }
            StyleKey::Syntax(highlight_group) => highlight_group
                .to_highlight_name()
                .and_then(|name| self.syntax.get_style(&name))
                .unwrap_or_default(),
            StyleKey::KeymapHint => self.ui.keymap_hint,
            StyleKey::KeymapArrow => self.ui.keymap_arrow,
            StyleKey::KeymapKey => self.ui.keymap_key,
            StyleKey::UiFuzzyMatchedChar => self.ui.fuzzy_matched_char,
            StyleKey::ParentLine => Style::new().background_color(self.ui.parent_lines_background),
            StyleKey::UiPrimarySelectionSecondaryCursor => {
                self.ui.primary_selection_secondary_cursor
            }
            StyleKey::UiSecondarySelectionPrimaryCursor => {
                self.ui.secondary_selection_primary_cursor
            }
            StyleKey::UiSecondarySelectionSecondaryCursor => {
                self.ui.secondary_selection_secondary_cursor
            }
            StyleKey::UiSectionDivider => {
                Style::new().background_color(self.ui.section_divider_background)
            }
            StyleKey::UiFocusedTab => Style::new()
                .foreground_color(self.ui.background_color)
                .background_color(self.ui.text_foreground),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        let default_theme_descriptor = parse_env(
            "KI_EDITOR_THEME",
            &theme_descriptor::all(),
            |theme| theme.name(),
            theme_descriptor::ThemeDescriptor::default(),
        );

        default_theme_descriptor.to_theme()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct DiagnosticStyles {
    pub(crate) error: Style,
    pub(crate) warning: Style,
    pub(crate) info: Style,
    pub(crate) hint: Style,
    pub(crate) default: Style,
}

impl DiagnosticStyles {
    const fn default() -> Self {
        Self {
            error: Style::new().undercurl(hex!("#ff0000")),
            warning: Style::new().undercurl(hex!("#ffa500")),
            info: Style::new().undercurl(hex!("#007acc")),
            hint: Style::new().undercurl(hex!("#008000")),
            default: Style::new(),
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub(crate) struct UiStyles {
    pub(crate) fuzzy_matched_char: Style,
    pub(crate) global_title: Style,
    pub(crate) window_title_focused: Style,
    pub(crate) window_title_unfocused: Style,
    pub(crate) parent_lines_background: Color,
    pub(crate) section_divider_background: Color,
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
    pub(crate) incremental_search_match_background: Color,
    pub(crate) secondary_selection_primary_cursor: Style,
    pub(crate) secondary_selection_secondary_cursor: Style,
    pub(crate) line_number: Style,
    pub(crate) border: Style,
    pub(crate) mark: Style,
    pub(crate) keymap_key: Style,
    pub(crate) keymap_arrow: Style,
    pub(crate) keymap_hint: Style,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub(crate) struct SyntaxStyles {
    map: once_cell::sync::OnceCell<HashMap<HighlightName, Style>>,
    groups: Vec<(HighlightName, Style)>,
}

impl SyntaxStyles {
    pub fn new(groups: &[(HighlightName, Style)]) -> Self {
        Self {
            groups: groups.to_vec(),
            map: once_cell::sync::OnceCell::new(),
        }
    }

    fn map(&self) -> &HashMap<HighlightName, Style> {
        self.map.get_or_init(|| {
            self.groups
                .iter()
                .map(|(key, style)| (key.clone(), style.to_owned()))
                .collect()
        })
    }

    /// Obtain the style of a given highlight_name
    /// by recursively looking up its parent's name
    fn get_style(&self, highlight_name: &HighlightName) -> Option<Style> {
        self.map()
            .get(highlight_name)
            .cloned()
            .or_else(|| self.get_style(&highlight_name.parent()?))
    }
}

#[cfg(test)]
mod test_syntax_styles {
    use std::str::FromStr;

    use my_proc_macros::hex;

    use crate::style::fg;

    use super::HighlightName::*;
    use super::*;

    fn syntax_style() -> SyntaxStyles {
        SyntaxStyles::new(&[
            (String, fg(hex!("#267f99"))),
            (StringSpecial, fg(hex!("#e50000"))),
            (Variable, fg(hex!("#abcdef"))),
        ])
    }

    #[test]
    fn test_get_style() {
        assert_eq!(
            syntax_style()
                .get_style(&HighlightName::from_str("string").unwrap())
                .unwrap(),
            fg(hex!("#267f99"))
        );
        assert_eq!(
            syntax_style()
                .get_style(&HighlightName::from_str("string.special").unwrap())
                .unwrap(),
            fg(hex!("#e50000"))
        );
        assert_eq!(
            syntax_style()
                .get_style(&HighlightName::from_str("string.special.symbol").unwrap())
                .unwrap(),
            fg(hex!("#e50000"))
        );
        assert_eq!(
            syntax_style()
                .get_style(&HighlightName::from_str("variable.parameter.builtin").unwrap())
                .unwrap(),
            fg(hex!("#abcdef"))
        );
        assert_eq!(
            syntax_style().get_style(&HighlightName::from_str("character").unwrap()),
            None
        );
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
    Hash,
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
impl HighlightName {
    fn parent(&self) -> Option<HighlightName> {
        // We hardcode the branch instead of deriving it from the string
        // via separating the highlight name by period symbol
        // because this function is a hot path,
        // we need every ounce of speed here.
        use HighlightName::*;
        match self {
            // UI related
            UiBar => Some(Ui),
            Ui => None,

            // Syntax related
            SyntaxKeyword => None,
            SyntaxKeywordAsync => Some(SyntaxKeyword),

            // Variables
            Variable => None,
            VariableBuiltin => Some(Variable),
            VariableParameter => Some(Variable),
            VariableParameterBuiltin => Some(VariableParameter),
            VariableMember => Some(Variable),

            // Constants
            Constant => None,
            ConstantBuiltin => Some(Constant),
            ConstantMacro => Some(Constant),

            // Modules
            Module => None,
            ModuleBuiltin => Some(Module),

            // Label
            Label => None,

            // Strings
            String => None,
            StringDocumentation => Some(String),
            StringRegexp => Some(String),
            StringEscape => Some(String),
            StringSpecial => Some(String),
            StringSpecialSymbol => Some(StringSpecial),
            StringSpecialUrl => Some(StringSpecial),
            StringSpecialPath => Some(StringSpecial),

            // Characters
            Character => None,
            CharacterSpecial => Some(Character),

            // Boolean
            Boolean => None,

            // Numbers
            Number => None,
            NumberFloat => Some(Number),

            // Types
            Type => None,
            TypeBuiltin => Some(Type),
            TypeDefinition => Some(Type),

            // Attributes
            Attribute => None,
            AttributeBuiltin => Some(Attribute),

            // Properties
            Property => None,

            // Functions
            Function => None,
            FunctionBuiltin => Some(Function),
            FunctionCall => Some(Function),
            FunctionMacro => Some(Function),
            FunctionMethod => Some(Function),
            FunctionMethodCall => Some(FunctionMethod),

            // Constructor
            Constructor => None,

            // Operator
            Operator => None,

            // Keywords
            Keyword => None,
            KeywordCoroutine => Some(Keyword),
            KeywordFunction => Some(Keyword),
            KeywordOperator => Some(Keyword),
            KeywordImport => Some(Keyword),
            KeywordType => Some(Keyword),
            KeywordModifier => Some(Keyword),
            KeywordRepeat => Some(Keyword),
            KeywordReturn => Some(Keyword),
            KeywordDebug => Some(Keyword),
            KeywordException => Some(Keyword),
            KeywordConditional => Some(Keyword),
            KeywordConditionalTernary => Some(KeywordConditional),
            KeywordDirective => Some(Keyword),
            KeywordDirectiveDefine => Some(KeywordDirective),

            // Punctuation
            PunctuationDelimiter => None,
            PunctuationBracket => None,
            PunctuationSpecial => None,

            // Comments
            Comment => None,
            CommentDocumentation => Some(Comment),
            CommentError => Some(Comment),
            CommentWarning => Some(Comment),
            CommentTodo => Some(Comment),
            CommentNote => Some(Comment),

            // Markup
            MarkupStrong => None,
            MarkupItalic => None,
            MarkupStrikethrough => None,
            MarkupUnderline => None,
            MarkupHeading => None,
            MarkupHeading1 => Some(MarkupHeading),
            MarkupHeading2 => Some(MarkupHeading),
            MarkupHeading3 => Some(MarkupHeading),
            MarkupHeading4 => Some(MarkupHeading),
            MarkupHeading5 => Some(MarkupHeading),
            MarkupHeading6 => Some(MarkupHeading),
            MarkupQuote => None,
            MarkupMath => None,
            MarkupLink => None,
            MarkupLinkLabel => Some(MarkupLink),
            MarkupLinkUrl => Some(MarkupLink),
            MarkupRaw => None,
            MarkupRawBlock => Some(MarkupRaw),
            MarkupList => None,
            MarkupListChecked => Some(MarkupList),
            MarkupListUnchecked => Some(MarkupList),

            // Diff
            DiffPlus => None,
            DiffMinus => None,
            DiffDelta => None,

            // Tags
            Tag => None,
            TagBuiltin => Some(Tag),
            TagAttribute => Some(Tag),
            TagDelimiter => Some(Tag),
        }
    }
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
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, PartialOrd, Ord)]
pub(crate) struct Color {
    r: u8,
    g: u8,
    b: u8,
    /// Alpha channel, represents opacity, max value (#ff or 255) means totally opaque
    a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self {
            r,
            g,
            b,
            a: u8::MAX,
        }
    }

    // This is a function that convert RGBA to RGB, based on the given background
    fn apply_alpha(&self, background: Color) -> Color {
        let alpha = self.a as f32 / 255.0;
        self.apply_custom_alpha(background, alpha)
    }

    /// `alpha` should be between 0 to 1.0
    /// 0.5 means 50% opacity
    fn apply_custom_alpha(&self, background: Color, alpha: f32) -> Color {
        let inverted_alpha = 1.0 - alpha;
        Color {
            r: (alpha * self.r as f32 + inverted_alpha * background.r as f32) as u8,
            g: (alpha * self.g as f32 + inverted_alpha * background.g as f32) as u8,
            b: (alpha * self.b as f32 + inverted_alpha * background.b as f32) as u8,
            a: u8::MAX,
        }
    }

    pub(crate) fn from_hex(hex: &str) -> anyhow::Result<Color> {
        let regex = lazy_regex::regex!(r"^#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3}|[A-Fa-f0-9]{8})$");
        if !regex.is_match(hex) {
            return Err(anyhow::anyhow!("Invalid hex color: {}", hex));
        }
        let hex = &hex[1..];

        let r = u8::from_str_radix(&hex[0..2], 16)?;
        let g = u8::from_str_radix(&hex[2..4], 16)?;
        let b = u8::from_str_radix(&hex[4..6], 16)?;

        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16)?
        } else {
            u8::MAX
        };

        Ok(Color { r, g, b, a })
    }

    /// Refer https://docs.rs/colorsys/latest/src/colorsys/rgb/transform.rs.html#61
    /// Refer https://sl.bing.net/b69EKNHqrLw
    pub(crate) fn get_contrasting_color(&self) -> Color {
        let Color { r, g, b, a } = self;
        // Calculate the luminance of the color
        let luminance = (0.299 * (*r as f64) + 0.587 * (*g as f64) + 0.114 * (*b as f64)) / 255.0;
        // Return black for bright colors, white for dark colors
        if luminance > 0.5 {
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: *a,
            }
        } else {
            Color {
                r: 255,
                g: 255,
                b: 255,
                a: *a,
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
