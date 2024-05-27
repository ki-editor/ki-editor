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

#[derive(serde::Deserialize)]
struct ZedThemeManiftest {
    themes: Vec<ZedTheme>,
}

#[derive(serde::Deserialize)]
struct ZedTheme {
    name: String,
    style: ZedThemeStyles,
}

#[derive(serde::Deserialize)]
struct ZedThemeStyles {
    syntax: ZedThemeStyleSyntax,
    #[serde(rename(deserialize = "editor.foreground"))]
    editor_foreground: String,
    #[serde(rename(deserialize = "editor.background"))]
    editor_background: String,
    #[serde(rename(deserialize = "editor.line_number"))]
    editor_line_number: String,
    #[serde(rename(deserialize = "status_bar.background"))]
    status_bar_background: Option<String>,
    #[serde(rename(deserialize = "tab_bar.background"))]
    tab_bar_background: Option<String>,
    #[serde(rename(deserialize = "search.match_background"))]
    search_match_background: Option<String>,
    text: String,
    players: Vec<ZedThemeStylesPlayer>,
}

#[derive(serde::Deserialize)]
struct ZedThemeStylesPlayer {
    selection: String,
}

#[derive(serde::Deserialize)]
struct ZedThemeStyleSyntax {
    keyword: Option<ZedThemeStyle>,
    variable: Option<ZedThemeStyle>,
    function: Option<ZedThemeStyle>,
    attribute: Option<ZedThemeStyle>,
    tag: Option<ZedThemeStyle>,
    comment: Option<ZedThemeStyle>,
    string: Option<ZedThemeStyle>,
    r#type: Option<ZedThemeStyle>,
}

#[derive(serde::Deserialize, Clone)]
struct ZedThemeStyle {
    color: String,
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

pub fn from_zed_theme(url: &str) -> anyhow::Result<Vec<Theme>> {
    let json_str = cache_download(
        url,
        "zed-themes",
        &std::path::PathBuf::from(url)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    )?;
    let manifest: ZedThemeManiftest = serde_json5::from_str(&json_str).unwrap();
    Ok(manifest
        .themes
        .into_iter()
        .flat_map(|theme| -> anyhow::Result<Theme> {
            let text_color = Color::from_hex(&theme.style.text)?;
            let to_style = |style: Option<ZedThemeStyle>| {
                style
                    .and_then(|style| Some(fg(Color::from_hex(&style.color).ok()?)))
                    .or_else(|| Some(fg(text_color)))
                    .unwrap_or_default()
            };
            Ok(Theme {
                name: theme.name,
                syntax: SyntaxStyles::new(&{
                    use HighlightName::*;

                    [
                        (Variable, to_style(theme.style.syntax.variable)),
                        (Keyword, to_style(theme.style.syntax.keyword.clone())),
                        (KeywordModifier, to_style(theme.style.syntax.keyword)),
                        (Function, to_style(theme.style.syntax.function)),
                        (Type, to_style(theme.style.syntax.r#type.clone())),
                        (TypeBuiltin, to_style(theme.style.syntax.r#type)),
                        (String, to_style(theme.style.syntax.string)),
                        (Comment, to_style(theme.style.syntax.comment)),
                        (Tag, to_style(theme.style.syntax.tag)),
                        (TagAttribute, to_style(theme.style.syntax.attribute)),
                    ]
                }),
                ui: UiStyles {
                    global_title: Style::new()
                        .foreground_color(hex!("#ffffff"))
                        .set_some_background_color(
                            theme
                                .style
                                .status_bar_background
                                .as_ref()
                                .and_then(|color| Color::from_hex(&color).ok()),
                        ),
                    window_title: Style::new()
                        .foreground_color(hex!("#ffffff"))
                        .set_some_background_color(
                            theme
                                .style
                                .tab_bar_background
                                .as_ref()
                                .and_then(|color| Color::from_hex(&color).ok()),
                        ),
                    parent_lines_background: hex!("#E6EBF0"),
                    jump_mark_odd: Style::new()
                        .background_color(hex!("#b5485d"))
                        .foreground_color(hex!("#ffffff")),
                    jump_mark_even: Style::new()
                        .background_color(hex!("#84b701"))
                        .foreground_color(hex!("#ffffff")),
                    background_color: Color::from_hex(&theme.style.editor_background)?,
                    text_foreground: Color::from_hex(&theme.style.text)?,
                    primary_selection_background: theme
                        .style
                        .players
                        .first()
                        .and_then(|player| Color::from_hex(&player.selection).ok())
                        .unwrap_or_default(),
                    primary_selection_anchor_background: theme
                        .style
                        .players
                        .first()
                        .and_then(|player| Color::from_hex(&player.selection).ok())
                        .unwrap_or_default(),
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
                    line_number: Style::new().set_some_foreground_color(
                        Color::from_hex(&theme.style.editor_line_number).ok(),
                    ),
                    line_number_separator: Style::new().foreground_color(hex!("#d7d7d7")),
                    bookmark: Style::new().background_color(hex!("#ffcc00")),
                    possible_selection_background: theme
                        .style
                        .search_match_background
                        .and_then(|color| Color::from_hex(&color).ok())
                        .unwrap_or_default(),
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
        })
        .collect_vec())
}

#[cfg(test)]
mod test_from_vscode_theme_json {
    #[test]
    fn test() -> anyhow::Result<()> {
        super::from_zed_theme(
            "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/one/one.json",
        )?;
        Ok(())
    }
}
