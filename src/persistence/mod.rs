//! NOTE:
//!
//! Each schema file (e.g. v1.rs, v2.rs, etc.) should not be modified
//! once they are committed to the master branch.
//!
//! To make changes to the existing schema:
//! 1. Create a new version file.
//! 2. Update the `migrate_to_current` method of the last version file.  
//! 3. Each version file should expose a struct call `Root`
//!    and implement the `MigrateToCurrent` trait.  
//! 4. Update the `LatestRoot` of this file.  
use std::collections::HashMap;

use itertools::Itertools;
use once_cell::unsync::Lazy;
use serde_json;
use shared::canonicalized_path::CanonicalizedPath;

pub(crate) mod v1;
pub(crate) mod v2;

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Version(pub(crate) u8);

type LatestRoot = v2::Root;

#[derive(Default)]
pub(crate) struct Persistence {
    root: LatestRoot,
}

impl Persistence {
    fn load(path: &CanonicalizedPath) -> anyhow::Result<Self> {
        let content = path.read()?;
        let value: serde_json::Value = serde_json::from_str(&content)?;

        let version = value
            .get("version")
            .and_then(|value| match value {
                serde_json::Value::String(version) => Some(version),
                _ => None,
            })
            .ok_or_else(|| anyhow::anyhow!("Unable to obtain version from:\n\n{content}"))?;

        let root = Self::load_from_version(version, content)?;
        Ok(Self { root })
    }

    fn get_parser(version: &str) -> Option<VersionParser> {
        [v1::Root::as_version_parser(), v2::Root::as_version_parser()]
            .into_iter()
            .find(|parser| parser.version == version)
    }

    fn load_from_version(version: &str, content: String) -> anyhow::Result<LatestRoot> {
        let Some(parser) = Self::get_parser(version) else {
            return Err(anyhow::anyhow!("Unknown version: {version}"));
        };

        (parser.parse)(content)
    }

    pub(crate) fn load_or_default(path: &CanonicalizedPath) -> Self {
        Persistence::load(path)
            .map_err(|err| log::error!("Unable to load persisted data due to {err:?}"))
            .unwrap_or_default()
    }
}

pub(crate) trait MigrateToCurrent:
    Default + serde::de::DeserializeOwned + serde::Serialize
{
    fn migrate_to_current(self) -> anyhow::Result<LatestRoot>;

    /// The implementation of this method should be always `file!()`;
    fn version() -> &'static str;

    fn try_parse(content: String) -> anyhow::Result<LatestRoot> {
        serde_json::from_str::<Self>(&content)?.migrate_to_current()
    }

    fn as_version_parser() -> VersionParser {
        VersionParser {
            version: Self::version(),
            parse: Box::new(|content| Self::try_parse(content)),
        }
    }
}

struct VersionParser {
    version: &'static str,
    parse: Box<fn(String) -> anyhow::Result<LatestRoot>>,
}
