use std::{collections::HashMap, path::PathBuf};

use crate::persistence::Migration;

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct Root {
    pub(crate) version: String,
    pub(crate) workspace_sessions: HashMap<PathBuf, WorkspaceSession>,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct WorkspaceSession {
    /// We use PathBuf instead of CanonicalizedPath because
    /// the stored path might be deleted after Root is serialized and stored,
    /// and we don't want the deserialization of Root to fail because some
    /// path inside marked_files no longer exists.
    pub(crate) marked_files: Vec<PathBuf>,
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
    type PreviousVersion = super::_00001::Root;

    fn version() -> &'static str {
        file!()
    }

    fn migrate_to_current(self) -> anyhow::Result<super::Root> {
        Ok(self)
    }

    fn from_previous_version(_: Self::PreviousVersion) -> Self {
        Self::default()
    }
}
