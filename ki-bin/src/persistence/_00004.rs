use std::{collections::HashMap, path::PathBuf};

use indexmap::IndexSet;

use crate::{
    char_index_range::CharIndexRange, components::prompt::PromptHistoryKey, persistence::Migration,
};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub(crate) struct Root {
    pub(crate) version: String,
    pub(crate) workspace_sessions: HashMap<PathBuf, WorkspaceSession>,
}

#[derive(Default, serde::Serialize, serde::Deserialize, Debug)]
pub(crate) struct WorkspaceSession {
    /// We use PathBuf instead of CanonicalizedPath because
    /// the stored path might be deleted after Root is serialized and stored,
    /// and we don't want the deserialization of Root to fail because some
    /// path inside marked_files no longer exists.
    pub(crate) marked_files: Vec<PathBuf>,
    pub(crate) marks: HashMap<PathBuf, Vec<CharIndexRange>>,
    pub(crate) prompt_histories: HashMap<PromptHistoryKey, IndexSet<String>>,
}

impl Default for Root {
    fn default() -> Self {
        Self {
            workspace_sessions: Default::default(),
            version: file!().to_string(),
        }
    }
}

impl Migration for Root {
    type PreviousVersion = super::_00003::Root;

    fn version() -> &'static str {
        file!()
    }

    fn migrate_to_current(self) -> anyhow::Result<super::Root> {
        Ok(self)
    }

    fn from_previous_version(previous: Self::PreviousVersion) -> Self {
        Self {
            workspace_sessions: previous
                .workspace_sessions
                .into_iter()
                .map(|(path_buf, workspace_session)| {
                    (
                        path_buf,
                        WorkspaceSession {
                            marked_files: workspace_session.marked_files,
                            marks: workspace_session.marks,
                            prompt_histories: Default::default(),
                        },
                    )
                })
                .collect(),
            version: Self::version().to_string(),
        }
    }
}
