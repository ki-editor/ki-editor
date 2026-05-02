use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Controls which icon set is used throughout the UI.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum IconStyle {
    /// Emoji icons (default). No special font required.
    #[default]
    Emoji,
    /// Nerd Font icons. Requires a Nerd Font (<https://www.nerdfonts.com/>) to be installed.
    NerdFont,
    /// Disable icons entirely.
    None,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconsConfig {
    pub(crate) file: String,
    pub folder: String,
    pub folder_expanded: String,
    pub(crate) file_extensions: HashMap<String, String>,
    pub(crate) file_names: HashMap<String, String>,
    pub completion: HashMap<String, String>,
}

/// Build an [`IconsConfig`] for the given style.
pub fn build_icon_config(style: &IconStyle) -> IconsConfig {
    let config = match style {
        IconStyle::Emoji => {
            serde_json::from_str(include_str!("../../contrib/emoji-icon-theme.json")).unwrap()
        }
        IconStyle::NerdFont => {
            serde_json::from_str(include_str!("../../contrib/nerd-font-icon-theme.json")).unwrap()
        }
        IconStyle::None => IconsConfig {
            file: String::new(),
            folder: String::new(),
            folder_expanded: String::new(),
            file_extensions: HashMap::new(),
            file_names: HashMap::new(),
            completion: HashMap::new(),
        },
    };
    validate_config(&config);
    config
}

/// Format `text` with a leading icon, omitting the icon (and the separating
/// space) when `icon` is empty.
pub fn format_with_icon(icon: &str, text: &str) -> String {
    if icon.is_empty() {
        text.to_string()
    } else {
        format!("{icon} {text}")
    }
}

const ZERO_WIDTH_JOINER: char = '\u{200D}';

fn validate_config(config: &IconsConfig) {
    config.file_extensions.values().for_each(check_for_zwj);
    config.file_names.values().for_each(check_for_zwj);
    config.completion.values().for_each(check_for_zwj);
}

fn check_for_zwj(value: &String) {
    if value.chars().any(|c| c == ZERO_WIDTH_JOINER) {
        panic!("The value {value} contains ZWJ (Zero-width joiner). Joined characters are not supported for now.");
    }
}
