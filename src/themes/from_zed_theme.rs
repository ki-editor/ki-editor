use super::{
    theme_descriptor::ThemeDescriptor, Color, DiagnosticStyles, HighlightName, Theme, UiStyles,
};
use crate::{style::Style, themes::SyntaxStyles};
use itertools::Itertools;
use my_proc_macros::hex;
use shared::download::cache_download;
use zed_theme::*;

const ZED_THEME_ANDROMEDA_URL: &str = "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/andromeda/andromeda.json";
const ZED_THEME_ATELIER_URL: &str =
    "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/atelier/atelier.json";
const ZED_THEME_AYU_URL: &str =
    "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/ayu/ayu.json";
const ZED_THEME_CATPPUCCIN_URL: &str =
    "https://raw.githubusercontent.com/catppuccin/zed/main/themes/catppuccin-mauve.json";
const ZED_THEME_GRUVBOX_URL: &str =
    "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/gruvbox/gruvbox.json";
const ZED_THEME_MONOKAI_ST3_URL: &str =
    "https://raw.githubusercontent.com/epmoyer/Zed-Monokai-Theme/main/monokai_st3.json";
const ZED_THEME_MONOKAI_URL: &str =
    "https://raw.githubusercontent.com/epmoyer/Zed-Monokai-Theme/main/monokai.json";
const ZED_THEME_ROSE_PINE_URL: &str = "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/rose_pine/rose_pine.json";
const ZED_THEME_SANDCASTLE_URL: &str = "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/sandcastle/sandcastle.json";
const ZED_THEME_SOLARIZED_URL: &str = "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/solarized/solarized.json";
const ZED_THEME_SUMMERCAMP_URL: &str = "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/summercamp/summercamp.json";

#[derive(serde::Deserialize)]
struct ZedThemeManiftest {
    themes: Vec<ThemeContent>,
}

/// Get all known Zed themes as a `ThemeDescriptor`.
pub(crate) fn theme_descriptors() -> Vec<ThemeDescriptor> {
    [
        ThemeDescriptor::ZedThemeURLMap("Andromeda", ZED_THEME_ANDROMEDA_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Cave Dark", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Cave Light", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Dune Dark", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Dune Light", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Estuary Dark", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Estuary Light", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Forest Dark", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Forest Light", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Heath Dark", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Heath Light", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Lakeside Dark", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Lakeside Light", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Plateau Dark", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Plateau Light", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Savanna Dark", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Savanna Light", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Seaside Dark", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Seaside Light", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Sulphurpool Dark", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Atelier Sulphurpool Light", ZED_THEME_ATELIER_URL),
        ThemeDescriptor::ZedThemeURLMap("Ayu Dark", ZED_THEME_AYU_URL),
        ThemeDescriptor::ZedThemeURLMap("Ayu Light", ZED_THEME_AYU_URL),
        ThemeDescriptor::ZedThemeURLMap("Ayu Mirage", ZED_THEME_AYU_URL),
        ThemeDescriptor::ZedThemeURLMap("Catppuccin Frappé", ZED_THEME_CATPPUCCIN_URL),
        ThemeDescriptor::ZedThemeURLMap("Catppuccin Latte", ZED_THEME_CATPPUCCIN_URL),
        ThemeDescriptor::ZedThemeURLMap("Catppuccin Macchiato", ZED_THEME_CATPPUCCIN_URL),
        ThemeDescriptor::ZedThemeURLMap("Catppuccin Mocha", ZED_THEME_CATPPUCCIN_URL),
        ThemeDescriptor::ZedThemeURLMap("Gruvbox Dark Hard", ZED_THEME_GRUVBOX_URL),
        ThemeDescriptor::ZedThemeURLMap("Gruvbox Dark Soft", ZED_THEME_GRUVBOX_URL),
        ThemeDescriptor::ZedThemeURLMap("Gruvbox Dark", ZED_THEME_GRUVBOX_URL),
        ThemeDescriptor::ZedThemeURLMap("Gruvbox Light Hard", ZED_THEME_GRUVBOX_URL),
        ThemeDescriptor::ZedThemeURLMap("Gruvbox Light Soft", ZED_THEME_GRUVBOX_URL),
        ThemeDescriptor::ZedThemeURLMap("Gruvbox Light", ZED_THEME_GRUVBOX_URL),
        ThemeDescriptor::ZedThemeURLMap("Monokai", ZED_THEME_MONOKAI_URL),
        ThemeDescriptor::ZedThemeURLMap("Monokai-ST3", ZED_THEME_MONOKAI_ST3_URL),
        ThemeDescriptor::ZedThemeURLMap("Rosé Pine Dawn", ZED_THEME_ROSE_PINE_URL),
        ThemeDescriptor::ZedThemeURLMap("Rosé Pine Moon", ZED_THEME_ROSE_PINE_URL),
        ThemeDescriptor::ZedThemeURLMap("Rosé Pine", ZED_THEME_ROSE_PINE_URL),
        ThemeDescriptor::ZedThemeURLMap("Sandcastle", ZED_THEME_SANDCASTLE_URL),
        ThemeDescriptor::ZedThemeURLMap("Solarized Dark", ZED_THEME_SOLARIZED_URL),
        ThemeDescriptor::ZedThemeURLMap("Solarized Light", ZED_THEME_SOLARIZED_URL),
        ThemeDescriptor::ZedThemeURLMap("Summercamp", ZED_THEME_SUMMERCAMP_URL),
    ]
    .to_vec()
}

pub(crate) fn from_url(name: &'static str, url: &'static str) -> anyhow::Result<Theme> {
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

    let theme_content = manifest.themes.into_iter().find_map(|theme| {
        if theme.name == name {
            Some(theme)
        } else {
            None
        }
    });

    match theme_content {
        Some(theme) => Ok(theme.into()),
        None => Err(anyhow::anyhow!("could not parse theme")),
    }
}

impl From<ThemeContent> for Theme {
    fn from(theme: ThemeContent) -> Self {
        let background = theme
            .style
            .editor_background
            .and_then(|hex| Color::from_hex(&hex).ok())
            .unwrap_or_else(|| match theme.appearance {
                AppearanceContent::Light => hex!("#ffffff"),
                AppearanceContent::Dark => hex!("#000000"),
            });
        let from_hex =
            |hex: &str| -> anyhow::Result<_> { Ok(Color::from_hex(hex)?.apply_alpha(background)) };
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
        Theme {
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
                    .set_some_background_color(from_some_hex(theme.style.status_bar_background)),
                window_title_focused: Style::new()
                    .set_some_foreground_color(from_some_hex(theme.style.tab_bar_background))
                    .set_some_background_color(Some(text_color)),
                window_title_unfocused: Style::new()
                    .foreground_color(text_color)
                    .set_some_background_color(from_some_hex(theme.style.tab_inactive_background)),
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
                possible_selection_background: from_some_hex(theme.style.search_match_background)
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
        }
    }
}

#[cfg(test)]
mod test_from_zed_theme {
    // use crate::themes::Theme;

    // #[test]
    // fn test() -> anyhow::Result<()> {
    // // Expect no failure
    // let _: Vec<Theme> = super::theme_descriptor_from_zed_url(
    // "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/one/one.json",
    // )?
    // .iter()
    // .map(|theme| (*theme).clone().into())
    // .collect();
    //
    // Ok(())
    // }
}
