use crate::components::editor::DispatchEditor;
use crate::components::suggestive_editor::Info;
use crate::config::AppConfig;
use crate::position::Position;
use crate::{app::Dispatch, components::editor_keymap::QWERTY};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shared::canonicalized_path::CanonicalizedPath;
use std::{io::Write, ops::Range, process::Stdio};

pub fn custom_keymap() -> Vec<CustomActionKeymap> {
    AppConfig::singleton()
        .leader_keymap()
        .keybindings()
        .iter()
        .flat_map(|keybindings| {
            keybindings
                .iter()
                .filter_map(|keybinding| keybinding.clone())
        })
        .zip(QWERTY.iter().flatten())
        .map(|(keybinding, key)| {
            (
                key.to_string(),
                keybinding.name.clone(),
                keybinding.script.clone(),
            )
        })
        .collect()
}

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
pub struct Keybinding {
    pub name: String,
    pub script: Script,
}

pub type CustomActionKeymap = (String, String, Script);

#[derive(Serialize, Clone, JsonSchema)]
pub struct ScriptInput {
    pub current_file_path: Option<String>,
    /// 0-based index
    pub selections: Vec<Selection>,
}

#[derive(Serialize, Clone, JsonSchema)]
pub struct Selection {
    pub content: String,
    pub range: Range<Position>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ScriptOutput {
    pub dispatches: Vec<ScriptDispatch>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, PartialEq, Clone)]
pub enum ScriptDispatch {
    ShowInfo { title: String, content: String },
    ReplaceSelections(Vec<String>),
}

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(transparent)]
struct ScriptName(String);

#[derive(Clone, Deserialize, Serialize, JsonSchema, Debug)]
#[serde(try_from = "ScriptName", into = "ScriptName")]
pub struct Script {
    pub path: CanonicalizedPath,
    pub name: String,
}

impl Script {
    pub fn execute(&self, context: crate::scripting::ScriptInput) -> anyhow::Result<ScriptOutput> {
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
    pub fn into_app_dispatch(self) -> Dispatch {
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
                    "The current selected texts are [\"pub(crate) struct Foo {\"]".to_string(),
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
