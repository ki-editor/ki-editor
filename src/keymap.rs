use crate::{
    app::{Dispatch, FilePickerKind, HistoryNavigationMovement, Scope},
    components::{
        editor::{
            Direction, DispatchEditor, Editor, IfCurrentNotFound, Movement, PriorChange, Reveal,
            SurroundKind,
        },
        editor_keymap::{possibly_alted, QWERTY_EVENT, QWERTY_STR},
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
        Keybinding::new_undocumented(key!("m"), "( )", get_dispatch(EnclosureKind::Parentheses)),
        Keybinding::new_undocumented(
            key!(","),
            "[ ]",
            get_dispatch(EnclosureKind::SquareBrackets),
        ),
        Keybinding::new_undocumented(key!("."), "{ }", get_dispatch(EnclosureKind::CurlyBraces)),
        Keybinding::new_undocumented(
            key!("/"),
            "< >",
            get_dispatch(EnclosureKind::AngularBrackets),
        ),
        Keybinding::new_undocumented(key!("j"), "' '", get_dispatch(EnclosureKind::SingleQuotes)),
        Keybinding::new_undocumented(
            key!("k"),
            "\" \"",
            get_dispatch(EnclosureKind::DoubleQuotes),
        ),
        Keybinding::new_undocumented(key!("l"), "` `", get_dispatch(EnclosureKind::Backticks)),
    ])
}

pub fn multicursor_menu_keymap(editor: &Editor) -> Keymap {
    let primary_selection_mode_keybindings =
        keymap_primary_selection_modes(editor, Some(PriorChange::EnterMultiCursorMode));
    let secondary_selection_mode_keybindings =
        keymap_secondary_selection_modes_init(editor, Some(PriorChange::EnterMultiCursorMode));
    let other_keybindings = [
        Keybinding::new_undocumented(key!("j"), "Curs All", Dispatch::AddCursorToAllSelections),
        Keybinding::new_undocumented(
            key!("i"),
            "Keep Match",
            Dispatch::OpenFilterSelectionsPrompt { maintain: true },
        ),
        Keybinding::new_undocumented(
            key!("k"),
            "Remove Match",
            Dispatch::OpenFilterSelectionsPrompt { maintain: false },
        ),
        Keybinding::new_undocumented(
            key!("l"),
            "Keep Primary Curs",
            Dispatch::KeepCursorPrimaryOnly,
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
                    key!("n"),
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
            key!("e"),
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
            key!("t"),
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
            key!("g"),
            "Hunk@",
            diff_mode_to_dispatch(DiffMode::UnstagedAgainstCurrentBranch),
        ),
        Keybinding::new_undocumented(
            key!("G"),
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
            key!("a"),
            "All",
            doc_format!("space/lsp_all.md"),
            severity_to_dispatch(DiagnosticSeverityRange::All),
        ),
        Keybinding::new(
            key!("s"),
            "Error",
            doc_format!("space/lsp_severity.md", { severity: "Error" }),
            severity_to_dispatch(DiagnosticSeverityRange::Error),
        ),
        Keybinding::new(
            key!("q"),
            "Hint",
            doc_format!("space/lsp_severity.md", { severity: "Hint" }),
            severity_to_dispatch(DiagnosticSeverityRange::Hint),
        ),
        Keybinding::new(
            key!("Q"),
            "Info",
            doc_format!("space/lsp_severity.md", { severity: "Info" }),
            severity_to_dispatch(DiagnosticSeverityRange::Information),
        ),
        Keybinding::new(
            key!("w"),
            "Warn",
            doc_format!("space/lsp_severity.md", { severity: "Warn" }),
            severity_to_dispatch(DiagnosticSeverityRange::Warning),
        ),
    ];

    let lsp_keybindings = [
        Keybinding::new(
            key!("x"),
            "Def",
            doc_format!("Def.md"),
            Dispatch::RequestDefinitions(scope),
        ),
        Keybinding::new(
            key!("X"),
            "Decl",
            doc_format!("Decl.md"),
            Dispatch::RequestDeclarations(scope),
        ),
        Keybinding::new(
            key!("b"),
            "Impl",
            doc_format!("Impl.md"),
            Dispatch::RequestImplementations(scope),
        ),
        Keybinding::new(
            key!("v"),
            "Ref-",
            doc_format!("Ref-.md"),
            Dispatch::RequestReferences {
                include_declaration: false,
                scope,
            },
        ),
        Keybinding::new(
            key!("V"),
            "Ref+",
            doc_format!("Ref+.md"),
            Dispatch::RequestReferences {
                include_declaration: true,
                scope,
            },
        ),
        Keybinding::new(
            key!("c"),
            "Type",
            doc_format!("Type.md"),
            Dispatch::RequestTypeDefinitions(scope),
        ),
        Keybinding::new(
            key!("z"),
            "In Calls",
            doc_format!("In Calls.md"),
            Dispatch::RequestIncomingCalls(scope),
        ),
        Keybinding::new(
            key!("Z"),
            "Out Calls",
            doc_format!("Out Calls.md"),
            Dispatch::RequestOutgoingCalls(scope),
        ),
    ];
    let scope_specific_keybindings = match scope {
        Scope::Local => [{
            let search = Search {
                search: r"\d+".to_string(),
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
            Keybinding::new_undocumented(key!("Y"), "Int", dispatch)
        }]
        .into_iter()
        .chain([
            Keybinding::new_undocumented(
                key!("d"),
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
                key!("D"),
                "With",
                Dispatch::OpenSearchPromptWithCurrentSelection {
                    scope: Scope::Local,
                    prior_change,
                },
            ),
            Keybinding::new_undocumented(
                key!("y"),
                "One",
                Dispatch::ToEditor(FindOneChar(if_current_not_found)),
            ),
            Keybinding::new_undocumented(
                key!("r"),
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
                key!("d"),
                "Search",
                Dispatch::OpenSearchPrompt {
                    scope,
                    if_current_not_found,
                },
            ),
            Keybinding::new_undocumented(
                key!("D"),
                "With",
                Dispatch::OpenSearchPromptWithCurrentSelection {
                    scope,
                    prior_change,
                },
            ),
            Keybinding::new_undocumented(
                key!("r"),
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
                    key!("f"),
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
            key!("f"),
            "Search This",
            Dispatch::ToEditor(DispatchEditor::SearchCurrentSelection(
                if_current_not_found,
                scope,
            )),
        ),
        Keybinding::new_undocumented(
            key!("F"),
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
                .zip(QWERTY_EVENT.iter().flatten())
                .filter_map(|(key, key_event)| {
                    let (_, description, _) =
                        custom_keymap().into_iter().find(|(k, _, _)| k == key)?;
                    Some(Keybinding::new_dynamic(
                        *key_event,
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
                Keybinding::new_undocumented(
                    key!("u"),
                    "÷ Selection",
                    Dispatch::ToggleRevealSelections,
                ),
                Keybinding::new_undocumented(
                    key!("i"),
                    "÷ Cursor",
                    Dispatch::ToEditor(DispatchEditor::ToggleReveal(Reveal::Cursor)),
                ),
                Keybinding::new_undocumented(
                    key!("o"),
                    "÷ Mark",
                    Dispatch::ToEditor(DispatchEditor::ToggleReveal(Reveal::Mark)),
                ),
                Keybinding::new_undocumented(
                    key!("j"),
                    "Editor",
                    Dispatch::ShowMenu(space_editor_keymap_legend_config()),
                ),
                Keybinding::new_undocumented(
                    key!("k"),
                    "Pick",
                    Dispatch::ShowMenu(space_pick_keymap_legend_config()),
                ),
                Keybinding::new_undocumented(
                    key!("l"),
                    "Context",
                    Dispatch::ShowMenu(space_context_keymap_legend_config(editor)),
                ),
                Keybinding::new_undocumented(
                    key!(";"),
                    "Explorer",
                    Dispatch::RevealInExplorer(
                        editor
                            .path()
                            .unwrap_or_else(|| context.current_working_directory().clone()),
                    ),
                ),
                Keybinding::new_undocumented(
                    key!("/"),
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
                key!("x"),
                "Replace all",
                Dispatch::Replace {
                    scope: Scope::Global,
                },
            ),
            Keybinding::new_undocumented(
                key!("enter"),
                "Force Save",
                Dispatch::ToEditor(DispatchEditor::ForceSave),
            ),
            Keybinding::new_undocumented(key!("c"), "Save All", Dispatch::SaveAll),
            Keybinding::new_undocumented(key!("q"), "Quit No Save", Dispatch::QuitNoSave),
            Keybinding::new_undocumented(key!("v"), "Quit", Dispatch::SafeQuit),
            Keybinding::new_undocumented(
                key!("f"),
                "Change Work Dir",
                Dispatch::OpenChangeWorkingDirectoryPrompt,
            ),
            Keybinding::new_undocumented(
                key!("d"),
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
            Keybinding::new_undocumented(key!("d"), "Code Actions", {
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
            Keybinding::new_undocumented(key!("s"), "Hover", Dispatch::RequestHover),
            Keybinding::new_undocumented(key!("f"), "Rename", Dispatch::PrepareRename),
            Keybinding::new_undocumented(
                key!("g"),
                "Revert Hunk@",
                Dispatch::ToEditor(DispatchEditor::RevertHunk(
                    DiffMode::UnstagedAgainstCurrentBranch,
                )),
            ),
            Keybinding::new_undocumented(
                key!("G"),
                "Revert Hunk^",
                Dispatch::ToEditor(DispatchEditor::RevertHunk(
                    DiffMode::UnstagedAgainstMainBranch,
                )),
            ),
            Keybinding::new_undocumented(
                key!("b"),
                "Git Blame",
                Dispatch::ToEditor(DispatchEditor::GitBlame),
            ),
            Keybinding::new_undocumented(
                key!("x"),
                "Go to File",
                Dispatch::ToEditor(DispatchEditor::GoToFile),
            ),
            Keybinding::new_undocumented(
                key!("C"),
                "Copy Absolute Path",
                Dispatch::ToEditor(DispatchEditor::CopyAbsolutePath),
            ),
            Keybinding::new_undocumented(
                key!("c"),
                "Copy Relative Path",
                Dispatch::ToEditor(DispatchEditor::CopyRelativePath),
            ),
            Keybinding::new_undocumented(
                key!("t"),
                "TS Node Sexp",
                Dispatch::ToEditor(DispatchEditor::ShowCurrentTreeSitterNodeSexp),
            ),
            Keybinding::new_undocumented(key!("e"), "Pipe", Dispatch::OpenPipeToShellPrompt),
        ]),
    }
}
pub fn space_pick_keymap_legend_config() -> KeymapLegendConfig {
    KeymapLegendConfig {
        title: "Pick".to_string(),

        keymap: Keymap::new(
            &[
                Keybinding::new_undocumented(
                    key!("f"),
                    "Buffer",
                    Dispatch::OpenFilePicker(FilePickerKind::Opened),
                ),
                Keybinding::new_undocumented(
                    key!("d"),
                    "File",
                    Dispatch::OpenFilePicker(FilePickerKind::NonGitIgnored),
                ),
            ]
            .into_iter()
            .chain([
                Keybinding::new_undocumented(
                    key!("g"),
                    "Git status ^",
                    Dispatch::OpenFilePicker(FilePickerKind::GitStatus(
                        DiffMode::UnstagedAgainstCurrentBranch,
                    )),
                ),
                Keybinding::new_undocumented(
                    key!("G"),
                    "Git status @",
                    Dispatch::OpenFilePicker(FilePickerKind::GitStatus(
                        DiffMode::UnstagedAgainstMainBranch,
                    )),
                ),
            ])
            .chain(Some(Keybinding::new_undocumented(
                key!("s"),
                "Symbol (Document)",
                Dispatch::RequestDocumentSymbols,
            )))
            .chain(Some(Keybinding::new_undocumented(
                key!("S"),
                "Symbol (Workspace)",
                Dispatch::OpenWorkspaceSymbolsPicker,
            )))
            .chain(Some(Keybinding::new_undocumented(
                key!("a"),
                "Theme",
                Dispatch::OpenThemePicker,
            )))
            .chain(Some(Keybinding::new_undocumented(
                key!("t"),
                "Quickfix",
                Dispatch::OpenQuickfixItemsPicker,
            )))
            .chain(Some(Keybinding::new_undocumented(
                key!("b"),
                "Git Branch",
                Dispatch::OpenGitBranchPrompt,
            )))
            .collect_vec(),
        ),
    }
}
pub fn keymap_transform() -> Vec<Keybinding> {
    [
        Keybinding::new_undocumented(
            key!("q"),
            "UPPER CASE",
            Dispatch::ToEditor(Transform(Transformation::Case(Case::Upper))),
        ),
        Keybinding::new_undocumented(
            key!("w"),
            "UPPER_SNAKE_CASE",
            Dispatch::ToEditor(Transform(Transformation::Case(Case::UpperSnake))),
        ),
        Keybinding::new_undocumented(
            key!("e"),
            "PascalCase",
            Dispatch::ToEditor(Transform(Transformation::Case(Case::Pascal))),
        ),
        Keybinding::new_undocumented(
            key!("r"),
            "Upper-Kebab",
            Dispatch::ToEditor(Transform(Transformation::Case(Case::UpperKebab))),
        ),
        Keybinding::new_undocumented(
            key!("t"),
            "Title Case",
            Dispatch::ToEditor(Transform(Transformation::Case(Case::Title))),
        ),
        Keybinding::new_undocumented(
            key!("a"),
            "lower case",
            Dispatch::ToEditor(Transform(Transformation::Case(Case::Lower))),
        ),
        Keybinding::new_undocumented(
            key!("s"),
            "snake_case",
            Dispatch::ToEditor(Transform(Transformation::Case(Case::Snake))),
        ),
        Keybinding::new_undocumented(
            key!("d"),
            "camelCase",
            Dispatch::ToEditor(Transform(Transformation::Case(Case::Camel))),
        ),
        Keybinding::new_undocumented(
            key!("f"),
            "kebab-case",
            Dispatch::ToEditor(Transform(Transformation::Case(Case::Kebab))),
        ),
    ]
    .into_iter()
    .chain(Some(Keybinding::new_undocumented(
        key!("j"),
        "Wrap",
        Dispatch::ToEditor(Transform(Transformation::Wrap)),
    )))
    .chain(Some(Keybinding::new_undocumented(
        key!("h"),
        "Unwrap",
        Dispatch::ToEditor(Transform(Transformation::Unwrap)),
    )))
    .chain(Some(Keybinding::new_undocumented(
        key!("k"),
        "Line Comment",
        Dispatch::ToEditor(DispatchEditor::ToggleLineComment),
    )))
    .chain(Some(Keybinding::new_undocumented(
        key!("l"),
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
                    key!("f"),
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
        .chain(keymap_other_movements())
        .chain(keymap_primary_selection_modes(editor, prior_change))
        .chain(keymap_secondary_selection_modes_init(editor, prior_change))
        .chain(keymap_actions(&normal_mode_override, false, editor))
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
                    key!(";"),
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
            key!("r"),
            "Delete Surround",
            Dispatch::ShowMenu(delete_surround_keymap_legend_config()),
        ),
        Keybinding::new_undocumented(
            key!("s"),
            "Surround",
            Dispatch::ShowMenu(surround_keymap_legend_config()),
        ),
        Keybinding::new_undocumented(
            key!("f"),
            "Change Surround",
            Dispatch::ShowMenu(change_surround_from_keymap_legend_config()),
        ),
        Keybinding::new_undocumented(
            key!("d"),
            "Select Inside",
            Dispatch::ShowMenu(select_surround_keymap_legend_config(SurroundKind::Inside)),
        ),
        Keybinding::new_undocumented(
            key!("e"),
            "Select Around",
            Dispatch::ShowMenu(select_surround_keymap_legend_config(SurroundKind::Around)),
        ),
    ])
}

pub fn multicursor_momentary_layer_keymap(editor: &Editor) -> Keymap {
    Keymap::new(
        &[
            Keybinding::new_undocumented(
                key!("i"),
                "Add Curs ^",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::Up)),
            ),
            Keybinding::new_undocumented(
                key!("k"),
                "Add Curs v",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::Down)),
            ),
            Keybinding::new_undocumented(
                key!("j"),
                "<< Add Curs",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::Left)),
            ),
            Keybinding::new_undocumented(
                key!("l"),
                "Add Curs >>",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::Right)),
            ),
            Keybinding::new_undocumented(
                key!("u"),
                "< Add Curs",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::Previous)),
            ),
            Keybinding::new_undocumented(
                key!("o"),
                "Add Curs >",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::Next)),
            ),
            Keybinding::new_undocumented(
                key!("y"),
                "|< Add Curs",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::First)),
            ),
            Keybinding::new_undocumented(
                key!("p"),
                "Add Curs >|",
                Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(Movement::Last)),
            ),
        ]
        .into_iter()
        .chain([
            Keybinding::new_undocumented(
                key!("h"),
                "← Curs",
                Dispatch::CycleCursor(Direction::Start),
            ),
            Keybinding::new_undocumented(
                key!(";"),
                "Curs →",
                Dispatch::CycleCursor(Direction::End),
            ),
            Keybinding::new_undocumented(key!("n"), "Delete Curs", Dispatch::DeleteCursor),
            Keybinding::new_undocumented(
                key!("m"),
                "Jump Add Curs",
                Dispatch::ToEditor(ShowJumps {
                    use_current_selection_mode: true,
                    prior_change: Some(PriorChange::EnterMultiCursorMode),
                }),
            ),
            Keybinding::new_undocumented(
                key!("space"),
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
        Keybinding::new_undocumented(key!("G"), "Change X", Dispatch::ToEditor(ChangeCut)),
        Keybinding::momentary_layer(MomentaryLayer {
            event: key!("c"),
            name: "Copy/≡ Dup".to_string(),
            config: KeymapLegendConfig {
                title: "Copy/≡ Dup".to_string(),
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
            key!("j"),
            "<<",
            doc_format!("Left.md"),
            Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Left, prior_change)),
        ),
        Keybinding::new_undocumented(
            key!("l"),
            ">>",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Right, prior_change)),
        ),
        Keybinding::new_undocumented(
            key!("i"),
            "^",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Up, prior_change)),
        ),
        Keybinding::new_undocumented(
            key!("k"),
            "v",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Down, prior_change)),
        ),
        Keybinding::new_undocumented(
            key!("y"),
            "|<",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::First, prior_change)),
        ),
        Keybinding::new_undocumented(
            key!("p"),
            ">|",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Last, prior_change)),
        ),
        Keybinding::new_undocumented(
            key!("o"),
            ">",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Next, prior_change)),
        ),
        Keybinding::new_undocumented(
            key!("u"),
            "<",
            Dispatch::ToEditor(MoveSelectionWithPriorChange(
                Movement::Previous,
                prior_change,
            )),
        ),
        Keybinding::new_undocumented(
            key!("m"),
            "Jump",
            Dispatch::ToEditor(DispatchEditor::ShowJumps {
                use_current_selection_mode: true,
                prior_change,
            }),
        ),
        Keybinding::new_undocumented(
            key!("M"),
            "Index",
            Dispatch::OpenMoveToIndexPrompt(prior_change),
        ),
        Keybinding::new_undocumented(
            key!("."),
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
            key!("space"),
            "Space",
            Dispatch::ToEditor(DispatchEditor::PressSpace),
        ),
        Keybinding::new_undocumented(
            key!(","),
            "Surround",
            Dispatch::ShowMenu(surround_keymap_legend_config()),
        ),
        Keybinding::new_undocumented(
            key!("esc"),
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
        Keybinding::new_undocumented(key!("a"), "LINE", selection_mode_to_dispatch(Line)),
        Keybinding::new_undocumented(key!("A"), "LINE*", selection_mode_to_dispatch(LineFull)),
        Keybinding::new_undocumented(key!("d"), "NODE", selection_mode_to_dispatch(SyntaxNode)),
        Keybinding::new_undocumented(
            key!("D"),
            "NODE*",
            selection_mode_to_dispatch(SyntaxNodeFine),
        ),
        Keybinding::new_undocumented(key!("s"), "WORD", selection_mode_to_dispatch(Word)),
        Keybinding::new_undocumented(key!("S"), "WORD*", selection_mode_to_dispatch(BigWord)),
        Keybinding::new_undocumented(key!("w"), "SUBWORD", selection_mode_to_dispatch(Subword)),
        Keybinding::new_undocumented(key!("W"), "CHAR", selection_mode_to_dispatch(Character)),
        Keybinding::new_undocumented(
            key!("E"),
            "PARAGRAPH",
            selection_mode_to_dispatch(Paragraph),
        ),
    ]
    .into()
}

pub fn keymap_secondary_selection_modes_init(
    editor: &Editor,
    prior_change: Option<PriorChange>,
) -> Vec<Keybinding> {
    [Keybinding::new_undocumented(
        key!("n"),
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
        event: key!("v"),
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
            key!("alt+;"),
            "⇋ Align View",
            Dispatch::ToEditor(SwitchViewAlignment),
        ),
        Keybinding::new_undocumented(key!("alt+/"), "⇋ Window", Dispatch::OtherWindow),
        #[cfg(unix)]
        Keybinding::new_undocumented(key!("ctrl+z"), "Suspend", Dispatch::Suspend),
    ]
    .to_vec()
}

pub fn insert_mode_keymap_legend_config(include_universal_keymap: bool) -> KeymapLegendConfig {
    KeymapLegendConfig {
        title: "Insert mode keymap".to_string(),
        keymap: Keymap::new(
            &[
                Keybinding::new_undocumented(
                    key!("left"),
                    "Char ←",
                    Dispatch::ToEditor(MoveCharacterBack),
                ),
                Keybinding::new_undocumented(
                    key!("right"),
                    "Char →",
                    Dispatch::ToEditor(MoveCharacterForward),
                ),
                Keybinding::new_undocumented(
                    key!("alt+y"),
                    "Line ←",
                    Dispatch::ToEditor(MoveToLineStart),
                ),
                Keybinding::new_undocumented(
                    key!("alt+p"),
                    "Line →",
                    Dispatch::ToEditor(MoveToLineEnd),
                ),
                Keybinding::new_undocumented(
                    key!("alt+backspace"),
                    "Delete Word ←",
                    Dispatch::ToEditor(DeleteWordBackward { short: true }),
                ),
                Keybinding::new_undocumented(
                    key!("esc"),
                    "Enter normal mode",
                    Dispatch::ToEditor(EnterNormalMode),
                ),
                Keybinding::new_undocumented(
                    key!("backspace"),
                    "Delete character backward",
                    Dispatch::ToEditor(Backspace),
                ),
                Keybinding::new_undocumented(
                    key!("enter"),
                    "Enter new line",
                    Dispatch::ToEditor(EnterNewline),
                ),
                Keybinding::new_undocumented(
                    key!("tab"),
                    "Enter tab",
                    Dispatch::ToEditor(Insert("\t".to_string())),
                ),
                Keybinding::new_undocumented(
                    key!("home"),
                    "Move to line start",
                    Dispatch::ToEditor(MoveToLineStart),
                ),
                Keybinding::new_undocumented(
                    key!("end"),
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
                event: key!("alt+e"),
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
                event: key!("alt+v"),
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
                key!("alt+y"),
                "Kill Line ←",
                Dispatch::ToEditor(KillLine(Direction::Start)),
            ),
            Keybinding::new_undocumented(
                key!("alt+p"),
                "Kill Line →",
                Dispatch::ToEditor(KillLine(Direction::End)),
            ),
            Keybinding::new_undocumented(
                key!("alt+j"),
                "← Delete Word",
                Dispatch::ToEditor(DeleteWord {
                    short: false,
                    direction: Direction::Start,
                }),
            ),
            Keybinding::new_undocumented(
                key!("alt+l"),
                "Delete Word →",
                Dispatch::ToEditor(DeleteWord {
                    short: false,
                    direction: Direction::End,
                }),
            ),
            Keybinding::new_undocumented(
                key!("alt+u"),
                "← Delete Subword",
                Dispatch::ToEditor(DeleteWord {
                    short: true,
                    direction: Direction::Start,
                }),
            ),
            Keybinding::new_undocumented(
                key!("alt+o"),
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
    editor: &Editor,
) -> Vec<Keybinding> {
    [
        Keybinding::new_undocumented(key!("I"), "Join", Dispatch::ToEditor(JoinSelection)),
        Keybinding::new_undocumented(key!("K"), "Break", Dispatch::ToEditor(BreakSelection)),
        Keybinding::new_undocumented(
            key!("Y"),
            "← Align",
            Dispatch::ToEditor(AlignSelections(Direction::Start)),
        ),
        Keybinding::new_undocumented(
            key!("P"),
            "Align →",
            Dispatch::ToEditor(AlignSelections(Direction::End)),
        ),
        Keybinding::momentary_layer(MomentaryLayer {
            event: key!("z"),
            name: "≡ Undo/Redo".to_string(),
            config: KeymapLegendConfig {
                title: "≡ Undo/Redo".to_string(),
                keymap: undo_redo_keymap(),
            },
            on_tap: Some(OnTap::new("Coarse Undo", Dispatch::ToEditor(CoarseUndo))),
        }),
        Keybinding::new_undocumented(key!("enter"), "Save", Dispatch::SaveFile),
        Keybinding::new_undocumented(key!("shift+enter"), "Save As", Dispatch::OpenSaveAsPrompt),
        Keybinding::new_undocumented(
            key!("F"),
            "Transform",
            Dispatch::ShowMenu(transform_keymap_legend_config()),
        ),
        Keybinding::new_undocumented(key!("L"), "Indent", Dispatch::ToEditor(Indent)),
        Keybinding::new_undocumented(key!("J"), "Dedent", Dispatch::ToEditor(Dedent)),
        Keybinding::new_undocumented(key!("*"), "Keyboard", Dispatch::OpenKeyboardLayoutPrompt),
        Keybinding::new(
            key!("Z"),
            "Coarse Redo",
            doc_format!("Coarse Redo.md"),
            Dispatch::ToEditor(CoarseRedo),
        ),
        Keybinding::new_undocumented(
            key!("backslash"),
            "Leader",
            Dispatch::ShowMenu(leader_keymap_legend_config()),
        ),
        Keybinding::momentary_layer(MomentaryLayer {
            event: key!("b"),
            name: "≡ Multi-cursor".to_string(),
            config: KeymapLegendConfig {
                title: "≡ Multi-cursor".to_string(),
                keymap: multicursor_momentary_layer_keymap(editor),
            },
            on_tap: None,
        }),
        Keybinding::momentary_layer(MomentaryLayer {
            event: key!("t"),
            name: "≡ Swap".to_string(),
            config: KeymapLegendConfig {
                title: "≡ Swap".to_string(),
                keymap: swap_keymap(),
            },
            on_tap: None,
        }),
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
            key!("j"),
            "Coarse Undo",
            doc_format!("Coarse Undo.md"),
            Dispatch::ToEditor(CoarseUndo),
        ),
        Keybinding::new(
            key!("l"),
            "Coarse Redo",
            doc_format!("Coarse Redo.md"),
            Dispatch::ToEditor(CoarseRedo),
        ),
        Keybinding::new(
            key!("u"),
            "Fine Undo",
            doc_format!("Fine Undo.md"),
            Dispatch::ToEditor(FineUndo),
        ),
        Keybinding::new(
            key!("o"),
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
        Keybinding::momentary_layer(MomentaryLayer {
            event: key!("g"),
            name: "≡ Open".to_string(),
            config: KeymapLegendConfig {
                title: "≡ Open".to_string(),
                keymap: open_keymap(),
            },
            on_tap: Some(OnTap::new(
                "Change",
                Dispatch::ToEditor(DispatchEditor::Change),
            )),
        })
        .override_keymap(normal_mode_override.change.as_ref(), none_if_no_override),
        Keybinding::momentary_layer(MomentaryLayer {
            event: key!("x"),
            name: "≡ Cut".to_string(),
            config: KeymapLegendConfig {
                title: "≡ Cut".to_string(),
                keymap: cut_keymap(),
            },
            on_tap: Some(OnTap::new(
                "Cut One",
                Dispatch::ToEditor(DispatchEditor::CutOne),
            )),
        })
        .override_keymap(normal_mode_override.cut.as_ref(), none_if_no_override),
        Keybinding::new_undocumented(
            key!("r"),
            "≡ Delete/Eat",
            Dispatch::ShowJointMomentaryLayer {
                swap_key: key!("space"),
                active_config: KeymapLegendConfig {
                    title: "≡ Delete".to_string(),
                    keymap: delete_keymap(),
                },
                release_key: ReleaseKey::new(
                    key!("r"),
                    Some(OnTap::new(
                        "Delete One",
                        Dispatch::ToEditor(DispatchEditor::DeleteOne),
                    )),
                ),
                inactive_config: KeymapLegendConfig {
                    title: "≡ Eat".to_string(),
                    keymap: eat_keymap(),
                },
                inactive_tap: None,
            },
        )
        .override_keymap(normal_mode_override.delete.as_ref(), none_if_no_override),
        Keybinding::new_undocumented(
            key!("h"),
            "← Insert",
            Dispatch::ToEditor(EnterInsertMode(Direction::Start)),
        )
        .override_keymap(normal_mode_override.insert.as_ref(), none_if_no_override),
        Keybinding::new_undocumented(
            key!(";"),
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
        Keybinding::new_undocumented(
            key!("alt+k"),
            "Scroll ↓",
            Dispatch::ToEditor(ScrollPageDown),
        ),
        Keybinding::new_undocumented(key!("alt+i"), "Scroll ↑", Dispatch::ToEditor(ScrollPageUp)),
        Keybinding::app_momentary_layer(MomentaryLayer {
            event: key!("q"),
            name: "≡ Move Hist".to_string(),
            config: KeymapLegendConfig {
                title: "≡ Move Hist".to_string(),
                keymap: movement_history_keymap(),
            },
            on_tap: None,
        }),
        Keybinding::app_momentary_layer(MomentaryLayer {
            event: key!("e"),
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
        Keybinding::new_undocumented(
            key!("?"),
            "⇋ Anchor",
            Dispatch::ToEditor(SwapExtensionAnchor),
        ),
        Keybinding::new_undocumented(
            key!("/"),
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
            key!("i"),
            "Swap ^",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::Up)),
        ),
        Keybinding::new_undocumented(
            key!("j"),
            "<< Swap",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::Left)),
        ),
        Keybinding::new_undocumented(
            key!("l"),
            "Swap >>",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::Right)),
        ),
        Keybinding::new_undocumented(
            key!("k"),
            "Swap v",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::Down)),
        ),
        Keybinding::new_undocumented(
            key!("u"),
            "< Swap",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::Previous)),
        ),
        Keybinding::new_undocumented(
            key!("y"),
            "|< Swap",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::First)),
        ),
        Keybinding::new_undocumented(
            key!("p"),
            "Swap >|",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::Last)),
        ),
        Keybinding::new_undocumented(
            key!("o"),
            "Swap >",
            Dispatch::ToEditor(DispatchEditor::SwapWithMovement(Movement::Next)),
        ),
        Keybinding::new_undocumented(
            key!("m"),
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
            key!("i"),
            "Eat ^",
            doc_format!("eat/movement.md", { movement: "^", old: "foo bar\n[bar] baz", new: "[bar] baz" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::Up)),
        ),
        Keybinding::new(
            key!("j"),
            "<< Eat",
            doc_format!("eat/movement.md", { movement: "<<", old: "foo / [bar]", new: "[bar]" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::Left)),
        ),
        Keybinding::new(
            key!("l"),
            "Eat >>",
            doc_format!("eat/movement.md", { movement: ">>", old: "[foo] / bar", new: "[foo]" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::Right)),
        ),
        Keybinding::new(
            key!("k"),
            "Eat v",
            doc_format!("eat/movement.md", { movement: "v", old: "[foo] bar\nbar baz", new: "[foo] baz" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::Down)),
        ),
        Keybinding::new(
            key!("u"),
            "< Eat",
            doc_format!("eat/movement.md", { movement: "<", old: "foo / [bar]", new: "foo [bar]" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::Previous)),
        ),
        Keybinding::new(
            key!("y"),
            "|< Eat",
            doc_format!("eat/movement.md", { movement: "|<", old: "foo bar [baz]", new: "[baz]" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::First)),
        ),
        Keybinding::new(
            key!("p"),
            "Eat >|",
            doc_format!("eat/movement.md", { movement: ">|", old: "[foo] bar baz", new: "[foo]" }),
            Dispatch::ToEditor(DispatchEditor::Eat(Movement::Last)),
        ),
        Keybinding::new(
            key!("o"),
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
                key!("j"),
                "<< Gap Paste",
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::Left)),
            ),
            Keybinding::new_undocumented(
                key!("l"),
                "Gap Paste >>",
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::Right)),
            ),
            Keybinding::new_undocumented(
                key!("o"),
                "Gap Paste >",
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::Next)),
            ),
            Keybinding::new_undocumented(
                key!("u"),
                "< Gap Paste",
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::Previous)),
            ),
            Keybinding::new_undocumented(
                key!(";"),
                "Paste >",
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::AfterWithoutGap)),
            ),
            Keybinding::new_undocumented(
                key!("h"),
                "< Paste",
                Dispatch::ToEditor(PasteWithMovement(GetGapMovement::BeforeWithoutGap)),
            ),
            Keybinding::new_undocumented(
                key!("m"),
                "Replace w/ pattern",
                Dispatch::ToEditor(ReplaceWithPattern),
            ),
            Keybinding::new_undocumented(
                key!("y"),
                "← Replace History",
                Dispatch::ToEditor(ReplaceWithPreviousCopiedText),
            ),
            Keybinding::new_undocumented(
                key!("p"),
                "Replace History →",
                Dispatch::ToEditor(ReplaceWithNextCopiedText),
            ),
            Keybinding::new_undocumented(
                key!("i"),
                "Paste ^",
                Dispatch::ToEditor(PasteVertically(Direction::Start)),
            ),
            Keybinding::new_undocumented(
                key!("k"),
                "Paste v",
                Dispatch::ToEditor(PasteVertically(Direction::End)),
            ),
            Keybinding::new_undocumented(
                key!("n"),
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
                key!("j"),
                "<< Gap Dup",
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::Left)),
            ),
            Keybinding::new_undocumented(
                key!("l"),
                "Gap Dup >>",
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::Right)),
            ),
            Keybinding::new_undocumented(
                key!("o"),
                "Gap Dup >",
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::Next)),
            ),
            Keybinding::new_undocumented(
                key!("u"),
                "< Gap Dup",
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::Previous)),
            ),
            Keybinding::new_undocumented(
                key!(";"),
                "Dup >",
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::AfterWithoutGap)),
            ),
            Keybinding::new_undocumented(
                key!("h"),
                "< Dup",
                Dispatch::ToEditor(DuplicateWithMovement(GetGapMovement::BeforeWithoutGap)),
            ),
            Keybinding::new_undocumented(
                key!("i"),
                "Dup ^",
                Dispatch::ToEditor(DuplicateVertically(Direction::Start)),
            ),
            Keybinding::new_undocumented(
                key!("k"),
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
            key!("j"),
            "<< Cut",
            Dispatch::ToEditor(CutWithMovement(Movement::Left)),
        ),
        Keybinding::new_undocumented(
            key!("l"),
            "Cut >>",
            Dispatch::ToEditor(CutWithMovement(Movement::Right)),
        ),
        Keybinding::new_undocumented(
            key!("u"),
            "< Cut",
            Dispatch::ToEditor(CutWithMovement(Movement::Previous)),
        ),
        Keybinding::new_undocumented(
            key!("o"),
            "Cut >",
            Dispatch::ToEditor(CutWithMovement(Movement::Next)),
        ),
        Keybinding::new_undocumented(
            key!("y"),
            "|< Cut",
            Dispatch::ToEditor(CutWithMovement(Movement::First)),
        ),
        Keybinding::new_undocumented(
            key!("p"),
            "Cut >|",
            Dispatch::ToEditor(CutWithMovement(Movement::Last)),
        ),
    ])
}

pub fn buffer_keymap(is_alted: bool) -> Keymap {
    Keymap::new(
        &[
            Keybinding::new_undocumented(
                possibly_alted(key!("j"), is_alted),
                "<< Marked File",
                Dispatch::CycleMarkedFile(Movement::Left),
            ),
            Keybinding::new_undocumented(
                possibly_alted(key!("l"), is_alted),
                "Marked File >>",
                Dispatch::CycleMarkedFile(Movement::Right),
            ),
            Keybinding::new_undocumented(
                possibly_alted(key!("y"), is_alted),
                "|< Marked File",
                Dispatch::CycleMarkedFile(Movement::First),
            ),
            Keybinding::new_undocumented(
                possibly_alted(key!("p"), is_alted),
                "Marked File >|",
                Dispatch::CycleMarkedFile(Movement::Last),
            ),
            Keybinding::new_undocumented(
                possibly_alted(key!("u"), is_alted),
                "Marked File >",
                Dispatch::CycleMarkedFile(Movement::Previous),
            ),
            Keybinding::new_undocumented(
                possibly_alted(key!("o"), is_alted),
                "< Marked File",
                Dispatch::CycleMarkedFile(Movement::Next),
            ),
        ]
        .into_iter()
        .chain(Some(Keybinding::new_undocumented(
            possibly_alted(key!("k"), is_alted),
            "Mark File",
            Dispatch::ToggleFileMark,
        )))
        .chain(Some(Keybinding::new_undocumented(
            possibly_alted(key!("n"), is_alted),
            "Close",
            Dispatch::CloseCurrentWindow,
        )))
        .chain(Some(Keybinding::new_undocumented(
            possibly_alted(key!("i"), is_alted),
            "Unmark Others",
            Dispatch::UnmarkAllOthers,
        )))
        .chain(Some(Keybinding::new_undocumented(
            possibly_alted(key!("m"), is_alted),
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
            key!("j"),
            "<< Move Hist",
            Dispatch::MovementHistoryNavigation(HistoryNavigationMovement::CoarseBack),
        ),
        Keybinding::new_undocumented(
            key!("l"),
            "Move Hist >>",
            Dispatch::MovementHistoryNavigation(HistoryNavigationMovement::CoarseForward),
        ),
        Keybinding::new_undocumented(
            key!("u"),
            "< Move Hist",
            Dispatch::MovementHistoryNavigation(HistoryNavigationMovement::FineBack),
        ),
        Keybinding::new_undocumented(
            key!("o"),
            "Move Hist >",
            Dispatch::MovementHistoryNavigation(HistoryNavigationMovement::FineForward),
        ),
    ])
}

pub fn delete_keymap() -> Keymap {
    Keymap::new(&[
        Keybinding::new_undocumented(
            key!("j"),
            "<< Delete",
            Dispatch::ToEditor(DeleteWithMovement(Movement::Left)),
        ),
        Keybinding::new_undocumented(
            key!("l"),
            "Delete >>",
            Dispatch::ToEditor(DeleteWithMovement(Movement::Right)),
        ),
        Keybinding::new_undocumented(
            key!("u"),
            "< Delete",
            Dispatch::ToEditor(DeleteWithMovement(Movement::Previous)),
        ),
        Keybinding::new_undocumented(
            key!("o"),
            "Delete >",
            Dispatch::ToEditor(DeleteWithMovement(Movement::Next)),
        ),
        Keybinding::new_undocumented(
            key!("y"),
            "|< Delete",
            Dispatch::ToEditor(DeleteWithMovement(Movement::First)),
        ),
        Keybinding::new_undocumented(
            key!("p"),
            "Delete >|",
            Dispatch::ToEditor(DeleteWithMovement(Movement::Last)),
        ),
    ])
}

pub fn open_keymap() -> Keymap {
    Keymap::new(
        &[
            Keybinding::new_undocumented(
                key!("j"),
                "<< Open",
                Dispatch::ToEditor(DispatchEditor::Open(GetGapMovement::Left)),
            ),
            Keybinding::new_undocumented(
                key!("l"),
                "Open >>",
                Dispatch::ToEditor(DispatchEditor::Open(GetGapMovement::Right)),
            ),
            Keybinding::new_undocumented(
                key!("u"),
                "< Open",
                Dispatch::ToEditor(DispatchEditor::Open(GetGapMovement::Previous)),
            ),
            Keybinding::new_undocumented(
                key!("o"),
                "Open >",
                Dispatch::ToEditor(DispatchEditor::Open(GetGapMovement::Next)),
            ),
            Keybinding::new_undocumented(
                key!("h"),
                "< Insert",
                Dispatch::ToEditor(DispatchEditor::Open(GetGapMovement::BeforeWithoutGap)),
            ),
            Keybinding::new_undocumented(
                key!(";"),
                "Insert >",
                Dispatch::ToEditor(DispatchEditor::Open(GetGapMovement::AfterWithoutGap)),
            ),
        ]
        .into_iter()
        .chain([
            Keybinding::new_undocumented(
                key!("i"),
                "Open ^",
                Dispatch::ToEditor(OpenVertically(Direction::Start)),
            ),
            Keybinding::new_undocumented(
                key!("k"),
                "Open v",
                Dispatch::ToEditor(OpenVertically(Direction::End)),
            ),
        ])
        .collect_vec(),
    )
}
