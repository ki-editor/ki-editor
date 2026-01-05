use crate::app::Dispatch;
use crate::components::editor::DispatchEditor;
use crate::components::editor_keymap::KeyboardMeaningLayout;
use crate::components::editor_keymap::Meaning::{self, *};
use crate::components::suggestive_editor::Info;
use crate::config::AppConfig;
use crate::position::Position;
use itertools::Itertools;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shared::canonicalized_path::CanonicalizedPath;
use std::{io::Write, ops::Range, process::Stdio};

pub(crate) const LEADER_KEYMAP_LAYOUT: KeyboardMeaningLayout = [
    [
        __Q__, __W__, __E__, __R__, __T__, /****/ __Y__, __U__, __I__, __O__, __P__,
    ],
    [
        __A__, __S__, __D__, __F__, __G__, /****/ __H__, __J__, __K__, __L__, _SEMI,
    ],
    [
        __Z__, __X__, __C__, __V__, __B__, /****/ __N__, __M__, _COMA, _DOT_, _SLSH,
    ],
];

pub(crate) fn leader_meanings() -> Vec<Meaning> {
    LEADER_KEYMAP_LAYOUT
        .iter()
        .flat_map(|row| row.iter().cloned())
        .collect_vec()
}

pub(crate) fn custom_keymap() -> Vec<CustomActionKeymap> {
    AppConfig::singleton()
        .leader_keymap()
        .keybindings()
        .iter()
        .flat_map(|keybindings| {
            keybindings
                .iter()
                .filter_map(|keybinding| keybinding.clone())
        })
        .zip(leader_meanings())
        .map(|(keybinding, meaning)| (meaning, keybinding.name.clone(), keybinding.script.clone()))
        .collect()
}

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) struct Keybinding {
    pub(crate) name: String,
    pub(crate) script: Script,
}

pub(crate) type CustomActionKeymap = (Meaning, String, Script);

#[derive(Serialize, Clone, JsonSchema)]
pub(crate) struct ScriptInput {
    pub(crate) current_file_path: Option<String>,
    /// 0-based index
    pub(crate) selections: Vec<Selection>,
}

#[derive(Serialize, Clone, JsonSchema)]
pub(crate) struct Selection {
    pub(crate) content: String,
    pub(crate) range: Range<Position>,
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct ScriptOutput {
    pub(crate) dispatches: Vec<ScriptDispatch>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, PartialEq, Clone)]
pub(crate) enum ScriptDispatch {
    ShowInfo { title: String, content: String },
    ReplaceSelections(Vec<String>),
}

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(transparent)]
struct ScriptName(String);

#[derive(Clone, Deserialize, Serialize, JsonSchema, Debug)]
#[serde(try_from = "ScriptName", into = "ScriptName")]
pub(crate) struct Script {
    pub(crate) path: CanonicalizedPath,
    pub(crate) name: String,
}

impl Script {
    pub(crate) fn execute(
        &self,
        context: crate::scripting::ScriptInput,
    ) -> anyhow::Result<ScriptOutput> {
        let json = serde_json::to_string(&context)?;

        let mut child = std::process::Command::new(self.path.display_absolute())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(json.as_bytes())?;
        }

        let output = child.wait_with_output()?;

        // Check exit status first
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Script exited with status {}: {}", output.status, stderr);
        }

        let stdout = String::from_utf8(output.stdout)?;
        let deserializer = &mut serde_json::Deserializer::from_str(&stdout);

        serde_path_to_error::deserialize(deserializer)
            .map_err(|err| anyhow::anyhow!("{err}\n\nSTDOUT=\n\n{stdout}"))
    }
}

impl TryFrom<ScriptName> for Script {
    type Error = anyhow::Error;

    fn try_from(value: ScriptName) -> anyhow::Result<Self> {
        crate::config::load_script(&value.0)
    }
}

impl From<Script> for ScriptName {
    fn from(value: Script) -> Self {
        Self(value.name)
    }
}

impl ScriptDispatch {
    pub(crate) fn into_app_dispatch(self) -> Dispatch {
        match self {
            ScriptDispatch::ShowInfo { title, content } => {
                Dispatch::ShowGlobalInfo(Info::new(title, content))
            }
            ScriptDispatch::ReplaceSelections(replacements) => {
                Dispatch::ToEditor(DispatchEditor::ReplaceSelections(replacements))
            }
        }
    }
}

#[cfg(test)]
mod test_scripting {
    use my_proc_macros::keys;

    use crate::{
        app::Dispatch::*,
        buffer::BufferOwner,
        components::editor::{DispatchEditor::*, IfCurrentNotFound},
        selection::SelectionMode,
        test_app::{execute_test, ExpectKind::*, Step::*},
    };

    #[test]
    fn test_script_dispatch_show_info() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.foo_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                // This will execute the script from .ki/scripts/example_show_info.py
                App(HandleKeyEvents(keys!("backslash q").to_vec())),
                Expect(GlobalInfo(
                    "The current selected texts are [\"pub(crate) struct Foo {\"]",
                )),
            ])
        })
    }

    #[test]
    fn test_script_dispatch_replace_selections() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.foo_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foo bar spam".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Word,
                )),
                Expect(CurrentSelectedTexts(&["foo"])),
                // This will execute the script from .ki/scripts/example_replace_selections.py
                App(HandleKeyEvents(keys!("backslash w").to_vec())),
                Expect(CurrentSelectedTexts(&["Coming from Python script"])),
                Expect(CurrentComponentContent(
                    "Coming from Python script bar spam",
                )),
            ])
        })
    }
}
