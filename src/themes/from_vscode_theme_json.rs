use super::{Color, DiagnosticStyles, HighlightName, Theme, UiStyles};
use crate::{
    style::{fg, Style},
    themes::SyntaxStyles,
};
use itertools::Itertools;
use my_proc_macros::hex;
use shared::download::cache_download;

const styles: &[(HighlightName, Style)] = {
    use HighlightName::*;

    &[
        (Variable, fg(hex!("#001080"))),
        (Keyword, fg(hex!("#af00db"))),
        (KeywordModifier, fg(hex!("#0000ff"))),
        (Function, fg(hex!("#795e26"))),
        (Type, fg(hex!("#267f99"))),
        (TypeBuiltin, fg(hex!("#0000ff"))),
        (String, fg(hex!("#a31515"))),
        (Comment, fg(hex!("#008000"))),
        (Tag, fg(hex!("#267f99"))),
        (TagAttribute, fg(hex!("#e50000"))),
    ]
};

#[derive(serde::Deserialize)]
struct VsCodeTheme {
    #[serde(rename(deserialize = "tokenColors"))]
    token_colors: Vec<TokenColor>,
    colors: vscode_theme::Colors,
}

#[derive(serde::Deserialize)]
struct ThemeColors {
    #[serde(rename(deserialize = "editor.background"))]
    editor_background: Option<String>,

    #[serde(rename(deserialize = "editor.foreground"))]
    editor_foreground: Option<String>,

    #[serde(rename(deserialize = "editor.selectionHighlightBackground"))]
    editor_selection_highlight_background: Option<String>,

    #[serde(rename(deserialize = "activityBarBadge.background"))]
    activity_bar_badge_background: Option<String>,
}

fn from_hex(s: &Option<String>) -> Color {
    s.as_ref()
        .and_then(|s| Color::from_hex(&s).ok())
        .unwrap_or_default()
}

impl VsCodeTheme {
    fn get_token_style(&self, scope: &str) -> Style {
        self.try_get_token_style(scope)
            .or_else(|| self.try_get_token_style("editor.foreground"))
            .unwrap_or_default()
    }

    fn try_get_token_style(&self, scope: &str) -> Option<Style> {
        self.token_colors
            .iter()
            .find(|token_color| token_color.scope.contains(scope))
            .map(|token_color| token_color.settings.to_style())
    }
}

#[derive(serde::Deserialize)]
struct TokenColor {
    scope: Scope,
    settings: TokenColorSettings,
}

#[derive(serde::Deserialize)]
struct TokenColorSettings {
    foreground: Option<String>,
    background: Option<String>,
}
impl TokenColorSettings {
    fn to_style(&self) -> Style {
        Style::new()
            .set_some_background_color(
                self.background
                    .as_ref()
                    .and_then(|color| Color::from_hex(color).ok()),
            )
            .set_some_foreground_color(
                self.foreground
                    .as_ref()
                    .and_then(|color| Color::from_hex(color).ok()),
            )
    }
}

mod color {
    use serde::{self, Deserialize, Deserializer};

    use crate::themes::Color;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Color::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Scope {
    String(String),
    Array(Vec<String>),
}
impl Scope {
    fn contains(&self, scope: &str) -> bool {
        match self {
            Scope::String(s) => s == scope,
            Scope::Array(array) => array.contains(&scope.to_string()),
        }
    }
}

pub fn FROM_VSCODE_THEME() -> anyhow::Result<Theme> {
    let url = "https://raw.githubusercontent.com/microsoft/vscode/main/extensions/theme-defaults/themes/dark_vs.json";
    let json_str = cache_download(url, "vs-code-themes", "dark_vs.json")?;
    let theme: VsCodeTheme = serde_json5::from_str(&json_str).unwrap();
    Ok(Theme {
        name: "from-vs-code-theme",
        syntax: SyntaxStyles::new(&{
            use HighlightName::*;

            [
                (Variable, fg(hex!("#001080"))),
                (Keyword, theme.get_token_style("keyword")),
                (KeywordModifier, theme.get_token_style("keyword")),
                (Function, fg(hex!("#795e26"))),
                (Type, fg(hex!("#267f99"))),
                (TypeBuiltin, fg(hex!("#0000ff"))),
                (String, theme.get_token_style("string")),
                (Comment, theme.get_token_style("comment")),
                (Tag, fg(hex!("#267f99"))),
                (TagAttribute, fg(hex!("#e50000"))),
            ]
        }),
        ui: UiStyles {
            global_title: Style::new()
                .foreground_color(hex!("#ffffff"))
                .background_color(from_hex(&theme.colors.activity_bar.background)),
            window_title: Style::new()
                .foreground_color(hex!("#FFFFFF"))
                .background_color(hex!("#2C2C2C")),
            parent_lines_background: hex!("#E6EBF0"),
            jump_mark_odd: Style::new()
                .background_color(hex!("#b5485d"))
                .foreground_color(hex!("#ffffff")),
            jump_mark_even: Style::new()
                .background_color(hex!("#84b701"))
                .foreground_color(hex!("#ffffff")),
            background_color: from_hex(&theme.colors.editor.background),
            text_foreground: from_hex(&theme.colors.editor.foreground),
            primary_selection_background: from_hex(
                &theme.colors.editor.selection_highlight_background,
            ),
            primary_selection_anchor_background: from_hex(
                &theme.colors.editor.selection_highlight_background,
            ),
            primary_selection_secondary_cursor: Style::new()
                .background_color(hex!("#808080"))
                .foreground_color(hex!("#ffffff")),
            secondary_selection_background: hex!("#ebebeb"),
            secondary_selection_anchor_background: hex!("#d7d7d7"),
            secondary_selection_primary_cursor: Style::new()
                .background_color(hex!("#000000"))
                .foreground_color(hex!("#ffffff")),
            secondary_selection_secondary_cursor: Style::new()
                .background_color(hex!("#808080"))
                .foreground_color(hex!("#ffffff")),
            line_number: Style::new().foreground_color(hex!("#6a9955")),
            line_number_separator: Style::new().foreground_color(hex!("#d7d7d7")),
            bookmark: Style::new().background_color(hex!("#ffcc00")),
            possible_selection_background: hex!("#f6f7b2"),
            keymap_hint: Style::new().underline(hex!("#af00db")),
            keymap_key: Style::new().bold().foreground_color(hex!("#af00db")),
            keymap_arrow: Style::new().foreground_color(hex!("#808080")),
            keymap_description: Style::new().foreground_color(hex!("#000000")),
            fuzzy_matched_char: Style::new().foreground_color(hex!("#ff0000")),
        },
        diagnostic: DiagnosticStyles::default(),
        hunk_new_background: hex!("#EBFEED"),
        hunk_old_background: hex!("#FCECEA"),
        hunk_old_emphasized_background: hex!("#F9D8D6"),
        hunk_new_emphasized_background: hex!("#BAF0C0"),
    })
}

#[cfg(test)]
mod test_from_vscode_theme_json {
    #[test]
    fn test() -> anyhow::Result<()> {
        super::FROM_VSCODE_THEME()?;
        Ok(())
    }
}
