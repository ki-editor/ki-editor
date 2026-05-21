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
    let from_some_hex = |hex: &Option<String>| {
        hex.as_ref()
            .and_then(|hex| Some(Color::from_hex(&hex).ok()?.apply_alpha(background)))
    };
    let text_color = from_some_hex(&theme.style.text).unwrap_or_else(|| match theme.appearance {
        AppearanceContent::Light => hex!("#000000"),
        AppearanceContent::Dark => hex!("#ffffff"),
    });
    let primary_selection_background = theme
        .style
        .players
        .first()
        .and_then(|player| from_some_hex(&player.selection))
        .unwrap_or_default();

    let primary_cursor_background = theme
        .style
        .players
        .first()
        .and_then(|player| from_some_hex(&player.cursor))
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
    let window_title_focused_foreground = from_some_hex(&theme.style.tab_bar_background);
    let window_title_focused = Style::new()
        .set_some_foreground_color(window_title_focused_foreground)
        .set_some_background_color(Some(text_color));
    let window_title_unfocused = Style::new()
        .foreground_color(text_color)
        .set_some_background_color(from_some_hex(&theme.style.tab_inactive_background));
    let focused_tab = Style::new()
        .set_some_foreground_color(window_title_focused.background_color)
        .set_some_background_color(
            from_some_hex(&theme.style.tab_active_background)
                .or(window_title_focused.foreground_color),
        );
    Theme {
        name: theme.name,
        syntax: SyntaxStyles::new(&{
            use HighlightName::*;

            [
                (Attribute, "attribute"),
                (AttributeBuiltin, "attribute.builtin"),
                (Boolean, "boolean"),
                (Character, "character"),
                (CharacterSpecial, "character.special"),
                (Comment, "comment"),
                (CommentDocumentation, "comment.doc"),
                (CommentError, "comment.error"),
                (CommentNote, "comment.note"),
                (CommentTodo, "comment.todo"),
                (CommentWarning, "comment.warning"),
                (Constant, "constant"),
                (ConstantBuiltin, "constant.builtin"),
                (ConstantMacro, "constant.macro"),
                (Constructor, "constructor"),
                (DiffDelta, "diff.delta"),
                (DiffMinus, "diff.minus"),
                (DiffPlus, "diff.plus"),
                (Function, "function"),
                (FunctionBuiltin, "function.builtin"),
                (FunctionCall, "function.call"),
                (FunctionMacro, "function.macro"),
                (FunctionMethod, "function.method"),
                (FunctionMethodCall, "function.method.call"),
                (Keyword, "keyword"),
                (KeywordConditional, "keyword.conditional"),
                (KeywordConditionalTernary, "keyword.conditional.ternary"),
                (KeywordCoroutine, "keyword.coroutine"),
                (KeywordDebug, "keyword.debug"),
                (KeywordDirective, "keyword.directive"),
                (KeywordDirectiveDefine, "keyword.directive.define"),
                (KeywordException, "keyword.exception"),
                (KeywordFunction, "keyword.function"),
                (KeywordImport, "keyword.import"),
                (KeywordModifier, "keyword.modifier"),
                (KeywordOperator, "keyword.operator"),
                (KeywordRepeat, "keyword.repeat"),
                (KeywordReturn, "keyword.return"),
                (KeywordType, "keyword.type"),
                (Label, "label"),
                (MarkupHeading, "markup.heading"),
                (MarkupHeading1, "markup.heading.1"),
                (MarkupHeading2, "markup.heading.2"),
                (MarkupHeading3, "markup.heading.3"),
                (MarkupHeading4, "markup.heading.4"),
                (MarkupHeading5, "markup.heading.5"),
                (MarkupHeading6, "markup.heading.6"),
                (MarkupItalic, "markup.italic"),
                (MarkupLink, "markup.link"),
                (MarkupLinkLabel, "markup.link.label"),
                (MarkupLinkUrl, "markup.link.url"),
                (MarkupList, "markup.list"),
                (MarkupListChecked, "markup.list.checked"),
                (MarkupListUnchecked, "markup.list.unchecked"),
                (MarkupMath, "markup.math"),
                (MarkupQuote, "markup.quote"),
                (MarkupRaw, "markup.raw"),
                (MarkupRawBlock, "markup.raw.block"),
                (MarkupStrikethrough, "markup.strikethrough"),
                (MarkupStrong, "markup.strong"),
                (MarkupUnderline, "markup.underline"),
                (Module, "module"),
                (ModuleBuiltin, "module.builtin"),
                (Number, "number"),
                (NumberFloat, "number.float"),
                (Operator, "operator"),
                (Property, "property"),
                (PunctuationBracket, "punctuation.bracket"),
                (PunctuationDelimiter, "punctuation.delimiter"),
                (PunctuationSpecial, "punctuation.special"),
                (String, "string"),
                (StringDocumentation, "string.doc"),
                (StringEscape, "string.escape"),
                (StringRegexp, "string.regex"),
                (StringSpecial, "string.special"),
                (StringSpecialPath, "string.special.path"),
                (StringSpecialSymbol, "string.special.symbol"),
                (StringSpecialUrl, "string.special.url"),
                (Tag, "tag"),
                (TagAttribute, "tag.attribute"),
                (TagBuiltin, "tag.builtin"),
                (TagDelimiter, "tag.delimiter"),
                (Type, "type"),
                (TypeBuiltin, "type.builtin"),
                (TypeDefinition, "type.definition"),
                (Variable, "variable"),
                (VariableBuiltin, "variable.builtin"),
                (VariableMember, "variable.member"),
                (VariableParameter, "variable.parameter"),
                (VariableParameterBuiltin, "variable.parameter.builtin"),
            ]
            .into_iter()
            .map(|(highlight, name)| {
                theme.style.syntax.get(name).map(|style| {
                    (
                        highlight,
                        Style::new().set_some_foreground_color(from_some_hex(&style.color)),
                    )
                })
            })
            .flatten()
            .collect_vec()
        }),
        ui: UiStyles {
            global_title: Style::new()
                .foreground_color(text_color)
                .set_some_background_color(from_some_hex(&theme.style.status_bar_background)),
            window_title_focused,
            window_title_unfocused,
            focused_tab,
            parent_lines_background,
            section_divider_background,
            jump_mark_odd: Style::new()
                .background_color(hex!("#b5485d"))
                .foreground_color(hex!("#ffffff")),
            jump_mark_even: Style::new()
                .background_color(hex!("#84b701"))
                .foreground_color(hex!("#ffffff")),
            default: Style::new()
                .background_color(background)
                .foreground_color(text_color),
            primary_selection_background,
            primary_selection_anchor_background: primary_selection_background,
            primary_selection_primary_cursor: primary_cursor,
            primary_selection_secondary_cursor: secondary_cursor,
            secondary_selection_background: primary_selection_background,
            secondary_selection_anchor_background: primary_selection_background,
            secondary_selection_primary_cursor: primary_cursor,
            secondary_selection_secondary_cursor: secondary_cursor,
            line_number: Style::new()
                .set_some_foreground_color(from_some_hex(&theme.style.editor_line_number)),
            border: Style::new()
                .foreground_color(from_some_hex(&theme.style.border).unwrap_or(text_color))
                .background_color(background),
            mark: Style::new()
                .set_some_background_color(from_some_hex(&theme.style.conflict_background)),
            possible_selection_background: from_some_hex(&theme.style.search_match_background)
                .unwrap_or_default(),
            incremental_search_match_background: from_some_hex(
                &theme.style.search_match_background,
            )
            .unwrap_or_default(),
            fuzzy_matched_char: Style::new()
                .foreground_color(text_accent)
                .underline(text_accent),
        },
        diagnostic: {
            let default = DiagnosticStyles::default();
            let undercurl = |hex: Option<String>, default: Style| {
                from_some_hex(&hex)
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
