use crate::{
    app::{Dispatch, FilePickerKind, Scope},
    components::{
        editor::{
            Direction, DispatchEditor, Editor, IfCurrentNotFound, Movement, PriorChange, Reveal,
            SurroundKind,
        },
        editor_keymap::{possibly_alted, QWERTY_STR},
        editor_keymap_legend::NormalModeOverride,
        keymap_legend::{
            Keybinding, Keymap, KeymapLegendConfig, MomentaryLayer, OnTap, ReleaseKey,
        },
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

use convert_case::Case;
use itertools::Itertools;
use my_proc_macros::{doc_format, key};
use DispatchEditor::*;
use SelectionMode::*;

pub fn transform_keymap_legend_config() -> KeymapLegendConfig {
    KeymapLegendConfig {
        title: "Transform".to_string(),

        keymap: Keymap::new(&keymap_transform()),
    }
}

fn generate_enclosures_keymap(get_dispatch: impl Fn(EnclosureKind) -> Dispatch) -> Keymap {
    Keymap::new(&[
        Keybinding::new_undocumented("m", "( )", get_dispatch(EnclosureKind::Parentheses)),
        Keybinding::new_undocumented(",", "[ ]", get_dispatch(EnclosureKind::SquareBrackets)),
        Keybinding::new_undocumented(".", "{ }", get_dispatch(EnclosureKind::CurlyBraces)),
        Keybinding::new_undocumented("/", "< >", get_dispatch(EnclosureKind::AngularBrackets)),
        Keybinding::new_undocumented("j", "' '", get_dispatch(EnclosureKind::SingleQuotes)),
        Keybinding::new_undocumented("k", "\" \"", get_dispatch(EnclosureKind::DoubleQuotes)),
        Keybinding::new_undocumented("l", "` `", get_dispatch(EnclosureKind::Backticks)),
    ])
}

pub fn multicursor_menu_keymap(editor: &Editor) -> Keymap {
    let primary_selection_mode_keybindings =
        keymap_primary_selection_modes(editor, Some(PriorChange::EnterMultiCursorMode));
    let secondary_selection_mode_keybindings =
        keymap_secondary_selection_modes_init(editor, Some(PriorChange::EnterMultiCursorMode));
    let other_keybindings = [
        Keybinding::new_undocumented("j", "Curs All", Dispatch::AddCursorToAllSelections),
        Keybinding::new_undocumented(
            "i",
            "Keep Match",
            Dispatch::OpenFilterSelectionsPrompt { maintain: true },
        ),
        Keybinding::new_undocumented(
            "k",
            "Remove Match",
            Dispatch::OpenFilterSelectionsPrompt { maintain: false },
        ),
        Keybinding::new_undocumented("l", "Keep Primary Curs", Dispatch::KeepCursorPrimaryOnly),
    ];
    Keymap::new(
        &[].into_iter()
            .chain(primary_selection_mode_keybindings)
            .chain(secondary_selection_mode_keybindings)
            .chain(other_keybindings)
            .collect_vec(),
    )
}
fn secondary_selection_modes_keybindings(
    editor: &Editor,
    scope: Scope,
    if_current_not_found: IfCurrentNotFound,
    prior_change: Option<PriorChange>,
) -> Vec<Keybinding> {
    let search_keybindings = {
        [].into_iter()
            .chain(
                [Keybinding::new_undocumented(
                    "n",
                    "Repeat",
                    Dispatch::UseLastNonContiguousSelectionMode(if_current_not_found),
                )]
                .to_vec(),
            )
            .collect_vec()
    };

    let diff_mode_to_dispatch = |diff_mode| match scope {
        Scope::Global => Dispatch::GetRepoGitHunks(diff_mode),
        Scope::Local => Dispatch::ToEditor(SetSelectionModeWithPriorChange(
            if_current_not_found,
            GitHunk(diff_mode),
            prior_change,
        )),
    };
    let misc_keybindings = [
        Keybinding::new_undocumented(
            "e",
            "Mark",
            match scope {
                Scope::Global => Dispatch::SetQuickfixList(QuickfixListType::Mark),
                Scope::Local => Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    if_current_not_found,
                    Mark,
                    prior_change,
                )),
            },
        ),
        Keybinding::new_undocumented(
            "t",
            "Quickfix",
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
    .chain([
        Keybinding::new_undocumented(
            "g",
            "Hunk@",
            diff_mode_to_dispatch(DiffMode::UnstagedAgainstCurrentBranch),
        ),
        Keybinding::new_undocumented(
            "G",
            "Hunk^",
            diff_mode_to_dispatch(DiffMode::UnstagedAgainstMainBranch),
        ),
    ])
    .collect_vec();

    let severity_to_dispatch = |severity| match scope {
        Scope::Local => Dispatch::ToEditor(SetSelectionModeWithPriorChange(
            if_current_not_found,
            Diagnostic(severity),
            prior_change,
        )),
        Scope::Global => Dispatch::SetQuickfixList(QuickfixListType::Diagnostic(severity)),
    };
    let diagnostics_keybindings = [
        Keybinding::new(
            "a",
            "All",
            doc_format!("space/lsp_all.md", { severity: "Info" }),
            severity_to_dispatch(DiagnosticSeverityRange::All),
        ),
        Keybinding::new(
            "s",
            "Error",
            doc_format!("space/lsp_severity.md", { severity: "Error" }),
            severity_to_dispatch(DiagnosticSeverityRange::Error),
        ),
        Keybinding::new(
            "q",
            "Hint",
            doc_format!("space/lsp_severity.md", { severity: "Hint" }),
            severity_to_dispatch(DiagnosticSeverityRange::Hint),
        ),
        Keybinding::new(
            "Q",
            "Info",
            doc_format!("space/lsp_severity.md", { severity: "Info" }),
            severity_to_dispatch(DiagnosticSeverityRange::Information),
        ),
        Keybinding::new(
            "w",
            "Warn",
            doc_format!("space/lsp_severity.md", { severity: "Warn" }),
            severity_to_dispatch(DiagnosticSeverityRange::Warning),
        ),
    ];

    let lsp_keybindings = [
        Keybinding::new(
            "x",
            "Def",
            doc_format!("Def.md"),
            Dispatch::RequestDefinitions(scope),
        ),
        Keybinding::new(
            "X",
            "Decl",
            doc_format!("Decl.md"),
            Dispatch::RequestDeclarations(scope),
        ),
        Keybinding::new(
            "b",
            "Impl",
            doc_format!("Impl.md"),
            Dispatch::RequestImplementations(scope),
        ),
        Keybinding::new(
            "v",
            "Ref-",
            doc_format!("Ref-.md"),
            Dispatch::RequestReferences {
                include_declaration: false,
                scope,
            },
        ),
        Keybinding::new(
            "V",
            "Ref+",
            doc_format!("Ref+.md"),
            Dispatch::RequestReferences {
                include_declaration: true,
                scope,
            },
        ),
        Keybinding::new(
            "c",
            "Type",
            doc_format!("Type.md"),
            Dispatch::RequestTypeDefinitions(scope),
        ),
        Keybinding::new(
            "z",
            "In Calls",
            doc_format!("In Calls.md"),
            Dispatch::RequestIncomingCalls(scope),
        ),
        Keybinding::new(
            "Z",
            "Out Calls",
            doc_format!("Out Calls.md"),
            Dispatch::RequestOutgoingCalls(scope),
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
                Keybinding::new_undocumented(key, description, dispatch)
            })
            .chain([
                Keybinding::new_undocumented(
                    "d",
                    "← Search",
                    Dispatch::OpenSearchPromptWithPriorChange {
                        scope: Scope::Local,
                        if_current_not_found: editor
                            .cursor_direction
                            .reverse()
                            .to_if_current_not_found(),
                        prior_change,
                    },
                ),
                Keybinding::new_undocumented(
                    "D",
                    "With",
                    Dispatch::OpenSearchPromptWithCurrentSelection {
                        scope: Scope::Local,
                        prior_change,
                    },
                ),
                Keybinding::new_undocumented(
                    "y",
                    "One",
                    Dispatch::ToEditor(FindOneChar(if_current_not_found)),
                ),
                Keybinding::new_undocumented(
                    "r",
                    "Repeat Search →",
                    Dispatch::ToEditor(DispatchEditor::RepeatSearch(
                        Scope::Local,
                        editor.cursor_direction.reverse().to_if_current_not_found(),
                        prior_change,
                    )),
                ),
            ])
            .chain(search_current_keymap(
                Scope::Local,
                editor.cursor_direction.reverse().to_if_current_not_found(),
            ))
            .collect_vec(),
        Scope::Global => [
            Keybinding::new_undocumented(
                "d",
                "Search",
                Dispatch::OpenSearchPrompt {
                    scope,
                    if_current_not_found,
                },
            ),
            Keybinding::new_undocumented(
                "D",
                "With",
                Dispatch::OpenSearchPromptWithCurrentSelection {
                    scope,
                    prior_change,
                },
            ),
            Keybinding::new_undocumented(
                "r",
                "Repeat Search",
                Dispatch::ToEditor(DispatchEditor::RepeatSearch(
                    Scope::Global,
                    IfCurrentNotFound::LookForward,
                    prior_change,
                )),
            ),
        ]
        .into_iter()
        .chain(search_current_keymap(scope, if_current_not_found))
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

pub fn normal_mode_keymap_legend_config(
    editor: &Editor,
    normal_mode_override: Option<NormalModeOverride>,
    prior_change: Option<PriorChange>,
) -> KeymapLegendConfig {
    KeymapLegendConfig {
        title: "Normal".to_string(),
        keymap: Keymap::new(
            &normal_mode_keymap(editor, normal_mode_override, prior_change)
                .into_iter()
                .chain(Some(Keybinding::new_undocumented(
                    "g",
                    "Extend",
                    Dispatch::ShowMenu(extend_mode_keymap_legend_config(editor)),
                )))
                .collect_vec(),
        ),
    }
}

fn search_current_keymap(scope: Scope, if_current_not_found: IfCurrentNotFound) -> Vec<Keybinding> {
    [
        Keybinding::new_undocumented(
            "f",
            "Search This",
            Dispatch::ToEditor(DispatchEditor::SearchCurrentSelection(
                if_current_not_found,
                scope,
            )),
        ),
        Keybinding::new_undocumented(
            "F",
            "Search Clipboard",
            Dispatch::ToEditor(DispatchEditor::SearchClipboardContent(scope)),
        ),
    ]
    .to_vec()
}
pub fn leader_keymap_legend_config() -> KeymapLegendConfig {
    KeymapLegendConfig {
        title: "Leader".to_string(),

        keymap: Keymap::new(
            &QWERTY_STR
                .iter()
                .flatten()
                .filter_map(|key| {
                    let (_, description, _) =
                        custom_keymap().into_iter().find(|(k, _, _)| k == key)?;
                    Some(Keybinding::new_dynamic(
                        key,
                        description,
                        Dispatch::ExecuteLeaderKey(key.to_string()),
                    ))
                })
                .collect_vec(),
        ),
    }
}
pub fn space_keymap_legend_config(editor: &Editor, context: &Context) -> KeymapLegendConfig {
    KeymapLegendConfig {
        title: "Space".to_string(),

        keymap: Keymap::new(
            &[
                Keybinding::new_undocumented("u", "÷ Selection", Dispatch::ToggleRevealSelections),
                Keybinding::new_undocumented(
                    "i",
                    "÷ Cursor",
                    Dispatch::ToEditor(DispatchEditor::ToggleReveal(Reveal::Cursor)),
                ),
                Keybinding::new_undocumented(
                    "o",
                    "÷ Mark",
                    Dispatch::ToEditor(DispatchEditor::ToggleReveal(Reveal::Mark)),
                ),
                Keybinding::new_undocumented(
                    "j",
                    "Editor",
                    Dispatch::ShowMenu(space_editor_keymap_legend_config()),
                ),
                Keybinding::new_undocumented(
                    "k",
                    "Pick",
                    Dispatch::ShowMenu(space_pick_keymap_legend_config()),
                ),
                Keybinding::new_undocumented(
                    "l",
                    "Context",
                    Dispatch::ShowMenu(space_context_keymap_legend_config(editor)),
                ),
                Keybinding::new_undocumented(
                    ";",
                    "Explorer",
                    Dispatch::RevealInExplorer(
                        editor
                            .path()
                            .unwrap_or_else(|| context.current_working_directory().clone()),
                    ),
                ),
                Keybinding::new_undocumented(
                    "/",
                    "Help",
                    Dispatch::ToEditor(DispatchEditor::ShowHelp),
                ),
            ]
            .into_iter()
            .chain(
                secondary_selection_modes_keymap_legend_config(
                    editor,
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
pub fn space_editor_keymap_legend_config() -> KeymapLegendConfig {
    KeymapLegendConfig {
        title: "Editor".to_string(),

        keymap: Keymap::new(&[
            Keybinding::new_undocumented(
                "x",
                "Replace all",
                Dispatch::Replace {
                    scope: Scope::Global,
                },
            ),
            Keybinding::new_undocumented(
                "enter",
                "Force Save",
                Dispatch::ToEditor(DispatchEditor::ForceSave),
            ),
            Keybinding::new_undocumented("c", "Save All", Dispatch::SaveAll),
            Keybinding::new_undocumented("q", "Quit No Save", Dispatch::QuitNoSave),
            Keybinding::new_undocumented("v", "Quit", Dispatch::SafeQuit),
            Keybinding::new_undocumented(
                "f",
                "Change Work Dir",
                Dispatch::OpenChangeWorkingDirectoryPrompt,
            ),
            Keybinding::new_undocumented(
                "d",
                "Reload File",
                Dispatch::ToEditor(ReloadFile { force: false }),
            ),
        ]),
    }
}
pub fn space_context_keymap_legend_config(editor: &Editor) -> KeymapLegendConfig {
    KeymapLegendConfig {
        title: "Context".to_string(),

        keymap: Keymap::new(&[
            Keybinding::new_undocumented("d", "Code Actions", {
                let cursor_char_index = editor.get_cursor_char_index();
                Dispatch::RequestCodeAction {
                    diagnostics: editor
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
            Keybinding::new_undocumented("s", "Hover", Dispatch::RequestHover),
            Keybinding::new_undocumented("f", "Rename", Dispatch::PrepareRename),
            Keybinding::new_undocumented(
                "g",
                "Revert Hunk@",
                Dispatch::ToEditor(DispatchEditor::RevertHunk(
                    DiffMode::UnstagedAgainstCurrentBranch,
                )),
            ),
            Keybinding::new_undocumented(
                "G",
                "Revert Hunk^",
                Dispatch::ToEditor(DispatchEditor::RevertHunk(
                    DiffMode::UnstagedAgainstMainBranch,
                )),
            ),
            Keybinding::new_undocumented(
                "b",
                "Git Blame",
                Dispatch::ToEditor(DispatchEditor::GitBlame),
            ),
            Keybinding::new_undocumented(
                "x",
                "Go to File",
                Dispatch::ToEditor(DispatchEditor::GoToFile),
            ),
            Keybinding::new_undocumented(
                "C",
                "Copy Absolute Path",
                Dispatch::ToEditor(DispatchEditor::CopyAbsolutePath),
            ),
            Keybinding::new_undocumented(
                "c",
                "Copy Relative Path",
                Dispatch::ToEditor(DispatchEditor::CopyRelativePath),
            ),
            Keybinding::new_undocumented(
                "t",
                "TS Node Sexp",
                Dispatch::ToEditor(DispatchEditor::ShowCurrentTreeSitterNodeSexp),
            ),
            Keybinding::new_undocumented("e", "Pipe", Dispatch::OpenPipeToShellPrompt),
        ]),
    }
}
pub fn space_pick_keymap_legend_config() -> KeymapLegendConfig {
    KeymapLegendConfig {
        title: "Pick".to_string(),

        keymap: Keymap::new(
            &[
                ("f", "Buffer", FilePickerKind::Opened),
                ("d", "File", FilePickerKind::NonGitIgnored),
            ]
            .into_iter()
            .map(|(key, description, kind)| {
                Keybinding::new_undocumented(key, description, Dispatch::OpenFilePicker(kind))
            })
            .chain([
                Keybinding::new_undocumented(
                    "g",
                    "Git status ^",
                    Dispatch::OpenFilePicker(FilePickerKind::GitStatus(
                        DiffMode::UnstagedAgainstCurrentBranch,
                    )),
                ),
                Keybinding::new_undocumented(
                    "G",
                    "Git status @",
                    Dispatch::OpenFilePicker(FilePickerKind::GitStatus(
                        DiffMode::UnstagedAgainstMainBranch,
                    )),
                ),
            ])
            .chain(Some(Keybinding::new_undocumented(
                "s",
                "Symbol (Document)",
                Dispatch::RequestDocumentSymbols,
            )))
            .chain(Some(Keybinding::new_undocumented(
                "S",
                "Symbol (Workspace)",
                Dispatch::OpenWorkspaceSymbolsPicker,
            )))
            .chain(Some(Keybinding::new_undocumented(
                "a",
                "Theme",
                Dispatch::OpenThemePicker,
            )))
            .chain(Some(Keybinding::new_undocumented(
                "t",
                "Quickfix",
                Dispatch::OpenQuickfixItemsPicker,
            )))
            .chain(Some(Keybinding::new_undocumented(
                "b",
                "Git Branch",
                Dispatch::OpenGitBranchPrompt,
            )))
            .collect_vec(),
        ),
    }
}
pub fn keymap_transform() -> Vec<Keybinding> {
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
        Keybinding::new_undocumented(
            keybinding,
            description,
            Dispatch::ToEditor(Transform(Transformation::Case(case))),
        )
    })
    .chain(Some(Keybinding::new_undocumented(
        "j",
        "Wrap",
        Dispatch::ToEditor(Transform(Transformation::Wrap)),
    )))
    .chain(Some(Keybinding::new_undocumented(
        "h",
        "Unwrap",
        Dispatch::ToEditor(Transform(Transformation::Unwrap)),
    )))
    .chain(Some(Keybinding::new_undocumented(
        "k",
        "Line Comment",
        Dispatch::ToEditor(DispatchEditor::ToggleLineComment),
    )))
    .chain(Some(Keybinding::new_undocumented(
        "l",
        "Block Comment",
        Dispatch::ToEditor(DispatchEditor::ToggleBlockComment),
    )))
    .collect_vec()
}
pub fn extend_mode_keymap_legend_config(editor: &Editor) -> KeymapLegendConfig {
    KeymapLegendConfig {
        title: "Extend".to_string(),
        keymap: Keymap::new(
            &normal_mode_keymap(editor, None, Some(PriorChange::EnableSelectionExtension))
                .into_iter()
                .chain(Some(Keybinding::new_undocumented(
                    "g",
                    "Select All",
                    Dispatch::ToEditor(SelectAll),
                )))
                .collect_vec(),
        ),
    }
}
pub fn normal_mode_keymap(
    editor: &Editor,
    normal_mode_override: Option<NormalModeOverride>,
    prior_change: Option<PriorChange>,
) -> Vec<Keybinding> {
    let normal_mode_override = normal_mode_override
        .clone()
        .or_else(|| editor.normal_mode_override.clone())
        .unwrap_or_default();
    keymap_core_movements(prior_change)
        .into_iter()
        .chain(keymap_sub_modes(editor))
        .chain(keymap_other_movements())
        .chain(keymap_primary_selection_modes(editor, prior_change))
        .chain(keymap_secondary_selection_modes_init(editor, prior_change))
        .chain(keymap_actions(&normal_mode_override, false, prior_change))
        .chain(keymap_others())
        .chain(keymap_universal())
        .collect_vec()
}

fn surround_keymap_legend_config() -> KeymapLegendConfig {
    KeymapLegendConfig {
        title: "Surround".to_string(),
        keymap: keymap_surround(),
    }
}

pub fn secondary_selection_modes_keymap_legend_config(
    editor: &Editor,
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

        keymap: Keymap::new(&secondary_selection_modes_keybindings(
            editor,
            scope,
            if_current_not_found,
            prior_change,
        )),
    }
}

pub fn keymap_surround() -> Keymap {
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
                .chain(Some(Keybinding::new_undocumented(
                    ";",
                    "<></>",
                    Dispatch::OpenSurroundXmlPrompt,
                )))
                .collect_vec(),
            ),
        }
    }

    fn change_surround_from_keymap_legend_config() -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Change Surround from:".to_string(),

            keymap: generate_enclosures_keymap(|enclosure| {
                Dispatch::ShowMenu(change_surround_to_keymap_legend_config(enclosure))
            }),
        }
    }

    fn change_surround_to_keymap_legend_config(from: EnclosureKind) -> KeymapLegendConfig {
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
    Keymap::new(&[
        Keybinding::new_undocumented(
            "v",
            "Delete Surround",
            Dispatch::ShowMenu(delete_surround_keymap_legend_config()),
        ),
        Keybinding::new_undocumented(
            "s",
            "Surround",
            Dispatch::ShowMenu(surround_keymap_legend_config()),
        ),
        Keybinding::new_undocumented(
            "f",
            "Change Surround",
            Dispatch::ShowMenu(change_surround_from_keymap_legend_config()),
        ),
        Keybinding::new_undocumented(
            "d",
            "Select Inside",
            Dispatch::ShowMenu(select_surround_keymap_legend_config(SurroundKind::Inside)),
        ),
        Keybinding::new_undocumented(
            "e",
            "Select Around",
            Dispatch::ShowMenu(select_surround_keymap_legend_config(SurroundKind::Around)),
        ),
    ])
}

pub fn multicursor_momentary_layer_keymap(editor: &Editor) -> Keymap {
    Keymap::new(
        &[
            Keybinding::new_undocumented(
                "i",
                "Add Curs ^",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::Up)),
            ),
            Keybinding::new_undocumented(
                "k",
                "Add Curs v",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::Down)),
            ),
            Keybinding::new_undocumented(
                "j",
                "<< Add Curs",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::Left)),
            ),
            Keybinding::new_undocumented(
                "l",
                "Add Curs >>",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::Right)),
            ),
            Keybinding::new_undocumented(
                "u",
                "< Add Curs",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::Previous)),
            ),
            Keybinding::new_undocumented(
                "o",
                "Add Curs >",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::Next)),
            ),
            Keybinding::new_undocumented(
                "y",
                "|< Add Curs",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::First)),
            ),
            Keybinding::new_undocumented(
                "p",
                "Add Curs >|",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::Last)),
            ),
        ]
        .into_iter()
        .chain([
            Keybinding::new_undocumented("n", "Delete Curs", Dispatch::DeleteCursor),
            Keybinding::new_undocumented("h", "← Curs", Dispatch::CycleCursor(Direction::Start)),
            Keybinding::new_undocumented(";", "Curs →", Dispatch::CycleCursor(Direction::End)),
            Keybinding::new_undocumented(
                "m",
                "Jump Add Curs",
                Dispatch::ToEditor(ShowJumps {
                    use_current_selection_mode: true,
                    prior_change: Some(PriorChange::EnterMultiCursorMode),
                }),
            ),
            Keybinding::new_undocumented(
                "space",
                "Open Multi-cursor Menu",
                Dispatch::ShowMenu(KeymapLegendConfig {
                    title: "Multi-cursor Menu".to_string(),
                    keymap: multicursor_menu_keymap(editor),
                }),
            ),
        ])
        .collect_vec(),
    )
}
pub fn keymap_sub_modes(editor: &Editor) -> Vec<Keybinding> {
    [
        Some(Keybinding::new_undocumented(
            "t",
            "≡ Swap",
            Dispatch::ShowJointMomentaryLayer {
                swap_key: key!("space"),
                active_config: KeymapLegendConfig {
                    title: "≡ Swap".to_string(),
                    keymap: swap_keymap(),
                },
                release_key: ReleaseKey::new("t", None),
                inactive_config: KeymapLegendConfig {
                    title: "≡ Eat".to_string(),
                    keymap: eat_keymap(),
                },
                inactive_tap: None,
            },
        )),
        Some(Keybinding::new_undocumented(
            "backslash",
            "Leader",
            Dispatch::ShowMenu(leader_keymap_legend_config()),
        )),
        Some(Keybinding::momentary_layer(MomentaryLayer {
            key: "r",
            name: "≡ Multi-cursor".to_string(),
            config: KeymapLegendConfig {
                title: "≡ Multi-cursor".to_string(),
                keymap: multicursor_momentary_layer_keymap(editor),
            },
            on_tap: None,
        })),
    ]
    .into_iter()
    .flatten()
    .collect_vec()
}

pub fn keymap_overridable(
    normal_mode_override: &NormalModeOverride,
    none_if_no_override: bool,
) -> Vec<Keybinding> {
    keymap_actions_overridable(normal_mode_override, none_if_no_override)
        .into_iter()
        .chain(keymap_clipboard_related_actions_overridable(
            normal_mode_override.clone(),
            none_if_no_override,
        ))
        .collect_vec()
}

fn keymap_clipboard_related_actions(normal_mode_override: NormalModeOverride) -> Vec<Keybinding> {
    [
        Keybinding::new_undocumented("F", "Change X", Dispatch::ToEditor(ChangeCut)),
        Keybinding::momentary_layer(MomentaryLayer {
            key: "c",
            name: "≡ Copy".to_string(),
            config: KeymapLegendConfig {
                title: "≡ Copy".to_string(),
                keymap: duplicate_keymap(),
            },
            on_tap: Some(OnTap::new("Copy", Dispatch::ToEditor(Copy))),
        }),
    ]
    .into_iter()
    .chain(keymap_clipboard_related_actions_overridable(
        normal_mode_override,
        false,
    ))
    .collect_vec()
}

pub fn keymap_core_movements(prior_change: Option<PriorChange>) -> Vec<Keybinding> {
    [
        Keybinding::new(
            "j",
            "<<",
            doc_format!("Left.md"),
            Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Left, prior_change)),
        ),
        Keybinding::new_undocumented(
            "l",
            ">>",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Right, prior_change)),
        ),
        Keybinding::new_undocumented(
            "i",
            "^",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Up, prior_change)),
        ),
        Keybinding::new_undocumented(
            "k",
            "v",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Down, prior_change)),
        ),
        Keybinding::new_undocumented(
            "y",
            "|<",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::First, prior_change)),
        ),
        Keybinding::new_undocumented(
            "p",
            ">|",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Last, prior_change)),
        ),
        Keybinding::new_undocumented(
            "o",
            ">",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Next, prior_change)),
        ),
        Keybinding::new_undocumented(
            "u",
            "<",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(
                Movement::Previous,
                prior_change,
            )),
        ),
        Keybinding::new_undocumented(
            "m",
            "Jump",
            Dispatch::ToEditor(DispatchEditor::ShowJumps {
                use_current_selection_mode: true,
                prior_change,
            }),
        ),
        Keybinding::new_undocumented("M", "Index", Dispatch::OpenMoveToIndexPrompt(prior_change)),
        Keybinding::new_undocumented(
            ".",
            "Parent Line",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(
                Movement::ParentLine,
                prior_change,
            )),
        ),
    ]
    .to_vec()
}

pub fn keymap_others() -> Vec<Keybinding> {
    [
        Keybinding::new_undocumented(
            "space",
            "Space",
            Dispatch::ToEditor(DispatchEditor::PressSpace),
        ),
        Keybinding::new_undocumented(
            ",",
            "Surround",
            Dispatch::ShowMenu(surround_keymap_legend_config()),
        ),
        Keybinding::new_undocumented(
            "esc",
            "Remain only this window",
            Dispatch::ToEditor(DispatchEditor::HandleEsc),
        ),
    ]
    .to_vec()
}

pub fn keymap_primary_selection_modes(
    editor: &Editor,
    prior_change: Option<PriorChange>,
) -> Vec<Keybinding> {
    let direction = editor.cursor_direction.reverse().to_if_current_not_found();
    let selection_mode_to_dispatch = |selection_mode| {
        Dispatch::ToEditor(SetSelectionModeWithPriorChange(
            direction,
            selection_mode,
            prior_change,
        ))
    };

    [
        Keybinding::new_undocumented("a", "LINE", selection_mode_to_dispatch(Line)),
        Keybinding::new_undocumented("A", "LINE*", selection_mode_to_dispatch(LineFull)),
        Keybinding::new_undocumented("d", "NODE", selection_mode_to_dispatch(SyntaxNode)),
        Keybinding::new_undocumented("D", "NODE*", selection_mode_to_dispatch(SyntaxNodeFine)),
        Keybinding::new_undocumented("s", "WORD", selection_mode_to_dispatch(Word)),
        Keybinding::new_undocumented("S", "WORD*", selection_mode_to_dispatch(BigWord)),
        Keybinding::new_undocumented("w", "SUBWORD", selection_mode_to_dispatch(Subword)),
        Keybinding::new_undocumented("W", "CHAR", selection_mode_to_dispatch(Character)),
        Keybinding::new_undocumented("E", "PARAGRAPH", selection_mode_to_dispatch(Paragraph)),
    ]
    .into()
}

pub fn keymap_secondary_selection_modes_init(
    editor: &Editor,
    prior_change: Option<PriorChange>,
) -> Vec<Keybinding> {
    [Keybinding::new_undocumented(
        "n",
        "⚲ Local",
        Dispatch::ShowMenu(secondary_selection_modes_keymap_legend_config(
            editor,
            Scope::Local,
            editor.cursor_direction.reverse().to_if_current_not_found(),
            prior_change,
        )),
    )]
    .to_vec()
}

fn keymap_clipboard_related_actions_overridable(
    normal_mode_override: NormalModeOverride,
    none_if_no_override: bool,
) -> Vec<Keybinding> {
    [Keybinding::momentary_layer(MomentaryLayer {
        key: "b",
        name: "≡ Paste".to_string(),
        config: KeymapLegendConfig {
            title: "≡ Paste".to_string(),
            keymap: paste_keymap(),
        },
        on_tap: Some(OnTap::new(
            "Replace",
            Dispatch::ToEditor(DispatchEditor::ReplaceWithCopiedText { cut: false }),
        )),
    })
    .override_keymap(
        normal_mode_override.paste.clone().as_ref(),
        none_if_no_override,
    )]
    .into_iter()
    .flatten()
    .collect_vec()
}

pub fn keymap_universal() -> Vec<Keybinding> {
    [
        Keybinding::new_undocumented(
            "alt+;",
            "⇋ Align View",
            Dispatch::ToEditor(SwitchViewAlignment),
        ),
        Keybinding::new_undocumented("alt+/", "⇋ Window", Dispatch::OtherWindow),
        #[cfg(unix)]
        Keybinding::new_undocumented("ctrl+z", "Suspend", Dispatch::Suspend),
    ]
    .to_vec()
}

pub fn insert_mode_keymap_legend_config(include_universal_keymap: bool) -> KeymapLegendConfig {
    KeymapLegendConfig {
        title: "Insert mode keymap".to_string(),
        keymap: Keymap::new(
            &[
                Keybinding::new_undocumented(
                    "left",
                    "Char ←",
                    Dispatch::ToEditor(MoveCharacterBack),
                ),
                Keybinding::new_undocumented(
                    "right",
                    "Char →",
                    Dispatch::ToEditor(MoveCharacterForward),
                ),
                Keybinding::new_undocumented(
                    "alt+y",
                    "Line ←",
                    Dispatch::ToEditor(MoveToLineStart),
                ),
                Keybinding::new_undocumented("alt+p", "Line →", Dispatch::ToEditor(MoveToLineEnd)),
                Keybinding::new_undocumented(
                    "alt+backspace",
                    "Delete Word ←",
                    Dispatch::ToEditor(DeleteWordBackward { short: true }),
                ),
                Keybinding::new_undocumented(
                    "esc",
                    "Enter normal mode",
                    Dispatch::ToEditor(EnterNormalMode),
                ),
                Keybinding::new_undocumented(
                    "backspace",
                    "Delete character backward",
                    Dispatch::ToEditor(Backspace),
                ),
                Keybinding::new_undocumented(
                    "enter",
                    "Enter new line",
                    Dispatch::ToEditor(EnterNewline),
                ),
                Keybinding::new_undocumented(
                    "tab",
                    "Enter tab",
                    Dispatch::ToEditor(Insert("\t".to_string())),
                ),
                Keybinding::new_undocumented(
                    "home",
                    "Move to line start",
                    Dispatch::ToEditor(MoveToLineStart),
                ),
                Keybinding::new_undocumented(
                    "end",
                    "Move to line end",
                    Dispatch::ToEditor(MoveToLineEnd),
                ),
            ]
            .into_iter()
            .chain(if include_universal_keymap {
                keymap_universal()
            } else {
                Vec::default()
            })
            .chain([Keybinding::momentary_layer(MomentaryLayer {
                key: "alt+e",
                name: "≡ Buffer".to_string(),
                config: KeymapLegendConfig {
                    title: "≡ Buffer".to_string(),
                    keymap: buffer_keymap(true),
                },
                on_tap: Some(OnTap::new(
                    "Toggle Selection Mark",
                    Dispatch::ToggleSelectionMark,
                )),
            })])
            .chain([Keybinding::momentary_layer(MomentaryLayer {
                key: "alt+v",
                name: "Delete".to_string(),
                config: KeymapLegendConfig {
                    title: "Delete".to_string(),
                    keymap: insert_mode_delete_keymap(),
                },
                on_tap: None,
            })])
            .collect_vec(),
        ),
    }
}

pub fn insert_mode_delete_keymap() -> Keymap {
    Keymap::new(
        [
            Keybinding::new_undocumented(
                "alt+y",
                "Kill Line ←",
                Dispatch::ToEditor(KillLine(Direction::Start)),
            ),
            Keybinding::new_undocumented(
                "alt+p",
                "Kill Line →",
                Dispatch::ToEditor(KillLine(Direction::End)),
            ),
            Keybinding::new_undocumented(
                "alt+j",
                "← Delete Word",
                Dispatch::ToEditor(DeleteWord {
                    short: false,
                    direction: Direction::Start,
                }),
            ),
            Keybinding::new_undocumented(
                "alt+l",
                "Delete Word →",
                Dispatch::ToEditor(DeleteWord {
                    short: false,
                    direction: Direction::End,
                }),
            ),
            Keybinding::new_undocumented(
                "alt+u",
                "← Delete Subword",
                Dispatch::ToEditor(DeleteWord {
                    short: true,
                    direction: Direction::Start,
                }),
            ),
            Keybinding::new_undocumented(
                "alt+o",
                "Delete Subword →",
                Dispatch::ToEditor(DeleteWord {
                    short: true,
                    direction: Direction::End,
                }),
            ),
        ]
        .as_ref(),
    )
}

pub fn keymap_actions(
    normal_mode_override: &NormalModeOverride,
    none_if_no_override: bool,
    _prior_change: Option<PriorChange>,
) -> Vec<Keybinding> {
    [
        Keybinding::new_undocumented("I", "Join", Dispatch::ToEditor(JoinSelection)),
        Keybinding::new_undocumented("K", "Break", Dispatch::ToEditor(BreakSelection)),
        Keybinding::new_undocumented(
            "Y",
            "← Align",
            Dispatch::ToEditor(AlignSelections(Direction::Start)),
        ),
        Keybinding::new_undocumented(
            "P",
            "Align →",
            Dispatch::ToEditor(AlignSelections(Direction::End)),
        ),
        Keybinding::momentary_layer(MomentaryLayer {
            key: "z",
            name: "≡ Undo/Redo".to_string(),
            config: KeymapLegendConfig {
                title: "≡ Undo/Redo".to_string(),
                keymap: undo_redo_keymap(),
            },
            on_tap: Some(OnTap::new("Coarse Undo", Dispatch::ToEditor(CoarseUndo))),
        }),
        Keybinding::new_undocumented("enter", "Save", Dispatch::SaveFile),
        Keybinding::new_undocumented("shift+enter", "Save As", Dispatch::OpenSaveAsPrompt),
        Keybinding::new_undocumented(
            "G",
            "Transform",
            Dispatch::ShowMenu(transform_keymap_legend_config()),
        ),
        Keybinding::new_undocumented("L", "Indent", Dispatch::ToEditor(Indent)),
        Keybinding::new_undocumented("J", "Dedent", Dispatch::ToEditor(Dedent)),
        Keybinding::new_undocumented("*", "Keyboard", Dispatch::OpenKeyboardLayoutPrompt),
        Keybinding::new(
            "Z",
            "Coarse Redo",
            doc_format!("Coarse Redo.md"),
            Dispatch::ToEditor(CoarseRedo),
        ),
    ]
    .into_iter()
    .chain(keymap_actions_overridable(
        normal_mode_override,
        none_if_no_override,
    ))
    .chain(keymap_clipboard_related_actions(
        normal_mode_override.clone(),
    ))
    .collect_vec()
}

pub fn undo_redo_keymap() -> Keymap {
    Keymap::new(&[
        Keybinding::new(
            "j",
            "Coarse Undo",
            doc_format!("Coarse Undo.md"),
            Dispatch::ToEditor(CoarseUndo),
        ),
        Keybinding::new(
            "l",
            "Coarse Redo",
            doc_format!("Coarse Redo.md"),
            Dispatch::ToEditor(CoarseRedo),
        ),
        Keybinding::new(
            "u",
            "Fine Undo",
            doc_format!("Fine Undo.md"),
            Dispatch::ToEditor(FineUndo),
        ),
        Keybinding::new(
            "o",
            "Fine Redo",
            doc_format!("Fine Redo.md"),
            Dispatch::ToEditor(FineRedo),
        ),
    ])
}

pub fn keymap_actions_overridable(
    normal_mode_override: &NormalModeOverride,
    none_if_no_override: bool,
) -> Vec<Keybinding> {
    [
        Keybinding::new_undocumented("f", "Change", Dispatch::ToEditor(DispatchEditor::Change))
            .override_keymap(normal_mode_override.change.as_ref(), none_if_no_override),
        Keybinding::momentary_layer(MomentaryLayer {
            key: "x",
            name: "≡ Open".to_string(),
            config: KeymapLegendConfig {
                title: "≡ Open".to_string(),
                keymap: open_keymap(),
            },
            on_tap: None,
        })
        .override_keymap(normal_mode_override.cut.as_ref(), none_if_no_override),
        Keybinding::new_undocumented(
            "v",
            "≡ Delete",
            Dispatch::ShowJointMomentaryLayer {
                swap_key: key!("space"),
                active_config: KeymapLegendConfig {
                    title: "≡ Delete".to_string(),
                    keymap: delete_keymap(),
                },
                release_key: ReleaseKey::new(
                    "v",
                    Some(OnTap::new(
                        "Delete One",
                        Dispatch::ToEditor(DispatchEditor::DeleteOne),
                    )),
                ),
                inactive_config: KeymapLegendConfig {
                    title: "≡ Cut".to_string(),
                    keymap: cut_keymap(),
                },
                inactive_tap: Some(OnTap::new(
                    "Cut One",
                    Dispatch::ToEditor(DispatchEditor::CutOne),
                )),
            },
        )
        .override_keymap(normal_mode_override.delete.as_ref(), none_if_no_override),
        Keybinding::new_undocumented(
            "h",
            "← Insert",
            Dispatch::ToEditor(EnterInsertMode(Direction::Start)),
        )
        .override_keymap(normal_mode_override.insert.as_ref(), none_if_no_override),
        Keybinding::new_undocumented(
            ";",
            "Insert →",
            Dispatch::ToEditor(EnterInsertMode(Direction::End)),
        )
        .override_keymap(normal_mode_override.append.as_ref(), none_if_no_override),
    ]
    .into_iter()
    .flatten()
    .collect_vec()
}
pub fn keymap_other_movements() -> Vec<Keybinding> {
    [
        Keybinding::new_undocumented("alt+k", "Scroll ↓", Dispatch::ToEditor(ScrollPageDown)),
        Keybinding::new_undocumented("alt+i", "Scroll ↑", Dispatch::ToEditor(ScrollPageUp)),
        Keybinding::app_momentary_layer(MomentaryLayer {
            key: "q",
            name: "≡ Move Hist".to_string(),
            config: KeymapLegendConfig {
                title: "≡ Move Hist".to_string(),
                keymap: movement_history_keymap(),
            },
            on_tap: None,
        }),
        Keybinding::app_momentary_layer(MomentaryLayer {
            key: "e",
            name: "≡ Buffer".to_string(),
            config: KeymapLegendConfig {
                title: "≡ Buffer".to_string(),
                keymap: buffer_keymap(false),
            },
            on_tap: Some(OnTap::new(
                "Toggle Selection Mark",
                Dispatch::ToggleSelectionMark,
            )),
        }),
        Keybinding::new_undocumented("?", "⇋ Anchor", Dispatch::ToEditor(SwapExtensionAnchor)),
        Keybinding::new_undocumented(
            "/",
            "⇋ Curs",
            Dispatch::ToEditor(DispatchEditor::SwapCursor),
        ),
    ]
    .into_iter()
    .collect_vec()
}

pub fn swap_keymap() -> Keymap {
    Keymap::new(&[
        Keybinding::new_undocumented(
            "i",
            "Swap ^",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::Up)),
        ),
        Keybinding::new_undocumented(
            "j",
            "<< Swap",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::Left)),
        ),
        Keybinding::new_undocumented(
            "l",
            "Swap >>",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::Right)),
        ),
        Keybinding::new_undocumented(
            "k",
            "Swap v",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::Down)),
        ),
        Keybinding::new_undocumented(
            "u",
            "< Swap",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::Previous)),
        ),
        Keybinding::new_undocumented(
            "y",
            "|< Swap",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::First)),
        ),
        Keybinding::new_undocumented(
            "p",
            "Swap >|",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::Last)),
        ),
        Keybinding::new_undocumented(
            "o",
            "Swap >",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::Next)),
        ),
        Keybinding::new_undocumented(
            "m",
            "Jump Swap",
            Dispatch::ToEditor(ShowJumps {
                use_current_selection_mode: true,
                prior_change: Some(PriorChange::EnterSwapMode),
            }),
        ),
    ])
}

pub fn eat_keymap() -> Keymap {
    Keymap::new(&[
        Keybinding::new(
            "i",
            "Eat ^",
            doc_format!("eat/movement.md", { movement: "^", old: "foo bar\n[bar] baz", new: "[bar] baz" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::Up)),
        ),
        Keybinding::new(
            "j",
            "<< Eat",
            doc_format!("eat/movement.md", { movement: "<<", old: "foo / [bar]", new: "[bar]" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::Left)),
        ),
        Keybinding::new(
            "l",
            "Eat >>",
            doc_format!("eat/movement.md", { movement: ">>", old: "[foo] / bar", new: "[foo]" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::Right)),
        ),
        Keybinding::new(
            "k",
            "Eat v",
            doc_format!("eat/movement.md", { movement: "v", old: "[foo] bar\nbar baz", new: "[foo] baz" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::Down)),
        ),
        Keybinding::new(
            "u",
            "< Eat",
            doc_format!("eat/movement.md", { movement: "<", old: "foo / [bar]", new: "foo [bar]" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::Previous)),
        ),
        Keybinding::new(
            "y",
            "|< Eat",
            doc_format!("eat/movement.md", { movement: "|<", old: "foo bar [baz]", new: "[baz]" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::First)),
        ),
        Keybinding::new(
            "p",
            "Eat >|",
            doc_format!("eat/movement.md", { movement: ">|", old: "[foo] bar baz", new: "[foo]" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::Last)),
        ),
        Keybinding::new(
            "o",
            "Eat >",
            doc_format!("eat/movement.md", { movement: ">", old: "[foo] / bar", new: "[foo] bar" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::Next)),
        ),
    ])
}

pub fn paste_keymap() -> Keymap {
    Keymap::new(
        [
            Keybinding::new_undocumented(
                "j",
                "<< Gap Paste",
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::Left)),
            ),
            Keybinding::new_undocumented(
                "l",
                "Gap Paste >>",
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::Right)),
            ),
            Keybinding::new_undocumented(
                "o",
                "Gap Paste >",
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::Next)),
            ),
            Keybinding::new_undocumented(
                "u",
                "< Gap Paste",
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::Previous)),
            ),
            Keybinding::new_undocumented(
                ";",
                "Paste >",
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::AfterWithoutGap)),
            ),
            Keybinding::new_undocumented(
                "h",
                "< Paste",
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::BeforeWithoutGap)),
            ),
            Keybinding::new_undocumented(
                "m",
                "Replace w/ pattern",
                Dispatch::ToEditor(ReplaceWithPattern),
            ),
            Keybinding::new_undocumented(
                "y",
                "← Replace History",
                Dispatch::ToEditor(ReplaceWithPreviousCopiedText),
            ),
            Keybinding::new_undocumented(
                "p",
                "Replace History →",
                Dispatch::ToEditor(ReplaceWithNextCopiedText),
            ),
            Keybinding::new_undocumented(
                "i",
                "Paste ^",
                Dispatch::ToEditor(PasteVertically(Direction::Start)),
            ),
            Keybinding::new_undocumented(
                "k",
                "Paste v",
                Dispatch::ToEditor(PasteVertically(Direction::End)),
            ),
            Keybinding::new_undocumented(
                "n",
                "Replace Cut",
                Dispatch::ToEditor(ReplaceWithCopiedText { cut: true }),
            ),
        ]
        .as_ref(),
    )
}

pub fn duplicate_keymap() -> Keymap {
    Keymap::new(
        [
            Keybinding::new_undocumented(
                "j",
                "<< Gap Dup",
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::Left)),
            ),
            Keybinding::new_undocumented(
                "l",
                "Gap Dup >>",
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::Right)),
            ),
            Keybinding::new_undocumented(
                "o",
                "Gap Dup >",
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::Next)),
            ),
            Keybinding::new_undocumented(
                "u",
                "< Gap Dup",
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::Previous)),
            ),
            Keybinding::new_undocumented(
                ";",
                "Dup >",
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::AfterWithoutGap)),
            ),
            Keybinding::new_undocumented(
                "h",
                "< Dup",
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::BeforeWithoutGap)),
            ),
            Keybinding::new_undocumented(
                "i",
                "Dup ^",
                Dispatch::ToEditor(DuplicateVertically(Direction::Start)),
            ),
            Keybinding::new_undocumented(
                "k",
                "Dup v",
                Dispatch::ToEditor(DuplicateVertically(Direction::End)),
            ),
        ]
        .as_ref(),
    )
}

pub fn cut_keymap() -> Keymap {
    Keymap::new(&[
        Keybinding::new_undocumented(
            "j",
            "<< Cut",
            Dispatch::ToEditor(CutWithMovement(Movement::Left)),
        ),
        Keybinding::new_undocumented(
            "l",
            "Cut >>",
            Dispatch::ToEditor(CutWithMovement(Movement::Right)),
        ),
        Keybinding::new_undocumented(
            "u",
            "< Cut",
            Dispatch::ToEditor(CutWithMovement(Movement::Previous)),
        ),
        Keybinding::new_undocumented(
            "o",
            "Cut >",
            Dispatch::ToEditor(CutWithMovement(Movement::Next)),
        ),
        Keybinding::new_undocumented(
            "y",
            "|< Cut",
            Dispatch::ToEditor(CutWithMovement(Movement::First)),
        ),
        Keybinding::new_undocumented(
            "p",
            "Cut >|",
            Dispatch::ToEditor(CutWithMovement(Movement::Last)),
        ),
    ])
}

pub fn buffer_keymap(is_alted: bool) -> Keymap {
    Keymap::new(
        &[
            Keybinding::new_undocumented(
                possibly_alted("j", is_alted),
                "<< Marked File",
                Dispatch::CycleMarkedFile(Movement::Left),
            ),
            Keybinding::new_undocumented(
                possibly_alted("l", is_alted),
                "Marked File >>",
                Dispatch::CycleMarkedFile(Movement::Right),
            ),
            Keybinding::new_undocumented(
                possibly_alted("y", is_alted),
                "|< Marked File",
                Dispatch::CycleMarkedFile(Movement::First),
            ),
            Keybinding::new_undocumented(
                possibly_alted("p", is_alted),
                "Marked File >|",
                Dispatch::CycleMarkedFile(Movement::Last),
            ),
            Keybinding::new_undocumented(
                possibly_alted("u", is_alted),
                "Marked File >",
                Dispatch::CycleMarkedFile(Movement::Previous),
            ),
            Keybinding::new_undocumented(
                possibly_alted("o", is_alted),
                "< Marked File",
                Dispatch::CycleMarkedFile(Movement::Next),
            ),
        ]
        .into_iter()
        .chain(Some(Keybinding::new_undocumented(
            possibly_alted("k", is_alted),
            "Mark File",
            Dispatch::ToggleFileMark,
        )))
        .chain(Some(Keybinding::new_undocumented(
            possibly_alted("n", is_alted),
            "Close",
            Dispatch::CloseCurrentWindow,
        )))
        .chain(Some(Keybinding::new_undocumented(
            possibly_alted("i", is_alted),
            "Unmark Others",
            Dispatch::UnmarkAllOthers,
        )))
        .chain(Some(Keybinding::new_undocumented(
            possibly_alted("m", is_alted),
            "Alternate",
            Dispatch::OpenAlternateFile,
        )))
        .collect_vec(),
    )
}

pub fn movement_history_keymap() -> Keymap {
    // movement_history_keymap should work in Insert Mode
    // as well: hold-{alt+q} {u,o,j,l}
    Keymap::new(&[
        Keybinding::new_undocumented(
            "j",
            "<< Move Hist",
            Dispatch::MovementHistoryNavigation(Movement::Left),
        ),
        Keybinding::new_undocumented(
            "l",
            "Move Hist >>",
            Dispatch::MovementHistoryNavigation(Movement::Right),
        ),
        Keybinding::new_undocumented(
            "u",
            "< Move Hist",
            Dispatch::MovementHistoryNavigation(Movement::Previous),
        ),
        Keybinding::new_undocumented(
            "o",
            "Move Hist >",
            Dispatch::MovementHistoryNavigation(Movement::Next),
        ),
    ])
}

pub fn delete_keymap() -> Keymap {
    Keymap::new(&[
        Keybinding::new_undocumented(
            "j",
            "<< Delete",
            Dispatch::ToEditor(DeleteWithMovement(Movement::Left)),
        ),
        Keybinding::new_undocumented(
            "l",
            "Delete >>",
            Dispatch::ToEditor(DeleteWithMovement(Movement::Right)),
        ),
        Keybinding::new_undocumented(
            "u",
            "< Delete",
            Dispatch::ToEditor(DeleteWithMovement(Movement::Previous)),
        ),
        Keybinding::new_undocumented(
            "o",
            "Delete >",
            Dispatch::ToEditor(DeleteWithMovement(Movement::Next)),
        ),
        Keybinding::new_undocumented(
            "y",
            "|< Delete",
            Dispatch::ToEditor(DeleteWithMovement(Movement::First)),
        ),
        Keybinding::new_undocumented(
            "p",
            "Delete >|",
            Dispatch::ToEditor(DeleteWithMovement(Movement::Last)),
        ),
    ])
}

pub fn open_keymap() -> Keymap {
    Keymap::new(
        &[
            Keybinding::new_undocumented(
                "j",
                "<< Open",
                Dispatch::ToEditor(DispatchEditor::Open(GetGapMovement::Left)),
            ),
            Keybinding::new_undocumented(
                "l",
                "Open >>",
                Dispatch::ToEditor(DispatchEditor::Open(GetGapMovement::Right)),
            ),
            Keybinding::new_undocumented(
                "u",
                "< Open",
                Dispatch::ToEditor(DispatchEditor::Open(GetGapMovement::Previous)),
            ),
            Keybinding::new_undocumented(
                "o",
                "Open >",
                Dispatch::ToEditor(DispatchEditor::Open(GetGapMovement::Next)),
            ),
            Keybinding::new_undocumented(
                "h",
                "< Insert",
                Dispatch::ToEditor(DispatchEditor::Open(GetGapMovement::BeforeWithoutGap)),
            ),
            Keybinding::new_undocumented(
                ";",
                "Insert >",
                Dispatch::ToEditor(DispatchEditor::Open(GetGapMovement::AfterWithoutGap)),
            ),
        ]
        .into_iter()
        .chain([
            Keybinding::new_undocumented(
                "i",
                "Open ^",
                Dispatch::ToEditor(OpenVertically(Direction::Start)),
            ),
            Keybinding::new_undocumented(
                "k",
                "Open v",
                Dispatch::ToEditor(OpenVertically(Direction::End)),
            ),
        ])
        .collect_vec(),
    )
}
