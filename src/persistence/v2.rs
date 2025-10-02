use std::collections::HashMap;

use shared::canonicalized_path::CanonicalizedPath;

use crate::persistence::MigrateToCurrent;

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct Root {
    version: u8,
    workspace_sessions: HashMap<CanonicalizedPath, WorkspaceSession>,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct WorkspaceSession {}

impl Default for Root {
    fn default() -> Self {
        Self {
            version: 2,
            workspace_sessions: Default::default(),
        }
    }
}

impl MigrateToCurrent for Root {
    fn migrate_to_current(self) -> anyhow::Result<super::LatestRoot> {
        Ok(self)
    }

    fn version() -> &'static str {
        file!()
    }
}
