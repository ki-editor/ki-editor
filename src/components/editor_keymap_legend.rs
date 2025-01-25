use crossterm::event::KeyCode;
use SelectionMode::*;

use convert_case::Case;
use event::KeyEvent;
use itertools::Itertools;

use crate::{
    app::{Dispatch, Dispatches, FilePickerKind, Scope},
    components::editor::Movement,
    context::{Context, LocalSearchConfigMode, Search},
    git::DiffMode,
    list::grep::RegexConfig,
    quickfix_list::{DiagnosticSeverityRange, QuickfixListType},
    selection::SelectionMode,
    surround::EnclosureKind,
    transformation::Transformation,
};

use super::{
    editor::{
        Direction, DispatchEditor, Editor, HandleEventResult, IfCurrentNotFound, Mode, SurroundKind,
    },
    editor_keymap::*,
    keymap_legend::{Keymap, KeymapLegendBody, KeymapLegendConfig, Keymaps},
};

use DispatchEditor::*;
use Movement::*;
impl Editor {
    pub(crate) fn keymap_core_movements(&self) -> Vec<Keymap> {
        [
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Left_),
                "◀".to_string(),
                "Left".to_string(),
                Dispatch::ToEditor(MoveSelection(Movement::Left)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Right),
                "▶".to_string(),
                "Right".to_string(),
                Dispatch::ToEditor(MoveSelection(Right)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Up___),
                "▲".to_string(),
                "Up".to_string(),
                Dispatch::ToEditor(MoveSelection(Up)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Down_),
                "▼".to_string(),
                "Down".to_string(),
                Dispatch::ToEditor(MoveSelection(Down)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::First),
                "◀◀".to_string(),
                "First".to_string(),
                Dispatch::ToEditor(MoveSelection(Movement::First)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Last_),
                "▶▶".to_string(),
                "Last".to_string(),
                Dispatch::ToEditor(MoveSelection(Movement::Last)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Jump_),
                "Jump".to_string(),
                "Jump".to_string(),
                Dispatch::ToEditor(DispatchEditor::ShowJumps {
                    use_current_selection_mode: true,
                }),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::ToIdx),
                "Index".to_string(),
                "To Index (1-based)".to_string(),
                Dispatch::OpenMoveToIndexPrompt,
            ),
        ]
        .to_vec()
    }

    pub(crate) fn keymap_other_movements(&self) -> Vec<Keymap> {
        [
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::CrsrP),
                Direction::Start.format_action("Curs"),
                Direction::Start.format_action("Cycle primary selection"),
                Dispatch::ToEditor(CyclePrimarySelection(Direction::Start)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::CrsrN),
                Direction::End.format_action("Curs"),
                Direction::End.format_action("Cycle primary selection"),
                Dispatch::ToEditor(CyclePrimarySelection(Direction::End)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::ScrlD),
                "Scroll ↓".to_string(),
                "Scroll down".to_string(),
                Dispatch::ToEditor(ScrollPageDown),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::ScrlU),
                "Scroll ↑".to_string(),
                "Scroll up".to_string(),
                Dispatch::ToEditor(ScrollPageUp),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::GBack),
                Direction::Start.format_action("Select"),
                "Go back".to_string(),
                Dispatch::ToEditor(GoBack),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::GForw),
                Direction::End.format_action("Select"),
                "Go forward".to_string(),
                Dispatch::ToEditor(GoForward),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::BuffP),
                Direction::Start.format_action("Buffer"),
                "Go to previous buffer".to_string(),
                Dispatch::CycleBuffer(Direction::Start),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::BuffN),
                Direction::End.format_action("Buffer"),
                "Go to next buffer".to_string(),
                Dispatch::CycleBuffer(Direction::End),
            ),
            Keymap::new(
                "1",
                "Quick Jump - #1".to_string(),
                Dispatch::JumpEditor('1'),
            ),
            Keymap::new(
                "2",
                "Quick Jump - #2".to_string(),
                Dispatch::JumpEditor('2'),
            ),
            Keymap::new(
                "3",
                "Quick Jump - #3".to_string(),
                Dispatch::JumpEditor('3'),
            ),
            Keymap::new(
                "4",
                "Quick Jump - #4".to_string(),
                Dispatch::JumpEditor('4'),
            ),
            Keymap::new(
                "5",
                "Quick Jump - #5".to_string(),
                Dispatch::JumpEditor('5'),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::SSEnd),
                "⇋ End".to_string(),
                "Switch extended selection end".to_string(),
                Dispatch::ToEditor(SwapExtensionDirection),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::XAchr),
                "⇋ Anchor".to_string(),
                "Swap cursor with anchor".to_string(),
                Dispatch::ToEditor(DispatchEditor::SwapCursorWithAnchor),
            ),
        ]
        .to_vec()
    }

    pub(crate) fn keymap_primary_selection_modes(&self, context: &Context) -> Vec<Keymap> {
        [
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Line_),
                "Line".to_string(),
                "Select Line".to_string(),
                Dispatch::ToEditor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::LineF),
                "Line*".to_string(),
                "Select Line*".to_string(),
                Dispatch::ToEditor(SetSelectionMode(IfCurrentNotFound::LookForward, LineFull)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Sytx_),
                "Syntax".to_string(),
                "Select Syntax Node".to_string(),
                Dispatch::ToEditor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::FStyx),
                "Syntax*".to_string(),
                "Select Syntax Node*".to_string(),
                Dispatch::ToEditor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SyntaxNodeFine,
                )),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Tokn_),
                "Token".to_string(),
                "Select Token".to_string(),
                Dispatch::ToEditor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    Token { skip_symbols: true },
                )),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::FTokn),
                "Token*".to_string(),
                "Select Token*".to_string(),
                Dispatch::ToEditor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    Token {
                        skip_symbols: false,
                    },
                )),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::WordF),
                "Word*".to_string(),
                "Select Word*".to_string(),
                Dispatch::ToEditor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    Word {
                        skip_symbols: false,
                    },
                )),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Word_),
                "Word".to_string(),
                "Select Word".to_string(),
                Dispatch::ToEditor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    Word { skip_symbols: true },
                )),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Char_),
                "Char".to_string(),
                "Select Character".to_string(),
                Dispatch::ToEditor(SetSelectionMode(IfCurrentNotFound::LookForward, Character)),
            ),
        ]
        .to_vec()
    }

    pub(crate) fn keymap_secondary_selection_modes_init(&self, context: &Context) -> Vec<Keymap> {
        [
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::FindP),
                Direction::Start.format_action("Find"),
                "Find (Local) - Backward".to_string(),
                Dispatch::ShowKeymapLegend(self.secondary_selection_modes_keymap_legend_config(
                    context,
                    Scope::Local,
                    IfCurrentNotFound::LookBackward,
                )),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::FindN),
                Direction::End.format_action("Find"),
                "Find (Local) - Forward".to_string(),
                Dispatch::ShowKeymapLegend(self.secondary_selection_modes_keymap_legend_config(
                    context,
                    Scope::Local,
                    IfCurrentNotFound::LookForward,
                )),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Globl),
                "Global".to_string(),
                "Find (Global)".to_string(),
                Dispatch::ShowKeymapLegend(self.secondary_selection_modes_keymap_legend_config(
                    context,
                    Scope::Global,
                    IfCurrentNotFound::LookForward,
                )),
            ),
        ]
        .to_vec()
    }
    pub(crate) fn keymap_actions(&self, normal_mode_override: &NormalModeOverride) -> Vec<Keymap> {
        [
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Join_),
                "Join".to_string(),
                "Join".to_string(),
                Dispatch::ToEditor(Transform(Transformation::Join)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Break),
                "Break".to_string(),
                "Break".to_string(),
                Dispatch::ToEditor(BreakSelection),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Raise),
                "Raise".to_string(),
                "Raise".to_string(),
                Dispatch::ToEditor(Replace(Expand)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Mark_),
                "Mark".to_string(),
                "Toggle Mark".to_string(),
                Dispatch::ToEditor(ToggleMark),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::SrchN),
                Direction::End.format_action("Search"),
                Direction::End.format_action("Search"),
                Dispatch::OpenSearchPrompt {
                    scope: Scope::Local,
                    if_current_not_found: IfCurrentNotFound::LookForward,
                },
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::SrchP),
                Direction::Start.format_action("Search"),
                Direction::Start.format_action("Search"),
                Dispatch::OpenSearchPrompt {
                    scope: Scope::Local,
                    if_current_not_found: IfCurrentNotFound::LookBackward,
                },
            ),
        ]
        .into_iter()
        .chain(
            [
                Keymap::new_extended(
                    KEYBOARD_LAYOUT.get_key(&Meaning::Chng_),
                    "Change".to_string(),
                    "Change".to_string(),
                    Dispatch::ToEditor(Change),
                )
                .override_keymap(normal_mode_override.change.as_ref()),
                Keymap::new_extended(
                    KEYBOARD_LAYOUT.get_key(&Meaning::DeltN),
                    Direction::End.format_action("Delete"),
                    Direction::End.format_action("Delete"),
                    Dispatch::ToEditor(Delete(Direction::End)),
                )
                .override_keymap(normal_mode_override.delete.as_ref()),
                Keymap::new_extended(
                    KEYBOARD_LAYOUT.get_key(&Meaning::DeltP),
                    Direction::Start.format_action("Delete"),
                    Direction::Start.format_action("Delete"),
                    Dispatch::ToEditor(Delete(Direction::Start)),
                )
                .override_keymap(normal_mode_override.delete_backward.as_ref()),
                Keymap::new_extended(
                    KEYBOARD_LAYOUT.get_key(&Meaning::InstP),
                    Direction::Start.format_action("Insert"),
                    Direction::Start.format_action("Insert"),
                    Dispatch::ToEditor(EnterInsertMode(Direction::Start)),
                )
                .override_keymap(normal_mode_override.insert.as_ref()),
                Keymap::new_extended(
                    KEYBOARD_LAYOUT.get_key(&Meaning::InstN),
                    Direction::End.format_action("Insert"),
                    Direction::End.format_action("Insert"),
                    Dispatch::ToEditor(EnterInsertMode(Direction::End)),
                )
                .override_keymap(normal_mode_override.append.as_ref()),
            ]
            .to_vec(),
        )
        .chain([
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::OpenP),
                Direction::Start.format_action("Open"),
                Direction::Start.format_action("Open"),
                Dispatch::ToEditor(Open(Direction::Start)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::OpenN),
                Direction::End.format_action("Open"),
                Direction::End.format_action("Open"),
                Dispatch::ToEditor(Open(Direction::End)),
            )
            .override_keymap(normal_mode_override.open.as_ref()),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Undo_),
                "Undo".to_string(),
                "Undo".to_string(),
                Dispatch::ToEditor(Undo),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Redo_),
                "Redo".to_string(),
                "Redo".to_string(),
                Dispatch::ToEditor(Redo),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::PRplc),
                "Replace #".to_string(),
                "Replace with pattern".to_string(),
                Dispatch::ToEditor(ReplaceWithPattern),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::RplcP),
                Direction::Start.format_action("Replace"),
                "Replace (with previous copied text)".to_string(),
                Dispatch::ToEditor(ReplaceWithPreviousCopiedText),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::RplcN),
                Direction::End.format_action("Replace"),
                "Replace (with next copied text)".to_string(),
                Dispatch::ToEditor(ReplaceWithNextCopiedText),
            ),
            Keymap::new_extended(
                "enter",
                "save".to_string(),
                "Save".to_string(),
                Dispatch::ToEditor(Save),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Trsfm),
                "Transform".to_string(),
                "Transform".to_string(),
                Dispatch::ShowKeymapLegend(self.transform_keymap_legend_config()),
            ),
            Keymap::new(
                "$",
                Direction::End.format_action("Collapse selection"),
                Dispatch::ToEditor(DispatchEditor::CollapseSelection(Direction::End)),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Indnt),
                "Indent".to_string(),
                "Indent".to_string(),
                Dispatch::ToEditor(Indent),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::DeDnt),
                "Dedent".to_string(),
                "Dedent".to_string(),
                Dispatch::ToEditor(Dedent),
            ),
        ])
        .chain(self.search_current_selection_keymap(
            KEYBOARD_LAYOUT.get_key(&Meaning::SrchC),
            Scope::Local,
            IfCurrentNotFound::LookForward,
        ))
        .chain(self.keymap_clipboard_related_actions(false, normal_mode_override.clone()))
        .collect_vec()
    }

    fn keymap_clipboard_related_actions(
        &self,
        use_system_clipboard: bool,
        normal_mode_override: NormalModeOverride,
    ) -> Vec<Keymap> {
        let extra = if use_system_clipboard { "+ " } else { "" };
        let format = |description: &str| format!("{extra}{description}");
        [
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::ChngX),
                format("Change X"),
                format!("{}{}", "Change Cut", extra),
                Dispatch::ToEditor(ChangeCut {
                    use_system_clipboard,
                }),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::PsteN),
                format("Paste →"),
                format!("{}{}", Direction::End.format_action("Paste"), extra),
                Dispatch::ToEditor(Paste {
                    direction: Direction::End,
                    use_system_clipboard,
                }),
            )
            .override_keymap(normal_mode_override.paste.clone().as_ref()),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::PsteP),
                format("Paste ←"),
                format!("{}{}", Direction::Start.format_action("Paste"), extra),
                Dispatch::ToEditor(Paste {
                    direction: Direction::Start,
                    use_system_clipboard,
                }),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::RplcX),
                format("Replace X"),
                format!("{}{}", "Replace Cut", extra),
                Dispatch::ToEditor(ReplaceWithCopiedText {
                    use_system_clipboard,
                    cut: true,
                }),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Copy_),
                format("Copy"),
                format!("{}{}", "Copy", extra),
                Dispatch::ToEditor(Copy {
                    use_system_clipboard,
                }),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Rplc_),
                format("Replace"),
                format!("{}{}", "Replace", extra),
                Dispatch::ToEditor(ReplaceWithCopiedText {
                    use_system_clipboard,
                    cut: false,
                }),
            )
            .override_keymap(normal_mode_override.replace.as_ref()),
        ]
        .into_iter()
        .collect_vec()
    }

    fn keymap_universal(&self) -> Vec<Keymap> {
        [
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::WClse),
                "❌ Window".to_string(),
                "Close current window".to_string(),
                Dispatch::CloseCurrentWindow,
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::SView),
                "⇋ Align".to_string(),
                "Switch view alignment".to_string(),
                Dispatch::ToEditor(SwitchViewAlignment),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::WSwth),
                "⇋ Window".to_string(),
                "Switch window".to_string(),
                Dispatch::OtherWindow,
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::UPstE),
                "Paste →".to_string(),
                "Paste".to_string(),
                Dispatch::ToEditor(Paste {
                    direction: Direction::End,
                    use_system_clipboard: false,
                }),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::CSrch),
                "🔍 Config".to_string(),
                "Configure Search".to_string(),
                Dispatch::ShowSearchConfig {
                    scope: Scope::Local,
                    if_current_not_found: IfCurrentNotFound::LookForward,
                    run_search_after_config_updated: self.mode != Mode::Insert,
                },
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_insert_key(&Meaning::SHelp),
                "Help".to_string(),
                "Help".to_string(),
                Dispatch::ToEditor(DispatchEditor::ShowHelp),
            ),
        ]
        .to_vec()
    }

    pub(crate) fn insert_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Insert mode keymaps".to_string(),
            body: KeymapLegendBody::Positional(Keymaps::new(
                &[
                    Keymap::new_extended(
                        "left",
                        "Char ←".to_string(),
                        "Move back a character".to_string(),
                        Dispatch::ToEditor(MoveCharacterBack),
                    ),
                    Keymap::new_extended(
                        "right",
                        "Char →".to_string(),
                        "Move forward a character".to_string(),
                        Dispatch::ToEditor(MoveCharacterForward),
                    ),
                    Keymap::new_extended(
                        KEYBOARD_LAYOUT.get_insert_key(&Meaning::LineP),
                        "Line ←".to_string(),
                        "Move to line start".to_string(),
                        Dispatch::ToEditor(MoveToLineStart),
                    ),
                    Keymap::new_extended(
                        KEYBOARD_LAYOUT.get_insert_key(&Meaning::LineN),
                        "Line →".to_string(),
                        "Move to line end".to_string(),
                        Dispatch::ToEditor(MoveToLineEnd),
                    ),
                    Keymap::new_extended(
                        KEYBOARD_LAYOUT.get_insert_key(&Meaning::KilLP),
                        "Kill Line ←".to_string(),
                        Direction::End.format_action("Kill line"),
                        Dispatch::ToEditor(KillLine(Direction::Start)),
                    ),
                    Keymap::new_extended(
                        KEYBOARD_LAYOUT.get_insert_key(&Meaning::KilLN),
                        "Kill Line →".to_string(),
                        Direction::End.format_action("Kill line"),
                        Dispatch::ToEditor(KillLine(Direction::End)),
                    ),
                    Keymap::new_extended(
                        KEYBOARD_LAYOUT.get_insert_key(&Meaning::DTknP),
                        "Delete Token ←".to_string(),
                        "Delete token backward".to_string(),
                        Dispatch::ToEditor(DeleteWordBackward { short: false }),
                    ),
                    Keymap::new_extended(
                        "alt+backspace",
                        "Delete Word ←".to_string(),
                        "Delete word backward".to_string(),
                        Dispatch::ToEditor(DeleteWordBackward { short: true }),
                    ),
                    Keymap::new(
                        "left",
                        "Move back a character".to_string(),
                        Dispatch::ToEditor(MoveCharacterBack),
                    ),
                    Keymap::new(
                        "right",
                        "Move forward a character".to_string(),
                        Dispatch::ToEditor(MoveCharacterForward),
                    ),
                    Keymap::new(
                        "esc",
                        "Enter normal mode".to_string(),
                        Dispatch::ToEditor(EnterNormalMode),
                    ),
                    Keymap::new(
                        "backspace",
                        "Delete character backward".to_string(),
                        Dispatch::ToEditor(Backspace),
                    ),
                    Keymap::new(
                        "enter",
                        "Enter new line".to_string(),
                        Dispatch::ToEditor(EnterNewline),
                    ),
                    Keymap::new(
                        "tab",
                        "Enter tab".to_string(),
                        Dispatch::ToEditor(Insert("\t".to_string())),
                    ),
                    Keymap::new(
                        "home",
                        "Move to line start".to_string(),
                        Dispatch::ToEditor(MoveToLineStart),
                    ),
                    Keymap::new(
                        "end",
                        "Move to line end".to_string(),
                        Dispatch::ToEditor(MoveToLineEnd),
                    ),
                ]
                .into_iter()
                .chain(self.keymap_universal())
                .collect_vec(),
            )),
        }
    }

    pub(crate) fn handle_insert_mode(&mut self, event: KeyEvent) -> anyhow::Result<Dispatches> {
        if let Some(dispatches) = self
            .insert_mode_keymaps()
            .iter()
            .find(|keymap| &event == keymap.event())
            .map(|keymap| keymap.get_dispatches())
        {
            Ok(dispatches)
        } else if let KeyCode::Char(c) = event.code {
            return self.insert(&c.to_string());
        } else {
            Ok(Default::default())
        }
    }

    pub(crate) fn handle_universal_key(
        &mut self,
        event: KeyEvent,
    ) -> anyhow::Result<HandleEventResult> {
        if let Some(keymap) = Keymaps::new(&self.keymap_universal()).get(&event) {
            Ok(HandleEventResult::Handled(keymap.get_dispatches()))
        } else {
            Ok(HandleEventResult::Ignored(event))
        }
    }

    pub(crate) fn keymap_others(&self, context: &Context) -> Vec<Keymap> {
        [
            Keymap::new(
                "space",
                "Search (List)".to_string(),
                Dispatch::ShowKeymapLegend(self.space_keymap_legend_config(context)),
            ),
            Keymap::new(
                "esc",
                "Remain only this window".to_string(),
                Dispatch::ToEditor(DispatchEditor::HandleEsc),
            ),
        ]
        .to_vec()
    }

    pub(crate) fn keymap_movement_actions(
        &self,
        normal_mode_override: &NormalModeOverride,
    ) -> Vec<Keymap> {
        [
            Keymap::new(
                "~",
                "Replace".to_string(),
                Dispatch::ToEditor(EnterReplaceMode),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::Swap_),
                "Swap".to_string(),
                "Enter Swap mode".to_string(),
                Dispatch::ToEditor(EnterSwapMode),
            ),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::MultC),
                "Multi Curs".to_string(),
                "Enter Multi-cursor mode".to_string(),
                Dispatch::ToEditor(EnterMultiCursorMode),
            )
            .override_keymap(normal_mode_override.multicursor.clone().as_ref()),
            Keymap::new_extended(
                KEYBOARD_LAYOUT.get_key(&Meaning::VMode),
                "V-mode".to_string(),
                "Enter V Mode".to_string(),
                Dispatch::ToEditor(EnterVMode),
            )
            .override_keymap(normal_mode_override.v.as_ref()),
        ]
        .into_iter()
        .collect_vec()
    }

    pub(crate) fn normal_mode_keymap_legend_config(
        &self,
        context: &Context,
        title: &'static str,
        normal_mode_override: Option<NormalModeOverride>,
    ) -> KeymapLegendConfig {
        let normal_mode_override = normal_mode_override
            .clone()
            .or_else(|| self.normal_mode_override.clone())
            .unwrap_or_default();
        KeymapLegendConfig {
            title: title.to_string(),
            body: KeymapLegendBody::Positional(Keymaps::new(
                &self
                    .keymap_core_movements()
                    .into_iter()
                    .chain(self.keymap_movement_actions(&normal_mode_override))
                    .chain(self.keymap_other_movements())
                    .chain(self.keymap_primary_selection_modes(context))
                    .chain(self.keymap_secondary_selection_modes_init(context))
                    .chain(self.keymap_actions(&normal_mode_override))
                    .chain(self.keymap_others(context))
                    .chain(self.keymap_universal())
                    .collect_vec(),
            )),
        }
    }

    pub(crate) fn normal_mode_keymaps(
        &self,
        context: &Context,
        normal_mode_override: Option<NormalModeOverride>,
    ) -> Keymaps {
        self.normal_mode_keymap_legend_config(context, "Normal", normal_mode_override)
            .keymaps()
    }

    pub(crate) fn multicursor_mode_keymap_legend_config(
        &self,
        context: &Context,
    ) -> KeymapLegendConfig {
        self.normal_mode_keymap_legend_config(
            context,
            "Multicursor",
            Some(NormalModeOverride {
                insert: Some(KeymapOverride {
                    description: "Keep Match",
                    dispatch: Dispatch::OpenFilterSelectionsPrompt { maintain: true },
                }),
                append: Some(KeymapOverride {
                    description: "Remove Match",
                    dispatch: Dispatch::OpenFilterSelectionsPrompt { maintain: false },
                }),
                change: Some(KeymapOverride {
                    description: "Keep Prime Curs",
                    dispatch: Dispatch::ToEditor(DispatchEditor::CursorKeepPrimaryOnly),
                }),
                delete: Some(KeymapOverride {
                    description: "Delete Curs →",
                    dispatch: Dispatch::ToEditor(DeleteCurrentCursor(Direction::Start)),
                }),
                delete_backward: Some(KeymapOverride {
                    description: "Delete Curs ←",
                    dispatch: Dispatch::ToEditor(DeleteCurrentCursor(Direction::End)),
                }),
                multicursor: Some(KeymapOverride {
                    description: "Curs All",
                    dispatch: Dispatch::ToEditor(CursorAddToAllSelections),
                }),
                ..Default::default()
            }),
        )
    }
    pub(crate) fn v_mode_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        self.normal_mode_keymap_legend_config(
            context,
            "V-mode",
            Some(NormalModeOverride {
                insert: Some(KeymapOverride {
                    description: "Inside",
                    dispatch: Dispatch::ShowKeymapLegend(
                        self.select_surround_keymap_legend_config(SurroundKind::Inside),
                    ),
                }),
                append: Some(KeymapOverride {
                    description: "Around",
                    dispatch: Dispatch::ShowKeymapLegend(
                        self.select_surround_keymap_legend_config(SurroundKind::Around),
                    ),
                }),
                delete: Some(KeymapOverride {
                    description: "Delete Surround",
                    dispatch: Dispatch::ShowKeymapLegend(
                        self.delete_surround_keymap_legend_config(),
                    ),
                }),
                change: Some(KeymapOverride {
                    description: "Change Surround",
                    dispatch: Dispatch::ShowKeymapLegend(
                        self.change_surround_from_keymap_legend_config(),
                    ),
                }),
                open: Some(KeymapOverride {
                    description: "Surround",
                    dispatch: Dispatch::ShowKeymapLegend(self.surround_keymap_legend_config()),
                }),
                v: Some(KeymapOverride {
                    description: "Select All",
                    dispatch: Dispatch::ToEditor(SelectAll),
                }),
                ..Default::default()
            }),
        )
    }
    pub(crate) fn transform_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Transform".to_string(),

            body: KeymapLegendBody::Positional(Keymaps::new(
                &[
                    (Meaning::Camel, "camelCase", Case::Camel),
                    (Meaning::Lower, "lower case", Case::Lower),
                    (Meaning::Kbab_, "kebab-case", Case::Kebab),
                    (Meaning::UKbab, "Upper-Kebab", Case::UpperKebab),
                    (Meaning::Pscal, "PascalCase", Case::Pascal),
                    (Meaning::Snke_, "snake_case", Case::Snake),
                    (Meaning::USnke, "UPPER_SNAKE_CASE", Case::UpperSnake),
                    (Meaning::Title, "Title Case", Case::Title),
                    (Meaning::Upper, "UPPER CASE", Case::Upper),
                ]
                .into_iter()
                .map(|(meaning, description, case)| {
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_transform_key(&meaning),
                        description.to_string(),
                        Dispatch::ToEditor(Transform(Transformation::Case(case))),
                    )
                })
                .chain(Some(Keymap::new(
                    KEYBOARD_LAYOUT.get_transform_key(&Meaning::Wrap_),
                    "Wrap".to_string(),
                    Dispatch::ToEditor(Transform(Transformation::Wrap)),
                )))
                .collect_vec(),
            )),
        }
    }

    fn space_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Space".to_string(),

            body: KeymapLegendBody::Positional(Keymaps::new(
                &[
                    (
                        KEYBOARD_LAYOUT.get_space_keymap(&Meaning::Buffr),
                        "Buffer",
                        FilePickerKind::Opened,
                    ),
                    (
                        KEYBOARD_LAYOUT.get_space_keymap(&Meaning::File_),
                        "File",
                        FilePickerKind::NonGitIgnored,
                    ),
                ]
                .into_iter()
                .map(|(key, description, kind)| {
                    Keymap::new(key, description.to_string(), Dispatch::OpenFilePicker(kind))
                })
                .chain(
                    [
                        (
                            KEYBOARD_LAYOUT.get_space_keymap(&Meaning::GitFC),
                            DiffMode::UnstagedAgainstCurrentBranch,
                        ),
                        (
                            KEYBOARD_LAYOUT.get_space_keymap(&Meaning::GitFM),
                            DiffMode::UnstagedAgainstMainBranch,
                        ),
                    ]
                    .into_iter()
                    .map(|(key, diff_mode)| {
                        Keymap::new(
                            key,
                            format!("Git status {}", diff_mode.display()),
                            Dispatch::OpenFilePicker(FilePickerKind::GitStatus(diff_mode)),
                        )
                    }),
                )
                .chain(Some(Keymap::new(
                    KEYBOARD_LAYOUT.get_space_keymap(&Meaning::Symbl),
                    "Symbol".to_string(),
                    Dispatch::RequestDocumentSymbols,
                )))
                .chain(Some(Keymap::new(
                    KEYBOARD_LAYOUT.get_space_keymap(&Meaning::Theme),
                    "Theme".to_string(),
                    Dispatch::OpenThemePrompt,
                )))
                .chain(context.contextual_keymaps())
                .chain(self.keymap_clipboard_related_actions(true, Default::default()))
                .chain([
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_space_keymap(&Meaning::Explr),
                        "Explorer".to_string(),
                        Dispatch::RevealInExplorer(
                            self.path()
                                .unwrap_or_else(|| context.current_working_directory().clone()),
                        ),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_space_keymap(&Meaning::UndoT),
                        "Undo Tree".to_string(),
                        Dispatch::ToEditor(DispatchEditor::EnterUndoTreeMode),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_space_keymap(&Meaning::TSNSx),
                        "TS Node Sexp".to_string(),
                        Dispatch::ToEditor(DispatchEditor::ShowCurrentTreeSitterNodeSexp),
                    ),
                    Keymap::new(
                        "enter",
                        "Force Save".to_string(),
                        Dispatch::ToEditor(DispatchEditor::ForceSave),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_space_keymap(&Meaning::SaveA),
                        "Save All".to_string(),
                        Dispatch::SaveAll,
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_space_keymap(&Meaning::QSave),
                        "Save All Quit".to_string(),
                        Dispatch::SaveQuitAll,
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_space_keymap(&Meaning::QNSav),
                        "Quit No Save".to_string(),
                        Dispatch::QuitAll,
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_space_keymap(&Meaning::Pipe_),
                        "Pipe".to_string(),
                        Dispatch::OpenPipeToShellPrompt,
                    ),
                ])
                .collect_vec(),
            )),
        }
    }

    fn search_current_selection_keymap(
        &self,
        key: &'static str,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
    ) -> Option<Keymap> {
        self.buffer()
            .slice(&self.selection_set.primary_selection().extended_range())
            .map(|search| {
                Keymap::new_extended(
                    key,
                    "This".to_string(),
                    "Search current selection".to_string(),
                    Dispatch::UpdateLocalSearchConfig {
                        scope,
                        if_current_not_found,
                        update: crate::app::LocalSearchConfigUpdate::Search(search.to_string()),
                        show_config_after_enter: false,
                        run_search_after_config_updated: true,
                    },
                )
            })
            .ok()
    }

    pub(crate) fn secondary_selection_modes_keymap_legend_config(
        &self,
        context: &Context,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
    ) -> KeymapLegendConfig {
        let search_keymaps = {
            let config = context.get_local_search_config(scope);
            [].into_iter()
                .chain(
                    [
                        Keymap::new_extended(
                            KEYBOARD_LAYOUT.get_find_keymap(scope, &Meaning::CSrch),
                            "Config".to_string(),
                            "Configure Search".to_string(),
                            Dispatch::ShowSearchConfig {
                                scope,
                                if_current_not_found,
                                run_search_after_config_updated: true,
                            },
                        ),
                        Keymap::new(
                            KEYBOARD_LAYOUT.get_find_keymap(scope, &Meaning::PSrch),
                            "Last".to_string(),
                            Dispatch::UpdateLocalSearchConfig {
                                scope,
                                if_current_not_found,
                                update: crate::app::LocalSearchConfigUpdate::Search(
                                    config
                                        .last_search()
                                        .map(|search| search.search.to_string())
                                        .unwrap_or_default(),
                                ),
                                show_config_after_enter: false,
                                run_search_after_config_updated: true,
                            },
                        ),
                        Keymap::new(
                            KEYBOARD_LAYOUT.get_find_keymap(
                                scope,
                                &match (scope, if_current_not_found) {
                                    (Scope::Local, IfCurrentNotFound::LookForward) => {
                                        Meaning::FindN
                                    }
                                    (Scope::Local, IfCurrentNotFound::LookBackward) => {
                                        Meaning::FindP
                                    }
                                    (Scope::Global, _) => Meaning::Globl,
                                },
                            ),
                            "Repeat".to_string(),
                            Dispatch::UseLastNonContiguousSelectionMode(if_current_not_found),
                        ),
                    ]
                    .to_vec(),
                )
                .collect_vec()
        };
        let misc_keymaps = [
            Keymap::new(
                KEYBOARD_LAYOUT.get_find_keymap(scope, &Meaning::Mark_),
                "Mark".to_string(),
                match scope {
                    Scope::Global => Dispatch::SetQuickfixList(QuickfixListType::Mark),
                    Scope::Local => {
                        Dispatch::ToEditor(SetSelectionMode(if_current_not_found, Mark))
                    }
                },
            ),
            Keymap::new(
                KEYBOARD_LAYOUT.get_find_keymap(scope, &Meaning::Qkfix),
                "Quickfix".to_string(),
                match scope {
                    Scope::Global => {
                        Dispatch::SetGlobalMode(Some(crate::context::GlobalMode::QuickfixListItem))
                    }
                    Scope::Local => Dispatch::ToEditor(SetSelectionMode(
                        if_current_not_found,
                        LocalQuickfix {
                            title: "LOCAL QUICKFIX".to_string(),
                        },
                    )),
                },
            ),
        ]
        .into_iter()
        .chain(
            [
                (Meaning::GHnkC, DiffMode::UnstagedAgainstCurrentBranch),
                (Meaning::GHnkM, DiffMode::UnstagedAgainstMainBranch),
            ]
            .map(|(meaning, diff_mode)| {
                Keymap::new(
                    KEYBOARD_LAYOUT.get_find_keymap(scope, &meaning),
                    format!("Hunk{}", diff_mode.display()),
                    match scope {
                        Scope::Global => Dispatch::GetRepoGitHunks(diff_mode),
                        Scope::Local => Dispatch::ToEditor(SetSelectionMode(
                            if_current_not_found,
                            GitHunk(diff_mode),
                        )),
                    },
                )
            }),
        )
        .collect_vec();
        let diagnostics_keymaps = [
            (Meaning::DgAll, "All", DiagnosticSeverityRange::All),
            (Meaning::DgErr, "Error", DiagnosticSeverityRange::Error),
            (Meaning::DgHnt, "Hint", DiagnosticSeverityRange::Hint),
            (Meaning::DgInf, "Info", DiagnosticSeverityRange::Information),
            (Meaning::DgWrn, "Warn", DiagnosticSeverityRange::Warning),
        ]
        .into_iter()
        .map(|(meaning, description, severity)| {
            Keymap::new(
                KEYBOARD_LAYOUT.get_find_keymap(scope, &meaning),
                description.to_string(),
                match scope {
                    Scope::Local => Dispatch::ToEditor(SetSelectionMode(
                        if_current_not_found,
                        Diagnostic(severity),
                    )),
                    Scope::Global => {
                        Dispatch::SetQuickfixList(QuickfixListType::Diagnostic(severity))
                    }
                },
            )
        })
        .collect_vec();
        let lsp_keymaps = [
            Keymap::new(
                KEYBOARD_LAYOUT.get_find_keymap(scope, &Meaning::LDefn),
                "Def".to_string(),
                Dispatch::RequestDefinitions(scope),
            ),
            Keymap::new(
                KEYBOARD_LAYOUT.get_find_keymap(scope, &Meaning::LDecl),
                "Decl".to_string(),
                Dispatch::RequestDeclarations(scope),
            ),
            Keymap::new(
                KEYBOARD_LAYOUT.get_find_keymap(scope, &Meaning::LImpl),
                "Impl".to_string(),
                Dispatch::RequestImplementations(scope),
            ),
            Keymap::new(
                KEYBOARD_LAYOUT.get_find_keymap(scope, &Meaning::LRfrE),
                "Ref-".to_string(),
                Dispatch::RequestReferences {
                    include_declaration: false,
                    scope,
                },
            ),
            Keymap::new(
                KEYBOARD_LAYOUT.get_find_keymap(scope, &Meaning::LRfrI),
                "Ref+".to_string(),
                Dispatch::RequestReferences {
                    include_declaration: true,
                    scope,
                },
            ),
            Keymap::new(
                KEYBOARD_LAYOUT.get_find_keymap(scope, &Meaning::LType),
                "Type".to_string(),
                Dispatch::RequestTypeDefinitions(scope),
            ),
        ];
        let scope_specific_keymaps = match scope {
            Scope::Local => [(
                KEYBOARD_LAYOUT.get_find_keymap(scope, &Meaning::NtrlN),
                "Int",
                r"\d+",
            )]
            .into_iter()
            .map(|(key, description, regex)| {
                let search = Search {
                    search: regex.to_string(),
                    mode: LocalSearchConfigMode::Regex(RegexConfig {
                        escaped: false,
                        match_whole_word: false,
                        case_sensitive: false,
                    }),
                };
                let dispatch =
                    Dispatch::ToEditor(SetSelectionMode(if_current_not_found, Find { search }));
                Keymap::new(key, description.to_string(), dispatch)
            })
            .chain([Keymap::new(
                KEYBOARD_LAYOUT.get_find_keymap(scope, &Meaning::OneCh),
                "One".to_string(),
                Dispatch::ToEditor(FindOneChar(if_current_not_found)),
            )])
            .collect_vec(),
            Scope::Global => [Keymap::new_extended(
                KEYBOARD_LAYOUT.get_find_keymap(scope, &Meaning::Srch_),
                "Search".to_string(),
                "Search".to_string(),
                Dispatch::OpenSearchPrompt {
                    scope,
                    if_current_not_found,
                },
            )]
            .into_iter()
            .chain(self.search_current_selection_keymap(
                KEYBOARD_LAYOUT.get_find_keymap(scope, &Meaning::SrchC),
                scope,
                if_current_not_found,
            ))
            .collect_vec(),
        };
        KeymapLegendConfig {
            title: format!(
                "Find ({})",
                match scope {
                    Scope::Local => "Local",
                    Scope::Global => "Global",
                }
            ),

            body: KeymapLegendBody::Positional(Keymaps::new(
                &search_keymaps
                    .into_iter()
                    .chain(misc_keymaps)
                    .chain(diagnostics_keymaps)
                    .chain(lsp_keymaps)
                    .chain(scope_specific_keymaps)
                    .collect_vec(),
            )),
        }
    }

    pub(crate) fn select_surround_keymap_legend_config(
        &self,
        kind: SurroundKind,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: format!("Select Surround ({:?})", kind),

            body: KeymapLegendBody::Positional(generate_enclosures_keymaps(|enclosure| {
                Dispatch::ToEditor(SelectSurround {
                    enclosure,
                    kind: kind.clone(),
                })
            })),
        }
    }

    pub(crate) fn delete_surround_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Delete Surround".to_string(),

            body: KeymapLegendBody::Positional(generate_enclosures_keymaps(|enclosure| {
                Dispatch::ToEditor(DeleteSurround(enclosure))
            })),
        }
    }

    pub(crate) fn surround_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Surround".to_string(),

            body: KeymapLegendBody::Positional(generate_enclosures_keymaps(|enclosure| {
                let (open, close) = enclosure.open_close_symbols_str();
                Dispatch::ToEditor(Surround(open.to_string(), close.to_string()))
            })),
        }
    }

    pub(crate) fn change_surround_from_keymap_legend_config(
        &self,
    ) -> super::keymap_legend::KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Change Surround from:".to_string(),

            body: KeymapLegendBody::Positional(generate_enclosures_keymaps(|enclosure| {
                Dispatch::ShowKeymapLegend(self.change_surround_to_keymap_legend_config(enclosure))
            })),
        }
    }

    pub(crate) fn change_surround_to_keymap_legend_config(
        &self,
        from: EnclosureKind,
    ) -> super::keymap_legend::KeymapLegendConfig {
        KeymapLegendConfig {
            title: format!("Change Surround from {} to:", from.to_str()),

            body: KeymapLegendBody::Positional(generate_enclosures_keymaps(|enclosure| {
                Dispatch::ToEditor(ChangeSurround {
                    from,
                    to: enclosure,
                })
            })),
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct NormalModeOverride {
    pub(crate) change: Option<KeymapOverride>,
    pub(crate) delete: Option<KeymapOverride>,
    pub(crate) insert: Option<KeymapOverride>,
    pub(crate) append: Option<KeymapOverride>,
    pub(crate) open: Option<KeymapOverride>,
    pub(crate) delete_backward: Option<KeymapOverride>,
    pub(crate) paste: Option<KeymapOverride>,
    pub(crate) replace: Option<KeymapOverride>,
    pub(crate) v: Option<KeymapOverride>,
    pub(crate) multicursor: Option<KeymapOverride>,
}

#[derive(Clone)]
pub(crate) struct KeymapOverride {
    pub(crate) description: &'static str,
    pub(crate) dispatch: Dispatch,
}

fn generate_enclosures_keymaps(get_dispatch: impl Fn(EnclosureKind) -> Dispatch) -> Keymaps {
    Keymaps::new(
        &[
            (Meaning::Anglr, EnclosureKind::AngularBrackets),
            (Meaning::Paren, EnclosureKind::Parentheses),
            (Meaning::Brckt, EnclosureKind::SquareBrackets),
            (Meaning::Brace, EnclosureKind::CurlyBraces),
            (Meaning::DQuot, EnclosureKind::DoubleQuotes),
            (Meaning::SQuot, EnclosureKind::SingleQuotes),
            (Meaning::BckTk, EnclosureKind::Backticks),
        ]
        .into_iter()
        .map(|(meaning, enclosure)| {
            let (open, close) = enclosure.open_close_symbols_str();
            Keymap::new(
                KEYBOARD_LAYOUT.get_surround_keymap(&meaning),
                format!("{open} {close}"),
                get_dispatch(enclosure),
            )
        })
        .collect_vec(),
    )
}
