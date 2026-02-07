use super::{Color, DiagnosticStyles, HighlightName, Theme, UiStyles};
use crate::{
    style::Style,
    themes::{GitGutterStyles, SyntaxStyles},
};
use itertools::Itertools;
use my_proc_macros::hex;
use zed_theme::*;

pub(super) fn from_theme_content(theme: ThemeContent) -> Theme {
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
        git_gutter: GitGutterStyles::default(),
    }
}
