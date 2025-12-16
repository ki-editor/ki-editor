use std::{collections::HashMap, io::Read, path::PathBuf, str::FromStr};

use crate::language::{self, Language};
use figment::providers;
use figment::providers::Format;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppConfig {
    languages: HashMap<String, Language>,
}

impl AppConfig {
    fn default() -> Self {
        Self {
            languages: crate::languages::languages(),
        }
    }
    pub(crate) fn load_from_current_directory() -> anyhow::Result<Self> {
        let config: AppConfig =
            figment::Figment::from(providers::Serialized::defaults(&AppConfig::default()))
                .merge(providers::Json::file(
                    PathBuf::from_str(".")?.join(".ki").join("config.json"),
                ))
                .merge(providers::Json::file(
                    ::grammar::config_dir().join("config.json"),
                ))
                .extract()?;
        Ok(config)
    }
    pub fn singleton() -> &'static AppConfig {
        static INSTANCE: OnceCell<AppConfig> = OnceCell::new();
        INSTANCE.get_or_init(|| match AppConfig::load_from_current_directory() {
            Ok(config) => config,
            Err(error) => {
                eprintln!("Error parsing Ki config: {error}");
                println!("\nConfig will be ignored. [Press any key to continue]");
                let _ = std::io::stdin().read(&mut [0u8]).unwrap();

                AppConfig::default()
            }
        })
    }

    pub(crate) fn languages(&self) -> &HashMap<std::string::String, language::Language> {
        &self.languages
    }
}
