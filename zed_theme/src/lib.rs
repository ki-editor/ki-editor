use std::{cell::LazyCell, collections::HashMap};

pub mod zed_theme_schema;

pub use zed_theme_schema::*;

const THEME_DOCUMENTS: [&'static str; 18] = [
    include_str!("../themes/alabaster-color-theme.json"),
    include_str!("../themes/alabaster-color-theme.json"),
    include_str!("../themes/apathy_ki.json"),
    include_str!("../themes/ayu.json"),
    include_str!("../themes/catppuccin-mauve.json"),
    include_str!("../themes/dracula.json"),
    include_str!("../themes/github_theme.json"),
    include_str!("../themes/gruber_darker_zed.json"),
    include_str!("../themes/gruvbox.json"),
    include_str!("../themes/modus.json"),
    include_str!("../themes/monokai.json"),
    include_str!("../themes/monokai_st3.json"),
    include_str!("../themes/mqual_blue_zed.json"),
    include_str!("../themes/nord.json"),
    include_str!("../themes/one.json"),
    include_str!("../themes/rose-pine.json"),
    include_str!("../themes/solarized.json"),
    include_str!("../themes/tokyo-night.json"),
];

pub const THEMES: LazyCell<HashMap<String, ThemeContent>> = LazyCell::new(|| {
    let mut map = HashMap::new();
    for theme_bundle in THEME_DOCUMENTS {
        let themes: ThemeFamilyContent = serde_json5::from_str(theme_bundle)
            .expect(&format!("Failed to parse bundle {theme_bundle}"));
        for theme in themes.themes {
            map.insert(theme.name.clone(), theme);
        }
    }
    map
});
