fn main() {
    // shared::grammar::fetch_grammars();
    // shared::grammar::build_grammars();

    // Get git commit hash at build time
    let git_hash = match std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "unknown".to_string(),
    };

    // Get current date and time in human readable format
    let build_time = chrono::Local::now()
        .format("%Y-%m-%d %I:%M %p %Z")
        .to_string();

    println!("cargo:rustc-env=GIT_HASH={git_hash}");
    println!("cargo:rustc-env=BUILD_TIME={build_time}");
}
