fn main() {
    let compiled_theme_path = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").expect("Cargo didn't give us an OUT_DIR?"),
    )
    .join("compiled_themes.bin");

    let data = format!(
        "[{}]",
        std::fs::read_dir("../themes")
            .expect("Failed to read themes dir")
            .map(|entry| {
                let entry = entry.expect("Failed to get particular file entry in themes dir");
                std::fs::read_to_string(entry.path())
                    .unwrap_or_else(|_| panic!("Failed to read {}", entry.path().to_string_lossy()))
            })
            .collect::<Vec<_>>()
            .join(",")
    );

    // 6 is the default compression level and a good compromise.
    let compressed_data = miniz_oxide::deflate::compress_to_vec(data.as_bytes(), 6);

    std::fs::write(compiled_theme_path, compressed_data)
        .expect("Failed to write compiled theme contents");
}
