use super::{
    theme_descriptor::ThemeDescriptor, Color, DiagnosticStyles, HighlightName, Theme, UiStyles,
};
use crate::{
    style::Style,
    themes::{GitGutterStyles, SyntaxStyles},
};
use itertools::Itertools;
use my_proc_macros::hex;
use shared::download::cache_download;
use zed_theme::*;

const ZED_THEME_AYU_URL: &str =
    "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/ayu/ayu.json";
const ZED_THEME_ALABASTER_URL: &str =
    "https://raw.githubusercontent.com/tsimoshka/zed-theme-alabaster/refs/heads/main/themes/alabaster-color-theme.json";
const ZED_THEME_CATPPUCCIN_URL: &str =
    "https://raw.githubusercontent.com/catppuccin/zed/main/themes/catppuccin-mauve.json";
const ZED_THEME_DRACULA_URL: &str =
    "https://raw.githubusercontent.com/dracula/zed/refs/heads/main/themes/dracula.json";
const ZED_THEME_GITHUB_URL: &str = "https://raw.githubusercontent.com/PyaeSoneAungRgn/github-zed-theme/refs/heads/main/themes/github_theme.json";
const ZED_THEME_GRUBER_DARKER_URL: &str =
    "https://raw.githubusercontent.com/mqual/themes/refs/heads/main/gruber_darker_zed.json";
const ZED_THEME_GRUVBOX_URL: &str =
    "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/gruvbox/gruvbox.json";
const ZED_THEME_ONE_URL: &str =
    "https://raw.githubusercontent.com/zed-industries/zed/main/assets/themes/one/one.json";
const ZED_THEME_MODUS_URL: &str = "https://raw.githubusercontent.com/vitallium/zed-modus-themes/refs/heads/main/themes/modus.json";
const ZED_THEME_MONOKAI_ST3_URL: &str =
    "https://raw.githubusercontent.com/epmoyer/Zed-Monokai-Theme/main/monokai_st3.json";
const ZED_THEME_MONOKAI_URL: &str =
    "https://raw.githubusercontent.com/epmoyer/Zed-Monokai-Theme/main/monokai.json";
const ZED_THEME_MQUAL_BLUE_URL: &str =
    "https://raw.githubusercontent.com/mqual/themes/main/mqual_blue_zed.json";
const ZED_THEME_NORD_URL: &str =
    "https://raw.githubusercontent.com/mikasius/zed-nord-theme/refs/heads/master/themes/nord.json";
const ZED_THEME_ROSE_PINE_URL: &str =
    "https://raw.githubusercontent.com/rose-pine/zed/refs/heads/main/themes/rose-pine.json";
const ZED_THEME_SOLARIZED_URL: &str = "https://raw.githubusercontent.com/harmtemolder/Solarized.zed/refs/heads/main/themes/solarized.json";
const ZED_THEME_TOKYO_NIGHT_URL: &str = "https://raw.githubusercontent.com/ssaunderss/zed-tokyo-night/refs/heads/main/themes/tokyo-night.json";

#[derive(serde::Deserialize)]
struct ZedThemeManiftest {
    themes: Vec<ThemeContent>,
}

/// Get all known Zed themes as a `ThemeDescriptor`.
pub(crate) fn theme_descriptors() -> Vec<ThemeDescriptor> {
    [
        ("Ayu Dark", ZED_THEME_AYU_URL),
        ("Ayu Light", ZED_THEME_AYU_URL),
        ("Ayu Mirage", ZED_THEME_AYU_URL),
        ("Alabaster", ZED_THEME_ALABASTER_URL),
        ("Alabaster Dark", ZED_THEME_ALABASTER_URL),
        ("Alabaster Mono", ZED_THEME_ALABASTER_URL),
        ("Alabaster Dark Mono", ZED_THEME_ALABASTER_URL),
        ("Alabaster BG", ZED_THEME_ALABASTER_URL),
        ("Catppuccin Frappé", ZED_THEME_CATPPUCCIN_URL),
        ("Catppuccin Latte", ZED_THEME_CATPPUCCIN_URL),
        ("Catppuccin Macchiato", ZED_THEME_CATPPUCCIN_URL),
        ("Catppuccin Mocha", ZED_THEME_CATPPUCCIN_URL),
        ("Dracula", ZED_THEME_DRACULA_URL),
        ("Github Dark Colorblind", ZED_THEME_GITHUB_URL),
        ("Github Dark Dimmed", ZED_THEME_GITHUB_URL),
        ("Github Dark High Contrast", ZED_THEME_GITHUB_URL),
        ("Github Dark Tritanopia", ZED_THEME_GITHUB_URL),
        ("Github Dark", ZED_THEME_GITHUB_URL),
        ("Github Light Colorblind", ZED_THEME_GITHUB_URL),
        ("Github Light High Contrast", ZED_THEME_GITHUB_URL),
        ("Github Light Tritanopia", ZED_THEME_GITHUB_URL),
        ("Github Light", ZED_THEME_GITHUB_URL),
        ("Gruber Darker", ZED_THEME_GRUBER_DARKER_URL),
        ("Gruvbox Dark Hard", ZED_THEME_GRUVBOX_URL),
        ("Gruvbox Dark Soft", ZED_THEME_GRUVBOX_URL),
        ("Gruvbox Dark", ZED_THEME_GRUVBOX_URL),
        ("Gruvbox Light Hard", ZED_THEME_GRUVBOX_URL),
        ("Gruvbox Light Soft", ZED_THEME_GRUVBOX_URL),
        ("Gruvbox Light", ZED_THEME_GRUVBOX_URL),
        ("Modus Operandi Tinted", ZED_THEME_MODUS_URL),
        ("Modus Operandi", ZED_THEME_MODUS_URL),
        ("Modus Vivendi Tinted", ZED_THEME_MODUS_URL),
        ("Modus Vivendi", ZED_THEME_MODUS_URL),
        ("Monokai", ZED_THEME_MONOKAI_URL),
        ("Monokai-ST3", ZED_THEME_MONOKAI_ST3_URL),
        ("Mqual Blue", ZED_THEME_MQUAL_BLUE_URL),
        ("Nord", ZED_THEME_NORD_URL),
        ("Nord Light", ZED_THEME_NORD_URL),
        ("One Dark", ZED_THEME_ONE_URL),
        ("One Light", ZED_THEME_ONE_URL),
        ("Rosé Pine Dawn", ZED_THEME_ROSE_PINE_URL),
        ("Rosé Pine Moon", ZED_THEME_ROSE_PINE_URL),
        ("Rosé Pine", ZED_THEME_ROSE_PINE_URL),
        ("Solarized Dark", ZED_THEME_SOLARIZED_URL),
        ("Solarized Light", ZED_THEME_SOLARIZED_URL),
        ("Tokyo Night Light", ZED_THEME_TOKYO_NIGHT_URL),
        ("Tokyo Night Storm", ZED_THEME_TOKYO_NIGHT_URL),
        ("Tokyo Night", ZED_THEME_TOKYO_NIGHT_URL),
    ]
    .iter()
    .map(|(name, url)| ThemeDescriptor::ZedTheme(name, url))
    .collect()
}

pub(crate) fn from_url(name: &'static str, url: &'static str) -> anyhow::Result<Theme> {
    let path = std::path::PathBuf::from(url);
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("The url ({:?}) should contain a file name.", url))?
        .to_string_lossy();
    let json_str = cache_download(url, "zed-themes", &file_name)?;
    let manifest: ZedThemeManiftest = serde_json5::from_str(&json_str).map_err(|error| {
        anyhow::anyhow!("Cannot parse JSON downloaded from {url:?} due to:\n{error:#?}")
    })?;

    manifest
        .themes
        .iter()
        .find_map(|theme| (theme.name == name).then(|| from_theme_content(theme.clone())))
        .ok_or_else(|| anyhow::anyhow!("could not find theme '{}'", name))
}

fn from_theme_content(theme: ThemeContent) -> Theme {
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
    let text_color = from_some_hex(theme.style.text).unwrap_or_else(|| match theme.appearance {
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

    let primary_cursor_background = theme
        .style
        .players
        .first()
        .and_then(|player| from_some_hex(player.cursor.clone()))
        .unwrap_or_default();
    let get_cursor_style = |background: Color| {
        let foreground = primary_cursor_background.get_contrasting_color();
        Style::new()
            .background_color(background)
            .foreground_color(foreground)
    };
    let primary_cursor = get_cursor_style(primary_cursor_background);
    let secondary_cursor =
        get_cursor_style(primary_cursor_background.apply_custom_alpha(background, 0.5));
    let parent_lines_background = primary_selection_background.apply_custom_alpha(background, 0.25);
    let section_divider_background =
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
            section_divider_background,
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
            primary_selection_secondary_cursor: secondary_cursor,
            secondary_selection_background: primary_selection_background,
            secondary_selection_anchor_background: primary_selection_background,
            secondary_selection_primary_cursor: primary_cursor,
            secondary_selection_secondary_cursor: secondary_cursor,
            line_number: Style::new()
                .set_some_foreground_color(from_some_hex(theme.style.editor_line_number)),
            border: Style::new()
                .foreground_color(from_some_hex(theme.style.border).unwrap_or(text_color))
                .background_color(background),
            mark: Style::new()
                .set_some_background_color(from_some_hex(theme.style.conflict_background)),
            possible_selection_background: from_some_hex(
                theme.style.search_match_background.clone(),
            )
            .unwrap_or_default(),
            incremental_search_match_background: from_some_hex(theme.style.search_match_background)
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
        git_gutter: GitGutterStyles::new(),
    }
}

#[cfg(test)]
mod test_from_zed_theme {
    use crate::themes::Theme;

    #[test]
    fn ensure_all_zed_themes_parse() -> anyhow::Result<()> {
        // Expect no failure
        let _: Vec<Theme> = super::theme_descriptors()
            .iter()
            .map(|theme| (*theme).to_theme())
            .collect();
        Ok(())
    }
}
