#![allow(clippy::single_range_in_vec_init)]
use crate::{
    app::{Dimension, Scope},
    grid::StyleKey,
    test_app::{execute_test, ExpectKind},
};

use my_proc_macros::{key, keys};
use serial_test::serial;

use crate::{
    app::Dispatch::*,
    buffer::BufferOwner,
    components::editor::{DispatchEditor::*, IfCurrentNotFound},
    selection::SelectionMode,
    test_app::{ExpectKind::*, Step::*},
};

#[test]
fn incremental_search_should_highlight_matches() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("snake_case kebab-case camelCase".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Word,
            )),
            Expect(CurrentSelectedTexts(&["snake_case"])),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("k e b").to_vec())),
            Expect(MainEditorRangeStyleKey(
                "keb",
                Some(StyleKey::UiIncrementalSearchMatch),
            )),
        ])
    })
}

#[test]
fn incremental_search_matches_highlight_should_have_higher_precedence_than_selection_highlights(
) -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(TerminalDimensionChanged(Dimension {
                height: 100,
                width: 100,
            })),
            Editor(SetContent("west bar foo".to_string())),
            // Select the whole line
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Line,
            )),
            Expect(CurrentSelectedTexts(&["west bar foo"])),
            Expect(MainEditorRangeStyleKey(
                "est bar fo",
                Some(StyleKey::UiPrimarySelectionAnchors),
            )),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("b a r").to_vec())),
            Expect(MainEditorRangeStyleKey(
                "bar",
                Some(StyleKey::UiIncrementalSearchMatch),
            )),
        ])
    })
}

#[test]
fn incremental_search_should_clear_matches_upon_prompt_closed() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("snake_case kebab-case camelCase".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Word,
            )),
            Expect(CurrentSelectedTexts(&["snake_case"])),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("k e b").to_vec())),
            Expect(Not(Box::new(CurrentEditorIncrementalSearchMatches(
                [].to_vec(),
            )))),
            App(HandleKeyEvents(keys!("esc esc").to_vec())),
            Expect(MainEditorRangeStyleKey("keb", None)),
            Expect(CurrentEditorIncrementalSearchMatches([].to_vec())),
        ])
    })
}

#[test]
fn incremental_search_should_clear_matches_upon_enter() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("snake_case kebab-case camelCase".to_string())),
            Editor(SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::Word,
            )),
            Expect(CurrentSelectedTexts(&["snake_case"])),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("k e b").to_vec())),
            Expect(Not(Box::new(CurrentEditorIncrementalSearchMatches(
                [].to_vec(),
            )))),
            App(HandleKeyEvents(keys!("enter").to_vec())),
            Expect(CurrentEditorIncrementalSearchMatches([].to_vec())),
        ])
    })
}

#[test]
fn incremental_search_should_work_with_custom_search_mode() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("hello fo-ba fo_ba".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("n / f o space b a").to_vec())),
            Expect(MainEditorRangeStyleKey(
                "fo_ba",
                Some(StyleKey::UiIncrementalSearchMatch),
            )),
        ])
    })
}

#[test]
fn incremental_search_should_work_with_tab_completion_and_backspace() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar spam".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            // Search for "f" and then uses tab to use the first completion "foo"
            App(HandleKeyEvents(keys!("f tab").to_vec())),
            // Expect the current selection of the editor is "foo"
            Expect(CurrentEditorIncrementalSearchMatches([0..3].to_vec())),
            // Press backspace
            App(HandleKeyEvents(keys!("backspace").to_vec())),
            Expect(CurrentEditorIncrementalSearchMatches([0..2].to_vec())),
        ])
    })
}

#[test]
#[serial]
fn incremental_search_should_work_with_terminal_paste_event() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("foo bar spam".to_string())),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleEvent(event::event::Event::Paste("spam".to_string()))),
            Expect(CurrentEditorIncrementalSearchMatches([8..12].to_vec())),
        ])
    })
}

#[test]
fn incremental_search_should_update_prompt_title_to_show_current_search_mode() -> anyhow::Result<()>
{
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            App(HandleKeyEvents(keys!("n space a").to_vec())),
            Expect(CurrentComponentTitle(
                "Local search (Naming Convention Agnostic)".to_string(),
            )),
        ])
    })
}

#[test]
fn possible_selections_background_should_be_cleared_when_local_search_prompt_is_opened(
) -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.main_rs(),
                owner: BufferOwner::User,
                focus: true,
            }),
            App(TerminalDimensionChanged(Dimension {
                height: 100,
                width: 100,
            })),
            Editor(SetContent("foo bar foo".to_string())),
            Editor(MatchLiteral("foo".to_string())),
            Expect(MainEditorRangeStyleKey(
                "foo",
                Some(StyleKey::UiPossibleSelection),
            )),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            Expect(MainEditorRangeStyleKey("foo", None)),
        ])
    })
}

#[test]
fn incremental_search_highlight_should_be_when_selection_mode_changes() -> anyhow::Result<()> {
    execute_test(move |s| {
        Box::new([
            App(OpenFile {
                path: s.gitignore(),
                owner: BufferOwner::User,
                focus: true,
            }),
            Editor(SetContent("bar foo spam foo".to_string())),
            App(TerminalDimensionChanged(Dimension {
                height: 100,
                width: 100,
            })),
            App(OpenSearchPrompt {
                scope: Scope::Local,
                if_current_not_found: IfCurrentNotFound::LookForward,
            }),
            Expect(CurrentComponentTitle("Local search".to_string())),
            App(HandleKeyEvents(keys!("s p a m").to_vec())),
            Expect(MainEditorRangeStyleKey(
                "spam",
                Some(StyleKey::UiIncrementalSearchMatch),
            )),
            // Switch to main component window
            App(OtherWindow),
            App(OtherWindow),
            Expect(CurrentComponentTitle(
                "\u{200b} 🙈 .gitignore [*] \u{200b}".to_string(),
            )),
            // Closes all other window by pressing esc
            App(HandleKeyEvent(key!("esc"))),
            // Since the prompt is not closed normally, the incremental search highlight should still be present
            Expect(MainEditorRangeStyleKey(
                "spam",
                Some(StyleKey::UiIncrementalSearchMatch),
            )),
            Editor(MatchLiteral("foo".to_string())),
            Expect(ExpectKind::RangeStyleKey(
                "foo",
                Some(StyleKey::UiPossibleSelection),
            )),
        ])
    })
}
