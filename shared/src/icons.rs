use once_cell::sync::OnceCell;
use std::collections::HashMap;

static ICON_CONFIG: OnceCell<IconsConfig> = OnceCell::new();

use serde::Deserialize;
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

const ZERO_WIDTH_JOINER: char = '\u{200D}';
pub fn get_icon_config() -> &'static IconsConfig {
    ICON_CONFIG.get_or_init(|| {
        let result: IconsConfig =
            serde_json::from_str(include_str!("../../contrib/emoji-icon-theme.json")).unwrap();
        result.file_extensions.values().for_each(check_for_zwj);
        result.file_names.values().for_each(check_for_zwj);
        result.completion.values().for_each(check_for_zwj);
        result
    })
}

fn check_for_zwj(value: &String) {
    if value.chars().any(|c| c == ZERO_WIDTH_JOINER) {
        panic!("The value {value} contains ZWJ (Zero-width joiner). Joined characters are not supported for now.");
    }
}
