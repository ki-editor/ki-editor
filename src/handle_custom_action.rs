#![allow(dead_code)]
#![allow(unused_variables)]

use crate::{components::editor_keymap::Meaning, position::Position};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shared::canonicalized_path::CanonicalizedPath;
use std::{io::Write, ops::Range, process::Stdio};

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) struct Keybinding {
    pub(crate) name: String,
    pub(crate) script: Script,
}

pub(crate) type CustomActionKeymap = (Meaning, String, Script);

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) enum CustomAction {
    /// The script is expected to be stored in either `~/.config/ki/scripts/*` (Global)
    /// or `.ki/scripts/* (Local).
    ///
    /// The script can be written in any languages or format as long as it is executable.
    ///
    /// The editor context will be fed to the script via STDIN.
    ///
    /// The script is expected to return an array of actions via STDOUT.
    RunScript(Script),
    ExecuteDispatches(Vec<ScriptDispatch>),
}

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
}

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(transparent)]
struct ScriptName(String);

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(try_from = "ScriptName", into = "ScriptName")]
pub(crate) struct Script {
    path: CanonicalizedPath,
    name: String,
    content: String,
}
impl Script {
    pub(crate) fn execute(
        &self,
        context: crate::handle_custom_action::ScriptInput,
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
        let (content, path) = crate::config::load_script(&value.0)?;
        Ok(Self {
            name: value.0,
            path,
            content,
        })
    }
}

impl From<Script> for ScriptName {
    fn from(value: Script) -> Self {
        Self(value.name)
    }
}
