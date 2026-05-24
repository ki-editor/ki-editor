use uuid::Uuid;

use crate::cli::{parse_path_arg, EditAction};

fn unique_name() -> String {
    Uuid::new_v4().to_string()
}

#[test]
fn nonexistent_path_with_existing_parent_creates_file() -> anyhow::Result<()> {
    let file_path = std::env::temp_dir().join(unique_name());

    let action = parse_path_arg(file_path.to_string_lossy().to_string())?;

    assert!(matches!(action, EditAction::Open(_)));
    assert!(file_path.exists());

    std::fs::remove_file(&file_path)?;
    Ok(())
}

#[test]
fn nonexistent_path_with_nonexistent_parent_returns_missing_parent() -> anyhow::Result<()> {
    let parent = std::env::temp_dir().join(unique_name());
    let file_path = parent.join(unique_name());

    let action = parse_path_arg(file_path.to_string_lossy().to_string())?;

    assert!(matches!(
        action,
        EditAction::MissingParentDirectory { parent: ref p } if p == &parent
    ));
    Ok(())
}
