use std::{collections::HashMap, io::Read, path::PathBuf, str::FromStr};

use config::Config;

use crate::language::{self, Language};
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
        let settings = Config::builder()
            .add_source(
                config::File::with_name(&::grammar::config_dir().join("config").to_string_lossy())
                    .required(false),
            )
            .add_source(
                config::File::with_name(
                    &PathBuf::from_str(".")?
                        .join(".ki")
                        .join("config")
                        .to_string_lossy(),
                )
                .required(false),
            )
            .build()?;
        let value = &settings.try_deserialize::<Value>()?;
        let mut default = serde_json::to_value(AppConfig::default())?;
        json_value_merge::Merge::merge(&mut default, value);
        let result: Value = serde_json::from_value(default)?;

        let stringified = serde_json::to_string_pretty(&result)?;

        let deserializer = &mut serde_json::Deserializer::from_str(&stringified);

        let result = serde_path_to_error::deserialize(deserializer)?;
        Ok(result)
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
