use super::{Color, DiagnosticStyles, HighlightName, Theme, UiStyles};
use crate::{style::Style, themes::SyntaxStyles};
use itertools::Itertools;
use my_proc_macros::hex;
use shared::download::cache_download;
use zed_theme::*;

#[derive(serde::Deserialize)]
struct ZedThemeManiftest {
    themes: Vec<ThemeContent>,
}

pub fn from_zed_theme(url: &str) -> anyhow::Result<Vec<Theme>> {
    let json_str = cache_download(
        url,
        "zed-themes",
        &std::path::PathBuf::from(url)
            .file_name()
            .unwrap_or_else(|| panic!("The url ({:?}) should contain file name.", url))
            .to_string_lossy(),
    )?;
    let manifest: ZedThemeManiftest = serde_json5::from_str(&json_str).unwrap_or_else(|error| {
        panic!("Cannot parse JSON downloaded from {url:?} due to:\n{error:#?}")
    });
    Ok(manifest
        .themes
        .into_iter()
        .flat_map(|theme| -> anyhow::Result<Theme> {
            let background = theme
                .style
                .editor_background
                .and_then(|hex| Color::from_hex(&hex).ok())
                .unwrap_or_else(|| match theme.appearance {
                    AppearanceContent::Light => hex!("#ffffff"),
                    AppearanceContent::Dark => hex!("#000000"),
                });
            let from_hex = |hex: &str| -> anyhow::Result<_> {
                Ok(Color::from_hex(hex)?.apply_alpha(background))
            };
            let from_some_hex = |hex: Option<String>| {
                hex.and_then(|hex| Some(Color::from_hex(&hex).ok()?.apply_alpha(background)))
            };
            let text_color =
                from_some_hex(theme.style.text).unwrap_or_else(|| match theme.appearance {
                    AppearanceContent::Light => hex!("#000000"),
                    AppearanceContent::Dark => hex!("#ffffff"),
                });
            let to_style = |highlight_name: HighlightName, style: Option<HighlightStyleContent>| {
                style.map(|style| {
                    (
                        highlight_name,
                        Style::new().set_some_foreground_color(from_some_hex(style.color)),
                    )
                })
            };
            let primary_selection_background = theme
                .style
                .players
                .first()
                .and_then(|player| from_some_hex(player.selection.clone()))
                .unwrap_or_default();
            let cursor = {
                let background = theme
                    .style
                    .players
                    .first()
                    .and_then(|player| from_some_hex(player.cursor.clone()))
                    .unwrap_or_default();
                let foreground = background.get_contrasting_color();
                Style::new()
                    .background_color(background)
                    .foreground_color(foreground)
            };
            let parent_lines_background =
                primary_selection_background.apply_custom_alpha(background, 0.25);
            let text_accent = theme
                .style
                .text_accent
                .and_then(|hex| from_hex(&hex).ok())
                .unwrap_or(text_color);
            Ok(Theme {
                name: theme.name,
                syntax: SyntaxStyles::new(&{
                    use HighlightName::*;
                    let get = |name: &str| theme.style.syntax.get(name).cloned();

                    [
                        to_style(Boolean, get("boolean")),
                        to_style(Comment, get("comment")),
                        to_style(CommentDocumentation, get("comment.documentation")),
                        to_style(Constant, get("constant")),
                        to_style(ConstantBuiltin, get("constant")),
                        to_style(Function, get("function")),
                        to_style(Keyword, get("keyword")),
                        to_style(KeywordModifier, get("keyword")),
                        to_style(MarkupHeading, get("keyword")),
                        to_style(MarkupItalic, get("attribute")),
                        to_style(MarkupLink, get("link_uri")),
                        to_style(MarkupList, get("tag")),
                        to_style(MarkupMath, get("number")),
                        to_style(MarkupQuote, get("comment")),
                        to_style(MarkupRaw, get("attribute")),
                        to_style(MarkupStrikethrough, get("variable")),
                        to_style(MarkupStrong, get("keywor")),
                        to_style(MarkupUnderline, get("type")),
                        to_style(Number, get("number")),
                        to_style(Operator, get("operator")),
                        to_style(PunctuationBracket, get("punctuation.bracket")),
                        to_style(PunctuationDelimiter, get("punctuation.delimiter")),
                        to_style(PunctuationSpecial, get("punctuation.special")),
                        to_style(String, get("string")),
                        to_style(StringEscape, get("string.escape")),
                        to_style(StringRegexp, get("string.regex")),
                        to_style(StringSpecial, get("string.special")),
                        to_style(Tag, get("tag")),
                        to_style(TagAttribute, get("attribute")),
                        to_style(Type, get("type")),
                        to_style(TypeBuiltin, get("type")),
                        to_style(Variable, get("variable")),
                    ]
                    .into_iter()
                    .flatten()
                    .collect_vec()
                }),
                ui: UiStyles {
                    global_title: Style::new()
                        .foreground_color(text_color)
                        .set_some_background_color(from_some_hex(
                            theme.style.status_bar_background,
                        )),
                    window_title_focused: Style::new()
                        .set_some_foreground_color(from_some_hex(theme.style.tab_bar_background))
                        .set_some_background_color(Some(text_color)),
                    window_title_unfocused: Style::new()
                        .foreground_color(text_color)
                        .set_some_background_color(from_some_hex(
                            theme.style.tab_inactive_background,
                        )),
                    parent_lines_background,
                    jump_mark_odd: Style::new()
                        .background_color(hex!("#b5485d"))
                        .foreground_color(hex!("#ffffff")),
                    jump_mark_even: Style::new()
                        .background_color(hex!("#84b701"))
                        .foreground_color(hex!("#ffffff")),
                    background_color: background,
                    text_foreground: text_color,
                    primary_selection_background,
                    primary_selection_anchor_background: primary_selection_background,
                    primary_selection_secondary_cursor: cursor,
                    secondary_selection_background: primary_selection_background,
                    secondary_selection_anchor_background: primary_selection_background,
                    secondary_selection_primary_cursor: cursor,
                    secondary_selection_secondary_cursor: cursor,
                    line_number: Style::new()
                        .set_some_foreground_color(from_some_hex(theme.style.editor_line_number)),
                    border: Style::new()
                        .foreground_color(from_some_hex(theme.style.border).unwrap_or(text_color))
                        .background_color(background),
                    mark: Style::new()
                        .set_some_background_color(from_some_hex(theme.style.conflict_background)),
                    possible_selection_background: from_some_hex(
                        theme.style.search_match_background,
                    )
                    .unwrap_or_default(),
                    keymap_hint: Style::new().underline(text_accent),
                    keymap_key: Style::new().bold().foreground_color(text_accent),
                    keymap_arrow: Style::new().set_some_foreground_color(
                        theme.style.text_muted.and_then(|hex| from_hex(&hex).ok()),
                    ),
                    fuzzy_matched_char: Style::new()
                        .foreground_color(text_accent)
                        .underline(text_accent),
                },
                diagnostic: {
                    let default = DiagnosticStyles::default();
                    let undercurl = |hex: Option<String>, default: Style| {
                        from_some_hex(hex)
                            .map(|color| Style::new().undercurl(color))
                            .unwrap_or(default)
                    };
                    DiagnosticStyles {
                        error: undercurl(theme.style.error, default.error),
                        warning: undercurl(theme.style.warning, default.warning),
                        info: undercurl(theme.style.info, default.info),
                        hint: undercurl(theme.style.hint, default.hint),
                        default: default.default,
                    }
                },
                hunk: if theme.appearance == AppearanceContent::Light {
                    super::HunkStyles::light()
                } else {
                    super::HunkStyles::dark()
                },
            })
        })
        .collect_vec())
}

#[cfg(test)]
mod test_from_zed_theme {
    #[test]
    fn test() -> anyhow::Result<()> {
        // Expect no failure
        super::from_zed_theme(
            "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/one/one.json",
        )?;
        Ok(())
    }
}
