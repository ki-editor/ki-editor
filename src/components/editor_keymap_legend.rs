use crossterm::event::{KeyCode, KeyEventKind};
use SelectionMode::*;

use convert_case::Case;
use event::KeyEvent;
use itertools::Itertools;

use crate::{
    app::{Dispatch, Dispatches, FilePickerKind, Scope},
    components::{
        editor::{Movement, PriorChange},
        editor_keymap::QWERTY,
        keymap_legend::{MomentaryLayer, OnSpacebarTapped, OnTap},
    },
    context::{Context, LocalSearchConfigMode, Search},
    git::DiffMode,
    list::grep::RegexConfig,
    quickfix_list::{DiagnosticSeverityRange, QuickfixListType},
    scripting::custom_keymap,
    selection::SelectionMode,
    selection_mode::GetGapMovement,
    surround::EnclosureKind,
    transformation::Transformation,
};

use super::{
    editor::{
        Direction, DispatchEditor, Editor, HandleEventResult, IfCurrentNotFound, Reveal,
        SurroundKind,
    },
    keymap_legend::{Keybinding, Keymap, KeymapLegendConfig},
};

use DispatchEditor::*;
use Movement::*;
impl Editor {
    pub fn keymap_core_movements(&self, prior_change: Option<PriorChange>) -> Vec<Keybinding> {
        [
            Keybinding::new(
                "j",
                "<<".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Left, prior_change)),
            ),
            Keybinding::new(
                "l",
                ">>".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Right, prior_change)),
            ),
            Keybinding::new(
                "i",
                "^".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Up, prior_change)),
            ),
            Keybinding::new(
                "k",
                "v".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Down, prior_change)),
            ),
            Keybinding::new(
                "y",
                "|<".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::First, prior_change)),
            ),
            Keybinding::new(
                "p",
                ">|".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Last, prior_change)),
            ),
            Keybinding::new(
                "o",
                ">".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Next, prior_change)),
            ),
            Keybinding::new(
                "u",
                "<".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(
                    Movement::Previous,
                    prior_change,
                )),
            ),
            Keybinding::new_extended(
                "m",
                "Jump".to_string(),
                "Jump".to_string(),
                Dispatch::ToEditor(DispatchEditor::ShowJumps {
                    use_current_selection_mode: true,
                    prior_change,
                }),
            ),
            Keybinding::new_extended(
                "M",
                "Index".to_string(),
                "To Index (1-based)".to_string(),
                Dispatch::OpenMoveToIndexPrompt(prior_change),
            ),
            Keybinding::new(
                ".",
                "Parent Line".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(
                    Movement::ParentLine,
                    prior_change,
                )),
            ),
        ]
        .to_vec()
    }

    pub fn keymap_other_movements(&self) -> Vec<Keybinding> {
        [
            Keybinding::new_extended(
                "alt+k",
                "Scroll ↓".to_string(),
                "Scroll down".to_string(),
                Dispatch::ToEditor(ScrollPageDown),
            ),
            Keybinding::new_extended(
                "alt+i",
                "Scroll ↑".to_string(),
                "Scroll up".to_string(),
                Dispatch::ToEditor(ScrollPageUp),
            ),
            Keybinding::new_extended(
                "backspace",
                Direction::Start.format_action("Select"),
                "Go back".to_string(),
                Dispatch::ToEditor(GoBack),
            ),
            Keybinding::new_extended(
                "tab",
                Direction::End.format_action("Select"),
                "Go forward".to_string(),
                Dispatch::ToEditor(GoForward),
            ),
            Keybinding::new_extended(
                "alt+y",
                Direction::Start.format_action("Nav"),
                "Navigate back".to_string(),
                Dispatch::NavigateBack,
            ),
            Keybinding::new_extended(
                "alt+p",
                Direction::End.format_action("Nav"),
                "Navigate forward".to_string(),
                Dispatch::NavigateForward,
            ),
            Keybinding::momentary_layer(MomentaryLayer {
                key: "e",
                description: "Buffer".to_string(),
                config: KeymapLegendConfig {
                    title: "Buffer".to_string(),
                    keymap: buffer_keymap(),
                },
                on_tap: Some(OnTap::new(
                    "Toggle Selection Mark",
                    Dispatches::one(Dispatch::MarkFileAndToggleMark),
                )),
                on_spacebar_tapped: None,
            }),
            Keybinding::new_extended(
                "?",
                "⇋ Anchor".to_string(),
                "Swap Anchor".to_string(),
                Dispatch::ToEditor(SwapExtensionAnchor),
            ),
            Keybinding::new_extended(
                "/",
                "⇋ Curs".to_string(),
                "Swap cursor".to_string(),
                Dispatch::ToEditor(DispatchEditor::SwapCursor),
            ),
        ]
        .into_iter()
        .collect()
    }

    pub fn keymap_primary_selection_modes(
        &self,
        prior_change: Option<PriorChange>,
    ) -> Vec<Keybinding> {
        let direction = self.cursor_direction.reverse().to_if_current_not_found();
        primary_selection_modes()
            .into_iter()
            .map(|(key, selection_mode)| {
                Keybinding::new(
                    key,
                    selection_mode.display(),
                    Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                        direction,
                        selection_mode,
                        prior_change,
                    )),
                )
            })
            .collect_vec()
    }

    pub fn keymap_secondary_selection_modes_init(
        &self,
        prior_change: Option<PriorChange>,
    ) -> Vec<Keybinding> {
        [Keybinding::new_extended(
            "n",
            Direction::End.format_action("Local"),
            "Find (Local)".to_string(),
            Dispatch::ShowKeymapLegend(self.secondary_selection_modes_keymap_legend_config(
                Scope::Local,
                self.cursor_direction.reverse().to_if_current_not_found(),
                prior_change,
            )),
        )]
        .to_vec()
    }

    pub fn keymap_actions(
        &self,
        normal_mode_override: &NormalModeOverride,
        none_if_no_override: bool,
        _prior_change: Option<PriorChange>,
    ) -> Vec<Keybinding> {
        [
            Keybinding::new_extended(
                "I",
                "Join".to_string(),
                "Join".to_string(),
                Dispatch::ToEditor(JoinSelection),
            ),
            Keybinding::new_extended(
                "K",
                "Break".to_string(),
                "Break".to_string(),
                Dispatch::ToEditor(BreakSelection),
            ),
            Keybinding::new(
                "Y",
                Direction::Start.format_action("Align"),
                Dispatch::ToEditor(AlignSelections(Direction::Start)),
            ),
            Keybinding::new(
                "P",
                Direction::End.format_action("Align"),
                Dispatch::ToEditor(AlignSelections(Direction::End)),
            ),
            Keybinding::new_extended(
                "T",
                "Raise".to_string(),
                "Raise".to_string(),
                Dispatch::ToEditor(Replace(Expand)),
            ),
            Keybinding::new_extended(
                "z",
                "Undo".to_string(),
                "Undo".to_string(),
                Dispatch::ToEditor(Undo),
            ),
            Keybinding::new_extended(
                "Z",
                "Redo".to_string(),
                "Redo".to_string(),
                Dispatch::ToEditor(Redo),
            ),
            Keybinding::new_extended(
                "enter",
                "save".to_string(),
                "Save".to_string(),
                Dispatch::ToEditor(Save),
            ),
            Keybinding::new_extended(
                "G",
                "Transform".to_string(),
                "Transform".to_string(),
                Dispatch::ShowKeymapLegend(self.transform_keymap_legend_config()),
            ),
            Keybinding::new(
                "$",
                Direction::End.format_action("Collapse selection"),
                Dispatch::ToEditor(DispatchEditor::CollapseSelection(Direction::End)),
            ),
            Keybinding::new_extended(
                "L",
                "Indent".to_string(),
                "Indent".to_string(),
                Dispatch::ToEditor(Indent),
            ),
            Keybinding::new_extended(
                "J",
                "Dedent".to_string(),
                "Dedent".to_string(),
                Dispatch::ToEditor(Dedent),
            ),
            Keybinding::new(
                "*",
                "Keyboard".to_string(),
                Dispatch::OpenKeyboardLayoutPrompt,
            ),
        ]
        .into_iter()
        .chain(self.keymap_actions_overridable(normal_mode_override, none_if_no_override))
        .chain(self.keymap_clipboard_related_actions(false, normal_mode_override.clone()))
        .collect_vec()
    }

    pub fn keymap_actions_overridable(
        &self,
        normal_mode_override: &NormalModeOverride,
        none_if_no_override: bool,
    ) -> Vec<Keybinding> {
        [
            Keybinding::momentary_layer(MomentaryLayer {
                key: "f",
                description: "Insert".to_string(),
                config: KeymapLegendConfig {
                    title: "Insert".to_string(),
                    keymap: insert_keymap(),
                },
                on_tap: Some(OnTap::new(
                    "Change",
                    Dispatches::one(Dispatch::ToEditor(Change)),
                )),
                on_spacebar_tapped: None,
            })
            .override_keymap(normal_mode_override.change.as_ref(), none_if_no_override),
            Keybinding::momentary_layer(MomentaryLayer {
                key: "v",
                description: "Delete".to_string(),
                config: KeymapLegendConfig {
                    title: "Delete".to_string(),
                    keymap: delete_keymap(),
                },
                on_tap: Some(OnTap::new(
                    "Delete One",
                    Dispatches::one(Dispatch::ToEditor(DispatchEditor::DeleteOne)),
                )),
                on_spacebar_tapped: None,
            })
            .override_keymap(normal_mode_override.delete.as_ref(), none_if_no_override),
            Keybinding::new_extended(
                "h",
                Direction::Start.format_action("Insert"),
                Direction::Start.format_action("Insert"),
                Dispatch::ToEditor(EnterInsertMode(Direction::Start)),
            )
            .override_keymap(normal_mode_override.insert.as_ref(), none_if_no_override),
            Keybinding::new_extended(
                ";",
                Direction::End.format_action("Insert"),
                Direction::End.format_action("Insert"),
                Dispatch::ToEditor(EnterInsertMode(Direction::End)),
            )
            .override_keymap(normal_mode_override.append.as_ref(), none_if_no_override),
            Keybinding::new(",", Direction::End.format_action(""), Dispatch::Null)
                .override_keymap(normal_mode_override.open.as_ref(), none_if_no_override),
        ]
        .into_iter()
        .flatten()
        .collect_vec()
    }

    pub fn keymap_overridable(
        &self,
        normal_mode_override: &NormalModeOverride,
        none_if_no_override: bool,
    ) -> Vec<Keybinding> {
        self.keymap_actions_overridable(normal_mode_override, none_if_no_override)
            .into_iter()
            .chain(self.keymap_clipboard_related_actions_overridable(
                false,
                normal_mode_override.clone(),
                none_if_no_override,
            ))
            .collect_vec()
    }

    fn keymap_clipboard_related_actions(
        &self,
        use_system_clipboard: bool,
        normal_mode_override: NormalModeOverride,
    ) -> Vec<Keybinding> {
        let extra = if use_system_clipboard { "+ " } else { "" };
        let format = |description: &str| format!("{extra}{description}");
        [
            Keybinding::new_extended(
                "F",
                format("Change X"),
                format!("{}{}", "Change Cut", extra),
                Dispatch::ToEditor(ChangeCut),
            ),
            Keybinding::momentary_layer(MomentaryLayer {
                key: "c",
                description: "Copy".to_string(),
                config: KeymapLegendConfig {
                    title: "Duplicate".to_string(),
                    keymap: duplicate_keymap(),
                },
                on_tap: Some(OnTap::new(
                    "Copy",
                    Dispatches::one(Dispatch::ToEditor(Copy)),
                )),
                on_spacebar_tapped: None,
            }),
        ]
        .into_iter()
        .chain(self.keymap_clipboard_related_actions_overridable(
            use_system_clipboard,
            normal_mode_override,
            false,
        ))
        .collect_vec()
    }

    fn keymap_clipboard_related_actions_overridable(
        &self,
        use_system_clipboard: bool,
        normal_mode_override: NormalModeOverride,
        none_if_no_override: bool,
    ) -> Vec<Keybinding> {
        let extra = if use_system_clipboard { "+ " } else { "" };
        let format = |description: &str| format!("{extra}{description}");
        [
            Keybinding::momentary_layer(MomentaryLayer {
                key: "b",
                description: format("Paste"),
                config: KeymapLegendConfig {
                    title: "Paste".to_string(),
                    keymap: paste_keymap(),
                },
                on_tap: Some(OnTap::new(
                    "Replace",
                    Dispatches::one(Dispatch::ToEditor(DispatchEditor::ReplaceWithCopiedText {
                        cut: false,
                    })),
                )),
                on_spacebar_tapped: None,
            })
            .override_keymap(
                normal_mode_override.paste.clone().as_ref(),
                none_if_no_override,
            ),
            Keybinding::momentary_layer(MomentaryLayer {
                key: "x",
                description: "Cut".to_string(),
                config: KeymapLegendConfig {
                    title: "Cut".to_string(),
                    keymap: cut_keymap(),
                },
                on_tap: Some(OnTap::new(
                    "Cut One",
                    Dispatches::one(Dispatch::ToEditor(DispatchEditor::CutOne)),
                )),
                on_spacebar_tapped: None,
            })
            .override_keymap(normal_mode_override.cut.as_ref(), none_if_no_override),
        ]
        .into_iter()
        .flatten()
        .collect_vec()
    }

    pub fn keymap_universal(&self) -> Vec<Keybinding> {
        [
            Keybinding::new_extended(
                "alt+;",
                "⇋ Align View".to_string(),
                "Switch view alignment".to_string(),
                Dispatch::ToEditor(SwitchViewAlignment),
            ),
            Keybinding::new_extended(
                "alt+/",
                "⇋ Window".to_string(),
                "Switch window".to_string(),
                Dispatch::OtherWindow,
            ),
            #[cfg(unix)]
            Keybinding::new("ctrl+z", "Suspend".to_string(), Dispatch::Suspend),
        ]
        .to_vec()
    }

    pub fn insert_mode_keymap_legend_config(
        &self,
        include_universal_keymap: bool,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Insert mode keymap".to_string(),
            keymap: Keymap::new(
                &[
                    Keybinding::new_extended(
                        "left",
                        "Char ←".to_string(),
                        "Move back a character".to_string(),
                        Dispatch::ToEditor(MoveCharacterBack),
                    ),
                    Keybinding::new_extended(
                        "right",
                        "Char →".to_string(),
                        "Move forward a character".to_string(),
                        Dispatch::ToEditor(MoveCharacterForward),
                    ),
                    Keybinding::new_extended(
                        "alt+s",
                        "Line ←".to_string(),
                        "Move to line start".to_string(),
                        Dispatch::ToEditor(MoveToLineStart),
                    ),
                    Keybinding::new_extended(
                        "alt+f",
                        "Line →".to_string(),
                        "Move to line end".to_string(),
                        Dispatch::ToEditor(MoveToLineEnd),
                    ),
                    Keybinding::new_extended(
                        "alt+q",
                        "Kill Line ←".to_string(),
                        Direction::End.format_action("Kill line"),
                        Dispatch::ToEditor(KillLine(Direction::Start)),
                    ),
                    Keybinding::new_extended(
                        "alt+t",
                        "Kill Line →".to_string(),
                        Direction::End.format_action("Kill line"),
                        Dispatch::ToEditor(KillLine(Direction::End)),
                    ),
                    Keybinding::new_extended(
                        "alt+h",
                        "Delete Word ←".to_string(),
                        "Delete word backward".to_string(),
                        Dispatch::ToEditor(DeleteWordBackward { short: false }),
                    ),
                    Keybinding::new_extended(
                        "alt+backspace",
                        "Delete Word ←".to_string(),
                        "Delete word backward".to_string(),
                        Dispatch::ToEditor(DeleteWordBackward { short: true }),
                    ),
                    Keybinding::new(
                        "left",
                        "Move back a character".to_string(),
                        Dispatch::ToEditor(MoveCharacterBack),
                    ),
                    Keybinding::new(
                        "right",
                        "Move forward a character".to_string(),
                        Dispatch::ToEditor(MoveCharacterForward),
                    ),
                    Keybinding::new(
                        "esc",
                        "Enter normal mode".to_string(),
                        Dispatch::ToEditor(EnterNormalMode),
                    ),
                    Keybinding::new(
                        "backspace",
                        "Delete character backward".to_string(),
                        Dispatch::ToEditor(Backspace),
                    ),
                    Keybinding::new(
                        "enter",
                        "Enter new line".to_string(),
                        Dispatch::ToEditor(EnterNewline),
                    ),
                    Keybinding::new(
                        "tab",
                        "Enter tab".to_string(),
                        Dispatch::ToEditor(Insert("\t".to_string())),
                    ),
                    Keybinding::new(
                        "home",
                        "Move to line start".to_string(),
                        Dispatch::ToEditor(MoveToLineStart),
                    ),
                    Keybinding::new(
                        "end",
                        "Move to line end".to_string(),
                        Dispatch::ToEditor(MoveToLineEnd),
                    ),
                ]
                .into_iter()
                .chain(if include_universal_keymap {
                    self.keymap_universal()
                } else {
                    Default::default()
                })
                .collect_vec(),
            ),
        }
    }

    pub fn handle_insert_mode(
        &mut self,
        context: &Context,
        event: KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        let translated_event = context
            .keyboard_layout_kind()
            .translate_key_event_to_qwerty(event.clone());
        if let Some(dispatches) = self
            .insert_mode_keymap(true)
            .iter()
            .find(|keymap| {
                keymap
                    .event()
                    .is_press_or_repeat_equivalent(&translated_event)
            })
            .map(|keymap| keymap.get_dispatches())
        {
            Ok(dispatches)
        } else if let (KeyCode::Char(c), KeyEventKind::Press | KeyEventKind::Repeat) =
            (event.code, event.kind)
        {
            let mut auto_pair = |enclosure: &str| -> anyhow::Result<Dispatches> {
                Ok(self
                    .insert(enclosure, context)?
                    .append(Dispatch::ToEditor(DispatchEditor::MoveCharacterBack)))
            };
            match c {
                '[' => auto_pair("[]"),
                '{' => auto_pair("{}"),
                '(' => auto_pair("()"),
                '\'' => auto_pair("''"),
                '"' => auto_pair("\"\""),
                '`' => auto_pair("``"),
                c => self.insert(&c.to_string(), context),
            }
        } else {
            Ok(Default::default())
        }
    }

    pub fn handle_universal_key(&mut self, event: KeyEvent) -> anyhow::Result<HandleEventResult> {
        if let Some(keymap) = Keymap::new(&self.keymap_universal()).get(&event) {
            Ok(HandleEventResult::Handled(keymap.get_dispatches()))
        } else {
            Ok(HandleEventResult::Ignored(event))
        }
    }

    pub fn keymap_others(&self) -> Vec<Keybinding> {
        [
            Keybinding::new(
                "space",
                "Space".to_string(),
                Dispatch::ToEditor(DispatchEditor::PressSpace),
            ),
            Keybinding::new(
                "esc",
                "Remain only this window".to_string(),
                Dispatch::ToEditor(DispatchEditor::HandleEsc),
            ),
        ]
        .to_vec()
    }

    pub fn keymap_sub_modes(&self) -> Vec<Keybinding> {
        [
            Some(Keybinding::new(
                "~",
                "Replace".to_string(),
                Dispatch::ToEditor(EnterReplaceMode),
            )),
            Some(Keybinding::momentary_layer(MomentaryLayer {
                key: "t",
                description: "Swap".to_string(),
                config: KeymapLegendConfig {
                    title: "Swap".to_string(),
                    keymap: swap_keymap(),
                },
                on_tap: None,
                on_spacebar_tapped: None,
            })),
            Some(Keybinding::new(
                "backslash",
                "Leader".to_string(),
                Dispatch::ShowKeymapLegend(self.leader_keymap_legend_config()),
            )),
            Some(Keybinding::momentary_layer(MomentaryLayer {
                key: "r",
                description: "Multi-cursor".to_string(),
                config: KeymapLegendConfig {
                    title: "Multi-cursor Momentary Layer".to_string(),
                    keymap: self.multicursor_momentary_layer_keymap(),
                },
                on_tap: None,
                on_spacebar_tapped: Some(OnSpacebarTapped::DeactivatesMomentaryLaterAndOpenMenu(
                    KeymapLegendConfig {
                        title: "Multi-cursor Menu".to_string(),
                        keymap: self.multicursor_menu_keymap(),
                    },
                )),
            })),
        ]
        .into_iter()
        .flatten()
        .collect_vec()
    }

    pub fn multicursor_momentary_layer_keymap(&self) -> Keymap {
        Keymap::new(
            &[
                (Movement::Up, "i"),
                (Movement::Down, "k"),
                (Movement::Left, "j"),
                (Movement::Right, "l"),
                (Movement::Previous, "u"),
                (Movement::Next, "o"),
                (Movement::First, "y"),
                (Movement::Last, "p"),
            ]
            .into_iter()
            .map(|(movement, key)| {
                Keybinding::new(
                    key,
                    movement.format_action("Add Curs"),
                    Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(movement)),
                )
            })
            .chain([
                Keybinding::new(
                    "n",
                    "Delete Curs".to_string(),
                    Dispatch::ToEditor(DeleteCurrentCursor(Direction::End)),
                ),
                Keybinding::new_extended(
                    "h",
                    Direction::Start.format_action("Curs"),
                    Direction::Start.format_action("Cycle primary selection"),
                    Dispatch::ToEditor(CyclePrimarySelection(Direction::Start)),
                ),
                Keybinding::new_extended(
                    ";",
                    Direction::End.format_action("Curs"),
                    Direction::End.format_action("Cycle primary selection"),
                    Dispatch::ToEditor(CyclePrimarySelection(Direction::End)),
                ),
                Keybinding::new(
                    "m",
                    "Jump Add Curs".to_string(),
                    Dispatch::ToEditor(ShowJumps {
                        use_current_selection_mode: true,
                        prior_change: Some(PriorChange::EnterMultiCursorMode),
                    }),
                ),
            ])
            .collect_vec(),
        )
    }

    pub fn normal_mode_keymap_legend_config(
        &self,
        normal_mode_override: Option<NormalModeOverride>,
        prior_change: Option<PriorChange>,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Normal".to_string(),
            keymap: Keymap::new(
                &self
                    .normal_mode_keymap(normal_mode_override, prior_change)
                    .into_iter()
                    .chain(Some(Keybinding::new(
                        "g",
                        "Extend".to_string(),
                        Dispatch::ShowKeymapLegend(self.extend_mode_keymap_legend_config()),
                    )))
                    .collect_vec(),
            ),
        }
    }

    pub fn normal_mode_keymap(
        &self,
        normal_mode_override: Option<NormalModeOverride>,
        prior_change: Option<PriorChange>,
    ) -> Vec<Keybinding> {
        let normal_mode_override = normal_mode_override
            .clone()
            .or_else(|| self.normal_mode_override.clone())
            .unwrap_or_default();
        self.keymap_core_movements(prior_change)
            .into_iter()
            .chain(self.keymap_sub_modes())
            .chain(self.keymap_other_movements())
            .chain(self.keymap_primary_selection_modes(prior_change))
            .chain(self.keymap_secondary_selection_modes_init(prior_change))
            .chain(self.keymap_actions(&normal_mode_override, false, prior_change))
            .chain(self.keymap_others())
            .chain(self.keymap_universal())
            .collect_vec()
    }
    pub fn extend_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Extend".to_string(),
            keymap: Keymap::new(
                &self
                    .normal_mode_keymap(
                        Some(extend_mode_normal_mode_override()),
                        Some(PriorChange::EnableSelectionExtension),
                    )
                    .into_iter()
                    .chain(Some(Keybinding::new(
                        "g",
                        "Select All".to_string(),
                        Dispatch::ToEditor(SelectAll),
                    )))
                    .collect_vec(),
            ),
        }
    }
    pub fn keymap_transform(&self) -> Vec<Keybinding> {
        [
            ("q", "UPPER CASE", Case::Upper),
            ("w", "UPPER_SNAKE_CASE", Case::UpperSnake),
            ("e", "PascalCase", Case::Pascal),
            ("r", "Upper-Kebab", Case::UpperKebab),
            ("t", "Title Case", Case::Title),
            ("a", "lower case", Case::Lower),
            ("s", "snake_case", Case::Snake),
            ("d", "camelCase", Case::Camel),
            ("f", "kebab-case", Case::Kebab),
        ]
        .into_iter()
        .map(|(keybinding, description, case)| {
            Keybinding::new(
                keybinding,
                description.to_string(),
                Dispatch::ToEditor(Transform(Transformation::Case(case))),
            )
        })
        .chain(Some(Keybinding::new(
            "j",
            "Wrap".to_string(),
            Dispatch::ToEditor(Transform(Transformation::Wrap)),
        )))
        .chain(Some(Keybinding::new(
            "h",
            "Unwrap".to_string(),
            Dispatch::ToEditor(Transform(Transformation::Unwrap)),
        )))
        .chain(Some(Keybinding::new(
            "k",
            "Line Comment".to_string(),
            Dispatch::ToEditor(DispatchEditor::ToggleLineComment),
        )))
        .chain(Some(Keybinding::new(
            "l",
            "Block Comment".to_string(),
            Dispatch::ToEditor(DispatchEditor::ToggleBlockComment),
        )))
        .collect_vec()
    }
    pub fn transform_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Transform".to_string(),

            keymap: Keymap::new(&self.keymap_transform()),
        }
    }

    pub fn space_pick_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Pick".to_string(),

            keymap: Keymap::new(
                &[
                    ("f", "Buffer", FilePickerKind::Opened),
                    ("d", "File", FilePickerKind::NonGitIgnored),
                ]
                .into_iter()
                .map(|(key, description, kind)| {
                    Keybinding::new(key, description.to_string(), Dispatch::OpenFilePicker(kind))
                })
                .chain(
                    [
                        ("g", DiffMode::UnstagedAgainstCurrentBranch),
                        ("G", DiffMode::UnstagedAgainstMainBranch),
                    ]
                    .into_iter()
                    .map(|(key, diff_mode)| {
                        Keybinding::new(
                            key,
                            format!("Git status {}", diff_mode.display()),
                            Dispatch::OpenFilePicker(FilePickerKind::GitStatus(diff_mode)),
                        )
                    }),
                )
                .chain(Some(Keybinding::new(
                    "s",
                    "Symbol (Document)".to_string(),
                    Dispatch::RequestDocumentSymbols,
                )))
                .chain(Some(Keybinding::new(
                    "S",
                    "Symbol (Workspace)".to_string(),
                    Dispatch::OpenWorkspaceSymbolsPrompt,
                )))
                .chain(Some(Keybinding::new(
                    "a",
                    "Theme".to_string(),
                    Dispatch::OpenThemePrompt,
                )))
                .collect_vec(),
            ),
        }
    }

    pub fn space_context_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Context".to_string(),

            keymap: Keymap::new(&[
                Keybinding::new("d", "Code Actions".to_string(), {
                    let cursor_char_index = self.get_cursor_char_index();
                    Dispatch::RequestCodeAction {
                        diagnostics: self
                            .buffer()
                            .diagnostics()
                            .into_iter()
                            .filter_map(|diagnostic| {
                                if diagnostic.range.contains(&cursor_char_index) {
                                    diagnostic.original_value.clone()
                                } else {
                                    None
                                }
                            })
                            .collect_vec(),
                    }
                }),
                Keybinding::new("s", "Hover".to_string(), Dispatch::RequestHover),
                Keybinding::new("f", "Rename".to_string(), Dispatch::PrepareRename),
                Keybinding::new(
                    "g",
                    format!(
                        "Revert Hunk{}",
                        DiffMode::UnstagedAgainstCurrentBranch.display()
                    ),
                    Dispatch::ToEditor(DispatchEditor::RevertHunk(
                        DiffMode::UnstagedAgainstCurrentBranch,
                    )),
                ),
                Keybinding::new(
                    "G",
                    format!(
                        "Revert Hunk{}",
                        DiffMode::UnstagedAgainstMainBranch.display()
                    ),
                    Dispatch::ToEditor(DispatchEditor::RevertHunk(
                        DiffMode::UnstagedAgainstMainBranch,
                    )),
                ),
                Keybinding::new(
                    "b",
                    "Git Blame".to_string(),
                    Dispatch::ToEditor(DispatchEditor::GitBlame),
                ),
                Keybinding::new(
                    "x",
                    "Go to File".to_string(),
                    Dispatch::ToEditor(DispatchEditor::GoToFile),
                ),
                Keybinding::new(
                    "C",
                    "Copy Absolute Path".to_string(),
                    Dispatch::ToEditor(DispatchEditor::CopyAbsolutePath),
                ),
                Keybinding::new(
                    "c",
                    "Copy Relative Path".to_string(),
                    Dispatch::ToEditor(DispatchEditor::CopyRelativePath),
                ),
                Keybinding::new(
                    "t",
                    "TS Node Sexp".to_string(),
                    Dispatch::ToEditor(DispatchEditor::ShowCurrentTreeSitterNodeSexp),
                ),
                Keybinding::new("e", "Pipe".to_string(), Dispatch::OpenPipeToShellPrompt),
            ]),
        }
    }

    pub fn space_editor_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Editor".to_string(),

            keymap: Keymap::new(&[
                Keybinding::new(
                    "x",
                    "Replace all".to_string(),
                    Dispatch::Replace {
                        scope: Scope::Global,
                    },
                ),
                Keybinding::new(
                    "enter",
                    "Force Save".to_string(),
                    Dispatch::ToEditor(DispatchEditor::ForceSave),
                ),
                Keybinding::new("c", "Save All".to_string(), Dispatch::SaveAll),
                Keybinding::new("v", "Quit No Save".to_string(), Dispatch::QuitAll),
                Keybinding::new(
                    "f",
                    "Change Work Dir".to_string(),
                    Dispatch::OpenChangeWorkingDirectoryPrompt,
                ),
                Keybinding::new(
                    "d",
                    "Reload File".to_string(),
                    Dispatch::ToEditor(ReloadFile { force: false }),
                ),
            ]),
        }
    }

    pub fn space_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Space".to_string(),

            keymap: Keymap::new(
                &[
                    Keybinding::new(
                        "u",
                        "÷ Selection".to_string(),
                        Dispatch::ToEditor(DispatchEditor::ToggleReveal(
                            Reveal::CurrentSelectionMode,
                        )),
                    ),
                    Keybinding::new(
                        "i",
                        "÷ Cursor".to_string(),
                        Dispatch::ToEditor(DispatchEditor::ToggleReveal(Reveal::Cursor)),
                    ),
                    Keybinding::new(
                        "o",
                        "÷ Mark".to_string(),
                        Dispatch::ToEditor(DispatchEditor::ToggleReveal(Reveal::Mark)),
                    ),
                    Keybinding::new(
                        "j",
                        "Editor".to_string(),
                        Dispatch::ShowKeymapLegend(self.space_editor_keymap_legend_config()),
                    ),
                    Keybinding::new(
                        "k",
                        "Pick".to_string(),
                        Dispatch::ShowKeymapLegend(self.space_pick_keymap_legend_config()),
                    ),
                    Keybinding::new(
                        "l",
                        "Context".to_string(),
                        Dispatch::ShowKeymapLegend(self.space_context_keymap_legend_config()),
                    ),
                    Keybinding::new(
                        ";",
                        "Explorer".to_string(),
                        Dispatch::RevealInExplorer(
                            self.path()
                                .unwrap_or_else(|| context.current_working_directory().clone()),
                        ),
                    ),
                    Keybinding::new(
                        "/",
                        "Help".to_string(),
                        Dispatch::ToEditor(DispatchEditor::ShowHelp),
                    ),
                ]
                .into_iter()
                .chain(
                    self.secondary_selection_modes_keymap_legend_config(
                        Scope::Global,
                        IfCurrentNotFound::LookForward,
                        None,
                    )
                    .keymap
                    .into_vec(),
                )
                .collect_vec(),
            ),
        }
    }

    pub fn leader_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Leader".to_string(),

            keymap: Keymap::new(
                &QWERTY
                    .iter()
                    .flatten()
                    .filter_map(|key| {
                        let (_, description, _) =
                            custom_keymap().into_iter().find(|(k, _, _)| k == key)?;
                        Some(Keybinding::new(
                            key,
                            description.to_string(),
                            Dispatch::ExecuteLeaderKey(key.to_string()),
                        ))
                    })
                    .collect_vec(),
            ),
        }
    }

    fn search_current_keymap(
        &self,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
    ) -> Vec<Keybinding> {
        [
            Keybinding::new_extended(
                "f",
                "Search This".to_string(),
                "Search current selection".to_string(),
                Dispatch::ToEditor(DispatchEditor::SearchCurrentSelection(
                    if_current_not_found,
                    scope,
                )),
            ),
            Keybinding::new_extended(
                "F",
                "Search Clipboard".to_string(),
                "Search clipboard content".to_string(),
                Dispatch::ToEditor(DispatchEditor::SearchClipboardContent(scope)),
            ),
        ]
        .to_vec()
    }

    pub fn secondary_selection_modes_keymap_legend_config(
        &self,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        prior_change: Option<PriorChange>,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: format!(
                "Find ({})",
                match scope {
                    Scope::Local => "Local",
                    Scope::Global => "Global",
                }
            ),

            keymap: Keymap::new(&self.secondary_selection_modes_keybindings(
                scope,
                if_current_not_found,
                prior_change,
            )),
        }
    }

    fn secondary_selection_modes_keybindings(
        &self,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        prior_change: Option<PriorChange>,
    ) -> Vec<Keybinding> {
        let search_keybindings = {
            [].into_iter()
                .chain(
                    [Keybinding::new(
                        "n",
                        "Repeat".to_string(),
                        Dispatch::UseLastNonContiguousSelectionMode(if_current_not_found),
                    )]
                    .to_vec(),
                )
                .collect_vec()
        };
        let misc_keybindings = [
            Keybinding::new(
                "e",
                "Mark".to_string(),
                match scope {
                    Scope::Global => Dispatch::SetQuickfixList(QuickfixListType::Mark),
                    Scope::Local => Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                        if_current_not_found,
                        Mark,
                        prior_change,
                    )),
                },
            ),
            Keybinding::new(
                "t",
                "Quickfix".to_string(),
                match scope {
                    Scope::Global => {
                        Dispatch::SetGlobalMode(Some(crate::context::GlobalMode::QuickfixListItem))
                    }
                    Scope::Local => Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                        if_current_not_found,
                        LocalQuickfix {
                            title: "LOCAL QUICKFIX".to_string(),
                        },
                        prior_change,
                    )),
                },
            ),
        ]
        .into_iter()
        .chain(
            [
                ("g", DiffMode::UnstagedAgainstCurrentBranch),
                ("G", DiffMode::UnstagedAgainstMainBranch),
            ]
            .map(|(key, diff_mode)| {
                Keybinding::new(
                    key,
                    format!("Hunk{}", diff_mode.display()),
                    match scope {
                        Scope::Global => Dispatch::GetRepoGitHunks(diff_mode),
                        Scope::Local => Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                            if_current_not_found,
                            GitHunk(diff_mode),
                            prior_change,
                        )),
                    },
                )
            }),
        )
        .collect_vec();
        let diagnostics_keybindings = [
            ("a", "All", DiagnosticSeverityRange::All),
            ("s", "Error", DiagnosticSeverityRange::Error),
            ("q", "Hint", DiagnosticSeverityRange::Hint),
            ("Q", "Info", DiagnosticSeverityRange::Information),
            ("w", "Warn", DiagnosticSeverityRange::Warning),
        ]
        .into_iter()
        .map(|(key, description, severity)| {
            Keybinding::new(
                key,
                description.to_string(),
                match scope {
                    Scope::Local => Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                        if_current_not_found,
                        Diagnostic(severity),
                        prior_change,
                    )),
                    Scope::Global => {
                        Dispatch::SetQuickfixList(QuickfixListType::Diagnostic(severity))
                    }
                },
            )
        })
        .collect_vec();
        let lsp_keybindings = [
            Keybinding::new("x", "Def".to_string(), Dispatch::RequestDefinitions(scope)),
            Keybinding::new(
                "X",
                "Decl".to_string(),
                Dispatch::RequestDeclarations(scope),
            ),
            Keybinding::new(
                "b",
                "Impl".to_string(),
                Dispatch::RequestImplementations(scope),
            ),
            Keybinding::new(
                "v",
                "Ref-".to_string(),
                Dispatch::RequestReferences {
                    include_declaration: false,
                    scope,
                },
            ),
            Keybinding::new(
                "V",
                "Ref+".to_string(),
                Dispatch::RequestReferences {
                    include_declaration: true,
                    scope,
                },
            ),
            Keybinding::new(
                "c",
                "Type".to_string(),
                Dispatch::RequestTypeDefinitions(scope),
            ),
        ];
        let scope_specific_keybindings = match scope {
            Scope::Local => [("Y", "Int", r"\d+")]
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
                    let dispatch = Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                        if_current_not_found,
                        Find { search },
                        prior_change,
                    ));
                    Keybinding::new(key, description.to_string(), dispatch)
                })
                .chain([
                    Keybinding::new_extended(
                        "d",
                        Direction::Start.format_action("Search"),
                        Direction::Start.format_action("Search"),
                        Dispatch::OpenSearchPromptWithPriorChange {
                            scope: Scope::Local,
                            if_current_not_found: self
                                .cursor_direction
                                .reverse()
                                .to_if_current_not_found(),
                            prior_change,
                        },
                    ),
                    Keybinding::new(
                        "D",
                        "With".to_string(),
                        Dispatch::OpenSearchPromptWithCurrentSelection {
                            scope: Scope::Local,
                            prior_change,
                        },
                    ),
                    Keybinding::new(
                        "y",
                        "One".to_string(),
                        Dispatch::ToEditor(FindOneChar(if_current_not_found)),
                    ),
                    Keybinding::new(
                        "r",
                        Direction::End.format_action("Repeat Search"),
                        Dispatch::ToEditor(DispatchEditor::RepeatSearch(
                            Scope::Local,
                            self.cursor_direction.reverse().to_if_current_not_found(),
                            prior_change,
                        )),
                    ),
                ])
                .chain(self.search_current_keymap(
                    Scope::Local,
                    self.cursor_direction.reverse().to_if_current_not_found(),
                ))
                .collect_vec(),
            Scope::Global => [
                Keybinding::new_extended(
                    "d",
                    "Search".to_string(),
                    "Search".to_string(),
                    Dispatch::OpenSearchPrompt {
                        scope,
                        if_current_not_found,
                    },
                ),
                Keybinding::new(
                    "D",
                    "With".to_string(),
                    Dispatch::OpenSearchPromptWithCurrentSelection {
                        scope,
                        prior_change,
                    },
                ),
                Keybinding::new(
                    "r",
                    "Repeat Search".to_string(),
                    Dispatch::ToEditor(DispatchEditor::RepeatSearch(
                        Scope::Global,
                        IfCurrentNotFound::LookForward,
                        prior_change,
                    )),
                ),
            ]
            .into_iter()
            .chain(self.search_current_keymap(scope, if_current_not_found))
            .collect_vec(),
        };
        search_keybindings
            .into_iter()
            .chain(misc_keybindings)
            .chain(diagnostics_keybindings)
            .chain(lsp_keybindings)
            .chain(scope_specific_keybindings)
            .collect_vec()
    }

    pub fn multicursor_menu_keymap(&self) -> Keymap {
        let primary_selection_mode_keybindings =
            self.keymap_primary_selection_modes(Some(PriorChange::EnterMultiCursorMode));
        let secondary_selection_mode_keybindings =
            self.keymap_secondary_selection_modes_init(Some(PriorChange::EnterMultiCursorMode));
        let other_keybindings = [
            Keybinding::new(
                "j",
                "Curs All".to_string(),
                Dispatch::ToEditor(CursorAddToAllSelections),
            ),
            Keybinding::new(
                "i",
                "Keep Match".to_string(),
                Dispatch::OpenFilterSelectionsPrompt { maintain: true },
            ),
            Keybinding::new(
                "k",
                "Remove Match".to_string(),
                Dispatch::OpenFilterSelectionsPrompt { maintain: false },
            ),
            Keybinding::new(
                "l",
                "Keep Primary Curs".to_string(),
                Dispatch::ToEditor(DispatchEditor::CursorKeepPrimaryOnly),
            ),
        ];
        Keymap::new(
            &[].into_iter()
                .chain(primary_selection_mode_keybindings)
                .chain(secondary_selection_mode_keybindings)
                .chain(other_keybindings)
                .collect_vec(),
        )
    }
}

pub fn swap_keymap() -> Keymap {
    Keymap::new(
        &[
            (Movement::Up, "i"),
            (Movement::Down, "k"),
            (Movement::Left, "j"),
            (Movement::Right, "l"),
            (Movement::Previous, "u"),
            (Movement::Next, "o"),
            (Movement::First, "y"),
            (Movement::Last, "p"),
        ]
        .into_iter()
        .map(|(movement, key)| {
            Keybinding::new(
                key,
                movement.format_action("Swap"),
                Dispatch::ToEditor(DispatchEditor::SwapWithMovement(movement)),
            )
        })
        .chain(Some(Keybinding::new(
            "m",
            "Jump Swap".to_string(),
            Dispatch::ToEditor(ShowJumps {
                use_current_selection_mode: true,
                prior_change: Some(PriorChange::EnterSwapMode),
            }),
        )))
        .collect_vec(),
    )
}

pub fn paste_keymap() -> Keymap {
    Keymap::new(
        [
            Keybinding::new(
                "j",
                Movement::Left.format_action("Gap Paste"),
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::Left)),
            ),
            Keybinding::new(
                "l",
                Movement::Right.format_action("Gap Paste"),
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::Right)),
            ),
            Keybinding::new(
                "o",
                Movement::Next.format_action("Gap Paste"),
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::Next)),
            ),
            Keybinding::new(
                "u",
                Movement::Previous.format_action("Gap Paste"),
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::Previous)),
            ),
            Keybinding::new(
                ";",
                "Paste >".to_string(),
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::AfterWithoutGap)),
            ),
            Keybinding::new(
                "h",
                "< Paste".to_string(),
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::BeforeWithoutGap)),
            ),
            Keybinding::new(
                "m",
                "Replace w/ pattern".to_string(),
                Dispatch::ToEditor(ReplaceWithPattern),
            ),
            Keybinding::new(
                "y",
                Direction::Start.format_action("Replace History"),
                Dispatch::ToEditor(ReplaceWithPreviousCopiedText),
            ),
            Keybinding::new(
                "p",
                Direction::End.format_action("Replace History"),
                Dispatch::ToEditor(ReplaceWithNextCopiedText),
            ),
            Keybinding::new(
                "i",
                Movement::Up.format_action("Paste"),
                Dispatch::ToEditor(PasteVertically(Direction::Start)),
            ),
            Keybinding::new(
                "k",
                Movement::Down.format_action("Paste"),
                Dispatch::ToEditor(PasteVertically(Direction::End)),
            ),
        ]
        .as_ref(),
    )
}

pub fn duplicate_keymap() -> Keymap {
    Keymap::new(
        [
            Keybinding::new(
                "j",
                Movement::Left.format_action("Gap Dup"),
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::Left)),
            ),
            Keybinding::new(
                "l",
                Movement::Right.format_action("Gap Dup"),
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::Right)),
            ),
            Keybinding::new(
                "o",
                Movement::Next.format_action("Gap Dup"),
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::Next)),
            ),
            Keybinding::new(
                "u",
                Movement::Previous.format_action("Gap Dup"),
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::Previous)),
            ),
            Keybinding::new(
                ";",
                "Dup >".to_string(),
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::AfterWithoutGap)),
            ),
            Keybinding::new(
                "h",
                "< Dup".to_string(),
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::BeforeWithoutGap)),
            ),
            Keybinding::new(
                "i",
                Movement::Up.format_action("Dup"),
                Dispatch::ToEditor(DuplicateVertically(Direction::Start)),
            ),
            Keybinding::new(
                "k",
                Movement::Down.format_action("Dup"),
                Dispatch::ToEditor(DuplicateVertically(Direction::End)),
            ),
        ]
        .as_ref(),
    )
}

pub fn cut_keymap() -> Keymap {
    Keymap::new(
        &[
            (Movement::Left, "j"),
            (Movement::Right, "l"),
            (Movement::Previous, "u"),
            (Movement::Next, "o"),
            (Movement::First, "y"),
            (Movement::Last, "p"),
        ]
        .into_iter()
        .map(|(movement, key)| {
            Keybinding::new(
                key,
                movement.format_action("Cut"),
                Dispatch::ToEditor(CutWithMovement(movement)),
            )
        })
        .chain(Some(Keybinding::new(
            "b",
            "Replace Cut".to_string(),
            Dispatch::ToEditor(ReplaceWithCopiedText { cut: true }),
        )))
        .collect_vec(),
    )
}

pub fn buffer_keymap() -> Keymap {
    Keymap::new(
        &[
            ("j", Movement::Left),
            ("l", Movement::Right),
            ("y", Movement::First),
            ("p", Movement::Last),
        ]
        .into_iter()
        .map(|(key, movement)| {
            Keybinding::new(
                key,
                movement.format_action("Marked File"),
                Dispatch::CycleMarkedFile(movement),
            )
        })
        .chain(
            [("u", Movement::Previous), ("o", Movement::Next)]
                .into_iter()
                .map(|(key, movement)| {
                    Keybinding::new(
                        key,
                        movement.format_action("Opened File"),
                        Dispatch::CycleMarkedFile(movement),
                    )
                }),
        )
        .chain(Some(Keybinding::new_extended(
            "k",
            "Mark File".to_string(),
            "Toggle File Mark".to_string(),
            Dispatch::ToggleFileMark,
        )))
        .chain(Some(Keybinding::new_extended(
            "n",
            "Close".to_string(),
            "Close current window".to_string(),
            Dispatch::CloseCurrentWindow,
        )))
        .chain(Some(Keybinding::new_extended(
            "i",
            "Unmark Others".to_string(),
            "Unmark all other buffers".to_string(),
            Dispatch::UnmarkAllOthers,
        )))
        .collect_vec(),
    )
}

pub fn delete_keymap() -> Keymap {
    Keymap::new(
        &[
            (Movement::Left, "j"),
            (Movement::Right, "l"),
            (Movement::Previous, "u"),
            (Movement::Next, "o"),
            (Movement::First, "y"),
            (Movement::Last, "p"),
        ]
        .into_iter()
        .map(|(movement, key)| {
            Keybinding::new(
                key,
                movement.format_action("Delete"),
                Dispatch::ToEditor(DeleteWithMovement(movement)),
            )
        })
        .collect_vec(),
    )
}

pub fn insert_keymap() -> Keymap {
    Keymap::new(
        &[
            (GetGapMovement::Left, "<< Open", "j"),
            (GetGapMovement::Right, "Open >>", "l"),
            (GetGapMovement::Previous, "< Open", "u"),
            (GetGapMovement::Next, "Open >", "o"),
            (GetGapMovement::BeforeWithoutGap, "< Insert", "h"),
            (GetGapMovement::AfterWithoutGap, "Insert >", ";"),
        ]
        .into_iter()
        .map(|(movement, description, key)| {
            Keybinding::new(
                key,
                description.to_string(),
                Dispatch::ToEditor(DispatchEditor::Open(movement)),
            )
        })
        .chain([
            Keybinding::new(
                "i",
                "Open ^".to_string(),
                Dispatch::ToEditor(OpenVertically(Direction::Start)),
            ),
            Keybinding::new(
                "k",
                "Open v".to_string(),
                Dispatch::ToEditor(OpenVertically(Direction::End)),
            ),
        ])
        .collect_vec(),
    )
}

#[derive(Default, Clone)]
pub struct NormalModeOverride {
    pub change: Option<KeymapOverride>,
    pub delete: Option<KeymapOverride>,
    pub insert: Option<KeymapOverride>,
    pub append: Option<KeymapOverride>,
    pub open: Option<KeymapOverride>,
    pub paste: Option<KeymapOverride>,
    pub cut: Option<KeymapOverride>,
    pub multicursor: Option<KeymapOverride>,
}

#[derive(Clone)]
pub struct KeymapOverride {
    pub description: &'static str,
    pub dispatch: Dispatch,
}

fn generate_enclosures_keymap(get_dispatch: impl Fn(EnclosureKind) -> Dispatch) -> Keymap {
    Keymap::new(
        &[
            ("j", EnclosureKind::Parentheses),
            ("k", EnclosureKind::SquareBrackets),
            ("l", EnclosureKind::CurlyBraces),
            (";", EnclosureKind::AngularBrackets),
            ("u", EnclosureKind::SingleQuotes),
            ("i", EnclosureKind::DoubleQuotes),
            ("o", EnclosureKind::Backticks),
        ]
        .into_iter()
        .map(|(key, enclosure)| {
            let (open, close) = enclosure.open_close_symbols_str();
            Keybinding::new(key, format!("{open} {close}"), get_dispatch(enclosure))
        })
        .collect_vec(),
    )
}

pub fn extend_mode_normal_mode_override() -> NormalModeOverride {
    fn select_surround_keymap_legend_config(kind: SurroundKind) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: format!("Select Surround ({kind:?})"),

            keymap: generate_enclosures_keymap(|enclosure| {
                Dispatch::ToEditor(SelectSurround {
                    enclosure,
                    kind: kind.clone(),
                })
            }),
        }
    }

    fn delete_surround_keymap_legend_config() -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Delete Surround".to_string(),

            keymap: generate_enclosures_keymap(|enclosure| {
                Dispatch::ToEditor(DeleteSurround(enclosure))
            }),
        }
    }

    fn surround_keymap_legend_config() -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Surround".to_string(),

            keymap: Keymap::new(
                &generate_enclosures_keymap(|enclosure| {
                    let (open, close) = enclosure.open_close_symbols_str();
                    Dispatch::ToEditor(Surround(open.to_string(), close.to_string()))
                })
                .into_vec()
                .into_iter()
                .chain(Some(Keybinding::new(
                    "p",
                    "<></>".to_string(),
                    Dispatch::OpenSurroundXmlPrompt,
                )))
                .collect_vec(),
            ),
        }
    }

    fn change_surround_from_keymap_legend_config() -> super::keymap_legend::KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Change Surround from:".to_string(),

            keymap: generate_enclosures_keymap(|enclosure| {
                Dispatch::ShowKeymapLegend(change_surround_to_keymap_legend_config(enclosure))
            }),
        }
    }

    fn change_surround_to_keymap_legend_config(
        from: EnclosureKind,
    ) -> super::keymap_legend::KeymapLegendConfig {
        KeymapLegendConfig {
            title: format!("Change Surround from {} to:", from.to_str()),

            keymap: generate_enclosures_keymap(|enclosure| {
                Dispatch::ToEditor(ChangeSurround {
                    from,
                    to: enclosure,
                })
            }),
        }
    }
    NormalModeOverride {
        insert: Some(KeymapOverride {
            description: "Inside",
            dispatch: Dispatch::ShowKeymapLegend(select_surround_keymap_legend_config(
                SurroundKind::Inside,
            )),
        }),
        append: Some(KeymapOverride {
            description: "Around",
            dispatch: Dispatch::ShowKeymapLegend(select_surround_keymap_legend_config(
                SurroundKind::Around,
            )),
        }),
        delete: Some(KeymapOverride {
            description: "Delete Surround",
            dispatch: Dispatch::ShowKeymapLegend(delete_surround_keymap_legend_config()),
        }),
        change: Some(KeymapOverride {
            description: "Change Surround",
            dispatch: Dispatch::ShowKeymapLegend(change_surround_from_keymap_legend_config()),
        }),
        open: Some(KeymapOverride {
            description: "Surround",
            dispatch: Dispatch::ShowKeymapLegend(surround_keymap_legend_config()),
        }),
        ..Default::default()
    }
}

fn primary_selection_modes() -> Vec<(&'static str, SelectionMode)> {
    [
        ("a", Line),
        ("A", LineFull),
        ("d", SyntaxNode),
        ("D", SyntaxNodeFine),
        ("s", Word),
        ("S", BigWord),
        ("w", Subword),
        ("q", Character),
    ]
    .to_vec()
}
