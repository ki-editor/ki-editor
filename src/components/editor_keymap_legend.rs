use crossterm::event::KeyCode;
use SelectionMode::*;

use convert_case::Case;
use event::KeyEvent;
use itertools::Itertools;

use crate::{
    app::{Dispatch, Dispatches, FilePickerKind, MakeFilterMechanism, Scope},
    components::{editor::Movement, keymap_legend::KeymapLegendSection},
    context::{Context, LocalSearchConfigMode, Search},
    list::grep::RegexConfig,
    quickfix_list::{DiagnosticSeverityRange, QuickfixListType},
    selection::{FilterKind, FilterTarget, SelectionMode},
    selection_mode::inside::InsideKind,
    transformation::Transformation,
};

use super::{
    component::Component,
    editor::{Direction, DispatchEditor, Editor, HandleEventResult},
    keymap_legend::{Keymap, KeymapLegendBody, KeymapLegendConfig, Keymaps},
    suggestive_editor::Info,
};

use DispatchEditor::*;
use Movement::*;
impl Editor {
    pub(crate) fn keymap_core_movements(&self) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Movement (Core, synergies with Movement Mode)".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    ",",
                    "First".to_string(),
                    Dispatch::ToEditor(MoveSelection(First)),
                ),
                Keymap::new(
                    ".",
                    "Last".to_string(),
                    Dispatch::ToEditor(MoveSelection(Last)),
                ),
                Keymap::new(
                    "h",
                    "Higher (Previous)".to_string(),
                    Dispatch::ToEditor(MoveSelection(Movement::Previous)),
                ),
                Keymap::new(
                    "j",
                    "Down / First Child".to_string(),
                    Dispatch::ToEditor(MoveSelection(Down)),
                ),
                Keymap::new(
                    "k",
                    "Up / Parent".to_string(),
                    Dispatch::ToEditor(MoveSelection(Up)),
                ),
                Keymap::new(
                    "l",
                    "Lower (Next)".to_string(),
                    Dispatch::ToEditor(MoveSelection(Next)),
                ),
                Keymap::new(
                    "f",
                    "Find (Jump)".to_string(),
                    Dispatch::ToEditor(DispatchEditor::ShowJumps),
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
            title: "Movement (Other)".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    "%",
                    "Swap cursor with anchor".to_string(),
                    Dispatch::ToEditor(DispatchEditor::SwapCursorWithAnchor),
                ),
                Keymap::new(
                    "ctrl+d",
                    "Scroll page down".to_string(),
                    Dispatch::ToEditor(ScrollPageDown),
                ),
                Keymap::new(
                    "ctrl+u",
                    "Scroll page up".to_string(),
                    Dispatch::ToEditor(ScrollPageUp),
                ),
                Keymap::new(
                    "backspace",
                    "Go to previous selection".to_string(),
                    Dispatch::GoToPreviousSelection,
                ),
                Keymap::new(
                    "tab",
                    "Go to next selection".to_string(),
                    Dispatch::GoToNextSelection,
                ),
            ]),
        }
    }

    pub(crate) fn keymap_selection_modes(&self, context: &Context) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Selection mode".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    "b",
                    "Between".to_string(),
                    Dispatch::ShowKeymapLegend(self.inside_mode_keymap_legend_config()),
                ),
                Keymap::new(
                    "e",
                    "Line Trimmed".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(LineTrimmed)),
                ),
                Keymap::new(
                    "E",
                    "Line (Extended)".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(LineFull)),
                ),
                Keymap::new(
                    "g",
                    "Global".to_string(),
                    Dispatch::ShowKeymapLegend(self.find_global_keymap_legend_config(context)),
                ),
                Keymap::new(
                    "n",
                    "Native".to_string(),
                    Dispatch::ShowKeymapLegend(self.find_local_keymap_legend_config(context)),
                ),
                Keymap::new(
                    "s",
                    "Syntax Tree".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(SyntaxTree)),
                ),
                Keymap::new(
                    "t",
                    "Token".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(BottomNode)),
                ),
                Keymap::new(
                    "u",
                    "Character (Unicode)".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(Character)),
                ),
                Keymap::new(
                    "w",
                    "Word (Short)".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(WordShort)),
                ),
                Keymap::new(
                    "W",
                    "Word (Long)".to_string(),
                    Dispatch::ToEditor(SetSelectionMode(WordLong)),
                ),
            ]),
        }
    }
    pub(crate) fn keymap_actions(&self) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Action".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    "c",
                    "Change".to_string(),
                    Dispatch::ToEditor(Change { cut: false }),
                ),
                Keymap::new(
                    "C",
                    "Change Cut".to_string(),
                    Dispatch::ToEditor(Change { cut: true }),
                ),
                Keymap::new(
                    "d",
                    "Delete".to_string(),
                    Dispatch::ToEditor(Delete { cut: false }),
                ),
                Keymap::new(
                    "D",
                    "Delete Cut".to_string(),
                    Dispatch::ToEditor(Delete { cut: true }),
                ),
                Keymap::new(
                    "o",
                    "Open (after selection)".to_string(),
                    Dispatch::ToEditor(Open(Direction::End)),
                ),
                Keymap::new(
                    "O",
                    "Open (before selection)".to_string(),
                    Dispatch::ToEditor(Open(Direction::Start)),
                ),
                Keymap::new(
                    "m",
                    "Mark (Toggle)".to_string(),
                    Dispatch::ToEditor(ToggleBookmark),
                ),
                Keymap::new(
                    "p",
                    "Paste (after selection)".to_string(),
                    Dispatch::ToEditor(Paste(Direction::End)),
                ),
                Keymap::new(
                    "P",
                    "Paste (before selection)".to_string(),
                    Dispatch::ToEditor(Paste(Direction::Start)),
                ),
                Keymap::new(
                    "!",
                    "Transform".to_string(),
                    Dispatch::ShowKeymapLegend(self.transform_keymap_legend_config()),
                ),
                Keymap::new(
                    "r",
                    "Replace".to_string(),
                    Dispatch::ToEditor(ReplaceWithCopiedText),
                ),
                Keymap::new(
                    "R",
                    "Replace Cut".to_string(),
                    Dispatch::ToEditor(ReplaceCut),
                ),
                Keymap::new("y", "Yank (Copy)".to_string(), Dispatch::ToEditor(Copy)),
                Keymap::new("enter", "Save".to_string(), Dispatch::ToEditor(Save)),
            ]),
        }
    }

    fn keymap_universal(&self) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Universal keymaps (works in every mode)".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    "ctrl+l",
                    "Switch view alignment".to_string(),
                    Dispatch::ToEditor(SwitchViewAlignment),
                ),
                Keymap::new(
                    "ctrl+o",
                    "Oscillate window".to_string(),
                    Dispatch::OscillateWindow,
                ),
                Keymap::new(
                    "ctrl+q",
                    "Close current window".to_string(),
                    Dispatch::CloseCurrentWindow,
                ),
                Keymap::new(
                    "ctrl+v",
                    "Paste".to_string(),
                    Dispatch::ToEditor(Paste(Direction::End)),
                ),
                Keymap::new("ctrl+y", "Redo".to_string(), Dispatch::ToEditor(Redo)),
                Keymap::new("ctrl+z", "Undo".to_string(), Dispatch::ToEditor(Undo)),
            ]),
        }
    }

    pub fn insert_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Insert mode keymaps".to_string(),
            owner_id: self.id(),
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
                                "Kill line forward".to_string(),
                                Dispatch::ToEditor(KillLine(Direction::End)),
                            ),
                            Keymap::new(
                                "ctrl+u",
                                "Kill line backward".to_string(),
                                Dispatch::ToEditor(KillLine(Direction::Start)),
                            ),
                            Keymap::new(
                                "ctrl+w",
                                "Delete word (long) backward".to_string(),
                                Dispatch::ToEditor(DeleteWordBackward { short: false }),
                            ),
                            Keymap::new(
                                "alt+backspace",
                                "Delete word (short) backward".to_string(),
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
                                Dispatch::ToEditor(Insert("\n".to_string())),
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

    pub fn handle_insert_mode(&mut self, event: KeyEvent) -> anyhow::Result<Dispatches> {
        if let Some(dispatches) = self
            .insert_mode_keymap_legend_config()
            .keymaps()
            .iter()
            .find(|keymap| &event == keymap.event())
            .map(|keymap| Dispatches::one(keymap.dispatch()))
        {
            Ok(dispatches)
        } else if let KeyCode::Char(c) = event.code {
            return self.insert(&c.to_string());
        } else {
            Ok(Default::default())
        }
    }

    pub fn handle_universal_key(&mut self, event: KeyEvent) -> anyhow::Result<HandleEventResult> {
        if let Some(keymap) = self.keymap_universal().keymaps.get(&event) {
            Ok(HandleEventResult::Handled(Dispatches::one(
                keymap.dispatch(),
            )))
        } else {
            Ok(HandleEventResult::Ignored(event))
        }
    }

    pub fn keymap_modes(&self) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Mode".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    "a",
                    "Insert (after selection)".to_string(),
                    Dispatch::ToEditor(EnterInsertMode(Direction::End)),
                ),
                Keymap::new(
                    "i",
                    "Insert (before selection)".to_string(),
                    Dispatch::ToEditor(EnterInsertMode(Direction::Start)),
                ),
                Keymap::new(
                    "v",
                    "Visual (Extend selection)".to_string(),
                    Dispatch::ToEditor(ToggleHighlightMode),
                ),
            ]),
        }
    }

    pub fn keymap_movement_modes(&self) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Movement Mode".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    "enter",
                    "Replace".to_string(),
                    Dispatch::ToEditor(EnterReplaceMode),
                ),
                Keymap::new(
                    "q",
                    "Multi-qursor".to_string(),
                    Dispatch::ToEditor(EnterMultiCursorMode),
                ),
                Keymap::new(
                    "x",
                    "Exchange".to_string(),
                    Dispatch::ToEditor(EnterExchangeMode),
                ),
            ]),
        }
    }
    pub fn keymap_others(&self) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Others".to_string(),
            keymaps: Keymaps::new(&[
                Keymap::new(
                    "space",
                    "Search (List)".to_string(),
                    Dispatch::ShowKeymapLegend(self.search_list_mode_keymap_legend_config()),
                ),
                Keymap::new(
                    "\\",
                    "Context menu".to_string(),
                    Dispatch::ShowKeymapLegend(self.context_menu_legend_config()),
                ),
                Keymap::new("*", "Select all".to_string(), Dispatch::ToEditor(SelectAll)),
                Keymap::new(
                    ":",
                    "Open command prompt".to_string(),
                    Dispatch::OpenCommandPrompt,
                ),
                Keymap::new(
                    "/",
                    "Omit selection".to_string(),
                    Dispatch::ShowKeymapLegend(self.omit_mode_keymap_legend_config()),
                ),
                Keymap::new(
                    "?",
                    "Help (Normal mode)".to_string(),
                    Dispatch::ToEditor(DispatchEditor::ShowKeymapLegendHelp),
                ),
                Keymap::new(
                    "esc",
                    "Remain only this window".to_string(),
                    Dispatch::RemainOnlyCurrentComponent,
                ),
            ]),
        }
    }

    pub fn help_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Help".to_string(),
            body: KeymapLegendBody::SingleSection {
                keymaps: Keymaps::new(&[
                    Keymap::new(
                        "i",
                        "Insert mode".to_string(),
                        Dispatch::ToEditor(ShowKeymapLegendInsertMode),
                    ),
                    Keymap::new(
                        "n",
                        "Normal mode".to_string(),
                        Dispatch::ToEditor(ShowKeymapLegendNormalMode),
                    ),
                ]),
            },
            owner_id: self.id(),
        }
    }

    pub fn normal_mode_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Normal mode".to_string(),
            body: KeymapLegendBody::MultipleSections {
                sections: [
                    self.keymap_core_movements(),
                    self.keymap_other_movements(),
                    self.keymap_selection_modes(context),
                    self.keymap_actions(),
                    self.keymap_modes(),
                    self.keymap_movement_modes(),
                    self.keymap_surround(),
                    self.keymap_others(),
                    self.keymap_universal(),
                ]
                .to_vec(),
            },
            owner_id: self.id(),
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
    pub(crate) fn context_menu_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Context menu".to_string(),
            body: KeymapLegendBody::MultipleSections {
                sections: [self.get_request_params().map(|params| KeymapLegendSection {
                    title: "LSP".to_string(),
                    keymaps: Keymaps::new(&[
                        Keymap::new(
                            "h",
                            "Hover".to_string(),
                            Dispatch::RequestHover(params.clone()),
                        ),
                        Keymap::new(
                            "r",
                            "Rename".to_string(),
                            Dispatch::PrepareRename(params.clone()),
                        ),
                        Keymap::new("c", "Code Actions".to_string(), {
                            let cursor_char_index = self.get_cursor_char_index();
                            Dispatch::RequestCodeAction {
                                params,
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
                    ]),
                })]
                .into_iter()
                .flatten()
                .chain(Some(KeymapLegendSection {
                    title: "Multi-cursor".to_string(),
                    keymaps: Keymaps::new(&[
                        Keymap::new(
                            "a",
                            "Add cursor to all selections".to_string(),
                            Dispatch::ToEditor(CursorAddToAllSelections),
                        ),
                        Keymap::new(
                            "o",
                            "Keep only primary cursor".to_string(),
                            Dispatch::ToEditor(DispatchEditor::CursorKeepPrimaryOnly),
                        ),
                    ]),
                }))
                .chain(
                    self.buffer()
                        .get_current_node(&self.selection_set.primary, false)
                        .ok()
                        .and_then(|node| {
                            Some(KeymapLegendSection {
                                title: "Tree-sitter node".to_string(),
                                keymaps: Keymaps::new(&[Keymap::new(
                                    "s",
                                    "S-expression".to_string(),
                                    Dispatch::ShowEditorInfo(Info::new(
                                        "Tree-sitter node S-expression".to_string(),
                                        node?.to_sexp(),
                                    )),
                                )]),
                            })
                        }),
                )
                .collect(),
            },
            owner_id: self.id(),
        }
    }
    pub fn handle_normal_mode(
        &mut self,
        context: &Context,
        event: KeyEvent,
    ) -> anyhow::Result<Dispatches> {
        if let Some(keymap) = self.normal_mode_keymaps(context).get(&event) {
            return Ok([keymap.dispatch()].to_vec().into());
        }
        log::info!("unhandled event: {:?}", event);
        Ok(vec![].into())
    }

    pub fn transform_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Transform".to_string(),
            owner_id: self.id(),
            body: KeymapLegendBody::MultipleSections {
                sections: [
                    KeymapLegendSection {
                        title: "Letter case".to_string(),
                        keymaps: Keymaps::new(
                            &[
                                ("a", "aLtErNaTiNg CaSe", Case::Toggle),
                                ("c", "camelCase", Case::Camel),
                                ("l", "lowercase", Case::Lower),
                                ("k", "kebab-case", Case::Kebab),
                                ("K", "Upper-Kebab", Case::UpperKebab),
                                ("p", "PascalCase", Case::Pascal),
                                ("s", "snake_case", Case::Snake),
                                ("S", "UPPER_SNAKE_CASE", Case::UpperSnake),
                                ("t", "Title Case", Case::Title),
                                ("u", "UPPERCASE", Case::Upper),
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
                        keymaps: Keymaps::new(&[
                            Keymap::new(
                                "j",
                                "Join".to_string(),
                                Dispatch::ToEditor(Transform(Transformation::Join)),
                            ),
                            Keymap::new(
                                "w",
                                "Wrap".to_string(),
                                Dispatch::ToEditor(Transform(Transformation::Wrap)),
                            ),
                        ]),
                    },
                ]
                .to_vec(),
            },
        }
    }

    pub fn keymap_surround(&self) -> KeymapLegendSection {
        KeymapLegendSection {
            title: "Surround".to_string(),
            keymaps: Keymaps::new(
                &[
                    ("<", "Angular bracket", "<", ">"),
                    ("(", "Parentheses", "(", ")"),
                    ("[", "Square bracket", "[", "]"),
                    ("{", "Curly bracket", "{", "}"),
                    ("\"", "Double quote", "\"", "\""),
                    ("'", "Single quote", "'", "'"),
                    ("`", "Backtick", "`", "`"),
                ]
                .into_iter()
                .map(|(key, description, open, close)| {
                    Keymap::new(
                        key,
                        description.to_string(),
                        Dispatch::ToEditor(Surround(open.to_string(), close.to_string())),
                    )
                })
                .collect_vec(),
            ),
        }
    }

    pub fn search_list_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Space".to_string(),
            owner_id: self.id(),
            body: KeymapLegendBody::MultipleSections {
                sections: [KeymapLegendSection {
                    title: "Files".to_string(),
                    keymaps: Keymaps::new(
                        &[
                            ("g", "Git status", FilePickerKind::GitStatus),
                            (
                                "f",
                                "Files (Not git ignored)",
                                FilePickerKind::NonGitIgnored,
                            ),
                            ("o", "Opened files", FilePickerKind::Opened),
                        ]
                        .into_iter()
                        .map(|(key, description, kind)| {
                            Keymap::new(
                                key,
                                description.to_string(),
                                Dispatch::OpenFilePicker(kind),
                            )
                        })
                        .collect_vec(),
                    ),
                }]
                .into_iter()
                .chain(self.get_request_params().map(|params| KeymapLegendSection {
                    title: "LSP".to_string(),
                    keymaps: Keymaps::new(&[Keymap::new(
                        "s",
                        "Symbols".to_string(),
                        Dispatch::RequestDocumentSymbols(params.set_description("Symbols")),
                    )]),
                }))
                .chain(Some(KeymapLegendSection {
                    title: "Misc".to_string(),
                    keymaps: Keymaps::new(
                        &[Keymap::new(
                            "z",
                            "Undo Tree".to_string(),
                            Dispatch::ToEditor(EnterUndoTreeMode),
                        )]
                        .into_iter()
                        .chain(self.path().map(|path| {
                            Keymap::new(
                                "e",
                                "Explorer".to_string(),
                                Dispatch::RevealInExplorer(path),
                            )
                        }))
                        .collect_vec(),
                    ),
                }))
                .collect(),
            },
        }
    }

    fn keymap_diagnostics(&self, scope: Scope) -> KeymapLegendSection {
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
                    Scope::Local => Dispatch::ToEditor(SetSelectionMode(Diagnostic(severity))),
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
    }

    pub(crate) fn keymap_text_search(
        &self,
        context: &Context,
        scope: Scope,
    ) -> KeymapLegendSection {
        let config = context.get_local_search_config(scope);
        KeymapLegendSection {
            title: "Text".to_string(),
            keymaps: Keymaps::new(
                &[
                    Keymap::new(
                        ",",
                        "Configure Search".to_string(),
                        Dispatch::ShowSearchConfig {
                            owner_id: self.id(),
                            scope,
                        },
                    ),
                    Keymap::new(
                        "s",
                        "Search".to_string(),
                        Dispatch::OpenSearchPrompt {
                            scope,
                            owner_id: self.id(),
                        },
                    ),
                ]
                .into_iter()
                .chain(
                    self.buffer()
                        .slice(&self.selection_set.primary.extended_range())
                        .map(|search| {
                            Keymap::new(
                                "c",
                                "Current selection".to_string(),
                                Dispatch::UpdateLocalSearchConfig {
                                    owner_id: self.id(),
                                    scope,
                                    update: crate::app::LocalSearchConfigUpdate::Search(
                                        search.to_string(),
                                    ),
                                    show_config_after_enter: false,
                                },
                            )
                        }),
                )
                .chain(match scope {
                    Scope::Local => [
                        Keymap::new(
                            "l",
                            "Literal".to_string(),
                            Dispatch::ShowKeymapLegend(self.show_literal_keymap_legend_config()),
                        ),
                        Keymap::new(
                            "space",
                            "Empty line".to_string(),
                            Dispatch::ToEditor(SetSelectionMode(EmptyLine)),
                        ),
                    ]
                    .to_vec(),
                    Scope::Global => Vec::new(),
                })
                .chain(config.last_search().map(|search| {
                    Keymap::new(
                        "p",
                        "Search (using previous search)".to_string(),
                        Dispatch::ToEditor(SetSelectionMode(Find { search })),
                    )
                }))
                .collect_vec(),
            ),
        }
    }

    pub fn find_local_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        let owner_id = self.id();
        let scope = Scope::Local;
        KeymapLegendConfig {
            title: "Native".to_string(),
            owner_id,
            body: KeymapLegendBody::MultipleSections {
                sections: Some(self.keymap_text_search(context, Scope::Local))
                    .into_iter()
                    .chain(Some(KeymapLegendSection {
                        title: "Misc".to_string(),
                        keymaps: Keymaps::new(&[
                            Keymap::new(
                                "m",
                                "Mark".to_string(),
                                Dispatch::ToEditor(SetSelectionMode(Bookmark)),
                            ),
                            Keymap::new(
                                "o",
                                "One character".to_string(),
                                Dispatch::ToEditor(FindOneChar),
                            ),
                            Keymap::new(
                                "g",
                                "Git hunk".to_string(),
                                Dispatch::ToEditor(SetSelectionMode(GitHunk)),
                            ),
                            Keymap::new(
                                "q",
                                "Quickfix".to_string(),
                                Dispatch::ToEditor(SetSelectionMode(LocalQuickfix {
                                    title: "LOCAL QUICKFIX".to_string(),
                                })),
                            ),
                        ]),
                    }))
                    .chain(Some(self.keymap_diagnostics(scope)))
                    .chain(Some(self.lsp_keymap(scope)))
                    .collect_vec(),
            },
        }
    }

    fn lsp_keymap(&self, scope: Scope) -> KeymapLegendSection {
        let keymaps = Keymaps::new(
            &self
                .get_request_params()
                .map(|params| {
                    let params = params.set_kind(Some(scope));
                    [
                        Keymap::new(
                            "d",
                            "Definitions".to_string(),
                            Dispatch::RequestDefinitions(
                                params.clone().set_description("Definitions"),
                            ),
                        ),
                        Keymap::new(
                            "D",
                            "Declarations".to_string(),
                            Dispatch::RequestDeclarations(
                                params.clone().set_description("Declarations"),
                            ),
                        ),
                        Keymap::new(
                            "i",
                            "Implementations".to_string(),
                            Dispatch::RequestImplementations(
                                params.clone().set_description("Implementations"),
                            ),
                        ),
                        Keymap::new(
                            "r",
                            "References".to_string(),
                            Dispatch::RequestReferences {
                                params: params.clone().set_description("References"),
                                include_declaration: false,
                            },
                        ),
                        Keymap::new(
                            "R",
                            "References (include declaration)".to_string(),
                            Dispatch::RequestReferences {
                                params: params
                                    .clone()
                                    .set_description("References (include declaration)"),
                                include_declaration: true,
                            },
                        ),
                        Keymap::new(
                            "t",
                            "Type Definitions".to_string(),
                            Dispatch::RequestTypeDefinitions(
                                params.clone().set_description("Type Definitions"),
                            ),
                        ),
                    ]
                    .into_iter()
                    .collect_vec()
                })
                .unwrap_or_default(),
        );

        KeymapLegendSection {
            title: "LSP".to_string(),
            keymaps,
        }
    }

    pub fn find_global_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        let scope = Scope::Global;
        KeymapLegendConfig {
            title: "Global".to_string(),
            owner_id: self.id(),
            body: KeymapLegendBody::MultipleSections {
                sections: [KeymapLegendSection {
                    title: "Misc".to_string(),
                    keymaps: Keymaps::new(&[
                        Keymap::new("g", "Git Hunk".to_string(), Dispatch::GetRepoGitHunks),
                        Keymap::new(
                            "m",
                            "Mark".to_string(),
                            Dispatch::SetQuickfixList(QuickfixListType::Bookmark),
                        ),
                        Keymap::new(
                            "q",
                            "Quickfix".to_string(),
                            Dispatch::SetGlobalMode(Some(
                                crate::context::GlobalMode::QuickfixListItem,
                            )),
                        ),
                    ]),
                }]
                .into_iter()
                .chain(Some(self.keymap_text_search(context, scope)))
                .chain(Some(self.keymap_diagnostics(scope)))
                .chain(Some(self.lsp_keymap(scope)))
                .collect_vec(),
            },
        }
    }

    pub fn inside_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Inside".to_string(),
            owner_id: self.id(),
            body: KeymapLegendBody::SingleSection {
                keymaps: Keymaps::new(
                    &[
                        ("<", "Angular Bracket", InsideKind::AngularBrackets),
                        ("`", "Back Quote", InsideKind::Backtick),
                        ("{", "Curly Brace", InsideKind::CurlyBraces),
                        ("\"", "Double Quote", InsideKind::DoubleQuotes),
                        ("(", "Parenthesis", InsideKind::Parentheses),
                        ("'", "Single Quote", InsideKind::SingleQuotes),
                        ("[", "Square Bracket", InsideKind::SquareBrackets),
                    ]
                    .into_iter()
                    .map(|(key, description, inside_kind)| {
                        Keymap::new(
                            key,
                            description.to_string(),
                            Dispatch::ToEditor(EnterInsideMode(inside_kind)),
                        )
                    })
                    .chain(Some(Keymap::new(
                        "o",
                        "Other".to_string(),
                        Dispatch::OpenInsideOtherPromptOpen,
                    )))
                    .collect_vec(),
                ),
            },
        }
    }

    pub fn omit_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        let filter_mechanism_keymaps = |kind: FilterKind, target: FilterTarget| -> Dispatch {
            Dispatch::ShowKeymapLegend(KeymapLegendConfig {
                title: format!("Omit: {:?} {:?} matching", kind, target),
                owner_id: self.id(),
                body: KeymapLegendBody::SingleSection {
                    keymaps: Keymaps::new(
                        [
                            Keymap::new(
                                "l",
                                "Literal".to_string(),
                                Dispatch::OpenOmitPrompt {
                                    kind,
                                    target,
                                    make_mechanism: MakeFilterMechanism::Literal,
                                },
                            ),
                            Keymap::new(
                                "r",
                                "Regex".to_string(),
                                Dispatch::OpenOmitPrompt {
                                    kind,
                                    target,
                                    make_mechanism: MakeFilterMechanism::Regex,
                                },
                            ),
                        ]
                        .as_ref(),
                    ),
                },
            })
        };
        let filter_target_keymaps = |kind: FilterKind| -> Dispatch {
            Dispatch::ShowKeymapLegend(KeymapLegendConfig {
                title: format!("Omit: {:?}", kind),
                owner_id: self.id(),
                body: KeymapLegendBody::SingleSection {
                    keymaps: Keymaps::new(
                        [
                            Keymap::new(
                                "c",
                                "Content".to_string(),
                                filter_mechanism_keymaps(kind, FilterTarget::Content),
                            ),
                            Keymap::new(
                                "i",
                                "Info".to_string(),
                                filter_mechanism_keymaps(kind, FilterTarget::Info),
                            ),
                        ]
                        .as_ref(),
                    ),
                },
            })
        };
        KeymapLegendConfig {
            title: "Omit".to_string(),
            owner_id: self.id(),
            body: KeymapLegendBody::SingleSection {
                keymaps: Keymaps::new(
                    [
                        Keymap::new("c", "Clear".to_string(), Dispatch::ToEditor(FilterClear)),
                        Keymap::new(
                            "k",
                            "keep".to_string(),
                            filter_target_keymaps(FilterKind::Keep),
                        ),
                        Keymap::new(
                            "r",
                            "remove".to_string(),
                            filter_target_keymaps(FilterKind::Remove),
                        ),
                    ]
                    .as_ref(),
                ),
            },
        }
    }

    pub fn show_literal_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Find literal".to_string(),
            owner_id: self.id(),
            body: KeymapLegendBody::SingleSection {
                keymaps: Keymaps::new(
                    &[
                        ("b", "Binary", r"\b[01]+\b"),
                        ("f", "Float", r"[-+]?\d*\.\d+|\d+"),
                        ("h", "Hexadecimal", r"[0-9a-fA-F]+"),
                        ("i", "Integer", r"-?\d+"),
                        ("n", "Natural", r"\d+"),
                        ("o", "Octal", r"\b[0-7]+\b"),
                        ("s", "Scientific", r"[-+]?\d*\.?\d+[eE][-+]?\d+"),
                    ]
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
                        let dispatch = Dispatch::ToEditor(SetSelectionMode(Find { search }));
                        Keymap::new(key, description.to_string(), dispatch)
                    })
                    .collect_vec(),
                ),
            },
        }
    }
}
