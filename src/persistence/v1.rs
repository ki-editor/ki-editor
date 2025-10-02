//! V1 contains nothing, but it serves as a placeholder
//! for demonstrating how new versions can be added.

use crate::persistence::MigrateToCurrent;

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct Root {
    version: u8,
}

impl Default for Root {
    fn default() -> Self {
        Self { version: 1 }
    }
}

impl MigrateToCurrent for Root {
    fn migrate_to_current(self) -> anyhow::Result<super::LatestRoot> {
        super::v2::Root::default().migrate_to_current()
    }

    fn version() -> &'static str {
        file!()
    }
}
