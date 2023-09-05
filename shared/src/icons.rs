use once_cell::sync::OnceCell;
use std::collections::HashMap;

static ICON_CONFIG: OnceCell<IconsConfig> = OnceCell::new();

use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconsConfig {
    pub file: String,
    pub folder: String,
    pub folder_expanded: String,
    pub file_extensions: HashMap<String, String>,
    pub file_names: HashMap<String, String>,
    pub completion: HashMap<String, String>,
}

pub fn get_icon_config() -> &'static IconsConfig {
    ICON_CONFIG.get_or_init(|| {
        serde_json::from_str(include_str!("../../contrib/emoji-icon-theme.json")).unwrap()
    })
}
