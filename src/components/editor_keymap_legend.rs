use crossterm::event::KeyCode;
use SelectionMode::*;

use convert_case::Case;
use event::KeyEvent;
use itertools::Itertools;

use crate::{
    app::{Dispatch, Dispatches, FilePickerKind, Scope},
    components::{editor::Movement, keymap_legend::KeymapLegendSection},
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
    keymap_legend::{Keymap, KeymapLegendBody, KeymapLegendConfig, Keymaps},
};

use DispatchEditor::*;
use Movement::*;
impl Editor {
    pub(crate) fn keymap_core_movements(&self) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Movements (Core)".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    "h",
                    "Left".to_string(),
                    Dispatch::ToEditor(MoveSelection(Movement::Left)),
                ),
                Keymap::new(
                    "l",
                    "Right".to_string(),
                    Dispatch::ToEditor(MoveSelection(Right)),
                ),
                Keymap::new("k", "Up".to_string(), Dispatch::ToEditor(MoveSelection(Up))),
                Keymap::new(
                    "j",
                    "Down".to_string(),
                    Dispatch::ToEditor(MoveSelection(Down)),
                ),
                Keymap::new(
                    "n",
                    "Next".to_string(),
                    Dispatch::ToEditor(MoveSelection(Next)),
                ),
                Keymap::new(
                    "N",
                    "Previous".to_string(),
                    Dispatch::ToEditor(MoveSelection(Previous)),
                ),
                Keymap::new(
                    "t",
                    "Expand".to_string(),
                    Dispatch::ToEditor(MoveSelection(Expand)),
                ),
                Keymap::new(
                    "b",
                    "Shrink".to_string(),
                    Dispatch::ToEditor(MoveSelection(Shrink)),
                ),
                Keymap::new(
                    ",",
                    "First".to_string(),
                    Dispatch::ToEditor(MoveSelection(Movement::First)),
                ),
                Keymap::new(
                    ".",
                    "Last".to_string(),
                    Dispatch::ToEditor(MoveSelection(Movement::Last)),
                ),
                Keymap::new(
                    "f",
                    "Jump".to_string(),
                    Dispatch::ToEditor(DispatchEditor::ShowJumps {
                        use_current_selection_mode: true,
                    }),
                ),
                Keymap::new(
                    "-",
                    "Parent Line".to_string(),
                    Dispatch::ToEditor(MoveSelection(ToParentLine)),
                ),
                Keymap::new(
                    "0",
                    "To Index (1-based)".to_string(),
                    Dispatch::OpenMoveToIndexPrompt,
                ),
            ]),
        }
    }

    pub(crate) fn keymap_other_movements(&self) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Movements (Other)".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    "%",
                    "Swap cursor with anchor".to_string(),
                    Dispatch::ToEditor(DispatchEditor::SwapCursorWithAnchor),
                ),
                Keymap::new(
                    "(",
                    Direction::Start.format_action("Cycle primary selection"),
                    Dispatch::ToEditor(CyclePrimarySelection(Direction::Start)),
                ),
                Keymap::new(
                    ")",
                    Direction::End.format_action("Cycle primary selection"),
                    Dispatch::ToEditor(CyclePrimarySelection(Direction::End)),
                ),
                Keymap::new(
                    "ctrl+d",
                    "Scroll down".to_string(),
                    Dispatch::ToEditor(ScrollPageDown),
                ),
                Keymap::new(
                    "ctrl+u",
                    "Scroll up".to_string(),
                    Dispatch::ToEditor(ScrollPageUp),
                ),
                Keymap::new("ctrl+o", "Go back".to_string(), Dispatch::ToEditor(GoBack)),
                Keymap::new(
                    "tab",
                    "Go forward".to_string(),
                    Dispatch::ToEditor(GoForward),
                ),
                Keymap::new(
                    "{",
                    "Go to previous file".to_string(),
                    Dispatch::GoToPreviousFile,
                ),
                Keymap::new("}", "Go to next file".to_string(), Dispatch::GoToNextFile),
            ]),
        }
    }

    pub(crate) fn keymap_selection_modes(&self, context: &Context) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Selection mode".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    "e",
                    "Select Line".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
                ),
                Keymap::new(
                    "E",
                    "Select Full Line".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(IfCurrentNotFound::LookForward, LineFull)),
                ),
                Keymap::new(
                    "g",
                    "Find (Global)".to_string(),
                    Dispatch::ShowKeymapLegend(self.find_keymap_legend_config(
                        context,
                        Scope::Global,
                        IfCurrentNotFound::LookForward,
                    )),
                ),
                Keymap::new(
                    "s",
                    "Select Syntax Node".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(
                        IfCurrentNotFound::LookForward,
                        SyntaxNode,
                    )),
                ),
                Keymap::new(
                    "S",
                    "Select Fine Syntax Node".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(
                        IfCurrentNotFound::LookForward,
                        SyntaxNodeFine,
                    )),
                ),
                Keymap::new(
                    "W",
                    "Select Subword".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(IfCurrentNotFound::LookForward, Subword)),
                ),
                Keymap::new(
                    "w",
                    "Select Word".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
                ),
                Keymap::new(
                    "z",
                    "Select Character".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(IfCurrentNotFound::LookForward, Column)),
                ),
                Keymap::new(
                    "[",
                    "(Local) - Backward".to_string(),
                    Dispatch::ShowKeymapLegend(self.find_keymap_legend_config(
                        context,
                        Scope::Local,
                        IfCurrentNotFound::LookBackward,
                    )),
                ),
                Keymap::new(
                    "]",
                    "Find (Local) - Forward".to_string(),
                    Dispatch::ShowKeymapLegend(self.find_keymap_legend_config(
                        context,
                        Scope::Local,
                        IfCurrentNotFound::LookForward,
                    )),
                ),
                Keymap::new(
                    ";",
                    "Select last non-contiguous selection mode".to_string(),
                    Dispatch::UseLastNonContiguousSelectionMode(IfCurrentNotFound::LookForward),
                ),
            ]),
        }
    }
    pub(crate) fn keymap_actions(&self) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Action".to_string(),
            keymaps: Keymaps::new(
                &[
                    Keymap::new(
                        "i",
                        Direction::Start.format_action("Insert"),
                        Dispatch::ToEditor(EnterInsertMode(Direction::Start)),
                    ),
                    Keymap::new(
                        "a",
                        Direction::End.format_action("Insert"),
                        Dispatch::ToEditor(EnterInsertMode(Direction::End)),
                    ),
                    Keymap::new("c", "Change".to_string(), Dispatch::ToEditor(Change)),
                    Keymap::new(
                        "d",
                        Direction::End.format_action("Delete"),
                        Dispatch::ToEditor(Delete(Direction::End)),
                    ),
                    Keymap::new(
                        "D",
                        Direction::Start.format_action("Delete"),
                        Dispatch::ToEditor(Delete(Direction::Start)),
                    ),
                    Keymap::new(
                        "J",
                        "Join".to_string(),
                        Dispatch::ToEditor(Transform(Transformation::Join)),
                    ),
                    Keymap::new(
                        "T",
                        "Raise".to_string(),
                        Dispatch::ToEditor(Replace(Expand)),
                    ),
                ]
                .into_iter()
                .chain(Some(if self.mode == Mode::MultiCursor {
                    Keymap::new(
                        "m",
                        "Maintain selections matching search".to_string(),
                        Dispatch::OpenFilterSelectionsPrompt { maintain: true },
                    )
                } else {
                    Keymap::new(
                        "m",
                        "Toggle Mark".to_string(),
                        Dispatch::ToEditor(ToggleMark),
                    )
                }))
                .chain(Some(if self.mode == Mode::MultiCursor {
                    Keymap::new(
                        "o",
                        "Keep primary cursor only".to_string(),
                        Dispatch::ToEditor(DispatchEditor::CursorKeepPrimaryOnly),
                    )
                } else if self.selection_set.is_extended() {
                    Keymap::new(
                        "o",
                        "Switch extended selection end".to_string(),
                        Dispatch::ToEditor(SwapExtensionDirection),
                    )
                } else {
                    Keymap::new(
                        "o",
                        Direction::End.format_action("Open"),
                        Dispatch::ToEditor(Open(Direction::End)),
                    )
                }))
                .chain([
                    Keymap::new(
                        "O",
                        Direction::Start.format_action("Open"),
                        Dispatch::ToEditor(Open(Direction::Start)),
                    ),
                    Keymap::new("u", "Undo".to_string(), Dispatch::ToEditor(Undo)),
                    Keymap::new("U", "Redo".to_string(), Dispatch::ToEditor(Redo)),
                    Keymap::new(
                        "ctrl+r",
                        "Replace with pattern".to_string(),
                        Dispatch::ToEditor(ReplaceWithPattern),
                    ),
                    Keymap::new(
                        "ctrl+p",
                        "Replace (with previous copied text)".to_string(),
                        Dispatch::ToEditor(ReplaceWithPreviousCopiedText),
                    ),
                    Keymap::new(
                        "ctrl+n",
                        "Replace (with next copied text)".to_string(),
                        Dispatch::ToEditor(ReplaceWithNextCopiedText),
                    ),
                    Keymap::new(
                        "v",
                        "Enter V Mode".to_string(),
                        Dispatch::ToEditor(EnterVMode),
                    ),
                    Keymap::new("V", "Select all".to_string(), Dispatch::ToEditor(SelectAll)),
                    Keymap::new("enter", "Save".to_string(), Dispatch::ToEditor(Save)),
                    Keymap::new(
                        "!",
                        "Transform".to_string(),
                        Dispatch::ShowKeymapLegend(self.transform_keymap_legend_config()),
                    ),
                    Keymap::new(
                        "'",
                        "Configure Search".to_string(),
                        Dispatch::ShowSearchConfig {
                            scope: Scope::Local,
                            if_current_not_found: IfCurrentNotFound::LookForward,
                        },
                    ),
                    Keymap::new(
                        "^",
                        Direction::Start.format_action("Collapse selection"),
                        Dispatch::ToEditor(DispatchEditor::CollapseSelection(Direction::Start)),
                    ),
                    Keymap::new(
                        "$",
                        Direction::End.format_action("Collapse selection"),
                        Dispatch::ToEditor(DispatchEditor::CollapseSelection(Direction::End)),
                    ),
                    Keymap::new(
                        "|",
                        "Pipe to shell".to_string(),
                        Dispatch::OpenPipeToShellPrompt,
                    ),
                    Keymap::new(
                        "/",
                        Direction::End.format_action("Search"),
                        Dispatch::OpenSearchPrompt {
                            scope: Scope::Local,
                            if_current_not_found: IfCurrentNotFound::LookForward,
                        },
                    ),
                    Keymap::new(
                        "?",
                        Direction::Start.format_action("Search"),
                        Dispatch::OpenSearchPrompt {
                            scope: Scope::Local,
                            if_current_not_found: IfCurrentNotFound::LookBackward,
                        },
                    ),
                    Keymap::new(">", "Indent".to_string(), Dispatch::ToEditor(Indent)),
                    Keymap::new("<", "Dedent".to_string(), Dispatch::ToEditor(Dedent)),
                ])
                .chain(
                    self.search_current_selection_keymap(
                        Scope::Local,
                        IfCurrentNotFound::LookForward,
                    ),
                )
                .collect_vec(),
            ),
        }
    }

    fn keymap_clipboard_related_actions(&self, use_system_clipboard: bool) -> KeymapLegendSection {
        let extra = if use_system_clipboard {
            " (system clipboard)"
        } else {
            ""
        };
        KeymapLegendSection {
            title: "Clipboard-related actions".to_string(),
            keymaps: Keymaps::new(
                &[
                    Keymap::new(
                        "C",
                        format!("{}{}", "Change Cut", extra),
                        Dispatch::ToEditor(ChangeCut {
                            use_system_clipboard,
                        }),
                    ),
                    Keymap::new(
                        "p",
                        format!("{}{}", Direction::End.format_action("Paste"), extra),
                        Dispatch::ToEditor(Paste {
                            direction: Direction::End,
                            use_system_clipboard,
                        }),
                    ),
                    Keymap::new(
                        "P",
                        format!("{}{}", Direction::Start.format_action("Paste"), extra),
                        Dispatch::ToEditor(Paste {
                            direction: Direction::Start,
                            use_system_clipboard,
                        }),
                    ),
                    Keymap::new(
                        "R",
                        format!("{}{}", "Replace Cut", extra),
                        Dispatch::ToEditor(ReplaceWithCopiedText {
                            use_system_clipboard,
                            cut: true,
                        }),
                    ),
                    Keymap::new(
                        "y",
                        format!("{}{}", "Yank (Copy)", extra),
                        Dispatch::ToEditor(Copy {
                            use_system_clipboard,
                        }),
                    ),
                ]
                .into_iter()
                .chain(Some(if self.mode == Mode::MultiCursor {
                    Keymap::new(
                        "r",
                        "Remove selections matching search".to_string(),
                        Dispatch::OpenFilterSelectionsPrompt { maintain: false },
                    )
                } else {
                    Keymap::new(
                        "r",
                        format!("{}{}", "Replace", extra),
                        Dispatch::ToEditor(ReplaceWithCopiedText {
                            use_system_clipboard,
                            cut: false,
                        }),
                    )
                }))
                .collect_vec(),
            ),
        }
    }

    fn keymap_universal(&self) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Universal keymaps (works in every mode)".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    "ctrl+c",
                    "Close current window".to_string(),
                    Dispatch::CloseCurrentWindow,
                ),
                Keymap::new(
                    "ctrl+l",
                    "Switch view alignment".to_string(),
                    Dispatch::ToEditor(SwitchViewAlignment),
                ),
                Keymap::new("ctrl+s", "Switch window".to_string(), Dispatch::OtherWindow),
                Keymap::new(
                    "ctrl+v",
                    "Paste".to_string(),
                    Dispatch::ToEditor(Paste {
                        direction: Direction::End,
                        use_system_clipboard: false,
                    }),
                ),
            ]),
        }
    }

    pub(crate) fn insert_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Insert mode keymaps".to_string(),
            body: KeymapLegendBody::MultipleSections {
                sections: [
                    KeymapLegendSection {
                        title: "GNU Readline movements".to_string(),
                        keymaps: Keymaps::new(&[
                            Keymap::new(
                                "ctrl+b",
                                "Move back a character".to_string(),
                                Dispatch::ToEditor(MoveCharacterBack),
                            ),
                            Keymap::new(
                                "ctrl+f",
                                "Move forward a character".to_string(),
                                Dispatch::ToEditor(MoveCharacterForward),
                            ),
                            Keymap::new(
                                "ctrl+a",
                                "Move to line start".to_string(),
                                Dispatch::ToEditor(MoveToLineStart),
                            ),
                            Keymap::new(
                                "ctrl+e",
                                "Move to line end".to_string(),
                                Dispatch::ToEditor(MoveToLineEnd),
                            ),
                            Keymap::new(
                                "ctrl+k",
                                Direction::End.format_action("Kill line"),
                                Dispatch::ToEditor(KillLine(Direction::End)),
                            ),
                            Keymap::new(
                                "ctrl+u",
                                Direction::Start.format_action("Kill line"),
                                Dispatch::ToEditor(KillLine(Direction::Start)),
                            ),
                            Keymap::new(
                                "ctrl+w",
                                "Delete word backward".to_string(),
                                Dispatch::ToEditor(DeleteWordBackward { short: false }),
                            ),
                            Keymap::new(
                                "alt+backspace",
                                "Delete subword backward".to_string(),
                                Dispatch::ToEditor(DeleteWordBackward { short: true }),
                            ),
                        ]),
                    },
                    KeymapLegendSection {
                        title: "Common".to_string(),
                        keymaps: Keymaps::new(&[
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
                        ]),
                    },
                ]
                .into_iter()
                .chain(Some(self.keymap_universal()))
                .collect_vec(),
            },
        }
    }

    pub(crate) fn handle_insert_mode(&mut self, event: KeyEvent) -> anyhow::Result<Dispatches> {
        if let Some(dispatches) = self
            .insert_mode_keymap_legend_config()
            .keymaps()
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
        if let Some(keymap) = self.keymap_universal().keymaps.get(&event) {
            Ok(HandleEventResult::Handled(keymap.get_dispatches()))
        } else {
            Ok(HandleEventResult::Ignored(event))
        }
    }

    pub(crate) fn keymap_others(&self, context: &Context) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Others".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    "space",
                    "Search (List)".to_string(),
                    Dispatch::ShowKeymapLegend(self.space_keymap_legend_config(context)),
                ),
                Keymap::new(
                    "\\",
                    "Clipboard-related actions (use system clipboard)".to_string(),
                    Dispatch::ShowKeymapLegend(KeymapLegendConfig {
                        title: "Clipboard-related actions (use system clipboard)".to_string(),
                        body: KeymapLegendBody::SingleSection {
                            keymaps: self.keymap_clipboard_related_actions(true).keymaps,
                        },
                    }),
                ),
                Keymap::new(
                    ":",
                    "Open command prompt".to_string(),
                    Dispatch::OpenCommandPrompt,
                ),
                Keymap::new(
                    "esc",
                    "Remain only this window".to_string(),
                    Dispatch::RemainOnlyCurrentComponent,
                ),
            ]),
        }
    }

    pub(crate) fn keymap_movement_actions(&self) -> KeymapLegendSection {
        KeymapLegendSection {
            keymaps: Keymaps::new(
                &[
                    Keymap::new(
                        "~",
                        "Replace".to_string(),
                        Dispatch::ToEditor(EnterReplaceMode),
                    ),
                    Keymap::new(
                        "x",
                        "Enter Exchange mode".to_string(),
                        Dispatch::ToEditor(EnterExchangeMode),
                    ),
                ]
                .into_iter()
                .chain(Some(if self.mode == Mode::MultiCursor {
                    Keymap::new(
                        "q",
                        "Add cursor to all selections".to_string(),
                        Dispatch::ToEditor(DispatchEditor::CursorAddToAllSelections),
                    )
                } else {
                    Keymap::new(
                        "q",
                        "Enter Multi-cursor mode".to_string(),
                        Dispatch::ToEditor(EnterMultiCursorMode),
                    )
                }))
                .collect_vec(),
            ),
            title: "Movement-action submodes".to_string(),
        }
    }

    pub(crate) fn help_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Help".to_string(),
            body: KeymapLegendBody::SingleSection {
                keymaps: Keymaps::new(&[
                    Keymap::new(
                        "i",
                        "Show help: Insert mode".to_string(),
                        Dispatch::ToEditor(ShowKeymapLegendInsertMode),
                    ),
                    Keymap::new(
                        "n",
                        "Show help: Normal mode".to_string(),
                        Dispatch::ToEditor(ShowKeymapLegendNormalMode),
                    ),
                ]),
            },
        }
    }

    pub(crate) fn normal_mode_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Normal mode".to_string(),
            body: KeymapLegendBody::MultipleSections {
                sections: [
                    self.keymap_core_movements(),
                    self.keymap_movement_actions(),
                    self.keymap_other_movements(),
                    self.keymap_selection_modes(context),
                    self.keymap_actions(),
                    self.keymap_clipboard_related_actions(false),
                    self.keymap_others(context),
                    self.keymap_universal(),
                ]
                .to_vec(),
            },
        }
    }
    fn normal_mode_keymaps(&self, context: &Context) -> Keymaps {
        Keymaps::new(
            &self
                .normal_mode_keymap_legend_config(context)
                .keymaps()
                .into_iter()
                .cloned()
                .collect_vec(),
        )
    }
    pub(crate) fn visual_mode_initialized_keymaps(&self) -> Keymaps {
        Keymaps::new(
            &self
                .visual_mode_initialized_keymap_legend_config()
                .keymaps()
                .into_iter()
                .cloned()
                .collect_vec(),
        )
    }
    fn visual_mode_initialized_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Visual Mode Initialized".to_string(),
            body: KeymapLegendBody::MultipleSections {
                sections: [
                    KeymapLegendSection {
                        title: "Select".to_string(),
                        keymaps: Keymaps::new(&[
                            Keymap::new(
                                "a",
                                "Select Around".to_string(),
                                Dispatch::ShowKeymapLegend(
                                    self.select_surround_keymap_legend_config(SurroundKind::Around),
                                ),
                            ),
                            Keymap::new(
                                "i",
                                "Select Inside".to_string(),
                                Dispatch::ShowKeymapLegend(
                                    self.select_surround_keymap_legend_config(SurroundKind::Inside),
                                ),
                            ),
                        ]),
                    },
                    KeymapLegendSection {
                        title: "Action".to_string(),
                        keymaps: Keymaps::new(&[
                            Keymap::new(
                                "c",
                                "Change Surround".to_string(),
                                Dispatch::ShowKeymapLegend(
                                    self.change_surround_from_keymap_legend_config(),
                                ),
                            ),
                            Keymap::new(
                                "d",
                                "Delete Surround".to_string(),
                                Dispatch::ShowKeymapLegend(
                                    self.delete_surround_keymap_legend_config(),
                                ),
                            ),
                            Keymap::new(
                                "s",
                                "Surround".to_string(),
                                Dispatch::ShowKeymapLegend(self.surround_keymap_legend_config()),
                            ),
                        ]),
                    },
                    KeymapLegendSection {
                        title: "Surround".to_string(),
                        keymaps: generate_enclosures_keymaps(|enclosure| {
                            let (open, close) = enclosure.open_close_symbols_str();
                            Dispatch::ToEditor(Surround(open.to_string(), close.to_string()))
                        }),
                    },
                ]
                .to_vec(),
            },
        }
    }
    pub(crate) fn handle_normal_mode(
        &mut self,
        context: &Context,
        event: KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        if let Some(keymap) = self.normal_mode_keymaps(context).get(&event) {
            return Ok(keymap.get_dispatches());
        }
        log::info!("unhandled event: {:?}", event);
        Ok(vec![].into())
    }

    pub(crate) fn transform_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Transform".to_string(),

            body: KeymapLegendBody::MultipleSections {
                sections: [
                    KeymapLegendSection {
                        title: "Letter case".to_string(),
                        keymaps: Keymaps::new(
                            &[
                                ("a", "aLtErNaTiNg CaSe", Case::Toggle),
                                ("c", "camelCase", Case::Camel),
                                ("l", "lower case", Case::Lower),
                                ("k", "kebab-case", Case::Kebab),
                                ("K", "Upper-Kebab", Case::UpperKebab),
                                ("p", "PascalCase", Case::Pascal),
                                ("s", "snake_case", Case::Snake),
                                ("S", "UPPER_SNAKE_CASE", Case::UpperSnake),
                                ("t", "Title Case", Case::Title),
                                ("u", "UPPER CASE", Case::Upper),
                            ]
                            .into_iter()
                            .map(|(key, description, case)| {
                                Keymap::new(
                                    key,
                                    description.to_string(),
                                    Dispatch::ToEditor(Transform(Transformation::Case(case))),
                                )
                            })
                            .collect_vec(),
                        ),
                    },
                    KeymapLegendSection {
                        title: "Other".to_string(),
                        keymaps: Keymaps::new(&[Keymap::new(
                            "w",
                            "Wrap".to_string(),
                            Dispatch::ToEditor(Transform(Transformation::Wrap)),
                        )]),
                    },
                ]
                .to_vec(),
            },
        }
    }

    fn space_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Space".to_string(),

            body: KeymapLegendBody::MultipleSections {
                sections: context
                    .contextual_keymaps()
                    .into_iter()
                    .chain([KeymapLegendSection {
                        title: "Pick".to_string(),
                        keymaps: Keymaps::new(
                            &[
                                ("b", "Pick Buffers", FilePickerKind::Opened),
                                (
                                    "f",
                                    "Pick Files (Non git ignored)",
                                    FilePickerKind::NonGitIgnored,
                                ),
                            ]
                            .into_iter()
                            .map(|(key, description, kind)| {
                                Keymap::new(
                                    key,
                                    description.to_string(),
                                    Dispatch::OpenFilePicker(kind),
                                )
                            })
                            .chain(
                                [
                                    ("g", DiffMode::UnstagedAgainstCurrentBranch),
                                    ("G", DiffMode::UnstagedAgainstMainBranch),
                                ]
                                .into_iter()
                                .map(|(key, diff_mode)| {
                                    Keymap::new(
                                        key,
                                        format!("Pick Git status ({})", diff_mode.display()),
                                        Dispatch::OpenFilePicker(FilePickerKind::GitStatus(
                                            diff_mode,
                                        )),
                                    )
                                }),
                            )
                            .chain(Some(Keymap::new(
                                "s",
                                "Pick Symbols".to_string(),
                                Dispatch::RequestDocumentSymbols,
                            )))
                            .chain(Some(Keymap::new(
                                "t",
                                "Pick Theme".to_string(),
                                Dispatch::OpenThemePrompt,
                            )))
                            .collect_vec(),
                        ),
                    }])
                    .chain(Some(KeymapLegendSection {
                        title: "Misc".to_string(),
                        keymaps: Keymaps::new(&[
                            Keymap::new(
                                "e",
                                "Reveal file in Explorer".to_string(),
                                Dispatch::RevealInExplorer(self.path().unwrap_or_else(|| {
                                    context.current_working_directory().clone()
                                })),
                            ),
                            Keymap::new(
                                "z",
                                "Undo Tree".to_string(),
                                Dispatch::ToEditor(DispatchEditor::EnterUndoTreeMode),
                            ),
                            Keymap::new(
                                "x",
                                "Tree-sitter node S-expr".to_string(),
                                Dispatch::ToEditor(DispatchEditor::ShowCurrentTreeSitterNodeSexp),
                            ),
                            Keymap::new(
                                "?",
                                "Help".to_string(),
                                Dispatch::ToEditor(DispatchEditor::ShowKeymapLegendHelp),
                            ),
                        ]),
                    }))
                    .collect(),
            },
        }
    }

    fn search_current_selection_keymap(
        &self,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
    ) -> Option<Keymap> {
        self.buffer()
            .slice(&self.selection_set.primary_selection().extended_range())
            .map(|search| {
                Keymap::new(
                    "*",
                    "Search current selection".to_string(),
                    Dispatch::UpdateLocalSearchConfig {
                        scope,
                        if_current_not_found,
                        update: crate::app::LocalSearchConfigUpdate::Search(search.to_string()),
                        show_config_after_enter: false,
                    },
                )
            })
            .ok()
    }

    pub(crate) fn find_keymap_legend_config(
        &self,
        context: &Context,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
    ) -> KeymapLegendConfig {
        let search_keymaps = {
            let config = context.get_local_search_config(scope);
            KeymapLegendSection {
                title: "Text".to_string(),
                keymaps: Keymaps::new(
                    &[
                        Keymap::new(
                            "'",
                            "Configure Search".to_string(),
                            Dispatch::ShowSearchConfig {
                                scope,
                                if_current_not_found,
                            },
                        ),
                        Keymap::new(
                            "/",
                            "Search".to_string(),
                            Dispatch::OpenSearchPrompt {
                                scope,
                                if_current_not_found,
                            },
                        ),
                    ]
                    .into_iter()
                    .chain(self.search_current_selection_keymap(scope, if_current_not_found))
                    .chain(config.last_search().map(|search| {
                        Keymap::new(
                            "p",
                            "Search (using previous search)".to_string(),
                            Dispatch::UpdateLocalSearchConfig {
                                scope,
                                if_current_not_found,
                                update: crate::app::LocalSearchConfigUpdate::Search(
                                    search.search.to_string(),
                                ),
                                show_config_after_enter: false,
                            },
                        )
                    }))
                    .collect_vec(),
                ),
            }
        };
        let misc_keymaps = KeymapLegendSection {
            title: "Misc".to_string(),
            keymaps: Keymaps::new(
                &[
                    Keymap::new(
                        "m",
                        "Mark".to_string(),
                        match scope {
                            Scope::Global => Dispatch::SetQuickfixList(QuickfixListType::Mark),
                            Scope::Local => {
                                Dispatch::ToEditor(SetSelectionMode(if_current_not_found, Mark))
                            }
                        },
                    ),
                    Keymap::new(
                        "q",
                        "Quickfix".to_string(),
                        match scope {
                            Scope::Global => Dispatch::SetGlobalMode(Some(
                                crate::context::GlobalMode::QuickfixListItem,
                            )),
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
                        ("g", DiffMode::UnstagedAgainstCurrentBranch),
                        ("G", DiffMode::UnstagedAgainstMainBranch),
                    ]
                    .map(|(key, diff_mode)| {
                        Keymap::new(
                            key,
                            format!("Git hunk ({})", diff_mode.display()),
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
                .collect_vec(),
            ),
        };
        let diagnostics_keymaps = {
            let keymaps = [
                ("a", "All", DiagnosticSeverityRange::All),
                ("e", "Error", DiagnosticSeverityRange::Error),
                ("h", "Hint", DiagnosticSeverityRange::Hint),
                ("I", "Information", DiagnosticSeverityRange::Information),
                ("w", "Warning", DiagnosticSeverityRange::Warning),
            ]
            .into_iter()
            .map(|(char, description, severity)| {
                Keymap::new(
                    char,
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
            KeymapLegendSection {
                title: "Diagnostics".to_string(),
                keymaps: Keymaps::new(&keymaps),
            }
        };
        let lsp_keymaps = {
            let keymaps = Keymaps::new(&[
                Keymap::new(
                    "d",
                    "Definitions".to_string(),
                    Dispatch::RequestDefinitions(scope),
                ),
                Keymap::new(
                    "D",
                    "Declarations".to_string(),
                    Dispatch::RequestDeclarations(scope),
                ),
                Keymap::new(
                    "i",
                    "Implementations".to_string(),
                    Dispatch::RequestImplementations(scope),
                ),
                Keymap::new(
                    "r",
                    "References".to_string(),
                    Dispatch::RequestReferences {
                        include_declaration: false,
                        scope,
                    },
                ),
                Keymap::new(
                    "R",
                    "References (include declaration)".to_string(),
                    Dispatch::RequestReferences {
                        include_declaration: true,
                        scope,
                    },
                ),
                Keymap::new(
                    "t",
                    "Type Definitions".to_string(),
                    Dispatch::RequestTypeDefinitions(scope),
                ),
            ]);

            KeymapLegendSection {
                title: "LSP".to_string(),
                keymaps,
            }
        };
        let local_keymaps = match scope {
            Scope::Local => Some(KeymapLegendSection {
                title: "Local only".to_string(),
                keymaps: Keymaps::new(
                    &[("n", "Natural Number", r"\d+")]
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
                            let dispatch = Dispatch::ToEditor(SetSelectionMode(
                                if_current_not_found,
                                Find { search },
                            ));
                            Keymap::new(key, description.to_string(), dispatch)
                        })
                        .chain([
                            Keymap::new(
                                "o",
                                "One character".to_string(),
                                Dispatch::ToEditor(FindOneChar(if_current_not_found)),
                            ),
                            Keymap::new(
                                "space",
                                "Empty line".to_string(),
                                Dispatch::ToEditor(SetSelectionMode(
                                    if_current_not_found,
                                    EmptyLine,
                                )),
                            ),
                        ])
                        .collect_vec(),
                ),
            }),
            Scope::Global => None,
        };
        KeymapLegendConfig {
            title: format!(
                "Find ({})",
                match scope {
                    Scope::Local => "Local",
                    Scope::Global => "Global",
                }
            ),

            body: KeymapLegendBody::MultipleSections {
                sections: Some(search_keymaps)
                    .into_iter()
                    .chain(Some(misc_keymaps))
                    .chain(Some(diagnostics_keymaps))
                    .chain(Some(lsp_keymaps))
                    .chain(local_keymaps)
                    .collect_vec(),
            },
        }
    }

    pub(crate) fn select_surround_keymap_legend_config(
        &self,
        kind: SurroundKind,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: format!("Select Surround ({:?})", kind),

            body: KeymapLegendBody::SingleSection {
                keymaps: generate_enclosures_keymaps(|enclosure| {
                    Dispatch::ToEditor(SelectSurround {
                        enclosure,
                        kind: kind.clone(),
                    })
                }),
            },
        }
    }

    pub(crate) fn delete_surround_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Delete Surround".to_string(),

            body: KeymapLegendBody::SingleSection {
                keymaps: generate_enclosures_keymaps(|enclosure| {
                    Dispatch::ToEditor(DeleteSurround(enclosure))
                }),
            },
        }
    }

    pub(crate) fn surround_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Surround".to_string(),

            body: KeymapLegendBody::SingleSection {
                keymaps: generate_enclosures_keymaps(|enclosure| {
                    let (open, close) = enclosure.open_close_symbols_str();
                    Dispatch::ToEditor(Surround(open.to_string(), close.to_string()))
                }),
            },
        }
    }

    pub(crate) fn change_surround_from_keymap_legend_config(
        &self,
    ) -> super::keymap_legend::KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Change Surround from:".to_string(),

            body: KeymapLegendBody::SingleSection {
                keymaps: generate_enclosures_keymaps(|enclosure| {
                    Dispatch::ShowKeymapLegend(
                        self.change_surround_to_keymap_legend_config(enclosure),
                    )
                }),
            },
        }
    }

    pub(crate) fn change_surround_to_keymap_legend_config(
        &self,
        from: EnclosureKind,
    ) -> super::keymap_legend::KeymapLegendConfig {
        KeymapLegendConfig {
            title: format!("Change Surround from {} to:", from.to_str()),

            body: KeymapLegendBody::SingleSection {
                keymaps: generate_enclosures_keymaps(|enclosure| {
                    Dispatch::ToEditor(ChangeSurround {
                        from,
                        to: enclosure,
                    })
                }),
            },
        }
    }
}

fn generate_enclosures_keymaps(get_dispatch: impl Fn(EnclosureKind) -> Dispatch) -> Keymaps {
    Keymaps::new(
        &[
            EnclosureKind::AngularBrackets,
            EnclosureKind::Parentheses,
            EnclosureKind::SquareBrackets,
            EnclosureKind::CurlyBraces,
            EnclosureKind::DoubleQuotes,
            EnclosureKind::SingleQuotes,
            EnclosureKind::Backticks,
        ]
        .into_iter()
        .map(|enclosure| {
            let (key, _) = enclosure.open_close_symbols_str();
            let description = enclosure.to_str();
            Keymap::new(key, description.to_string(), get_dispatch(enclosure))
        })
        .collect_vec(),
    )
}
