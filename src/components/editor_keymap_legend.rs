use crossterm::event::KeyCode;
use SelectionMode::*;

use convert_case::Case;
use event::KeyEvent;
use itertools::Itertools;

use crate::{
    app::{Dispatch, Dispatches, FilePickerKind, Scope},
    components::editor::{Movement, PriorChange},
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
        Direction, DispatchEditor, Editor, HandleEventResult, IfCurrentNotFound, Reveal,
        SurroundKind,
    },
    editor_keymap::*,
    keymap_legend::{Keymap, KeymapLegendConfig, Keymaps},
};

use DispatchEditor::*;
use Movement::*;
impl Editor {
    pub(crate) fn keymap_core_movements(
        &self,
        context: &Context,
        prior_change: Option<PriorChange>,
    ) -> Vec<Keymap> {
        [
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Left_),
                "◀".to_string(),
                "Left".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Left, prior_change)),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Right),
                "▶".to_string(),
                "Right".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Right, prior_change)),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Up___),
                "▲".to_string(),
                "Up".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Up, prior_change)),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Down_),
                "▼".to_string(),
                "Down".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Down, prior_change)),
            ),
            Keymap::new(
                context.keyboard_layout_kind().get_key(&Meaning::First),
                "First".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::First, prior_change)),
            ),
            Keymap::new(
                context.keyboard_layout_kind().get_key(&Meaning::Last_),
                "Last".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Last, prior_change)),
            ),
            Keymap::new(
                context.keyboard_layout_kind().get_key(&Meaning::Next_),
                "Next".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Next, prior_change)),
            ),
            Keymap::new(
                context.keyboard_layout_kind().get_key(&Meaning::Prev_),
                "Previous".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(
                    Movement::Previous,
                    prior_change,
                )),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Jump_),
                "Jump".to_string(),
                "Jump".to_string(),
                Dispatch::ToEditor(DispatchEditor::ShowJumps {
                    use_current_selection_mode: true,
                    prior_change,
                }),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::ToIdx),
                "Index".to_string(),
                "To Index (1-based)".to_string(),
                Dispatch::OpenMoveToIndexPrompt(prior_change),
            ),
        ]
        .to_vec()
    }

    pub(crate) fn keymap_other_movements(&self, context: &Context) -> Vec<Keymap> {
        [
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::CrsrP),
                Direction::Start.format_action("Curs"),
                Direction::Start.format_action("Cycle primary selection"),
                Dispatch::ToEditor(CyclePrimarySelection(Direction::Start)),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::CrsrN),
                Direction::End.format_action("Curs"),
                Direction::End.format_action("Cycle primary selection"),
                Dispatch::ToEditor(CyclePrimarySelection(Direction::End)),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::ScrlD),
                "Scroll ↓".to_string(),
                "Scroll down".to_string(),
                Dispatch::ToEditor(ScrollPageDown),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::ScrlU),
                "Scroll ↑".to_string(),
                "Scroll up".to_string(),
                Dispatch::ToEditor(ScrollPageUp),
            ),
            Keymap::new_extended(
                "backspace",
                Direction::Start.format_action("Select"),
                "Go back".to_string(),
                Dispatch::ToEditor(GoBack),
            ),
            Keymap::new_extended(
                "tab",
                Direction::End.format_action("Select"),
                "Go forward".to_string(),
                Dispatch::ToEditor(GoForward),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::NBack),
                Direction::Start.format_action("Nav"),
                "Navigate back".to_string(),
                Dispatch::NavigateBack,
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::NForw),
                Direction::End.format_action("Nav"),
                "Navigate forward".to_string(),
                Dispatch::NavigateForward,
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::MrkFP),
                Direction::Start.format_action("Marked"),
                "Go to previous marked file".to_string(),
                Dispatch::CycleMarkedFile(Direction::Start),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::MrkFN),
                Direction::End.format_action("Marked"),
                "Go to next marked file".to_string(),
                Dispatch::CycleMarkedFile(Direction::End),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::SSEnd),
                "⇋ Anchor".to_string(),
                "Swap Anchor".to_string(),
                Dispatch::ToEditor(SwapExtensionAnchor),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::XAchr),
                "⇋ Curs".to_string(),
                "Swap cursor".to_string(),
                Dispatch::ToEditor(DispatchEditor::SwapCursor),
            ),
        ]
        .into_iter()
        .collect()
    }

    pub(crate) fn keymap_primary_selection_modes(
        &self,
        context: &Context,
        prior_change: Option<PriorChange>,
    ) -> Vec<Keymap> {
        let direction = self.cursor_direction.reverse().to_if_current_not_found();
        [
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Line_),
                "Line".to_string(),
                "Select Line".to_string(),
                Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    direction,
                    Line,
                    prior_change,
                )),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::LineF),
                "Line*".to_string(),
                "Select Line*".to_string(),
                Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    direction,
                    LineFull,
                    prior_change,
                )),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Sytx_),
                "Syntax".to_string(),
                "Select Syntax Node".to_string(),
                Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    direction,
                    SyntaxNode,
                    prior_change,
                )),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::FStyx),
                "Syntax*".to_string(),
                "Select Syntax Node*".to_string(),
                Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    direction,
                    SyntaxNodeFine,
                    prior_change,
                )),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Word_),
                "Word".to_string(),
                "Select Word".to_string(),
                Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    direction,
                    Word,
                    prior_change,
                )),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::SWord),
                "Subword".to_string(),
                "Select Subword".to_string(),
                Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    direction,
                    Subword,
                    prior_change,
                )),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Char_),
                "Char".to_string(),
                "Select Character".to_string(),
                Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    direction,
                    Character,
                    prior_change,
                )),
            ),
        ]
        .to_vec()
    }

    pub(crate) fn keymap_secondary_selection_modes_init(
        &self,
        context: &Context,
        prior_change: Option<PriorChange>,
    ) -> Vec<Keymap> {
        [Keymap::new_extended(
            context.keyboard_layout_kind().get_key(&Meaning::LSrch),
            Direction::End.format_action("Local"),
            "Find (Local)".to_string(),
            Dispatch::ShowKeymapLegend(self.secondary_selection_modes_keymap_legend_config(
                context,
                Scope::Local,
                self.cursor_direction.reverse().to_if_current_not_found(),
                prior_change,
            )),
        )]
        .to_vec()
    }

    pub(crate) fn keymap_actions(
        &self,
        normal_mode_override: &NormalModeOverride,
        none_if_no_override: bool,
        context: &Context,
        prior_change: Option<PriorChange>,
    ) -> Vec<Keymap> {
        [
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Join_),
                "Join".to_string(),
                "Join".to_string(),
                Dispatch::ToEditor(Transform(Transformation::Join)),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Break),
                "Break".to_string(),
                "Break".to_string(),
                Dispatch::ToEditor(BreakSelection),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Raise),
                "Raise".to_string(),
                "Raise".to_string(),
                Dispatch::ToEditor(Replace(Expand)),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Mark_),
                "Mark Sel".to_string(),
                "Toggle Selection Mark".to_string(),
                Dispatch::ToEditor(ToggleMark),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::MarkF),
                "Mark File".to_string(),
                "Toggle File Mark".to_string(),
                Dispatch::ToggleFileMark,
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::SrchL),
                Direction::Start.format_action("Search"),
                Direction::Start.format_action("Search"),
                Dispatch::OpenSearchPromptWithPriorChange {
                    scope: Scope::Local,
                    if_current_not_found: self.cursor_direction.reverse().to_if_current_not_found(),
                    prior_change,
                },
            ),
            Keymap::new(
                context.keyboard_layout_kind().get_key(&Meaning::SchWC),
                "With".to_string(),
                Dispatch::OpenSearchPromptWithCurrentSelection {
                    scope: Scope::Local,
                    prior_change,
                },
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Undo_),
                "Undo".to_string(),
                "Undo".to_string(),
                Dispatch::ToEditor(Undo),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Redo_),
                "Redo".to_string(),
                "Redo".to_string(),
                Dispatch::ToEditor(Redo),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::PRplc),
                "Replace #".to_string(),
                "Replace with pattern".to_string(),
                Dispatch::ToEditor(ReplaceWithPattern),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::RplcP),
                Direction::Start.format_action("Replace"),
                "Replace (with previous copied text)".to_string(),
                Dispatch::ToEditor(ReplaceWithPreviousCopiedText),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::RplcN),
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
                context.keyboard_layout_kind().get_key(&Meaning::Trsfm),
                "Transform".to_string(),
                "Transform".to_string(),
                Dispatch::ShowKeymapLegend(self.transform_keymap_legend_config(context)),
            ),
            Keymap::new(
                "$",
                Direction::End.format_action("Collapse selection"),
                Dispatch::ToEditor(DispatchEditor::CollapseSelection(Direction::End)),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Indnt),
                "Indent".to_string(),
                "Indent".to_string(),
                Dispatch::ToEditor(Indent),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::DeDnt),
                "Dedent".to_string(),
                "Dedent".to_string(),
                Dispatch::ToEditor(Dedent),
            ),
            Keymap::new(
                context.keyboard_layout_kind().get_key(&Meaning::Del0G),
                "Delete 0 Gap".to_string(),
                Dispatch::ToEditor(DeleteNoGap),
            ),
            Keymap::new(
                "*",
                "Keyboard".to_string(),
                Dispatch::OpenKeyboardLayoutPrompt,
            ),
        ]
        .into_iter()
        .chain(Some(self.search_current_selection_keymap(
            context,
            Scope::Local,
            IfCurrentNotFound::LookForward,
        )))
        .chain(self.keymap_actions_overridable(normal_mode_override, none_if_no_override, context))
        .chain(self.keymap_clipboard_related_actions(false, normal_mode_override.clone(), context))
        .collect_vec()
    }

    pub(crate) fn keymap_actions_overridable(
        &self,
        normal_mode_override: &NormalModeOverride,
        none_if_no_override: bool,
        context: &Context,
    ) -> Vec<Keymap> {
        [
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Chng_),
                "Change".to_string(),
                "Change".to_string(),
                Dispatch::ToEditor(Change),
            )
            .override_keymap(normal_mode_override.change.as_ref(), none_if_no_override),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Delte),
                Direction::End.format_action("Delete"),
                Direction::End.format_action("Delete"),
                Dispatch::ToEditor(Delete),
            )
            .override_keymap(normal_mode_override.delete.as_ref(), none_if_no_override),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::InstP),
                Direction::Start.format_action("Insert"),
                Direction::Start.format_action("Insert"),
                Dispatch::ToEditor(EnterInsertMode(Direction::Start)),
            )
            .override_keymap(normal_mode_override.insert.as_ref(), none_if_no_override),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::InstN),
                Direction::End.format_action("Insert"),
                Direction::End.format_action("Insert"),
                Dispatch::ToEditor(EnterInsertMode(Direction::End)),
            )
            .override_keymap(normal_mode_override.append.as_ref(), none_if_no_override),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Open_),
                Direction::End.format_action("Open"),
                Direction::End.format_action("Open"),
                Dispatch::ToEditor(Open),
            )
            .override_keymap(normal_mode_override.open.as_ref(), none_if_no_override),
        ]
        .into_iter()
        .flatten()
        .collect_vec()
    }

    pub(crate) fn keymap_overridable(
        &self,
        normal_mode_override: &NormalModeOverride,
        none_if_no_override: bool,
        context: &Context,
    ) -> Vec<Keymap> {
        self.keymap_actions_overridable(normal_mode_override, none_if_no_override, context)
            .into_iter()
            .chain(self.keymap_sub_modes_overridable(
                normal_mode_override,
                none_if_no_override,
                context,
            ))
            .chain(self.keymap_clipboard_related_actions_overridable(
                false,
                normal_mode_override.clone(),
                none_if_no_override,
                context,
            ))
            .collect_vec()
    }

    fn keymap_clipboard_related_actions(
        &self,
        use_system_clipboard: bool,
        normal_mode_override: NormalModeOverride,
        context: &Context,
    ) -> Vec<Keymap> {
        let extra = if use_system_clipboard { "+ " } else { "" };
        let format = |description: &str| format!("{extra}{description}");
        [
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::ChngX),
                format("Change X"),
                format!("{}{}", "Change Cut", extra),
                Dispatch::ToEditor(ChangeCut),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::RplcX),
                format("Replace X"),
                format!("{}{}", "Replace Cut", extra),
                Dispatch::ToEditor(ReplaceWithCopiedText { cut: true }),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Copy_),
                format("Copy"),
                format!("{}{}", "Copy", extra),
                Dispatch::ToEditor(Copy),
            ),
            Keymap::new(
                context.keyboard_layout_kind().get_key(&Meaning::Pst0G),
                "Paste 0 Gap".to_string(),
                Dispatch::ToEditor(PasteNoGap),
            ),
        ]
        .into_iter()
        .chain(self.keymap_clipboard_related_actions_overridable(
            use_system_clipboard,
            normal_mode_override,
            false,
            context,
        ))
        .collect_vec()
    }

    fn keymap_clipboard_related_actions_overridable(
        &self,
        use_system_clipboard: bool,
        normal_mode_override: NormalModeOverride,
        none_if_no_override: bool,
        context: &Context,
    ) -> Vec<Keymap> {
        let extra = if use_system_clipboard { "+ " } else { "" };
        let format = |description: &str| format!("{extra}{description}");
        [
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Paste),
                format("Paste →"),
                format!("{}{}", Direction::End.format_action("Paste"), extra),
                Dispatch::ToEditor(Paste),
            )
            .override_keymap(
                normal_mode_override.paste.clone().as_ref(),
                none_if_no_override,
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Rplc_),
                format("Replace"),
                format!("{}{}", "Replace", extra),
                Dispatch::ToEditor(ReplaceWithCopiedText { cut: false }),
            )
            .override_keymap(normal_mode_override.replace.as_ref(), none_if_no_override),
        ]
        .into_iter()
        .flatten()
        .collect_vec()
    }

    pub(crate) fn keymap_universal(&self, context: &Context) -> Vec<Keymap> {
        [
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::WClse),
                "Close".to_string(),
                "Close current window".to_string(),
                Dispatch::CloseCurrentWindow,
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::SView),
                "⇋ Align".to_string(),
                "Switch view alignment".to_string(),
                Dispatch::ToEditor(SwitchViewAlignment),
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::WSwth),
                "⇋ Window".to_string(),
                "Switch window".to_string(),
                Dispatch::OtherWindow,
            ),
            #[cfg(unix)]
            Keymap::new("ctrl+z", "Suspend".to_string(), Dispatch::Suspend),
        ]
        .to_vec()
    }

    pub(crate) fn insert_mode_keymap_legend_config(
        &self,
        include_universal_keymaps: bool,
        context: &Context,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Insert mode keymaps".to_string(),
            keymaps: Keymaps::new(
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
                        context
                            .keyboard_layout_kind()
                            .get_insert_key(&Meaning::LineP),
                        "Line ←".to_string(),
                        "Move to line start".to_string(),
                        Dispatch::ToEditor(MoveToLineStart),
                    ),
                    Keymap::new_extended(
                        context
                            .keyboard_layout_kind()
                            .get_insert_key(&Meaning::LineN),
                        "Line →".to_string(),
                        "Move to line end".to_string(),
                        Dispatch::ToEditor(MoveToLineEnd),
                    ),
                    Keymap::new_extended(
                        context
                            .keyboard_layout_kind()
                            .get_insert_key(&Meaning::KilLP),
                        "Kill Line ←".to_string(),
                        Direction::End.format_action("Kill line"),
                        Dispatch::ToEditor(KillLine(Direction::Start)),
                    ),
                    Keymap::new_extended(
                        context
                            .keyboard_layout_kind()
                            .get_insert_key(&Meaning::KilLN),
                        "Kill Line →".to_string(),
                        Direction::End.format_action("Kill line"),
                        Dispatch::ToEditor(KillLine(Direction::End)),
                    ),
                    Keymap::new_extended(
                        context
                            .keyboard_layout_kind()
                            .get_insert_key(&Meaning::DWrdP),
                        "Delete Word ←".to_string(),
                        "Delete word backward".to_string(),
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
                .chain(if include_universal_keymaps {
                    self.keymap_universal(context)
                } else {
                    Default::default()
                })
                .collect_vec(),
            ),
        }
    }

    pub(crate) fn handle_insert_mode(
        &mut self,
        event: KeyEvent,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        if let Some(dispatches) = self
            .insert_mode_keymaps(true, context)
            .iter()
            .find(|keymap| &event == keymap.event())
            .map(|keymap| keymap.get_dispatches())
        {
            Ok(dispatches)
        } else if let KeyCode::Char(c) = event.code {
            return self.insert(&c.to_string(), context);
        } else {
            Ok(Default::default())
        }
    }

    pub(crate) fn handle_universal_key(
        &mut self,
        event: KeyEvent,
        context: &Context,
    ) -> anyhow::Result<HandleEventResult> {
        if let Some(keymap) = Keymaps::new(&self.keymap_universal(context)).get(&event) {
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

    pub(crate) fn keymap_sub_modes(&self, context: &Context) -> Vec<Keymap> {
        [
            Some(Keymap::new(
                "~",
                "Replace".to_string(),
                Dispatch::ToEditor(EnterReplaceMode),
            )),
            Some(Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Swap_),
                "Swap".to_string(),
                "Enter Swap mode".to_string(),
                Dispatch::ToEditor(EnterSwapMode),
            )),
        ]
        .into_iter()
        .flatten()
        .collect_vec()
    }

    fn keymap_sub_modes_overridable(
        &self,
        normal_mode_override: &NormalModeOverride,
        none_if_no_override: bool,
        context: &Context,
    ) -> Vec<Keymap> {
        [
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::MultC),
                "Multi Curs".to_string(),
                "Enter Multi-cursor mode".to_string(),
                Dispatch::ShowKeymapLegend(self.multicursor_mode_keymap_legend_config(context)),
            )
            .override_keymap(
                normal_mode_override.multicursor.clone().as_ref(),
                none_if_no_override,
            ),
            Keymap::new_extended(
                context.keyboard_layout_kind().get_key(&Meaning::Extnd),
                "Extend".to_string(),
                "Enter Extend Mode".to_string(),
                Dispatch::ToEditor(DispatchEditor::ShowKeymapLegendExtend),
            )
            .override_keymap(normal_mode_override.v.as_ref(), none_if_no_override),
        ]
        .into_iter()
        .flatten()
        .collect_vec()
    }

    pub(crate) fn normal_mode_keymap_legend_config(
        &self,
        context: &Context,
        normal_mode_override: Option<NormalModeOverride>,
        prior_change: Option<PriorChange>,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Normal".to_string(),
            keymaps: Keymaps::new(
                &self
                    .normal_mode_keymaps(context, normal_mode_override, prior_change)
                    .into_iter()
                    .chain(Some(Keymap::new(
                        context.keyboard_layout_kind().get_key(&Meaning::MultC),
                        "Multi-cursor".to_string(),
                        Dispatch::ShowKeymapLegend(
                            self.multicursor_mode_keymap_legend_config(context),
                        ),
                    )))
                    .chain(Some(Keymap::new(
                        context.keyboard_layout_kind().get_key(&Meaning::Extnd),
                        "Extend".to_string(),
                        Dispatch::ShowKeymapLegend(self.extend_mode_keymap_legend_config(context)),
                    )))
                    .collect_vec(),
            ),
        }
    }

    pub(crate) fn normal_mode_keymaps(
        &self,
        context: &Context,
        normal_mode_override: Option<NormalModeOverride>,
        prior_change: Option<PriorChange>,
    ) -> Vec<Keymap> {
        let normal_mode_override = normal_mode_override
            .clone()
            .or_else(|| self.normal_mode_override.clone())
            .unwrap_or_default();
        self.keymap_core_movements(context, prior_change)
            .into_iter()
            .chain(self.keymap_sub_modes(context))
            .chain(self.keymap_other_movements(context))
            .chain(self.keymap_primary_selection_modes(context, prior_change))
            .chain(self.keymap_secondary_selection_modes_init(context, prior_change))
            .chain(self.keymap_actions(&normal_mode_override, false, context, prior_change))
            .chain(self.keymap_others(context))
            .chain(self.keymap_universal(context))
            .collect_vec()
    }

    pub(crate) fn multicursor_mode_keymap_legend_config(
        &self,
        context: &Context,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Multi-cursor".to_string(),
            keymaps: Keymaps::new(
                &self
                    .normal_mode_keymaps(
                        context,
                        Some(multicursor_mode_normal_mode_override(
                            self.cursor_direction.reverse(),
                        )),
                        Some(PriorChange::EnterMultiCursorMode),
                    )
                    .into_iter()
                    .chain(Some(Keymap::new(
                        context.keyboard_layout_kind().get_key(&Meaning::MultC),
                        "Curs All".to_string(),
                        Dispatch::ToEditor(CursorAddToAllSelections),
                    )))
                    .collect_vec(),
            ),
        }
    }
    pub(crate) fn extend_mode_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Extend".to_string(),
            keymaps: Keymaps::new(
                &self
                    .normal_mode_keymaps(
                        context,
                        Some(extend_mode_normal_mode_override(context)),
                        Some(PriorChange::EnableSelectionExtension),
                    )
                    .into_iter()
                    .chain(Some(Keymap::new(
                        context.keyboard_layout_kind().get_key(&Meaning::Extnd),
                        "Select All".to_string(),
                        Dispatch::ToEditor(SelectAll),
                    )))
                    .collect_vec(),
            ),
        }
    }
    pub(crate) fn keymap_transform(&self, context: &Context) -> Vec<Keymap> {
        [
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
                context.keyboard_layout_kind().get_transform_key(&meaning),
                description.to_string(),
                Dispatch::ToEditor(Transform(Transformation::Case(case))),
            )
        })
        .chain(Some(Keymap::new(
            context
                .keyboard_layout_kind()
                .get_transform_key(&Meaning::Wrap_),
            "Wrap".to_string(),
            Dispatch::ToEditor(Transform(Transformation::Wrap)),
        )))
        .chain(Some(Keymap::new(
            context
                .keyboard_layout_kind()
                .get_transform_key(&Meaning::CmtLn),
            "Line Comment".to_string(),
            Dispatch::ToEditor(DispatchEditor::ToggleLineComment),
        )))
        .chain(Some(Keymap::new(
            context
                .keyboard_layout_kind()
                .get_transform_key(&Meaning::CmtBk),
            "Block Comment".to_string(),
            Dispatch::ToEditor(DispatchEditor::ToggleBlockComment),
        )))
        .collect_vec()
    }
    pub(crate) fn transform_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Transform".to_string(),

            keymaps: Keymaps::new(&self.keymap_transform(context)),
        }
    }

    pub(crate) fn space_pick_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Pick".to_string(),

            keymaps: Keymaps::new(
                &[
                    (
                        context
                            .keyboard_layout_kind()
                            .get_space_picker_keymap(&Meaning::Buffr),
                        "Buffer",
                        FilePickerKind::Opened,
                    ),
                    (
                        context
                            .keyboard_layout_kind()
                            .get_space_picker_keymap(&Meaning::File_),
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
                            context
                                .keyboard_layout_kind()
                                .get_space_picker_keymap(&Meaning::GitFC),
                            DiffMode::UnstagedAgainstCurrentBranch,
                        ),
                        (
                            context
                                .keyboard_layout_kind()
                                .get_space_picker_keymap(&Meaning::GitFM),
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
                    context
                        .keyboard_layout_kind()
                        .get_space_picker_keymap(&Meaning::SyblD),
                    "Symbol (Document)".to_string(),
                    Dispatch::RequestDocumentSymbols,
                )))
                .chain(Some(Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_picker_keymap(&Meaning::SyblW),
                    "Symbol (Workspace)".to_string(),
                    Dispatch::OpenWorkspaceSymbolsPrompt,
                )))
                .chain(Some(Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_picker_keymap(&Meaning::Theme),
                    "Theme".to_string(),
                    Dispatch::OpenThemePrompt,
                )))
                .collect_vec(),
            ),
        }
    }

    pub(crate) fn space_context_keymap_legend_config(
        &self,
        context: &Context,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Context".to_string(),

            keymaps: Keymaps::new(&[
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap(&Meaning::LCdAc),
                    "Code Actions".to_string(),
                    {
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
                    },
                ),
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap(&Meaning::LHovr),
                    "Hover".to_string(),
                    Dispatch::RequestHover,
                ),
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap(&Meaning::LRnme),
                    "Rename".to_string(),
                    Dispatch::PrepareRename,
                ),
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap(&Meaning::RvHkC),
                    format!(
                        "Revert Hunk{}",
                        DiffMode::UnstagedAgainstCurrentBranch.display()
                    ),
                    Dispatch::ToEditor(DispatchEditor::RevertHunk(
                        DiffMode::UnstagedAgainstCurrentBranch,
                    )),
                ),
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap(&Meaning::RvHkM),
                    format!(
                        "Revert Hunk{}",
                        DiffMode::UnstagedAgainstMainBranch.display()
                    ),
                    Dispatch::ToEditor(DispatchEditor::RevertHunk(
                        DiffMode::UnstagedAgainstMainBranch,
                    )),
                ),
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap(&Meaning::GtBlm),
                    "Git Blame".to_string(),
                    Dispatch::ToEditor(DispatchEditor::GitBlame),
                ),
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap(&Meaning::GoFil),
                    "Go to File".to_string(),
                    Dispatch::ToEditor(DispatchEditor::GoToFile),
                ),
            ]),
        }
    }

    pub(crate) fn space_editor_keymap_legend_config(
        &self,
        context: &Context,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Editor".to_string(),

            keymaps: Keymaps::new(&[
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_editor_keymap(&Meaning::RplcA),
                    "Replace all".to_string(),
                    Dispatch::Replace {
                        scope: Scope::Global,
                    },
                ),
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_editor_keymap(&Meaning::TSNSx),
                    "TS Node Sexp".to_string(),
                    Dispatch::ToEditor(DispatchEditor::ShowCurrentTreeSitterNodeSexp),
                ),
                Keymap::new(
                    "enter",
                    "Force Save".to_string(),
                    Dispatch::ToEditor(DispatchEditor::ForceSave),
                ),
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_editor_keymap(&Meaning::SaveA),
                    "Save All".to_string(),
                    Dispatch::SaveAll,
                ),
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_editor_keymap(&Meaning::QSave),
                    "Save All Quit".to_string(),
                    Dispatch::SaveQuitAll,
                ),
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_editor_keymap(&Meaning::QNSav),
                    "Quit No Save".to_string(),
                    Dispatch::QuitAll,
                ),
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_editor_keymap(&Meaning::Pipe_),
                    "Pipe".to_string(),
                    Dispatch::OpenPipeToShellPrompt,
                ),
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_editor_keymap(&Meaning::RlBfr),
                    "Reload File".to_string(),
                    Dispatch::ToEditor(ReloadFile { force: false }),
                ),
            ]),
        }
    }

    pub(crate) fn space_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Space".to_string(),

            keymaps: Keymaps::new(
                &[
                    Keymap::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap(&Meaning::RevlS),
                        "÷ Selection".to_string(),
                        Dispatch::ToEditor(DispatchEditor::ToggleReveal(
                            Reveal::CurrentSelectionMode,
                        )),
                    ),
                    Keymap::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap(&Meaning::RevlC),
                        "÷ Cursor".to_string(),
                        Dispatch::ToEditor(DispatchEditor::ToggleReveal(Reveal::Cursor)),
                    ),
                    Keymap::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap(&Meaning::RevlM),
                        "÷ Mark".to_string(),
                        Dispatch::ToEditor(DispatchEditor::ToggleReveal(Reveal::Mark)),
                    ),
                    Keymap::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap(&Meaning::SpEdt),
                        "Editor".to_string(),
                        Dispatch::ShowKeymapLegend(self.space_editor_keymap_legend_config(context)),
                    ),
                    Keymap::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap(&Meaning::SpCtx),
                        "Context".to_string(),
                        Dispatch::ShowKeymapLegend(
                            self.space_context_keymap_legend_config(context),
                        ),
                    ),
                    Keymap::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap(&Meaning::SpPck),
                        "Pick".to_string(),
                        Dispatch::ShowKeymapLegend(self.space_pick_keymap_legend_config(context)),
                    ),
                    Keymap::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap(&Meaning::SHelp),
                        "Help".to_string(),
                        Dispatch::ToEditor(DispatchEditor::ShowHelp),
                    ),
                    Keymap::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap(&Meaning::Explr),
                        "Explorer".to_string(),
                        Dispatch::RevealInExplorer(
                            self.path()
                                .unwrap_or_else(|| context.current_working_directory().clone()),
                        ),
                    ),
                ]
                .into_iter()
                .chain(
                    self.secondary_selection_modes_keymap_legend_config(
                        context,
                        Scope::Global,
                        IfCurrentNotFound::LookForward,
                        None,
                    )
                    .keymaps
                    .into_vec(),
                )
                .collect_vec(),
            ),
        }
    }

    fn search_current_selection_keymap(
        &self,
        context: &Context,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
    ) -> Keymap {
        Keymap::new_extended(
            context.keyboard_layout_kind().get_key(&Meaning::SrchC),
            "This".to_string(),
            "Search current selection".to_string(),
            Dispatch::ToEditor(DispatchEditor::SearchCurrentSelection(
                if_current_not_found,
                scope,
            )),
        )
    }

    pub(crate) fn secondary_selection_modes_keymap_legend_config(
        &self,
        context: &Context,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        prior_change: Option<PriorChange>,
    ) -> KeymapLegendConfig {
        let search_keymaps = {
            [].into_iter()
                .chain(
                    [Keymap::new(
                        match (scope, if_current_not_found) {
                            (Scope::Local, IfCurrentNotFound::LookForward) => context
                                .keyboard_layout_kind()
                                .get_find_keymap(scope, &Meaning::LRept),
                            (Scope::Local, IfCurrentNotFound::LookBackward) => context
                                .keyboard_layout_kind()
                                .get_find_keymap(scope, &Meaning::LRept),
                            (Scope::Global, _) => context
                                .keyboard_layout_kind()
                                .get_find_keymap(scope, &Meaning::GRept),
                        },
                        "Repeat".to_string(),
                        Dispatch::UseLastNonContiguousSelectionMode(if_current_not_found),
                    )]
                    .to_vec(),
                )
                .collect_vec()
        };
        let misc_keymaps = [
            Keymap::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap(scope, &Meaning::Mark_),
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
            Keymap::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap(scope, &Meaning::Qkfix),
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
                (Meaning::GHnkC, DiffMode::UnstagedAgainstCurrentBranch),
                (Meaning::GHnkM, DiffMode::UnstagedAgainstMainBranch),
            ]
            .map(|(meaning, diff_mode)| {
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_find_keymap(scope, &meaning),
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
                context
                    .keyboard_layout_kind()
                    .get_find_keymap(scope, &meaning),
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
        let lsp_keymaps = [
            Keymap::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap(scope, &Meaning::LDefn),
                "Def".to_string(),
                Dispatch::RequestDefinitions(scope),
            ),
            Keymap::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap(scope, &Meaning::LDecl),
                "Decl".to_string(),
                Dispatch::RequestDeclarations(scope),
            ),
            Keymap::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap(scope, &Meaning::LImpl),
                "Impl".to_string(),
                Dispatch::RequestImplementations(scope),
            ),
            Keymap::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap(scope, &Meaning::LRfrE),
                "Ref-".to_string(),
                Dispatch::RequestReferences {
                    include_declaration: false,
                    scope,
                },
            ),
            Keymap::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap(scope, &Meaning::LRfrI),
                "Ref+".to_string(),
                Dispatch::RequestReferences {
                    include_declaration: true,
                    scope,
                },
            ),
            Keymap::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap(scope, &Meaning::LType),
                "Type".to_string(),
                Dispatch::RequestTypeDefinitions(scope),
            ),
        ];
        let scope_specific_keymaps = match scope {
            Scope::Local => [(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap(scope, &Meaning::NtrlN),
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
                let dispatch = Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    if_current_not_found,
                    Find { search },
                    prior_change,
                ));
                Keymap::new(key, description.to_string(), dispatch)
            })
            .chain([
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_find_keymap(scope, &Meaning::OneCh),
                    "One".to_string(),
                    Dispatch::ToEditor(FindOneChar(if_current_not_found)),
                ),
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_find_keymap(scope, &Meaning::RSrch),
                    Direction::End.format_action("Repeat Search"),
                    Dispatch::ToEditor(DispatchEditor::RepeatSearch(
                        Scope::Local,
                        self.cursor_direction.reverse().to_if_current_not_found(),
                        prior_change,
                    )),
                ),
            ])
            .collect_vec(),
            Scope::Global => [
                Keymap::new_extended(
                    context
                        .keyboard_layout_kind()
                        .get_find_keymap(scope, &Meaning::Srch_),
                    "Search".to_string(),
                    "Search".to_string(),
                    Dispatch::OpenSearchPrompt {
                        scope,
                        if_current_not_found,
                    },
                ),
                Keymap::new(
                    context.keyboard_layout_kind().get_key(&Meaning::SchWC),
                    "With".to_string(),
                    Dispatch::OpenSearchPromptWithCurrentSelection {
                        scope,
                        prior_change,
                    },
                ),
                Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_find_keymap(scope, &Meaning::RSrch),
                    "Repeat Search".to_string(),
                    Dispatch::ToEditor(DispatchEditor::RepeatSearch(
                        Scope::Global,
                        IfCurrentNotFound::LookForward,
                        prior_change,
                    )),
                ),
            ]
            .into_iter()
            .chain(Some(self.search_current_selection_keymap(
                context,
                scope,
                if_current_not_found,
            )))
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

            keymaps: Keymaps::new(
                &search_keymaps
                    .into_iter()
                    .chain(misc_keymaps)
                    .chain(diagnostics_keymaps)
                    .chain(lsp_keymaps)
                    .chain(scope_specific_keymaps)
                    .collect_vec(),
            ),
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

fn generate_enclosures_keymaps(
    get_dispatch: impl Fn(EnclosureKind) -> Dispatch,
    context: &Context,
) -> Keymaps {
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
                context.keyboard_layout_kind().get_surround_keymap(&meaning),
                format!("{open} {close}"),
                get_dispatch(enclosure),
            )
        })
        .collect_vec(),
    )
}

pub(crate) fn extend_mode_normal_mode_override(context: &Context) -> NormalModeOverride {
    fn select_surround_keymap_legend_config(
        kind: SurroundKind,
        context: &Context,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: format!("Select Surround ({kind:?})"),

            keymaps: generate_enclosures_keymaps(
                |enclosure| {
                    Dispatch::ToEditor(SelectSurround {
                        enclosure,
                        kind: kind.clone(),
                    })
                },
                context,
            ),
        }
    }

    fn delete_surround_keymap_legend_config(context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Delete Surround".to_string(),

            keymaps: generate_enclosures_keymaps(
                |enclosure| Dispatch::ToEditor(DeleteSurround(enclosure)),
                context,
            ),
        }
    }

    fn surround_keymap_legend_config(context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Surround".to_string(),

            keymaps: Keymaps::new(
                &generate_enclosures_keymaps(
                    |enclosure| {
                        let (open, close) = enclosure.open_close_symbols_str();
                        Dispatch::ToEditor(Surround(open.to_string(), close.to_string()))
                    },
                    context,
                )
                .into_vec()
                .into_iter()
                .chain(Some(Keymap::new(
                    context
                        .keyboard_layout_kind()
                        .get_surround_keymap(&Meaning::XML__),
                    "<></>".to_string(),
                    Dispatch::OpenSurroundXmlPrompt,
                )))
                .collect_vec(),
            ),
        }
    }

    fn change_surround_from_keymap_legend_config(
        context: &Context,
    ) -> super::keymap_legend::KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Change Surround from:".to_string(),

            keymaps: generate_enclosures_keymaps(
                |enclosure| {
                    Dispatch::ShowKeymapLegend(change_surround_to_keymap_legend_config(
                        enclosure, context,
                    ))
                },
                context,
            ),
        }
    }

    fn change_surround_to_keymap_legend_config(
        from: EnclosureKind,
        context: &Context,
    ) -> super::keymap_legend::KeymapLegendConfig {
        KeymapLegendConfig {
            title: format!("Change Surround from {} to:", from.to_str()),

            keymaps: generate_enclosures_keymaps(
                |enclosure| {
                    Dispatch::ToEditor(ChangeSurround {
                        from,
                        to: enclosure,
                    })
                },
                context,
            ),
        }
    }
    NormalModeOverride {
        insert: Some(KeymapOverride {
            description: "Inside",
            dispatch: Dispatch::ShowKeymapLegend(select_surround_keymap_legend_config(
                SurroundKind::Inside,
                context,
            )),
        }),
        append: Some(KeymapOverride {
            description: "Around",
            dispatch: Dispatch::ShowKeymapLegend(select_surround_keymap_legend_config(
                SurroundKind::Around,
                context,
            )),
        }),
        delete: Some(KeymapOverride {
            description: "Delete Surround",
            dispatch: Dispatch::ShowKeymapLegend(delete_surround_keymap_legend_config(context)),
        }),
        change: Some(KeymapOverride {
            description: "Change Surround",
            dispatch: Dispatch::ShowKeymapLegend(change_surround_from_keymap_legend_config(
                context,
            )),
        }),
        open: Some(KeymapOverride {
            description: "Surround",
            dispatch: Dispatch::ShowKeymapLegend(surround_keymap_legend_config(context)),
        }),
        v: Some(KeymapOverride {
            description: "Select All",
            dispatch: Dispatch::ToEditor(SelectAll),
        }),
        ..Default::default()
    }
}

pub(crate) fn multicursor_mode_normal_mode_override(direction: Direction) -> NormalModeOverride {
    NormalModeOverride {
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
            description: "Delete Curs",
            dispatch: Dispatch::ToEditor(DeleteCurrentCursor(direction)),
        }),
        multicursor: Some(KeymapOverride {
            description: "Curs All",
            dispatch: Dispatch::ToEditor(CursorAddToAllSelections),
        }),
        ..Default::default()
    }
}
