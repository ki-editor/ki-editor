use convert_case::Case;
use itertools::Itertools;
use lsp_types::DiagnosticSeverity;

use crate::{
    app::{Dispatch, FilePickerKind, LocalSearchConfigUpdate, RequestParams, Scope},
    components::keymap_legend::KeymapLegendSection,
    context::{Context, LocalSearchConfigMode, Search},
    list::grep::RegexConfig,
    quickfix_list::QuickfixListType,
    selection::{FilterKind, FilterTarget, SelectionMode},
    selection_mode::inside::InsideKind,
};

use super::{
    component::Component,
    editor::{DispatchEditor, Editor},
    keymap_legend::{Keymap, KeymapLegendBody, KeymapLegendConfig, Keymaps},
};

impl Editor {
    pub fn space_mode_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
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
                                            "l",
                                            "LSP".to_string(),
                                            Dispatch::ShowKeymapLegend(
                                                self.lsp_action_keymap_legend_config(
                                                    context, params,
                                                ),
                                            ),
                                        ),
                                        Keymap::new(
                                            "t",
                                            "Transform".to_string(),
                                            Dispatch::ShowKeymapLegend(
                                                self.transform_keymap_legend_config(),
                                            ),
                                        ),
                                        Keymap::new(
                                            "z",
                                            "Undo Tree".to_string(),
                                            Dispatch::DispatchEditor(
                                                DispatchEditor::EnterUndoTreeMode,
                                            ),
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

    pub fn transform_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Transform".to_string(),
            owner_id: self.id(),
            body: KeymapLegendBody::SingleSection {
                keymaps: Keymaps::new(
                    &[
                        ("a", "aLtErNaTiNg CaSe", Case::Toggle),
                        ("c", "camelCase", Case::Camel),
                        ("l", "lowercase", Case::Lower),
                        ("k", "kebab-case", Case::Kebab),
                        ("K", "Upper-Kebab", Case::UpperKebab),
                        ("p", "PascalCase", Case::Pascal),
                        ("s", "snake_case", Case::Snake),
                        ("m", "MARCO_CASE", Case::UpperSnake),
                        ("t", "Title Case", Case::Title),
                        ("u", "UPPERCASE", Case::Upper),
                    ]
                    .into_iter()
                    .map(|(key, description, case)| {
                        Keymap::new(
                            key,
                            description.to_string(),
                            Dispatch::DispatchEditor(DispatchEditor::Transform(case)),
                        )
                    })
                    .collect_vec(),
                ),
            },
        }
    }

    pub fn list_mode_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "List".to_string(),
            owner_id: self.id(),
            body: KeymapLegendBody::SingleSection {
                keymaps: Keymaps::new(
                    &[
                        ("g", "Git status", FilePickerKind::GitStatus),
                        ("n", "Not git ignored files", FilePickerKind::NonGitIgnored),
                        ("o", "Opened files", FilePickerKind::Opened),
                    ]
                    .into_iter()
                    .map(|(key, description, kind)| {
                        Keymap::new(key, description.to_string(), Dispatch::OpenFilePicker(kind))
                    })
                    .collect_vec(),
                ),
            },
        }
    }

    pub fn x_mode_keymap_legend_config(&self) -> anyhow::Result<KeymapLegendConfig> {
        Ok(KeymapLegendConfig {
            title: "X (Regex/Bracket/Quote)".to_string(),
            owner_id: self.id(),
            body: KeymapLegendBody::SingleSection {
                keymaps: Keymaps::new(
                    &[
                        Keymap::new(
                            "e",
                            "Empty line".to_string(),
                            Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                                SelectionMode::EmptyLine,
                            )),
                        ),
                        Keymap::new(
                            "n",
                            "Number".to_string(),
                            Dispatch::ShowKeymapLegend(self.show_number_keymap_legend_config()),
                        ),
                        Keymap::new(
                            "o",
                            "One character".to_string(),
                            Dispatch::DispatchEditor(DispatchEditor::FindOneChar),
                        ),
                    ]
                    .into_iter()
                    .collect_vec(),
                ),
            },
        })
    }

    fn diagnostics_keymap(&self, scope: Scope) -> KeymapLegendSection {
        let keymaps = [
            ("a", "Any", None),
            ("e", "Error", Some(DiagnosticSeverity::ERROR)),
            ("h", "Hint", Some(DiagnosticSeverity::HINT)),
            ("I", "Information", Some(DiagnosticSeverity::INFORMATION)),
            ("w", "Warning", Some(DiagnosticSeverity::WARNING)),
        ]
        .into_iter()
        .into_iter()
        .map(|(char, description, severity)| {
            Keymap::new(
                char,
                description.to_string(),
                match scope {
                    Scope::Local => Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                        SelectionMode::Diagnostic(severity),
                    )),
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

    pub fn find_local_keymap_legend_config(
        &self,
        context: &Context,
    ) -> anyhow::Result<KeymapLegendConfig> {
        let owner_id = self.id();
        let scope = Scope::Local;
        Ok(KeymapLegendConfig {
            title: Self::find_submenu_title("", scope),
            owner_id,
            body: KeymapLegendBody::MultipleSections {
                sections: Some(KeymapLegendSection {
                    title: "Misc".to_string(),
                    keymaps: Keymaps::new(
                        &[
                            Keymap::new(
                                "b",
                                "Bookmark".to_string(),
                                Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                                    SelectionMode::Bookmark,
                                )),
                            ),
                            Keymap::new(
                                "g",
                                "Git hunk".to_string(),
                                Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                                    SelectionMode::GitHunk,
                                )),
                            ),
                            Keymap::new(
                                "q",
                                "Quickfix list (current)".to_string(),
                                Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                                    SelectionMode::LocalQuickfix {
                                        title: "LOCAL QUICKFIX".to_string(),
                                    },
                                )),
                            ),
                            {
                                let mode = context.local_search_config().mode.clone();
                                Keymap::new(
                                    "f",
                                    format!("Search ({})", mode.display()),
                                    Dispatch::OpenSearchPrompt(mode, Scope::Local),
                                )
                            },
                            Keymap::new(
                                "F",
                                "Search Config".to_string(),
                                Dispatch::ShowSearchConfig {
                                    owner_id: self.id(),
                                    scope: Scope::Local,
                                },
                            ),
                        ]
                        .into_iter()
                        .chain(context.last_search().map(|search| {
                            Keymap::new(
                                "f",
                                "Enter Find Mode (using previous search)".to_string(),
                                Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                                    SelectionMode::Find { search },
                                )),
                            )
                        }))
                        .collect_vec(),
                    ),
                })
                .into_iter()
                .chain(Some(self.diagnostics_keymap(scope)))
                .chain(Some(self.lsp_keymap(scope)))
                .collect_vec(),
            },
        })
    }

    fn lsp_keymap(&self, scope: Scope) -> KeymapLegendSection {
        let keymaps = Keymaps::new(
            &self
                .get_request_params()
                .map(|params| {
                    let params = params.set_kind(Some(scope));
                    [
                        Keymap::new(
                            "c",
                            "Declarations".to_string(),
                            Dispatch::RequestDeclarations(
                                params.clone().set_description("Declarations"),
                            ),
                        ),
                        Keymap::new(
                            "d",
                            "Definitions".to_string(),
                            Dispatch::RequestDefinitions(
                                params.clone().set_description("Definitions"),
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
                        Keymap::new(
                            "s",
                            "Symbols".to_string(),
                            Dispatch::RequestDocumentSymbols(params.set_description("Symbols")),
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
                        {
                            let mode = context.local_search_config().mode;
                            Keymap::new(
                                "f",
                                format!("Search ({})", mode.display()),
                                Dispatch::OpenSearchPrompt(mode, Scope::Global),
                            )
                        },
                        Keymap::new(
                            "F",
                            "Search Config".to_string(),
                            Dispatch::ShowSearchConfig {
                                owner_id: self.id(),
                                scope: Scope::Global,
                            },
                        ),
                        Keymap::new("g", "Git Hunk".to_string(), Dispatch::GetRepoGitHunks),
                        Keymap::new(
                            "b",
                            "Bookmark".to_string(),
                            Dispatch::SetQuickfixList(QuickfixListType::Bookmark),
                        ),
                    ]),
                }]
                .into_iter()
                .chain(Some(self.diagnostics_keymap(scope)))
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
                        ("a", "Angular Bracket <>", InsideKind::AngularBrackets),
                        ("b", "Back Quote ``", InsideKind::BackQuotes),
                        ("c", "Curly Brace {}", InsideKind::CurlyBraces),
                        ("d", "Double Quote \"\"", InsideKind::DoubleQuotes),
                        ("p", "Parenthesis ()", InsideKind::Parentheses),
                        ("q", "Single Quote ''", InsideKind::SingleQuotes),
                        ("s", "Square Bracket []", InsideKind::SquareBrackets),
                    ]
                    .into_iter()
                    .map(|(key, description, inside_kind)| {
                        Keymap::new(
                            key,
                            description.to_string(),
                            Dispatch::DispatchEditor(DispatchEditor::EnterInsideMode(inside_kind)),
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
                        &[
                            Keymap::new(
                                "l",
                                "Literal".to_string(),
                                Dispatch::OpenOmitLiteralPrompt { kind, target },
                            ),
                            Keymap::new(
                                "r",
                                "Regex".to_string(),
                                Dispatch::OpenOmitRegexPrompt { kind, target },
                            ),
                        ]
                        .to_vec(),
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
                        &[
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
                        .to_vec(),
                    ),
                },
            })
        };
        KeymapLegendConfig {
            title: "Omit".to_string(),
            owner_id: self.id(),
            body: KeymapLegendBody::SingleSection {
                keymaps: Keymaps::new(
                    &[
                        Keymap::new(
                            "c",
                            "Clear".to_string(),
                            Dispatch::DispatchEditor(DispatchEditor::FilterClear),
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
                    .to_vec(),
                ),
            },
        }
    }

    pub fn show_number_keymap_legend_config(&self) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Find number".to_string(),
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
                        let dispatch = Dispatch::DispatchEditor(DispatchEditor::SetSelectionMode(
                            SelectionMode::Find { search },
                        ));
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
                    &[Keymap::new(
                        "r",
                        "Reveal in Explorer".to_string(),
                        Dispatch::RevealInExplorer(params.path.clone()),
                    )]
                    .to_vec(),
                ),
            },
            owner_id: self.id(),
        }
    }

    pub fn lsp_action_keymap_legend_config(
        &self,
        context: &Context,
        params: RequestParams,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Space: LSP".to_string(),
            body: KeymapLegendBody::SingleSection {
                keymaps: Keymaps::new(
                    &[
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
                            "a",
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
                    ]
                    .to_vec(),
                ),
            },
            owner_id: self.id(),
        }
    }
}
