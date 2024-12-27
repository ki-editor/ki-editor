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
    editor_keymap_qwerty_positional::*,
    keymap_legend::{Keymap, KeymapLegendBody, KeymapLegendConfig, Keymaps},
};

use DispatchEditor::*;
use Meaning::*;
use Movement::*;
impl Editor {
    pub(crate) fn keymap_core_movements(&self) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Movements (Core)".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&LEFT_),
                    "Left".to_string(),
                    Dispatch::ToEditor(MoveSelection(Movement::Left)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&RIGHT),
                    "Right".to_string(),
                    Dispatch::ToEditor(MoveSelection(Right)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&UP___),
                    "Up".to_string(),
                    Dispatch::ToEditor(MoveSelection(Up)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&DOWN_),
                    "Down".to_string(),
                    Dispatch::ToEditor(MoveSelection(Down)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&NEXT_),
                    "Next".to_string(),
                    Dispatch::ToEditor(MoveSelection(Next)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&PREV_),
                    "Previous".to_string(),
                    Dispatch::ToEditor(MoveSelection(Previous)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&FIRST),
                    "First".to_string(),
                    Dispatch::ToEditor(MoveSelection(Movement::First)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&LAST_),
                    "Last".to_string(),
                    Dispatch::ToEditor(MoveSelection(Movement::Last)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&JUMP_),
                    "Jump".to_string(),
                    Dispatch::ToEditor(DispatchEditor::ShowJumps {
                        use_current_selection_mode: true,
                    }),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&TOIDX),
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
                    KEYBOARD_LAYOUT.get_key(&XACHR),
                    "Swap cursor with anchor".to_string(),
                    Dispatch::ToEditor(DispatchEditor::SwapCursorWithAnchor),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&CRSRP),
                    Direction::Start.format_action("Cycle primary selection"),
                    Dispatch::ToEditor(CyclePrimarySelection(Direction::Start)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&CRSRN),
                    Direction::End.format_action("Cycle primary selection"),
                    Dispatch::ToEditor(CyclePrimarySelection(Direction::End)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&SCRLD),
                    "Scroll down".to_string(),
                    Dispatch::ToEditor(ScrollPageDown),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&SCRLU),
                    "Scroll up".to_string(),
                    Dispatch::ToEditor(ScrollPageUp),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&GBACK),
                    "Go back".to_string(),
                    Dispatch::ToEditor(GoBack),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&GFORW),
                    "Go forward".to_string(),
                    Dispatch::ToEditor(GoForward),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&FILEP),
                    "Go to previous file".to_string(),
                    Dispatch::GoToPreviousFile,
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&FILEN),
                    "Go to next file".to_string(),
                    Dispatch::GoToNextFile,
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&BUFFP),
                    "Go to previous buffer".to_string(),
                    Dispatch::CycleBuffer(Direction::Start),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&BUFFN),
                    "Go to next buffer".to_string(),
                    Dispatch::CycleBuffer(Direction::End),
                ),
            ]),
        }
    }

    pub(crate) fn keymap_selection_modes(&self, context: &Context) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Selection mode".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&LINE_),
                    "Select Line".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&LINEF),
                    "Select Full Line".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(IfCurrentNotFound::LookForward, LineFull)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&GLOBL),
                    "Find (Global)".to_string(),
                    Dispatch::ShowKeymapLegend(self.find_keymap_legend_config(
                        context,
                        Scope::Global,
                        IfCurrentNotFound::LookForward,
                    )),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&SYTX_),
                    "Select Syntax Node".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(
                        IfCurrentNotFound::LookForward,
                        SyntaxNode,
                    )),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&SYTXF),
                    "Select Fine Syntax Node".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(
                        IfCurrentNotFound::LookForward,
                        SyntaxNodeFine,
                    )),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&TOKEN),
                    "Select Token".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(IfCurrentNotFound::LookForward, Token)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&WORD_),
                    "Select Word".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(IfCurrentNotFound::LookForward, Word)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&CHAR_),
                    "Select Character".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(IfCurrentNotFound::LookForward, Character)),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&FINDP),
                    "(Local) - Backward".to_string(),
                    Dispatch::ShowKeymapLegend(self.find_keymap_legend_config(
                        context,
                        Scope::Local,
                        IfCurrentNotFound::LookBackward,
                    )),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&FINDN),
                    "Find (Local) - Forward".to_string(),
                    Dispatch::ShowKeymapLegend(self.find_keymap_legend_config(
                        context,
                        Scope::Local,
                        IfCurrentNotFound::LookForward,
                    )),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&LSTNC),
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
                        KEYBOARD_LAYOUT.get_key(&JOIN_),
                        "Join".to_string(),
                        Dispatch::ToEditor(Transform(Transformation::Join)),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&BREAK),
                        "Break".to_string(),
                        Dispatch::ToEditor(BreakSelection),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&RAISE),
                        "Raise".to_string(),
                        Dispatch::ToEditor(Replace(Expand)),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&MARK_),
                        "Toggle Mark".to_string(),
                        Dispatch::ToEditor(ToggleMark),
                    ),
                ]
                .into_iter()
                .chain(if self.mode == Mode::MultiCursor {
                    [
                        Keymap::new(
                            KEYBOARD_LAYOUT.get_key(&CHNG_),
                            "Keep primary cursor only".to_string(),
                            Dispatch::ToEditor(DispatchEditor::CursorKeepPrimaryOnly),
                        ),
                        Keymap::new(
                            KEYBOARD_LAYOUT.get_key(&DELTN),
                            Direction::End.format_action("Delete primary cursor"),
                            Dispatch::ToEditor(DeleteCurrentCursor(Direction::End)),
                        ),
                        Keymap::new(
                            KEYBOARD_LAYOUT.get_key(&DELTP),
                            Direction::Start.format_action("Delete primary cursor"),
                            Dispatch::ToEditor(DeleteCurrentCursor(Direction::Start)),
                        ),
                        Keymap::new(
                            KEYBOARD_LAYOUT.get_key(&INSTP),
                            "Keep selections matching search".to_string(),
                            Dispatch::OpenFilterSelectionsPrompt { maintain: true },
                        ),
                        Keymap::new(
                            KEYBOARD_LAYOUT.get_key(&INSTN),
                            "Remove selections matching search".to_string(),
                            Dispatch::OpenFilterSelectionsPrompt { maintain: false },
                        ),
                    ]
                    .to_vec()
                } else {
                    [
                        Keymap::new(
                            KEYBOARD_LAYOUT.get_key(&CHNG_),
                            "Change".to_string(),
                            Dispatch::ToEditor(Change),
                        ),
                        Keymap::new(
                            KEYBOARD_LAYOUT.get_key(&DELTN),
                            Direction::End.format_action("Delete"),
                            Dispatch::ToEditor(Delete(Direction::End)),
                        ),
                        Keymap::new(
                            KEYBOARD_LAYOUT.get_key(&DELTP),
                            Direction::Start.format_action("Delete"),
                            Dispatch::ToEditor(Delete(Direction::Start)),
                        ),
                        Keymap::new(
                            KEYBOARD_LAYOUT.get_key(&INSTP),
                            Direction::Start.format_action("Insert"),
                            Dispatch::ToEditor(EnterInsertMode(Direction::Start)),
                        ),
                        Keymap::new(
                            KEYBOARD_LAYOUT.get_key(&INSTN),
                            Direction::End.format_action("Insert"),
                            Dispatch::ToEditor(EnterInsertMode(Direction::End)),
                        ),
                    ]
                    .to_vec()
                })
                .chain(Some(if self.selection_set.is_extended() {
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&EXCHG),
                        "Switch extended selection end".to_string(),
                        Dispatch::ToEditor(SwapExtensionDirection),
                    )
                } else {
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&EXCHG),
                        "Enter Exchange mode".to_string(),
                        Dispatch::ToEditor(EnterExchangeMode),
                    )
                }))
                .chain([
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&OPENN),
                        Direction::End.format_action("Open"),
                        Dispatch::ToEditor(Open(Direction::End)),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&OPENP),
                        Direction::Start.format_action("Open"),
                        Dispatch::ToEditor(Open(Direction::Start)),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&UNDO_),
                        "Undo".to_string(),
                        Dispatch::ToEditor(Undo),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&REDO_),
                        "Redo".to_string(),
                        Dispatch::ToEditor(Redo),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&PRPLC),
                        "Replace with pattern".to_string(),
                        Dispatch::ToEditor(ReplaceWithPattern),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&RPLCP),
                        "Replace (with previous copied text)".to_string(),
                        Dispatch::ToEditor(ReplaceWithPreviousCopiedText),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&RPLCN),
                        "Replace (with next copied text)".to_string(),
                        Dispatch::ToEditor(ReplaceWithNextCopiedText),
                    ),
                    Keymap::new("enter", "Save".to_string(), Dispatch::ToEditor(Save)),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&TRSFM),
                        "Transform".to_string(),
                        Dispatch::ShowKeymapLegend(self.transform_keymap_legend_config()),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&CSRCH),
                        "Configure Search".to_string(),
                        Dispatch::ShowSearchConfig {
                            scope: Scope::Local,
                            if_current_not_found: IfCurrentNotFound::LookForward,
                        },
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
                        KEYBOARD_LAYOUT.get_key(&SRCHN),
                        Direction::End.format_action("Search"),
                        Dispatch::OpenSearchPrompt {
                            scope: Scope::Local,
                            if_current_not_found: IfCurrentNotFound::LookForward,
                        },
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&SRCHP),
                        Direction::Start.format_action("Search"),
                        Dispatch::OpenSearchPrompt {
                            scope: Scope::Local,
                            if_current_not_found: IfCurrentNotFound::LookBackward,
                        },
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&INDNT),
                        "Indent".to_string(),
                        Dispatch::ToEditor(Indent),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&DEDNT),
                        "Dedent".to_string(),
                        Dispatch::ToEditor(Dedent),
                    ),
                ])
                .chain(Some(if self.selection_set.is_extended() {
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&VMODE),
                        "Enter V Mode".to_string(),
                        Dispatch::ToEditor(EnterVMode),
                    )
                } else {
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&VMODE),
                        "Select all".to_string(),
                        Dispatch::ToEditor(SelectAll),
                    )
                }))
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
                        KEYBOARD_LAYOUT.get_key(&CHNGX),
                        format!("{}{}", "Change Cut", extra),
                        Dispatch::ToEditor(ChangeCut {
                            use_system_clipboard,
                        }),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&PSTEN),
                        format!("{}{}", Direction::End.format_action("Paste"), extra),
                        Dispatch::ToEditor(Paste {
                            direction: Direction::End,
                            use_system_clipboard,
                        }),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&PSTEP),
                        format!("{}{}", Direction::Start.format_action("Paste"), extra),
                        Dispatch::ToEditor(Paste {
                            direction: Direction::Start,
                            use_system_clipboard,
                        }),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&RPLCX),
                        format!("{}{}", "Replace Cut", extra),
                        Dispatch::ToEditor(ReplaceWithCopiedText {
                            use_system_clipboard,
                            cut: true,
                        }),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&COPY_),
                        format!("{}{}", "Copy", extra),
                        Dispatch::ToEditor(Copy {
                            use_system_clipboard,
                        }),
                    ),
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&RPLC_),
                        format!("{}{}", "Replace", extra),
                        Dispatch::ToEditor(ReplaceWithCopiedText {
                            use_system_clipboard,
                            cut: false,
                        }),
                    ),
                ]
                .into_iter()
                .collect_vec(),
            ),
        }
    }

    fn keymap_universal(&self) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Universal keymaps (works in every mode)".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&WCLSE),
                    "Close current window".to_string(),
                    Dispatch::CloseCurrentWindow,
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&SVIEW),
                    "Switch view alignment".to_string(),
                    Dispatch::ToEditor(SwitchViewAlignment),
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&WSWTH),
                    "Switch window".to_string(),
                    Dispatch::OtherWindow,
                ),
                Keymap::new(
                    KEYBOARD_LAYOUT.get_key(&UPSTE),
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
                                "Delete token backward".to_string(),
                                Dispatch::ToEditor(DeleteWordBackward { short: false }),
                            ),
                            Keymap::new(
                                "alt+backspace",
                                "Delete word backward".to_string(),
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
                &[Keymap::new(
                    "~",
                    "Replace".to_string(),
                    Dispatch::ToEditor(EnterReplaceMode),
                )]
                .into_iter()
                .chain(Some(if self.selection_set.is_extended() {
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&MULTC),
                        "Surround".to_string(),
                        Dispatch::ShowKeymapLegend(self.surround_keymap_legend_config()),
                    )
                } else if self.mode == Mode::MultiCursor {
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&MULTC),
                        "Add cursor to all selections".to_string(),
                        Dispatch::ToEditor(DispatchEditor::CursorAddToAllSelections),
                    )
                } else {
                    Keymap::new(
                        KEYBOARD_LAYOUT.get_key(&MULTC),
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
                                KEYBOARD_LAYOUT.get_key(&INSTN),
                                "Select Around".to_string(),
                                Dispatch::ShowKeymapLegend(
                                    self.select_surround_keymap_legend_config(SurroundKind::Around),
                                ),
                            ),
                            Keymap::new(
                                KEYBOARD_LAYOUT.get_key(&INSTP),
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
                                KEYBOARD_LAYOUT.get_key(&CHNG_),
                                "Change Surround".to_string(),
                                Dispatch::ShowKeymapLegend(
                                    self.change_surround_from_keymap_legend_config(),
                                ),
                            ),
                            Keymap::new(
                                KEYBOARD_LAYOUT.get_key(&DELTN),
                                "Delete Surround".to_string(),
                                Dispatch::ShowKeymapLegend(
                                    self.delete_surround_keymap_legend_config(),
                                ),
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
                        ]),
                    }))
                    .chain(Some(KeymapLegendSection {
                        title: "File/Quitting".to_string(),
                        keymaps: Keymaps::new(&[
                            Keymap::new("w", "Write All".to_string(), Dispatch::SaveAll),
                            Keymap::new(
                                "q",
                                "Write All and Quit".to_string(),
                                Dispatch::SaveQuitAll,
                            ),
                            Keymap::new("Q", "Quit WITHOUT saving".to_string(), Dispatch::QuitAll),
                        ]),
                    }))
                    .chain(Some(KeymapLegendSection {
                        title: "Help".to_string(),
                        keymaps: Keymaps::new(&[Keymap::new(
                            "?",
                            "Help".to_string(),
                            Dispatch::ToEditor(DispatchEditor::ShowKeymapLegendHelp),
                        )]),
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
                    KEYBOARD_LAYOUT.get_key(&SRCHC),
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
