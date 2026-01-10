//! NOTE:
//!
//! Each schema file (e.g. v1.rs, v2.rs, etc.) should not be modified
//! once they are committed to the master branch.
//!
//! To make changes to the existing schema:
//! 1. Create a new version file.
//! 2. Update the `migrate_to_current` method of the last version file.  
//! 3. Each version file should expose a struct call `Root`
//!    and implement the `Migration` trait.  
//! 4. Update the `Root` of this file.  
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::char_index_range::CharIndexRange;

pub mod _00001;
pub mod _00002;
pub mod _00003;
pub mod _00004;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Version(pub u8);

pub type Root = _00004::Root;
pub type WorkspaceSession = _00004::WorkspaceSession;

pub struct Persistence {
    path: PathBuf,
    root: Root,
}

impl Persistence {
    fn load(path: PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path.clone())?;
        let value: serde_json::Value = serde_json::from_str(&content)?;

        let version = value
            .get("version")
            .and_then(|value| match value {
                serde_json::Value::String(version) => Some(version),
                _ => None,
            })
            .ok_or_else(|| anyhow::anyhow!("Unable to obtain version from:\n\n{content}"))?;

        let root = Self::load_from_version(version, content)?;
        Ok(Self { path, root })
    }

    fn get_parser(version: &str) -> Option<Parser> {
        let parser = Root::as_parser();
        parser.get_matching_version(version)
    }

    fn load_from_version(version: &str, content: String) -> anyhow::Result<Root> {
        let Some(parser) = Self::get_parser(version) else {
            return Err(anyhow::anyhow!("Unknown version: {version}"));
        };

        (parser.parse)(content)
    }

    pub fn load_or_default(path: PathBuf) -> Self {
        Persistence::load(path.clone())
            .map_err(|err| {
                #[cfg(test)]
                dbg!("Persistence::load_or_default error: {err}");

                log::error!("Unable to load persisted data due to {err:?}")
            })
            .unwrap_or_else(|_| Self {
                path,
                root: Default::default(),
            })
    }

    pub fn write(&self) -> anyhow::Result<()> {
        std::fs::write(self.path.clone(), serde_json::to_string_pretty(&self.root)?)?;
        Ok(())
    }

    pub fn set_workspace_session(&mut self, working_directory: &Path, session: WorkspaceSession) {
        self.root
            .workspace_sessions
            .insert(working_directory.to_path_buf(), session);
    }

    pub fn get_marked_files(&self, workding_directory: &PathBuf) -> Option<Vec<PathBuf>> {
        self.root
            .workspace_sessions
            .get(workding_directory)
            .map(|session| session.marked_files.clone())
    }

    pub fn get_marks(
        &self,
        working_directory: &PathBuf,
    ) -> Option<HashMap<PathBuf, Vec<CharIndexRange>>> {
        self.root
            .workspace_sessions
            .get(working_directory)
            .map(|session| session.marks.clone())
    }

    pub fn get_prompt_histories(
        &self,
        working_directory: &Path,
    ) -> Option<HashMap<crate::components::prompt::PromptHistoryKey, indexmap::IndexSet<String>>>
    {
        self.root
            .workspace_sessions
            .get(working_directory)
            .map(|session| session.prompt_histories.clone())
    }
}

pub trait Migration:
    Default + serde::de::DeserializeOwned + serde::Serialize + std::fmt::Debug
{
    type PreviousVersion: Migration;

    /// The implementation of this method should be always `file!()`;
    fn version() -> &'static str;

    /// For the latest migration, this should be `self`,
    /// otherwise, this should be `self.to_next_version().migrate_to_current()`.
    fn migrate_to_current(self) -> anyhow::Result<Root>;

    fn from_previous_version(value: Self::PreviousVersion) -> Self;

    fn try_parse(content: String) -> anyhow::Result<Root> {
        serde_json::from_str::<Self>(&content)?.migrate_to_current()
    }
}

trait AsParser {
    fn as_parser() -> Parser;
}

impl<T: Migration> AsParser for T {
    fn as_parser() -> Parser {
        Parser {
            version: Self::version(),
            parse: Box::new(|content| Self::try_parse(content)),
            previous_version: if Self::version() == T::PreviousVersion::version() {
                None
            } else {
                Some(Box::new(T::PreviousVersion::as_parser()))
            },
        }
    }
}

struct Parser {
    version: &'static str,
    parse: Box<fn(String) -> anyhow::Result<Root>>,
    previous_version: Option<Box<Parser>>,
}

impl Parser {
    fn get_matching_version(self, version: &str) -> Option<Parser> {
        if self.version == version {
            Some(self)
        } else if let Some(parser) = self.previous_version {
            parser.get_matching_version(version)
        } else {
            None
        }
    }
}
