mod zed_theme_schema;

pub use zed_theme_schema::*;

use std::{collections::HashMap, fs};

const COMPILED_THEME_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/compiled_themes.bin"));

pub fn get_zed_themes() -> HashMap<String, ThemeContent> {
    let decompressed_themes = miniz_oxide::inflate::decompress_to_vec(COMPILED_THEME_BYTES)
        .expect("Compiled themes can't be decompressed?");
    let theme_families: Vec<ThemeFamilyContent> =
        serde_json_lenient::from_slice(&decompressed_themes)
            .expect("Compiled themes aren't valid lenient JSON?");

    theme_families
        .into_iter()
        .flat_map(|theme_family| {
            theme_family
                .themes
                .into_iter()
                .map(|theme| (theme.name.clone(), theme))
        })
        .collect()
}

pub fn get_custom_themes(themes_glob: &str) -> HashMap<String, ThemeContent> {
    let theme_families: Vec<ThemeFamilyContent> = glob::glob(themes_glob)
        .expect("Failed to read glob pattern")
        .map(|entry| match entry {
            Ok(path) => {
                let file = fs::File::open(&path)
                    .unwrap_or_else(|error| panic!("Failed to read file {path:?}, error: {error:?}"));
                match serde_json_lenient::from_reader(file) {
                    Ok(content) => content,
                    Err(error) =>panic!("Invalid JSON syntax in theme definition.\n\tPath: {path:?}\n\t Error: {error:?}"),
                }
            }
            Err(error) => panic!("Failed to read glob entry path. Error: {error:?}"),
        })
        .collect();

    theme_families
        .into_iter()
        .flat_map(|theme_family| {
            theme_family
                .themes
                .into_iter()
                .map(|theme| (theme.name.clone(), theme))
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_zed_themes_can_be_loaded() {
        assert!(!get_zed_themes().is_empty());
    }

    #[test]
    fn test_custom_themes_can_be_loaded() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| {
            panic!("Could not create a temp directory.\n\tError:{error:?}")
        });
        let theme_file_path = temp_dir.path().join("test_theme.json");

        let theme_json = r#"{
            "name": "Test Theme",
            "author": "Tester",
            "themes": [
                { 
                    "name": "My Awesome Theme", 
                    "appearance": "dark", 
                    "style": {} 
                }
            ]
        }"#;

        std::fs::write(theme_file_path, theme_json)
            .unwrap_or_else(|error| panic!("Could not write to temp file.\n\tError: {error:?}"));

        let theme_path_buf = temp_dir.path().join("*.json");
        let theme_glob = theme_path_buf.to_str().unwrap();
        assert!(!get_custom_themes(theme_glob).is_empty());
    }
}
