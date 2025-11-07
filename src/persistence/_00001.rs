//! V1 contains nothing, but it serves as a placeholder
//! for demonstrating how new versions can be added.

use crate::persistence::Migration;

#[derive(serde::Deserialize, serde::Serialize, Default, Debug)]
pub(crate) struct Root {
    version: String,
}

impl Migration for Root {
    type PreviousVersion = Self;

    fn version() -> &'static str {
        file!()
    }

    fn migrate_to_current(self) -> anyhow::Result<super::Root> {
        super::_00002::Root::from_previous_version(self).migrate_to_current()
    }

    fn from_previous_version(value: Self::PreviousVersion) -> Self {
        value
    }
}
