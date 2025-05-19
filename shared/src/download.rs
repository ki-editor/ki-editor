

/// Download the file from `url` and cache it under `folder_name` as `file_name`,
/// so that it will not be downloaded again.
pub fn cache_download(url: &str, folder_name: &str, file_name: &str) -> anyhow::Result<String> {
    let cache_dir = grammar::cache_dir().join(folder_name);
    std::fs::create_dir_all(cache_dir.clone())?;
    let cache_path = cache_dir.join(file_name);
    if let Ok(text) = std::fs::read_to_string(cache_path.clone()) {
        Ok(text)
    } else {
        let text = reqwest::blocking::get(url)?.text()?;
        std::fs::write(cache_path, &text)?;

        Ok(text)
    }
}
