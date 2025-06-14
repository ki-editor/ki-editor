fn main() {
    // shared::grammar::fetch_grammars();
    // shared::grammar::build_grammars();

    if std::env::var("CARGO_FEATURE_VSCODE").is_ok() {
        println!("Building with vscode feature enabled, generating types...");

        let out_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let ts_output_path = out_dir.join("ki-vscode").join("src").join("protocol");
        let kotlin_output_path = out_dir
            .join("ki-jetbrains")
            .join("src")
            .join("kotlin")
            .join("protocol");

        // Ensure the parent directories exist
        for path in [&ts_output_path, &kotlin_output_path] {
            if let Some(parent_dir) = path.parent() {
                if !parent_dir.exists() {
                    std::fs::create_dir_all(parent_dir).expect("Failed to create parent directory");
                }
            }
        }

        // Ensure the kotlin directory structure exists
        std::fs::create_dir_all(&kotlin_output_path)
            .expect("Failed to create kotlin output directory");

        // Clear the target directories if they exist, then recreate them
        for path in [&ts_output_path, &kotlin_output_path] {
            if path.exists() {
                std::fs::remove_dir_all(path)
                    .expect("Failed to remove existing protocol directory");
            }
            std::fs::create_dir(path).expect("Failed to create protocol directory");
        }

        // Generate TypeScript types
        let typeshare_ts_status = std::process::Command::new("typeshare")
            .arg("ki-protocol-types/src")
            .arg("--lang=typescript")
            .arg(format!(
                "--output-file={}/types.ts",
                ts_output_path.display()
            ))
            .status()
            .expect("Failed to execute typeshare for TypeScript");

        if !typeshare_ts_status.success() {
            panic!("typeshare failed to generate TypeScript types");
        }

        // Generate Kotlin types
        let typeshare_kotlin_status = std::process::Command::new("typeshare")
            .arg("ki-protocol-types/src")
            .arg("--lang=kotlin")
            .arg(format!(
                "--output-file={}/Types.kt",
                kotlin_output_path.display()
            ))
            .status()
            .expect("Failed to execute typeshare for Kotlin");

        if !typeshare_kotlin_status.success() {
            panic!("typeshare failed to generate Kotlin types");
        }

        println!("TypeScript definitions generated at {:?}", ts_output_path);
        println!("Kotlin definitions generated at {:?}", kotlin_output_path);
    } else {
        println!("Building without vscode feature, skipping type generation.");
    }

    println!("cargo:rerun-if-changed=build.rs");

    {
        println!("cargo:rerun-if-changed=src/vscode_ipc.rs"); // Rerun if message handling logic changes
        println!("cargo:rerun-if-changed=src/vscode.rs"); // Rerun if dependent types change
        println!("cargo:rerun-if-changed=ki-protocol-types/src/lib.rs"); // Rerun if protocol types change
    }
}
