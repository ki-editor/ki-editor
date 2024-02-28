use super::editor::Enclosure;
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
                "h",
                "Previous".to_string(),
                Dispatch::DispatchEditor(MoveSelection(Movement::Previous)),
            ),
            Keymap::new(
                "j",
                "Down".to_string(),
                Dispatch::DispatchEditor(MoveSelection(Down)),
            ),
            Keymap::new(
                "k",
                "Up".to_string(),
                Dispatch::DispatchEditor(MoveSelection(Up)),
            ),
            Keymap::new(
                "l",
                "Next".to_string(),
                Dispatch::DispatchEditor(MoveSelection(Next)),
            ),
            Keymap::new(
                "o",
                "Oscillate (Jump)".to_string(),
                Dispatch::DispatchEditor(DispatchEditor::Jump),
            ),
            Keymap::new(
                "p",
                "Parent".to_string(),
                Dispatch::DispatchEditor(MoveSelection(Parent)),
            ),
            Keymap::new(
                "q",
                "First Child".to_string(),
                Dispatch::DispatchEditor(MoveSelection(FirstChild)),
            ),
            Keymap::new(
                ",",
                "First".to_string(),
                Dispatch::DispatchEditor(MoveSelection(First)),
            ),
            Keymap::new(
                ".",
                "Last".to_string(),
                Dispatch::DispatchEditor(MoveSelection(Last)),
            ),
            Keymap::new(
                "-",
                "Parent Line".to_string(),
                Dispatch::DispatchEditor(MoveSelection(ToParentLine)),
            ),
        ]
        .to_vec()
    }

    pub(crate) fn keymap_selection_modes(&self, context: &Context) -> anyhow::Result<Vec<Keymap>> {
        Ok([
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
                Dispatch::ShowKeymapLegend(self.find_local_keymap_legend_config(context)?),
            ),
            Keymap::new(
                "g",
                "Find (Global)".to_string(),
                Dispatch::ShowKeymapLegend(self.find_global_keymap_legend_config(context)),
            ),
            Keymap::new(
                "n",
                "Bottom Node".to_string(),
                Dispatch::DispatchEditor(SetSelectionMode(BottomNode)),
            ),
            Keymap::new(
                "s",
                "Syntax Tree".to_string(),
                Dispatch::DispatchEditor(SetSelectionMode(SyntaxTree)),
            ),
            Keymap::new(
                "t",
                "Top Node".to_string(),
                Dispatch::DispatchEditor(SetSelectionMode(TopNode)),
            ),
            Keymap::new(
                "u",
                "Character".to_string(),
                Dispatch::DispatchEditor(SetSelectionMode(Character)),
            ),
            Keymap::new(
                "w",
                "Word".to_string(),
                Dispatch::DispatchEditor(SetSelectionMode(Word)),
            ),
        ]
        .to_vec())
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
                "/",
                "Omit selection".to_string(),
                Dispatch::ShowKeymapLegend(self.omit_mode_keymap_legend_config()),
            ),
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
    fn normal_mode_keymap_legend_config(
        &self,
        context: &Context,
    ) -> anyhow::Result<KeymapLegendConfig> {
        Ok(KeymapLegendConfig {
            title: "Normal mode".to_string(),
            body: KeymapLegendBody::MultipleSections {
                sections: [
                    KeymapLegendSection {
                        title: "Movement".to_string(),
                        keymaps: Keymaps::new(&self.keymap_movements()),
                    },
                    KeymapLegendSection {
                        title: "Selection mode".to_string(),
                        keymaps: Keymaps::new(&self.keymap_selection_modes(context)?),
                    },
                    KeymapLegendSection {
                        title: "Action".to_string(),
                        keymaps: Keymaps::new(&self.keymap_actions()),
                    },
                    KeymapLegendSection {
                        title: "Mode".to_string(),
                        keymaps: Keymaps::new(&self.keymap_modes()),
                    },
                ]
                .to_vec(),
            },
            owner_id: self.id(),
        })
    }
    fn normal_mode_keymaps(&self, context: &Context) -> anyhow::Result<Keymaps> {
        Ok(Keymaps::new(
            &self
                .keymap_movements()
                .into_iter()
                .chain(self.keymap_selection_modes(context)?)
                .chain(self.keymap_actions())
                .chain(self.keymap_modes())
                .collect_vec(),
        ))
    }
    pub fn handle_normal_mode(
        &mut self,
        context: &Context,
        event: KeyEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        if let Some(keymap) = self.normal_mode_keymaps(context)?.get(&event) {
            return Ok([keymap.dispatch()].to_vec());
        }
        match event {
            key!("?") => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.normal_mode_keymap_legend_config(context)?,
                )]
                .to_vec())
            }
            key!("'") => {
                return Ok([Dispatch::ShowKeymapLegend(
                    self.list_mode_keymap_legend_config(),
                )]
                .to_vec())
            }
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
            key!("shift+R") => return self.replace(context),
            // y = unused
            key!("enter") => return self.open_new_line(),
            key!("%") => self.change_cursor_direction(),
            key!("(") | key!(")") => return self.enclose(Enclosure::RoundBracket),
            // key!("{") => {
            // return Ok([Dispatch::GotoSelectionHistoryContiguous(Movement::Previous)].to_vec())
            // }
            // key!("}") => {
            // return Ok([Dispatch::GotoSelectionHistoryContiguous(Movement::Next)].to_vec())
            // }
            key!("ctrl+o") => return Ok([Dispatch::GoToPreviousSelection].to_vec()),
            key!("tab") => return Ok([Dispatch::GoToNextSelection].to_vec()),
            key!("[") | key!("]") => return self.enclose(Enclosure::SquareBracket),
            key!('{') | key!('}') => return self.enclose(Enclosure::CurlyBracket),
            key!('<') | key!('>') => return self.enclose(Enclosure::AngleBracket),

            key!("space") => {
                return Ok(vec![Dispatch::ShowKeymapLegend(
                    self.space_mode_keymap_legend_config(context),
                )])
            }
            key!("0") => return Ok([Dispatch::OpenMoveToIndexPrompt].to_vec()),
            _ => {
                log::info!("event: {:?}", event);
            }
        };
        Ok(vec![])
    }
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

    fn diagnostics_keymap(&self, scope: Scope) -> KeymapLegendSection {
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
        let config = context.local_search_config();
        let mode = config.mode;
        Ok(KeymapLegendConfig {
            title: Self::find_submenu_title("", scope),
            owner_id,
            body: KeymapLegendBody::MultipleSections {
                sections: Some(KeymapLegendSection {
                    title: "Misc".to_string(),
                    keymaps: Keymaps::new(
                        &[
                            Keymap::new(
                                "m",
                                "Mark".to_string(),
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
                            Keymap::new(
                                "f",
                                format!("Search ({})", mode.display()),
                                Dispatch::OpenSearchPrompt {
                                    scope: Scope::Local,
                                    owner_id: self.id(),
                                },
                            ),
                            Keymap::new(
                                "c",
                                "Configure Search".to_string(),
                                Dispatch::ShowSearchConfig {
                                    owner_id: self.id(),
                                    scope: Scope::Local,
                                },
                            ),
                            Keymap::new(
                                "space",
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
                        .chain(config.last_search().map(|search| {
                            Keymap::new(
                                "p",
                                "Search (using previous search)".to_string(),
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
                                Dispatch::OpenSearchPrompt {
                                    scope: Scope::Global,
                                    owner_id: self.id(),
                                },
                            )
                        },
                        Keymap::new(
                            "c",
                            "Configure Search".to_string(),
                            Dispatch::ShowSearchConfig {
                                owner_id: self.id(),
                                scope: Scope::Global,
                            },
                        ),
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
                        ("<", "Angular Bracket <>", InsideKind::AngularBrackets),
                        ("`", "Back Quote ``", InsideKind::BackQuotes),
                        ("{", "Curly Brace {}", InsideKind::CurlyBraces),
                        ("\"", "Double Quote \"\"", InsideKind::DoubleQuotes),
                        ("(", "Parenthesis ()", InsideKind::Parentheses),
                        ("'", "Single Quote ''", InsideKind::SingleQuotes),
                        ("[", "Square Bracket []", InsideKind::SquareBrackets),
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
                    .as_ref(),
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

    pub fn lsp_action_keymap_legend_config(
        &self,
        context: &Context,
        params: RequestParams,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Space: LSP".to_string(),
            body: KeymapLegendBody::SingleSection {
                keymaps: Keymaps::new(
                    [
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
                    .as_ref(),
                ),
            },
            owner_id: self.id(),
        }
    }
}
