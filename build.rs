#[cfg(feature = "vscode")]
use ts_rs::TS;

fn main() {
    shared::grammar::fetch_grammars();
    shared::grammar::build_grammars();

    #[cfg(feature = "vscode")]
    if std::env::var("CARGO_FEATURE_VSCODE").is_ok() {
        println!("Building with vscode feature enabled, generating TS types...");

        let out_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let ts_output_path = out_dir.join("ki-vscode").join("src").join("protocol");

        // Ensure the parent directory exists
        if let Some(parent_dir) = ts_output_path.parent() {
            if !parent_dir.exists() {
                std::fs::create_dir_all(parent_dir)
                    .expect("Failed to create parent directory for TypeScript file");
            }
        }
        // Clear the target directory if it exists, then recreate it
        if ts_output_path.exists() {
            std::fs::remove_dir_all(&ts_output_path)
                .expect("Failed to remove existing protocol directory");
        }
        std::fs::create_dir(&ts_output_path).expect("Failed to create protocol directory");

        // Export KiMessage and its dependencies from the protocol crate
        ki_protocol_types::OutputMessage::export_all_to(&ts_output_path)
            .expect("Failed to write TypeScript bindings to file");
        ki_protocol_types::InputMessage::export_all_to(&ts_output_path)
            .expect("Failed to write TypeScript bindings to file");
        ki_protocol_types::OutputMessageWrapper::export_all_to(&ts_output_path)
            .expect("Failed to write TypeScript bindings to file");
        ki_protocol_types::InputMessageWrapper::export_all_to(&ts_output_path)
            .expect("Failed to write TypeScript bindings to file");

        println!("TypeScript definitions generated at {:?}", ts_output_path);
    } else {
        println!("Building without vscode feature, skipping TS type generation.");
    }

    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(feature = "vscode")]
    {
        println!("cargo:rerun-if-changed=src/vscode_ipc.rs"); // Rerun if message handling logic changes
        println!("cargo:rerun-if-changed=src/vscode.rs"); // Rerun if dependent types change
        println!("cargo:rerun-if-changed=ki-protocol-types/src/lib.rs"); // Rerun if protocol types change
    }
}
