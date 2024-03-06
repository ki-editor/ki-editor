use my_proc_macros::key;

use SelectionMode::*;

use convert_case::Case;
use event::KeyEvent;
use itertools::Itertools;
use lsp_types::DiagnosticSeverity;

use crate::{
    app::{Dispatch, FilePickerKind, MakeFilterMechanism, RequestParams, Scope},
    components::{editor::Movement, keymap_legend::KeymapLegendSection},
    context::{Context, LocalSearchConfigMode, Search},
    list::grep::RegexConfig,
    quickfix_list::QuickfixListType,
    selection::{FilterKind, FilterTarget, SelectionMode},
    selection_mode::inside::InsideKind,
};

use super::{
    component::Component,
    editor::{Direction, DispatchEditor, Editor},
    keymap_legend::{Keymap, KeymapLegendBody, KeymapLegendConfig, Keymaps},
};

use DispatchEditor::*;
use Movement::*;
impl Editor {
    pub(crate) fn keymap_movements(&self) -> Vec<Keymap> {
        [
            Keymap::new(
                "H",
                "Highest (First)".to_string(),
                Dispatch::DispatchEditor(MoveSelection(First)),
            ),
            Keymap::new(
                "h",
                "Higher (Previous)".to_string(),
                Dispatch::DispatchEditor(MoveSelection(Movement::Previous)),
            ),
            Keymap::new(
                "j",
                "Down / First Child".to_string(),
                Dispatch::DispatchEditor(MoveSelection(Down)),
            ),
            Keymap::new(
                "k",
                "Up / Parent".to_string(),
                Dispatch::DispatchEditor(MoveSelection(Up)),
            ),
            Keymap::new(
                "l",
                "Lower (Next)".to_string(),
                Dispatch::DispatchEditor(MoveSelection(Next)),
            ),
            Keymap::new(
                "L",
                "Lowest (Last)".to_string(),
                Dispatch::DispatchEditor(MoveSelection(Last)),
            ),
            Keymap::new(
                "s",
                "Skip (Jump)".to_string(),
                Dispatch::DispatchEditor(DispatchEditor::Jump),
            ),
            Keymap::new(
                "-",
                "Parent Line".to_string(),
                Dispatch::DispatchEditor(MoveSelection(ToParentLine)),
            ),
            Keymap::new("0", "To Index".to_string(), Dispatch::OpenMoveToIndexPrompt),
        ]
        .to_vec()
    }

    pub(crate) fn keymap_selection_modes(&self, context: &Context) -> Vec<Keymap> {
        [
            Keymap::new(
                "b",
                "Between".to_string(),
                Dispatch::ShowKeymapLegend(self.inside_mode_keymap_legend_config()),
            ),
            Keymap::new(
                "e",
                "Line (Trimmed)".to_string(),
                Dispatch::DispatchEditor(SetSelectionMode(LineTrimmed)),
            ),
            Keymap::new(
                "E",
                "Line (Full)".to_string(),
                Dispatch::DispatchEditor(SetSelectionMode(LineFull)),
            ),
            Keymap::new(
                "f",
                "Find (Local)".to_string(),
                Dispatch::ShowKeymapLegend(self.find_local_keymap_legend_config(context)),
            ),
            Keymap::new(
                "g",
                "Find (Global)".to_string(),
                Dispatch::ShowKeymapLegend(self.find_global_keymap_legend_config(context)),
            ),
            Keymap::new(
                "n",
                "Node".to_string(),
                Dispatch::DispatchEditor(SetSelectionMode(SyntaxTree)),
            ),
            Keymap::new(
                "t",
                "Token".to_string(),
                Dispatch::DispatchEditor(SetSelectionMode(BottomNode)),
            ),
            Keymap::new(
                "o",
                "Column".to_string(),
                Dispatch::DispatchEditor(SetSelectionMode(Character)),
            ),
            Keymap::new(
                "w",
                "Word".to_string(),
                Dispatch::DispatchEditor(SetSelectionMode(Word)),
            ),
        ]
        .to_vec()
    }
    pub(crate) fn keymap_actions(&self) -> Vec<Keymap> {
        [
            Keymap::new("c", "Change".to_string(), Dispatch::DispatchEditor(Change)),
            Keymap::new("d", "Delete".to_string(), Dispatch::DispatchEditor(Kill)),
            Keymap::new(
                "m",
                "Toggle Mark".to_string(),
                Dispatch::DispatchEditor(ToggleBookmark),
            ),
            Keymap::new("r", "Raise".to_string(), Dispatch::DispatchEditor(Raise)),
            Keymap::new(
                "R",
                "Replace".to_string(),
                Dispatch::DispatchEditor(ReplaceCut),
            ),
            Keymap::new(
                "u",
                "Update".to_string(),
                Dispatch::ShowKeymapLegend(self.update_keymap_legend_config()),
            ),
            Keymap::new(
                "y",
                "Yank (Copy)".to_string(),
                Dispatch::DispatchEditor(Copy),
            ),
        ]
        .to_vec()
    }
    pub fn keymap_modes(&self) -> Vec<Keymap> {
        [
            Keymap::new(
                "a",
                "Insert after selection".to_string(),
                Dispatch::DispatchEditor(EnterInsertMode(Direction::End)),
            ),
            Keymap::new(
                "i",
                "Insert before selection".to_string(),
                Dispatch::DispatchEditor(EnterInsertMode(Direction::Start)),
            ),
            Keymap::new(
                "v",
                "Visual (Extend selection)".to_string(),
                Dispatch::DispatchEditor(ToggleHighlightMode),
            ),
            Keymap::new(
                "x",
                "Exchange".to_string(),
                Dispatch::DispatchEditor(EnterExchangeMode),
            ),
            Keymap::new(
                "z",
                "Multi-cursor".to_string(),
                Dispatch::DispatchEditor(EnterMultiCursorMode),
            ),
        ]
        .to_vec()
    }
    pub fn keymap_others(&self, context: &Context) -> Vec<Keymap> {
        [
            Keymap::new(
                "/",
                "Omit selection".to_string(),
                Dispatch::ShowKeymapLegend(self.omit_mode_keymap_legend_config()),
            ),
            Keymap::new(
                "q",
                "Context menu".to_string(),
                Dispatch::ShowKeymapLegend(self.context_menu_legend_config(context)),
            ),
            Keymap::new(
                "space",
                "Search (List)".to_string(),
                Dispatch::ShowKeymapLegend(self.search_list_mode_keymap_legend_config()),
            ),
            Keymap::new(
                "?",
                "Help (Normal mode)".to_string(),
                Dispatch::DispatchEditor(ShowKeymapLegendNormalMode),
            ),
        ]
        .to_vec()
    }
    pub fn normal_mode_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Normal mode".to_string(),
            body: KeymapLegendBody::MultipleSections {
                sections: [
                    KeymapLegendSection {
                        title: "Movement".to_string(),
                        keymaps: Keymaps::new(&self.keymap_movements()),
                    },
                    KeymapLegendSection {
                        title: "Selection mode".to_string(),
                        keymaps: Keymaps::new(&self.keymap_selection_modes(context)),
                    },
                    KeymapLegendSection {
                        title: "Action".to_string(),
                        keymaps: Keymaps::new(&self.keymap_actions()),
                    },
                    KeymapLegendSection {
                        title: "Mode".to_string(),
                        keymaps: Keymaps::new(&self.keymap_modes()),
                    },
                    KeymapLegendSection {
                        title: "Surround".to_string(),
                        keymaps: Keymaps::new(&self.keymap_surround()),
                    },
                    KeymapLegendSection {
                        title: "Others".to_string(),
                        keymaps: Keymaps::new(&self.keymap_others(context)),
                    },
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
                .map(|keymap| keymap.clone())
                .collect_vec(),
        )
    }
    pub(crate) fn context_menu_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Context menu".to_string(),
            body: KeymapLegendBody::MultipleSections {
                sections: [
                    self.get_request_params().map(|params| KeymapLegendSection {
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
                            Keymap::new(
                                "c",
                                "Code Actions".to_string(),
                                Dispatch::RequestCodeAction {
                                    params,
                                    diagnostics: context
                                        .get_diagnostics(self.path())
                                        .into_iter()
                                        .filter_map(|diagnostic| {
                                            if diagnostic
                                                .range
                                                .contains(&self.get_cursor_position().ok()?)
                                            {
                                                diagnostic.original_value.clone()
                                            } else {
                                                None
                                            }
                                        })
                                        .collect_vec(),
                                },
                            ),
                        ]),
                    }),
                    Some(KeymapLegendSection {
                        title: "Other".to_string(),
                        keymaps: Keymaps::new(
                            &[Keymap::new(
                                "z",
                                "Undo Tree".to_string(),
                                Dispatch::DispatchEditor(EnterUndoTreeMode),
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
                    }),
                ]
                .to_vec()
                .into_iter()
                .flatten()
                .collect(),
            },
            owner_id: self.id(),
        }
    }
    pub fn handle_normal_mode(
        &mut self,
        context: &Context,
        event: KeyEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        if let Some(keymap) = self.normal_mode_keymaps(context).get(&event) {
            return Ok([keymap.dispatch()].to_vec());
        }
        match event {
            // I think command should be nested inside `'` pickers
            key!(":") => return Ok([Dispatch::OpenCommandPrompt].to_vec()),
            key!("*") => return Ok(self.select_all()),
            key!("ctrl+d") => {
                return self.scroll_page_down();
            }
            key!("ctrl+u") => {
                return self.scroll_page_up();
            }

            key!("left") => return self.handle_movement(context, Movement::Previous),
            key!("shift+left") => return self.handle_movement(context, Movement::First),
            key!("right") => return self.handle_movement(context, Movement::Next),
            key!("shift+right") => return self.handle_movement(context, Movement::Last),
            key!("esc") => {
                self.reset();
                return Ok(vec![
                    Dispatch::CloseAllExceptMainPanel,
                    Dispatch::SetGlobalMode(None),
                ]);
            }
            key!("shift+K") => return self.select_kids(),
            // r for rotate? more general than swapping/exchange, which does not warp back to first
            // selection
            // y = unused
            key!("enter") => return self.open_new_line(),
            key!("%") => self.change_cursor_direction(),

            key!("ctrl+o") => return Ok([Dispatch::GoToPreviousSelection].to_vec()),
            key!("tab") => return Ok([Dispatch::GoToNextSelection].to_vec()),

            key!("space") => {
                return Ok(vec![Dispatch::ShowKeymapLegend(
                    self.space_mode_keymap_legend_config(),
                )])
            }
            _ => {
                log::info!("event: {:?}", event);
            }
        };
        Ok(vec![])
    }

    // TODO: where to put this?
    pub fn space_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Space".to_string(),
            owner_id: self.id(),
            body: KeymapLegendBody::SingleSection {
                keymaps: Keymaps::new(
                    &vec![]
                        .into_iter()
                        .chain(
                            self.get_request_params()
                                .map(|params| {
                                    [
                                        Keymap::new(
                                            "e",
                                            "Explorer".to_string(),
                                            Dispatch::ShowKeymapLegend(
                                                self.explorer_keymap_legend_config(params.clone()),
                                            ),
                                        ),
                                        Keymap::new(
                                            "z",
                                            "Undo Tree".to_string(),
                                            Dispatch::DispatchEditor(EnterUndoTreeMode),
                                        ),
                                    ]
                                    .to_vec()
                                })
                                .unwrap_or_default(),
                        )
                        .collect_vec(),
                ),
            },
        }
    }

    pub fn update_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Transform".to_string(),
            owner_id: self.id(),
            body: KeymapLegendBody::MultipleSections {
                sections: [KeymapLegendSection {
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
                                Dispatch::DispatchEditor(Transform(case)),
                            )
                        })
                        .collect_vec(),
                    ),
                }]
                .to_vec(),
            },
        }
    }

    pub fn keymap_surround(&self) -> Vec<Keymap> {
        [
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
                Dispatch::DispatchEditor(Surround(open.to_string(), close.to_string())),
            )
        })
        .collect_vec()
    }

    pub fn search_list_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Search (List)".to_string(),
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
                .to_vec()
                .into_iter()
                .chain(self.get_request_params().map(|params| KeymapLegendSection {
                    title: "LSP".to_string(),
                    keymaps: Keymaps::new(&[Keymap::new(
                        "s",
                        "Symbols".to_string(),
                        Dispatch::RequestDocumentSymbols(params.set_description("Symbols")),
                    )]),
                }))
                .collect(),
            },
        }
    }

    fn keymap_diagnostics(&self, scope: Scope) -> KeymapLegendSection {
        let keymaps = [
            ("a", "Any", None),
            ("e", "Error", Some(DiagnosticSeverity::ERROR)),
            ("h", "Hint", Some(DiagnosticSeverity::HINT)),
            ("I", "Information", Some(DiagnosticSeverity::INFORMATION)),
            ("w", "Warning", Some(DiagnosticSeverity::WARNING)),
        ]
        .into_iter()
        .map(|(char, description, severity)| {
            Keymap::new(
                char,
                description.to_string(),
                match scope {
                    Scope::Local => {
                        Dispatch::DispatchEditor(SetSelectionMode(Diagnostic(severity)))
                    }
                    Scope::Global => {
                        Dispatch::SetQuickfixList(QuickfixListType::LspDiagnostic(severity))
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
                                "Search current selection".to_string(),
                                Dispatch::UpdateLocalSearchConfig {
                                    owner_id: self.id(),
                                    scope,
                                    update: crate::app::LocalSearchConfigUpdate::SetSearch(
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
                            "o",
                            "One character".to_string(),
                            Dispatch::DispatchEditor(FindOneChar),
                        ),
                        Keymap::new(
                            "space",
                            "Empty line".to_string(),
                            Dispatch::DispatchEditor(SetSelectionMode(EmptyLine)),
                        ),
                    ]
                    .to_vec(),
                    Scope::Global => Vec::new(),
                })
                .chain(config.last_search().map(|search| {
                    Keymap::new(
                        "p",
                        "Search (using previous search)".to_string(),
                        Dispatch::DispatchEditor(SetSelectionMode(Find { search })),
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
            title: Self::find_submenu_title("", scope),
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
                                Dispatch::DispatchEditor(SetSelectionMode(Bookmark)),
                            ),
                            Keymap::new(
                                "g",
                                "Git hunk".to_string(),
                                Dispatch::DispatchEditor(SetSelectionMode(GitHunk)),
                            ),
                            Keymap::new(
                                "q",
                                "Quickfix".to_string(),
                                Dispatch::DispatchEditor(SetSelectionMode(LocalQuickfix {
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
            title: Self::find_submenu_title("", scope),
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
                        ("`", "Back Quote", InsideKind::BackQuotes),
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
                            Dispatch::DispatchEditor(EnterInsideMode(inside_kind)),
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
                        Keymap::new(
                            "c",
                            "Clear".to_string(),
                            Dispatch::DispatchEditor(FilterClear),
                        ),
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
                        let dispatch = Dispatch::DispatchEditor(SetSelectionMode(Find { search }));
                        Keymap::new(key, description.to_string(), dispatch)
                    })
                    .collect_vec(),
                ),
            },
        }
    }

    pub fn explorer_keymap_legend_config(&self, params: RequestParams) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Space: Explorer".to_string(),
            body: KeymapLegendBody::SingleSection {
                keymaps: Keymaps::new(
                    [Keymap::new(
                        "r",
                        "Reveal in Explorer".to_string(),
                        Dispatch::RevealInExplorer(params.path.clone()),
                    )]
                    .as_ref(),
                ),
            },
            owner_id: self.id(),
        }
    }
}
