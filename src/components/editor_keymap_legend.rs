use crossterm::event::{KeyCode, KeyEventKind};
use SelectionMode::*;

use convert_case::Case;
use event::KeyEvent;
use itertools::Itertools;

use crate::{
    app::{Dispatch, Dispatches, FilePickerKind, Scope},
    components::{
        editor::{Movement, PriorChange},
        keymap_legend::{MomentaryLayer, OnTap},
    },
    context::{Context, LocalSearchConfigMode, Search},
    git::DiffMode,
    list::grep::RegexConfig,
    quickfix_list::{DiagnosticSeverityRange, QuickfixListType},
    scripting::{custom_keymap, leader_meanings},
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
    keymap_legend::{Keybinding, Keymap, KeymapLegendConfig},
};

use DispatchEditor::*;
use Movement::*;
impl Editor {
    pub fn keymap_core_movements(
        &self,
        context: &Context,
        prior_change: Option<PriorChange>,
    ) -> Vec<Keybinding> {
        [
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Left_),
                "◀".to_string(),
                "Left".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Left, prior_change)),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Right),
                "▶".to_string(),
                "Right".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Right, prior_change)),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Up___),
                "▲".to_string(),
                "Up".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Up, prior_change)),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Down_),
                "▼".to_string(),
                "Down".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Down, prior_change)),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::First),
                "First".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::First, prior_change)),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Last_),
                "Last".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Last, prior_change)),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Next_),
                "Next".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(Movement::Next, prior_change)),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Prev_),
                "Previous".to_string(),
                Dispatch::ToEditor(MoveSelectionWithPriorChange(
                    Movement::Previous,
                    prior_change,
                )),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Jump_),
                "Jump".to_string(),
                "Jump".to_string(),
                Dispatch::ToEditor(DispatchEditor::ShowJumps {
                    use_current_selection_mode: true,
                    prior_change,
                }),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::ToIdx),
                "Index".to_string(),
                "To Index (1-based)".to_string(),
                Dispatch::OpenMoveToIndexPrompt(prior_change),
            ),
        ]
        .to_vec()
    }

    pub fn keymap_other_movements(&self, context: &Context) -> Vec<Keybinding> {
        [
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::ScrlD),
                "Scroll ↓".to_string(),
                "Scroll down".to_string(),
                Dispatch::ToEditor(ScrollPageDown),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::ScrlU),
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
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::NBack),
                Direction::Start.format_action("Nav"),
                "Navigate back".to_string(),
                Dispatch::NavigateBack,
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::NForw),
                Direction::End.format_action("Nav"),
                "Navigate forward".to_string(),
                Dispatch::NavigateForward,
            ),
            Keybinding::momentary_layer(
                context,
                MomentaryLayer {
                    meaning: Meaning::Mark_,
                    description: "Mark".to_string(),
                    config: KeymapLegendConfig {
                        title: "Marked File Keymap".to_string(),
                        keymap: marked_file_keymap(context),
                    },
                    on_tap: Some(OnTap::new(
                        "Toggle Selection Mark",
                        Dispatches::one(Dispatch::MarkFileAndToggleMark),
                    )),
                },
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::SSEnd),
                "⇋ Anchor".to_string(),
                "Swap Anchor".to_string(),
                Dispatch::ToEditor(SwapExtensionAnchor),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::XAchr),
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
        context: &Context,
        prior_change: Option<PriorChange>,
    ) -> Vec<Keybinding> {
        let direction = self.cursor_direction.reverse().to_if_current_not_found();
        [
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Line_),
                "Line".to_string(),
                "Select Line".to_string(),
                Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    direction,
                    Line,
                    prior_change,
                )),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::LineF),
                "Line*".to_string(),
                "Select Line*".to_string(),
                Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    direction,
                    LineFull,
                    prior_change,
                )),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Sytx_),
                "Syntax".to_string(),
                "Select Syntax Node".to_string(),
                Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    direction,
                    SyntaxNode,
                    prior_change,
                )),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::FStyx),
                "Syntax*".to_string(),
                "Select Syntax Node*".to_string(),
                Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    direction,
                    SyntaxNodeFine,
                    prior_change,
                )),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Word_),
                "Word".to_string(),
                "Select Word".to_string(),
                Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    direction,
                    Word,
                    prior_change,
                )),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::BWord),
                "Word*".to_string(),
                "Select Big Word".to_string(),
                Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    direction,
                    BigWord,
                    prior_change,
                )),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::SWord),
                "Subword".to_string(),
                "Select Subword".to_string(),
                Dispatch::ToEditor(SetSelectionModeWithPriorChange(
                    direction,
                    Subword,
                    prior_change,
                )),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Char_),
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

    pub fn keymap_secondary_selection_modes_init(
        &self,
        context: &Context,
        prior_change: Option<PriorChange>,
    ) -> Vec<Keybinding> {
        [Keybinding::new_extended(
            context
                .keyboard_layout_kind()
                .get_normal_keymap_keybinding(&Meaning::LSrch),
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

    pub fn keymap_actions(
        &self,
        normal_mode_override: &NormalModeOverride,
        none_if_no_override: bool,
        context: &Context,
        _prior_change: Option<PriorChange>,
    ) -> Vec<Keybinding> {
        [
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Join_),
                "Join".to_string(),
                "Join".to_string(),
                Dispatch::ToEditor(JoinSelection),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Break),
                "Break".to_string(),
                "Break".to_string(),
                Dispatch::ToEditor(BreakSelection),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::AgSlL),
                Direction::Start.format_action("Align"),
                Dispatch::ToEditor(AlignSelections(Direction::Start)),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::AgSlR),
                Direction::End.format_action("Align"),
                Dispatch::ToEditor(AlignSelections(Direction::End)),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Raise),
                "Raise".to_string(),
                "Raise".to_string(),
                Dispatch::ToEditor(Replace(Expand)),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Undo_),
                "Undo".to_string(),
                "Undo".to_string(),
                Dispatch::ToEditor(Undo),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Redo_),
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
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Trsfm),
                "Transform".to_string(),
                "Transform".to_string(),
                Dispatch::ShowKeymapLegend(self.transform_keymap_legend_config(context)),
            ),
            Keybinding::new(
                "$",
                Direction::End.format_action("Collapse selection"),
                Dispatch::ToEditor(DispatchEditor::CollapseSelection(Direction::End)),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Indnt),
                "Indent".to_string(),
                "Indent".to_string(),
                Dispatch::ToEditor(Indent),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::DeDnt),
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
        .chain(self.keymap_actions_overridable(normal_mode_override, none_if_no_override, context))
        .chain(self.keymap_clipboard_related_actions(false, normal_mode_override.clone(), context))
        .collect_vec()
    }

    pub fn keymap_actions_overridable(
        &self,
        normal_mode_override: &NormalModeOverride,
        none_if_no_override: bool,
        context: &Context,
    ) -> Vec<Keybinding> {
        [
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Chng_),
                "Change".to_string(),
                "Change".to_string(),
                Dispatch::ToEditor(Change),
            )
            .override_keymap(normal_mode_override.change.as_ref(), none_if_no_override),
            Keybinding::momentary_layer(
                context,
                MomentaryLayer {
                    meaning: Meaning::Delte,
                    description: "Delete".to_string(),
                    config: KeymapLegendConfig {
                        title: "Delete".to_string(),
                        keymap: delete_keymap(context),
                    },
                    on_tap: Some(OnTap::new(
                        "Delete One",
                        Dispatches::one(Dispatch::ToEditor(DispatchEditor::DeleteOne)),
                    )),
                },
            )
            .override_keymap(normal_mode_override.delete.as_ref(), none_if_no_override),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::InstP),
                Direction::Start.format_action("Insert"),
                Direction::Start.format_action("Insert"),
                Dispatch::ToEditor(EnterInsertMode(Direction::Start)),
            )
            .override_keymap(normal_mode_override.insert.as_ref(), none_if_no_override),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::InstN),
                Direction::End.format_action("Insert"),
                Direction::End.format_action("Insert"),
                Dispatch::ToEditor(EnterInsertMode(Direction::End)),
            )
            .override_keymap(normal_mode_override.append.as_ref(), none_if_no_override),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Open_),
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

    pub fn keymap_overridable(
        &self,
        normal_mode_override: &NormalModeOverride,
        none_if_no_override: bool,
        context: &Context,
    ) -> Vec<Keybinding> {
        self.keymap_actions_overridable(normal_mode_override, none_if_no_override, context)
            .into_iter()
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
    ) -> Vec<Keybinding> {
        let extra = if use_system_clipboard { "+ " } else { "" };
        let format = |description: &str| format!("{extra}{description}");
        [
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::ChngX),
                format("Change X"),
                format!("{}{}", "Change Cut", extra),
                Dispatch::ToEditor(ChangeCut),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Copy_),
                format("Copy"),
                format!("{}{}", "Copy", extra),
                Dispatch::ToEditor(Copy),
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
    ) -> Vec<Keybinding> {
        let extra = if use_system_clipboard { "+ " } else { "" };
        let format = |description: &str| format!("{extra}{description}");
        [
            Keybinding::momentary_layer(
                context,
                MomentaryLayer {
                    meaning: Meaning::Paste,
                    description: format("Paste"),
                    config: KeymapLegendConfig {
                        title: "Paste".to_string(),
                        keymap: paste_keymap(context),
                    },
                    on_tap: Some(OnTap::new(
                        "Replace",
                        Dispatches::one(Dispatch::ToEditor(
                            DispatchEditor::ReplaceWithCopiedText { cut: false },
                        )),
                    )),
                },
            )
            .override_keymap(
                normal_mode_override.paste.clone().as_ref(),
                none_if_no_override,
            ),
            Keybinding::momentary_layer(
                context,
                MomentaryLayer {
                    meaning: Meaning::Cut__,
                    description: "Cut".to_string(),
                    config: KeymapLegendConfig {
                        title: "Cut".to_string(),
                        keymap: cut_keymap(context),
                    },
                    on_tap: Some(OnTap::new(
                        "Cut One",
                        Dispatches::one(Dispatch::ToEditor(DispatchEditor::CutOne)),
                    )),
                },
            )
            .override_keymap(normal_mode_override.cut.as_ref(), none_if_no_override),
        ]
        .into_iter()
        .flatten()
        .collect_vec()
    }

    pub fn keymap_universal(&self, context: &Context) -> Vec<Keybinding> {
        [
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::WClse),
                "Close".to_string(),
                "Close current window".to_string(),
                Dispatch::CloseCurrentWindow,
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::SView),
                "⇋ Align View".to_string(),
                "Switch view alignment".to_string(),
                Dispatch::ToEditor(SwitchViewAlignment),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::WSwth),
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
        context: &Context,
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
                        context
                            .keyboard_layout_kind()
                            .get_insert_keymap_keybinding(&Meaning::LineP),
                        "Line ←".to_string(),
                        "Move to line start".to_string(),
                        Dispatch::ToEditor(MoveToLineStart),
                    ),
                    Keybinding::new_extended(
                        context
                            .keyboard_layout_kind()
                            .get_insert_keymap_keybinding(&Meaning::LineN),
                        "Line →".to_string(),
                        "Move to line end".to_string(),
                        Dispatch::ToEditor(MoveToLineEnd),
                    ),
                    Keybinding::new_extended(
                        context
                            .keyboard_layout_kind()
                            .get_insert_keymap_keybinding(&Meaning::KilLP),
                        "Kill Line ←".to_string(),
                        Direction::End.format_action("Kill line"),
                        Dispatch::ToEditor(KillLine(Direction::Start)),
                    ),
                    Keybinding::new_extended(
                        context
                            .keyboard_layout_kind()
                            .get_insert_keymap_keybinding(&Meaning::KilLN),
                        "Kill Line →".to_string(),
                        Direction::End.format_action("Kill line"),
                        Dispatch::ToEditor(KillLine(Direction::End)),
                    ),
                    Keybinding::new_extended(
                        context
                            .keyboard_layout_kind()
                            .get_insert_keymap_keybinding(&Meaning::DWrdP),
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
                    self.keymap_universal(context)
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
        if let Some(dispatches) = self
            .insert_mode_keymap(true, context)
            .iter()
            .find(|keymap| keymap.event().is_press_or_repeat_equivalent(&event))
            .map(|keymap| keymap.get_dispatches())
        {
            Ok(dispatches)
        } else if let (KeyCode::Char(c), KeyEventKind::Press | KeyEventKind::Repeat) =
            (event.code, event.kind)
        {
            self.insert(&c.to_string(), context)
        } else {
            Ok(Default::default())
        }
    }

    pub fn handle_universal_key(
        &mut self,
        event: KeyEvent,
        context: &Context,
    ) -> anyhow::Result<HandleEventResult> {
        if let Some(keymap) = Keymap::new(&self.keymap_universal(context)).get(&event) {
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

    pub fn keymap_sub_modes(&self, context: &Context) -> Vec<Keybinding> {
        [
            Some(Keybinding::new(
                "~",
                "Replace".to_string(),
                Dispatch::ToEditor(EnterReplaceMode),
            )),
            Some(Keybinding::momentary_layer(
                context,
                MomentaryLayer {
                    meaning: Meaning::Swap_,
                    description: "Swap".to_string(),
                    config: KeymapLegendConfig {
                        title: "Swap".to_string(),
                        keymap: swap_keymap(context),
                    },
                    on_tap: None,
                },
            )),
            Some(Keybinding::new(
                "backslash",
                "Leader".to_string(),
                Dispatch::ShowKeymapLegend(self.leader_keymap_legend_config(context)),
            )),
            Some(Keybinding::momentary_layer(
                context,
                MomentaryLayer {
                    meaning: Meaning::MultC,
                    description: "Multi-cursor".to_string(),
                    config: KeymapLegendConfig {
                        title: "Multi-cursor".to_string(),
                        keymap: self.multicursor_keymap(context),
                    },
                    on_tap: Some(OnTap::new(
                        "Curs All",
                        Dispatches::one(Dispatch::ToEditor(CursorAddToAllSelections)),
                    )),
                },
            )),
        ]
        .into_iter()
        .flatten()
        .collect_vec()
    }

    pub fn normal_mode_keymap_legend_config(
        &self,
        context: &Context,
        normal_mode_override: Option<NormalModeOverride>,
        prior_change: Option<PriorChange>,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Normal".to_string(),
            keymap: Keymap::new(
                &self
                    .normal_mode_keymap(context, normal_mode_override, prior_change)
                    .into_iter()
                    .chain(Some(Keybinding::new(
                        context
                            .keyboard_layout_kind()
                            .get_normal_keymap_keybinding(&Meaning::Extnd),
                        "Extend".to_string(),
                        Dispatch::ShowKeymapLegend(self.extend_mode_keymap_legend_config(context)),
                    )))
                    .collect_vec(),
            ),
        }
    }

    pub fn normal_mode_keymap(
        &self,
        context: &Context,
        normal_mode_override: Option<NormalModeOverride>,
        prior_change: Option<PriorChange>,
    ) -> Vec<Keybinding> {
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
            .chain(self.keymap_others())
            .chain(self.keymap_universal(context))
            .collect_vec()
    }
    pub fn extend_mode_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Extend".to_string(),
            keymap: Keymap::new(
                &self
                    .normal_mode_keymap(
                        context,
                        Some(extend_mode_normal_mode_override(context)),
                        Some(PriorChange::EnableSelectionExtension),
                    )
                    .into_iter()
                    .chain(Some(Keybinding::new(
                        context
                            .keyboard_layout_kind()
                            .get_normal_keymap_keybinding(&Meaning::Extnd),
                        "Select All".to_string(),
                        Dispatch::ToEditor(SelectAll),
                    )))
                    .collect_vec(),
            ),
        }
    }
    pub fn keymap_transform(&self, context: &Context) -> Vec<Keybinding> {
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
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_transform_keymap_keybinding(&meaning),
                description.to_string(),
                Dispatch::ToEditor(Transform(Transformation::Case(case))),
            )
        })
        .chain(Some(Keybinding::new(
            context
                .keyboard_layout_kind()
                .get_transform_keymap_keybinding(&Meaning::Wrap_),
            "Wrap".to_string(),
            Dispatch::ToEditor(Transform(Transformation::Wrap)),
        )))
        .chain(Some(Keybinding::new(
            context
                .keyboard_layout_kind()
                .get_transform_keymap_keybinding(&Meaning::Unwrp),
            "Unwrap".to_string(),
            Dispatch::ToEditor(Transform(Transformation::Unwrap)),
        )))
        .chain(Some(Keybinding::new(
            context
                .keyboard_layout_kind()
                .get_transform_keymap_keybinding(&Meaning::CmtLn),
            "Line Comment".to_string(),
            Dispatch::ToEditor(DispatchEditor::ToggleLineComment),
        )))
        .chain(Some(Keybinding::new(
            context
                .keyboard_layout_kind()
                .get_transform_keymap_keybinding(&Meaning::CmtBk),
            "Block Comment".to_string(),
            Dispatch::ToEditor(DispatchEditor::ToggleBlockComment),
        )))
        .collect_vec()
    }
    pub fn transform_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Transform".to_string(),

            keymap: Keymap::new(&self.keymap_transform(context)),
        }
    }

    pub fn space_pick_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Pick".to_string(),

            keymap: Keymap::new(
                &[
                    (
                        context
                            .keyboard_layout_kind()
                            .get_space_picker_keymap_keybinding(&Meaning::Buffr),
                        "Buffer",
                        FilePickerKind::Opened,
                    ),
                    (
                        context
                            .keyboard_layout_kind()
                            .get_space_picker_keymap_keybinding(&Meaning::File_),
                        "File",
                        FilePickerKind::NonGitIgnored,
                    ),
                ]
                .into_iter()
                .map(|(key, description, kind)| {
                    Keybinding::new(key, description.to_string(), Dispatch::OpenFilePicker(kind))
                })
                .chain(
                    [
                        (
                            context
                                .keyboard_layout_kind()
                                .get_space_picker_keymap_keybinding(&Meaning::GitFC),
                            DiffMode::UnstagedAgainstCurrentBranch,
                        ),
                        (
                            context
                                .keyboard_layout_kind()
                                .get_space_picker_keymap_keybinding(&Meaning::GitFM),
                            DiffMode::UnstagedAgainstMainBranch,
                        ),
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
                    context
                        .keyboard_layout_kind()
                        .get_space_picker_keymap_keybinding(&Meaning::SyblD),
                    "Symbol (Document)".to_string(),
                    Dispatch::RequestDocumentSymbols,
                )))
                .chain(Some(Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_picker_keymap_keybinding(&Meaning::SyblW),
                    "Symbol (Workspace)".to_string(),
                    Dispatch::OpenWorkspaceSymbolsPrompt,
                )))
                .chain(Some(Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_picker_keymap_keybinding(&Meaning::Theme),
                    "Theme".to_string(),
                    Dispatch::OpenThemePrompt,
                )))
                .collect_vec(),
            ),
        }
    }

    pub fn space_context_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Context".to_string(),

            keymap: Keymap::new(&[
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap_keybinding(&Meaning::LCdAc),
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
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap_keybinding(&Meaning::LHovr),
                    "Hover".to_string(),
                    Dispatch::RequestHover,
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap_keybinding(&Meaning::LRnme),
                    "Rename".to_string(),
                    Dispatch::PrepareRename,
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap_keybinding(&Meaning::RvHkC),
                    format!(
                        "Revert Hunk{}",
                        DiffMode::UnstagedAgainstCurrentBranch.display()
                    ),
                    Dispatch::ToEditor(DispatchEditor::RevertHunk(
                        DiffMode::UnstagedAgainstCurrentBranch,
                    )),
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap_keybinding(&Meaning::RvHkM),
                    format!(
                        "Revert Hunk{}",
                        DiffMode::UnstagedAgainstMainBranch.display()
                    ),
                    Dispatch::ToEditor(DispatchEditor::RevertHunk(
                        DiffMode::UnstagedAgainstMainBranch,
                    )),
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap_keybinding(&Meaning::GtBlm),
                    "Git Blame".to_string(),
                    Dispatch::ToEditor(DispatchEditor::GitBlame),
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap_keybinding(&Meaning::GoFil),
                    "Go to File".to_string(),
                    Dispatch::ToEditor(DispatchEditor::GoToFile),
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap_keybinding(&Meaning::CpAbP),
                    "Copy Absolute Path".to_string(),
                    Dispatch::ToEditor(DispatchEditor::CopyAbsolutePath),
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap_keybinding(&Meaning::CpReP),
                    "Copy Relative Path".to_string(),
                    Dispatch::ToEditor(DispatchEditor::CopyRelativePath),
                ),
            ]),
        }
    }

    pub fn space_editor_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Editor".to_string(),

            keymap: Keymap::new(&[
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_editor_keymap_keybinding(&Meaning::RplcA),
                    "Replace all".to_string(),
                    Dispatch::Replace {
                        scope: Scope::Global,
                    },
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_context_keymap_keybinding(&Meaning::TSNSx),
                    "TS Node Sexp".to_string(),
                    Dispatch::ToEditor(DispatchEditor::ShowCurrentTreeSitterNodeSexp),
                ),
                Keybinding::new(
                    "enter",
                    "Force Save".to_string(),
                    Dispatch::ToEditor(DispatchEditor::ForceSave),
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_editor_keymap_keybinding(&Meaning::SaveA),
                    "Save All".to_string(),
                    Dispatch::SaveAll,
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_editor_keymap_keybinding(&Meaning::QNSav),
                    "Quit No Save".to_string(),
                    Dispatch::QuitAll,
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_editor_keymap_keybinding(&Meaning::Pipe_),
                    "Pipe".to_string(),
                    Dispatch::OpenPipeToShellPrompt,
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_editor_keymap_keybinding(&Meaning::CWDir),
                    "Change Work Dir".to_string(),
                    Dispatch::OpenChangeWorkingDirectoryPrompt,
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_space_editor_keymap_keybinding(&Meaning::RlBfr),
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
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap_keybinding(&Meaning::RevlS),
                        "÷ Selection".to_string(),
                        Dispatch::ToEditor(DispatchEditor::ToggleReveal(
                            Reveal::CurrentSelectionMode,
                        )),
                    ),
                    Keybinding::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap_keybinding(&Meaning::RevlC),
                        "÷ Cursor".to_string(),
                        Dispatch::ToEditor(DispatchEditor::ToggleReveal(Reveal::Cursor)),
                    ),
                    Keybinding::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap_keybinding(&Meaning::RevlM),
                        "÷ Mark".to_string(),
                        Dispatch::ToEditor(DispatchEditor::ToggleReveal(Reveal::Mark)),
                    ),
                    Keybinding::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap_keybinding(&Meaning::SpEdt),
                        "Editor".to_string(),
                        Dispatch::ShowKeymapLegend(self.space_editor_keymap_legend_config(context)),
                    ),
                    Keybinding::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap_keybinding(&Meaning::SpCtx),
                        "Context".to_string(),
                        Dispatch::ShowKeymapLegend(
                            self.space_context_keymap_legend_config(context),
                        ),
                    ),
                    Keybinding::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap_keybinding(&Meaning::SpPck),
                        "Pick".to_string(),
                        Dispatch::ShowKeymapLegend(self.space_pick_keymap_legend_config(context)),
                    ),
                    Keybinding::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap_keybinding(&Meaning::SHelp),
                        "Help".to_string(),
                        Dispatch::ToEditor(DispatchEditor::ShowHelp),
                    ),
                    Keybinding::new(
                        context
                            .keyboard_layout_kind()
                            .get_space_keymap_keybinding(&Meaning::Explr),
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
                    .keymap
                    .into_vec(),
                )
                .collect_vec(),
            ),
        }
    }

    pub fn leader_keymap_legend_config(&self, context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Leader".to_string(),

            keymap: Keymap::new(
                &leader_meanings()
                    .into_iter()
                    .filter_map(|meaning| {
                        let (_, description, _) = custom_keymap()
                            .into_iter()
                            .find(|(m, _, _)| &meaning == m)?;
                        Some(Keybinding::new(
                            context
                                .keyboard_layout_kind()
                                .get_leader_keymap_keybinding(&meaning),
                            description.to_string(),
                            Dispatch::ExecuteLeaderMeaning(meaning),
                        ))
                    })
                    .collect_vec(),
            ),
        }
    }

    fn search_current_keymap(
        &self,
        context: &Context,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
    ) -> Vec<Keybinding> {
        [
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap_keybinding(scope, &Meaning::SchCS),
                "Search This".to_string(),
                "Search current selection".to_string(),
                Dispatch::ToEditor(DispatchEditor::SearchCurrentSelection(
                    if_current_not_found,
                    scope,
                )),
            ),
            Keybinding::new_extended(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap_keybinding(scope, &Meaning::SchCC),
                "Search Clipboard".to_string(),
                "Search clipboard content".to_string(),
                Dispatch::ToEditor(DispatchEditor::SearchClipboardContent(scope)),
            ),
        ]
        .to_vec()
    }

    pub fn secondary_selection_modes_keymap_legend_config(
        &self,
        context: &Context,
        scope: Scope,
        if_current_not_found: IfCurrentNotFound,
        prior_change: Option<PriorChange>,
    ) -> KeymapLegendConfig {
        let search_keybindings = {
            [].into_iter()
                .chain(
                    [Keybinding::new(
                        match (scope, if_current_not_found) {
                            (Scope::Local, IfCurrentNotFound::LookForward) => context
                                .keyboard_layout_kind()
                                .get_find_keymap_keybinding(scope, &Meaning::LRept),
                            (Scope::Local, IfCurrentNotFound::LookBackward) => context
                                .keyboard_layout_kind()
                                .get_find_keymap_keybinding(scope, &Meaning::LRept),
                            (Scope::Global, _) => context
                                .keyboard_layout_kind()
                                .get_find_keymap_keybinding(scope, &Meaning::GRept),
                        },
                        "Repeat".to_string(),
                        Dispatch::UseLastNonContiguousSelectionMode(if_current_not_found),
                    )]
                    .to_vec(),
                )
                .collect_vec()
        };
        let misc_keybindings = [
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap_keybinding(scope, &Meaning::Mark_),
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
                context
                    .keyboard_layout_kind()
                    .get_find_keymap_keybinding(scope, &Meaning::Qkfix),
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
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_find_keymap_keybinding(scope, &meaning),
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
            (Meaning::DgAll, "All", DiagnosticSeverityRange::All),
            (Meaning::DgErr, "Error", DiagnosticSeverityRange::Error),
            (Meaning::DgHnt, "Hint", DiagnosticSeverityRange::Hint),
            (Meaning::DgInf, "Info", DiagnosticSeverityRange::Information),
            (Meaning::DgWrn, "Warn", DiagnosticSeverityRange::Warning),
        ]
        .into_iter()
        .map(|(meaning, description, severity)| {
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap_keybinding(scope, &meaning),
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
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap_keybinding(scope, &Meaning::LDefn),
                "Def".to_string(),
                Dispatch::RequestDefinitions(scope),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap_keybinding(scope, &Meaning::LDecl),
                "Decl".to_string(),
                Dispatch::RequestDeclarations(scope),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap_keybinding(scope, &Meaning::LImpl),
                "Impl".to_string(),
                Dispatch::RequestImplementations(scope),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap_keybinding(scope, &Meaning::LRfrE),
                "Ref-".to_string(),
                Dispatch::RequestReferences {
                    include_declaration: false,
                    scope,
                },
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap_keybinding(scope, &Meaning::LRfrI),
                "Ref+".to_string(),
                Dispatch::RequestReferences {
                    include_declaration: true,
                    scope,
                },
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap_keybinding(scope, &Meaning::LType),
                "Type".to_string(),
                Dispatch::RequestTypeDefinitions(scope),
            ),
        ];
        let scope_specific_keybindings = match scope {
            Scope::Local => [(
                context
                    .keyboard_layout_kind()
                    .get_find_keymap_keybinding(scope, &Meaning::NtrlN),
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
                Keybinding::new(key, description.to_string(), dispatch)
            })
            .chain([
                Keybinding::new_extended(
                    context
                        .keyboard_layout_kind()
                        .get_find_keymap_keybinding(scope, &Meaning::Srch_),
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
                    context
                        .keyboard_layout_kind()
                        .get_find_keymap_keybinding(scope, &Meaning::SchWC),
                    "With".to_string(),
                    Dispatch::OpenSearchPromptWithCurrentSelection {
                        scope: Scope::Local,
                        prior_change,
                    },
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_find_keymap_keybinding(scope, &Meaning::OneCh),
                    "One".to_string(),
                    Dispatch::ToEditor(FindOneChar(if_current_not_found)),
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_find_keymap_keybinding(scope, &Meaning::RSrch),
                    Direction::End.format_action("Repeat Search"),
                    Dispatch::ToEditor(DispatchEditor::RepeatSearch(
                        Scope::Local,
                        self.cursor_direction.reverse().to_if_current_not_found(),
                        prior_change,
                    )),
                ),
            ])
            .chain(self.search_current_keymap(
                context,
                Scope::Local,
                self.cursor_direction.reverse().to_if_current_not_found(),
            ))
            .collect_vec(),
            Scope::Global => [
                Keybinding::new_extended(
                    context
                        .keyboard_layout_kind()
                        .get_find_keymap_keybinding(scope, &Meaning::Srch_),
                    "Search".to_string(),
                    "Search".to_string(),
                    Dispatch::OpenSearchPrompt {
                        scope,
                        if_current_not_found,
                    },
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_find_keymap_keybinding(scope, &Meaning::SchWC),
                    "With".to_string(),
                    Dispatch::OpenSearchPromptWithCurrentSelection {
                        scope,
                        prior_change,
                    },
                ),
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_find_keymap_keybinding(scope, &Meaning::RSrch),
                    "Repeat Search".to_string(),
                    Dispatch::ToEditor(DispatchEditor::RepeatSearch(
                        Scope::Global,
                        IfCurrentNotFound::LookForward,
                        prior_change,
                    )),
                ),
            ]
            .into_iter()
            .chain(self.search_current_keymap(context, scope, if_current_not_found))
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

            keymap: Keymap::new(
                &search_keybindings
                    .into_iter()
                    .chain(misc_keybindings)
                    .chain(diagnostics_keybindings)
                    .chain(lsp_keybindings)
                    .chain(scope_specific_keybindings)
                    .collect_vec(),
            ),
        }
    }

    pub fn multicursor_keymap(&self, context: &Context) -> Keymap {
        let primary_selection_modes_keybindings =
            self.keymap_primary_selection_modes(context, Some(PriorChange::EnterMultiCursorMode));
        let secondary_selection_modes_init_keybindings = self
            .keymap_secondary_selection_modes_init(
                context,
                Some(PriorChange::EnterMultiCursorMode),
            );
        let other_keybindings = [
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_multicursor_keymap_keybinding(&Meaning::DlCrs),
                "Delete Curs".to_string(),
                Dispatch::ToEditor(DeleteCurrentCursor(Direction::End)),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&Meaning::Jump_),
                "Jump Add Curs".to_string(),
                Dispatch::ToEditor(ShowJumps {
                    use_current_selection_mode: true,
                    prior_change: Some(PriorChange::EnterMultiCursorMode),
                }),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_multicursor_keymap_keybinding(&Meaning::KpMch),
                "Keep Match".to_string(),
                Dispatch::OpenFilterSelectionsPrompt { maintain: true },
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_multicursor_keymap_keybinding(&Meaning::RmMch),
                "Remove Match".to_string(),
                Dispatch::OpenFilterSelectionsPrompt { maintain: false },
            ),
            Keybinding::new(
                "space",
                "Keep Primary Curs".to_string(),
                Dispatch::ToEditor(DispatchEditor::CursorKeepPrimaryOnly),
            ),
        ];
        Keymap::new(
            &[
                (Movement::Up, Meaning::Up___),
                (Movement::Down, Meaning::Down_),
                (Movement::Left, Meaning::Left_),
                (Movement::Right, Meaning::Right),
                (Movement::Previous, Meaning::Prev_),
                (Movement::Next, Meaning::Next_),
                (Movement::First, Meaning::First),
                (Movement::Last, Meaning::Last_),
            ]
            .into_iter()
            .map(|(movement, meaning)| {
                Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_normal_keymap_keybinding(&meaning),
                    movement.format_action("Add Curs"),
                    Dispatch::ToEditor(DispatchEditor::AddCursorWithMovement(movement)),
                )
            })
            .chain(other_keybindings)
            .chain(primary_selection_modes_keybindings)
            .chain(secondary_selection_modes_init_keybindings)
            .collect_vec(),
        )
    }
}

pub fn swap_keymap(context: &Context) -> Keymap {
    Keymap::new(
        &[
            (Movement::Up, Meaning::Up___),
            (Movement::Down, Meaning::Down_),
            (Movement::Left, Meaning::Left_),
            (Movement::Right, Meaning::Right),
            (Movement::Previous, Meaning::Prev_),
            (Movement::Next, Meaning::Next_),
            (Movement::First, Meaning::First),
            (Movement::Last, Meaning::Last_),
        ]
        .into_iter()
        .map(|(movement, meaning)| {
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&meaning),
                movement.format_action("Swap"),
                Dispatch::ToEditor(DispatchEditor::SwapWithMovement(movement)),
            )
        })
        .chain(Some(Keybinding::new(
            context
                .keyboard_layout_kind()
                .get_normal_keymap_keybinding(&Meaning::Jump_),
            "Jump Swap".to_string(),
            Dispatch::ToEditor(ShowJumps {
                use_current_selection_mode: true,
                prior_change: Some(PriorChange::EnterSwapMode),
            }),
        )))
        .collect_vec(),
    )
}

pub fn paste_keymap(context: &Context) -> Keymap {
    Keymap::new(
        [
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_paste_keymap_keybinding(&Meaning::PBWiG),
                Direction::Start.format_action("Paste with gaps"),
                Dispatch::ToEditor(PasteWithMovement(Movement::Left)),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_paste_keymap_keybinding(&Meaning::PAWiG),
                Direction::End.format_action("Paste with gaps"),
                Dispatch::ToEditor(PasteWithMovement(Right)),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_paste_keymap_keybinding(&Meaning::PAWoG),
                Direction::End.format_action("Paste"),
                Dispatch::ToEditor(PasteWithMovement(Movement::Next)),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_paste_keymap_keybinding(&Meaning::PBWoG),
                Direction::Start.format_action("Paste"),
                Dispatch::ToEditor(PasteWithMovement(Movement::Previous)),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_paste_keymap_keybinding(&Meaning::PRplc),
                "Replace with pattern".to_string(),
                Dispatch::ToEditor(ReplaceWithPattern),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_paste_keymap_keybinding(&Meaning::RplcP),
                Direction::Start.format_action("Replace with copied text"),
                Dispatch::ToEditor(ReplaceWithPreviousCopiedText),
            ),
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_paste_keymap_keybinding(&Meaning::RplcN),
                Direction::End.format_action("Replace with copied text"),
                Dispatch::ToEditor(ReplaceWithNextCopiedText),
            ),
        ]
        .as_ref(),
    )
}

pub fn cut_keymap(context: &Context) -> Keymap {
    Keymap::new(
        &[
            (Movement::Left, Meaning::Left_),
            (Movement::Right, Meaning::Right),
            (Movement::Previous, Meaning::Prev_),
            (Movement::Next, Meaning::Next_),
            (Movement::First, Meaning::First),
            (Movement::Last, Meaning::Last_),
        ]
        .into_iter()
        .map(|(movement, meaning)| {
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&meaning),
                movement.format_action("Cut"),
                Dispatch::ToEditor(CutWithMovement(movement)),
            )
        })
        .chain(Some(Keybinding::new(
            context
                .keyboard_layout_kind()
                .get_normal_keymap_keybinding(&Meaning::Paste),
            "Replace Cut".to_string(),
            Dispatch::ToEditor(ReplaceWithCopiedText { cut: true }),
        )))
        .collect_vec(),
    )
}

pub fn marked_file_keymap(context: &Context) -> Keymap {
    Keymap::new(
        &[
            (Meaning::Left_, Movement::Left),
            (Meaning::Right, Movement::Right),
            (Meaning::First, Movement::First),
            (Meaning::Last_, Movement::Last),
        ]
        .into_iter()
        .map(|(meaning, movement)| {
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&meaning),
                movement.format_action("Marked File"),
                Dispatch::CycleMarkedFile(movement),
            )
        })
        .chain(Some(Keybinding::new_extended(
            context
                .keyboard_layout_kind()
                .get_normal_keymap_keybinding(&Meaning::Down_),
            "Mark File".to_string(),
            "Toggle File Mark".to_string(),
            Dispatch::ToggleFileMark,
        )))
        .collect_vec(),
    )
}

pub fn delete_keymap(context: &Context) -> Keymap {
    Keymap::new(
        &[
            (Movement::Left, Meaning::Left_),
            (Movement::Right, Meaning::Right),
            (Movement::Previous, Meaning::Prev_),
            (Movement::Next, Meaning::Next_),
            (Movement::First, Meaning::First),
            (Movement::Last, Meaning::Last_),
        ]
        .into_iter()
        .map(|(movement, meaning)| {
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_normal_keymap_keybinding(&meaning),
                movement.format_action("Delete"),
                Dispatch::ToEditor(DeleteWithMovement(movement)),
            )
        })
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

fn generate_enclosures_keymap(
    get_dispatch: impl Fn(EnclosureKind) -> Dispatch,
    context: &Context,
) -> Keymap {
    Keymap::new(
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
            Keybinding::new(
                context
                    .keyboard_layout_kind()
                    .get_surround_keymap_keybinding(&meaning),
                format!("{open} {close}"),
                get_dispatch(enclosure),
            )
        })
        .collect_vec(),
    )
}

pub fn extend_mode_normal_mode_override(context: &Context) -> NormalModeOverride {
    fn select_surround_keymap_legend_config(
        kind: SurroundKind,
        context: &Context,
    ) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: format!("Select Surround ({kind:?})"),

            keymap: generate_enclosures_keymap(
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

            keymap: generate_enclosures_keymap(
                |enclosure| Dispatch::ToEditor(DeleteSurround(enclosure)),
                context,
            ),
        }
    }

    fn surround_keymap_legend_config(context: &Context) -> KeymapLegendConfig {
        KeymapLegendConfig {
            title: "Surround".to_string(),

            keymap: Keymap::new(
                &generate_enclosures_keymap(
                    |enclosure| {
                        let (open, close) = enclosure.open_close_symbols_str();
                        Dispatch::ToEditor(Surround(open.to_string(), close.to_string()))
                    },
                    context,
                )
                .into_vec()
                .into_iter()
                .chain(Some(Keybinding::new(
                    context
                        .keyboard_layout_kind()
                        .get_surround_keymap_keybinding(&Meaning::XML__),
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

            keymap: generate_enclosures_keymap(
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

            keymap: generate_enclosures_keymap(
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
        ..Default::default()
    }
}

pub fn multicursor_mode_normal_mode_override(direction: Direction) -> NormalModeOverride {
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
