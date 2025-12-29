use super::{
    theme_descriptor::ThemeDescriptor, Color, DiagnosticStyles, HighlightName, Theme, UiStyles,
};
use crate::{
    style::Style,
    themes::{GitGutterStyles, SyntaxStyles},
};
use itertools::Itertools;
use my_proc_macros::hex;
use zed_theme::*;

#[derive(serde::Deserialize)]
struct ZedThemeManiftest {
    themes: Vec<ThemeContent>,
}

/// Get all known Zed themes as a `ThemeDescriptor`.
pub(crate) fn theme_descriptors() -> Vec<ThemeDescriptor> {
    [
        "Ayu Dark",
        "Ayu Light",
        "Ayu Mirage",
        "Alabaster",
        "Alabaster Dark",
        "Alabaster Mono",
        "Alabaster Dark Mono",
        "Alabaster BG",
        "Apathy Dark",
        "Apathy Light",
        "Catppuccin Frappé",
        "Catppuccin Latte",
        "Catppuccin Macchiato",
        "Catppuccin Mocha",
        "Dracula",
        "Github Dark Colorblind",
        "Github Dark Dimmed",
        "Github Dark High Contrast",
        "Github Dark Tritanopia",
        "Github Dark",
        "Github Light Colorblind",
        "Github Light High Contrast",
        "Github Light Tritanopia",
        "Github Light",
        "Gruber Darker",
        "Gruvbox Dark Hard",
        "Gruvbox Dark Soft",
        "Gruvbox Dark",
        "Gruvbox Light Hard",
        "Gruvbox Light Soft",
        "Gruvbox Light",
        "Modus Operandi Tinted",
        "Modus Operandi",
        "Modus Vivendi Tinted",
        "Modus Vivendi",
        "Monokai",
        "Monokai-ST3",
        "Mqual Blue",
        "Nord",
        "Nord Light",
        "One Dark",
        "One Light",
        "Rosé Pine Dawn",
        "Rosé Pine Moon",
        "Rosé Pine",
        "Solarized Dark",
        "Solarized Light",
        "Tokyo Night Light",
        "Tokyo Night Storm",
        "Tokyo Night",
    ]
    .iter()
    .map(|name| ThemeDescriptor {
        name: name.to_string(),
    })
    .collect()
}

pub(super) fn from_name(name: &str) -> Theme {
    let theme_inner = &*THEMES;
    // themes module will ensure an invalid theme descriptor is never constructed
    let theme_content = theme_inner
        .get(name)
        .expect(&format!("Unknown theme {name}"));
    from_theme_content(theme_content)
}

fn from_theme_content(theme: &ThemeContent) -> Theme {
    let background = theme
        .style
        .editor_background
        .as_deref()
        .and_then(|hex| Color::from_hex(&hex).ok())
        .unwrap_or_else(|| match theme.appearance {
            AppearanceContent::Light => hex!("#ffffff"),
            AppearanceContent::Dark => hex!("#000000"),
        });
    let from_hex =
        |hex: &str| -> anyhow::Result<_> { Ok(Color::from_hex(hex)?.apply_alpha(background)) };
    let from_some_hex = |hex: Option<&str>| {
        hex.and_then(|hex| Some(Color::from_hex(&hex).ok()?.apply_alpha(background)))
    };
    let text_color =
        from_some_hex(theme.style.text.as_deref()).unwrap_or_else(|| match theme.appearance {
            AppearanceContent::Light => hex!("#000000"),
            AppearanceContent::Dark => hex!("#ffffff"),
        });
    let to_style = |highlight_name: HighlightName, style: Option<HighlightStyleContent>| {
        style.map(|style| {
            (
                highlight_name,
                Style::new().set_some_foreground_color(from_some_hex(style.color.as_deref())),
            )
        })
    };
    let primary_selection_background = theme
        .style
        .players
        .first()
        .and_then(|player| from_some_hex(player.selection.as_deref()))
        .unwrap_or_default();

    let primary_cursor_background = theme
        .style
        .players
        .first()
        .and_then(|player| from_some_hex(player.cursor.as_deref()))
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
        .as_deref()
        .and_then(|hex| from_hex(&hex).ok())
        .unwrap_or(text_color);
    Theme {
        name: theme.name.clone(),
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
                    theme.style.status_bar_background.as_deref(),
                )),
            window_title_focused: Style::new()
                .set_some_foreground_color(from_some_hex(theme.style.tab_bar_background.as_deref()))
                .set_some_background_color(Some(text_color)),
            window_title_unfocused: Style::new()
                .foreground_color(text_color)
                .set_some_background_color(from_some_hex(
                    theme.style.tab_inactive_background.as_deref(),
                )),
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
            line_number: Style::new().set_some_foreground_color(from_some_hex(
                theme.style.editor_line_number.as_deref(),
            )),
            border: Style::new()
                .foreground_color(
                    from_some_hex(theme.style.border.as_deref()).unwrap_or(text_color),
                )
                .background_color(background),
            mark: Style::new().set_some_background_color(from_some_hex(
                theme.style.conflict_background.as_deref(),
            )),
            possible_selection_background: from_some_hex(
                theme.style.search_match_background.as_deref(),
            )
            .unwrap_or_default(),
            incremental_search_match_background: from_some_hex(
                theme.style.search_match_background.as_deref(),
            )
            .unwrap_or_default(),
            keymap_hint: Style::new().underline(text_accent),
            keymap_key: Style::new().bold().foreground_color(text_accent),
            keymap_arrow: Style::new().set_some_foreground_color(
                theme
                    .style
                    .text_muted
                    .as_deref()
                    .and_then(|hex| from_hex(&hex).ok()),
            ),
            fuzzy_matched_char: Style::new()
                .foreground_color(text_accent)
                .underline(text_accent),
        },
        diagnostic: {
            let default = DiagnosticStyles::default();
            let undercurl = |hex: Option<&str>, default: Style| {
                from_some_hex(hex)
                    .map(|color| Style::new().undercurl(color))
                    .unwrap_or(default)
            };
            DiagnosticStyles {
                error: undercurl(theme.style.error.as_deref(), default.error),
                warning: undercurl(theme.style.warning.as_deref(), default.warning),
                info: undercurl(theme.style.info.as_deref(), default.info),
                hint: undercurl(theme.style.hint.as_deref(), default.hint),
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
