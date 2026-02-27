use itertools::Itertools;
use shared::{absolute_path::AbsolutePath, get_minimal_unique_paths};

use crate::components::render_editor::markup_focused_tab;

#[cfg(test)]
fn format_path_list(
    marked_paths: &[&AbsolutePath],
    dirty_paths: &[&AbsolutePath],
    current_path: &AbsolutePath,
    current_working_directory: &AbsolutePath,
) -> String {
    let formatted_paths = get_formatted_paths(
        marked_paths,
        dirty_paths,
        current_path,
        current_working_directory,
    );

    // Join all formatted paths
    formatted_paths.join("")
}

pub fn get_formatted_paths(
    marked_paths: &[&AbsolutePath],
    dirty_paths: &[&AbsolutePath],
    current_path: &AbsolutePath,
    current_working_directory: &AbsolutePath,
) -> Vec<String> {
    debug_assert_eq!(marked_paths.iter().unique().count(), marked_paths.len());
    // Check if current path is in the list
    let current_path_index = marked_paths.iter().position(|&p| p == current_path);
    let contains_current_path = current_path_index.is_some();
    let is_dirty = |path| dirty_paths.contains(&path);

    // Create a combined list for minimal unique paths calculation, including the current path
    // even though we won't use its minimal version (we need it for context in minimization)
    let paths_for_minimal = marked_paths
        .iter()
        .map(|&path| path.to_path_buf().clone())
        .chain(
            // Add current path if it's not already in the list (for context, not for display)
            if !contains_current_path {
                Some(current_path.to_path_buf().clone())
            } else {
                None
            },
        )
        .collect_vec();

    // Compute minimal unique paths with the current path included (for context)
    let minimal_unique_paths =
        get_minimal_unique_paths::get_minimal_unique_paths(&paths_for_minimal);

    // Helper function to format non-current paths

    let format_path_string = |path: &AbsolutePath| -> String {
        // For non-current paths, first try to get the relative display
        // and fall back to minimal unique if needed for disambiguation
        let relative_display = path
            .display_relative_to(current_working_directory)
            .unwrap_or_else(|_| path.display_absolute());

        // Only use minimal unique paths if needed (if they're different than relative display)
        if let Some(minimal) = minimal_unique_paths.get(path.to_path_buf()) {
            // Only use the minimal path if it's shorter than the relative display
            // and still within the current working directory context
            if minimal.len() < relative_display.len() {
                minimal.clone()
            } else {
                relative_display
            }
        } else {
            relative_display
        }
    };

    let current_path_string = format_path_string(current_path);

    let current_file_bracket = match (contains_current_path, is_dirty(current_path)) {
        (false, false) => "[ ]",
        (false, true) => "[:]",
        (true, false) => "[-]",
        (true, true) => "[÷]",
    };

    let current_path_display = markup_focused_tab(&format!(
        " {} {} {} ",
        current_file_bracket,
        current_path.icon(),
        current_path_string
    ));

    // No paths in the list
    if marked_paths.is_empty() {
        return Some(current_path_display).into_iter().collect();
    }

    // Generate formatted strings for all paths in the list
    let result: Vec<String> = marked_paths
        .iter()
        .map(|&path| {
            if path == current_path {
                current_path_display.clone()
            } else {
                let file_dirty = is_dirty(path);
                let bracket = if file_dirty { "[÷]" } else { "[-]" };
                format!(" {} {} {} ", bracket, path.icon(), format_path_string(path))
            }
        })
        .collect();
    if !contains_current_path {
        result
            .into_iter()
            .chain(Some(current_path_display))
            .collect()
    } else {
        result
    }
}

#[cfg(test)]
mod test_format_path_list {
    use super::*;
    use anyhow::Result;
    use itertools::Itertools;
    use std::fs;

    use tempfile::tempdir;

    fn run_test_case(
        file_paths: &[&str],
        marked_indices: &[usize],
        dirty_indices: &[usize],
        current_index: usize,
        expected: &str,
    ) -> Result<()> {
        // Setup test environment
        let temp_dir = tempdir()?;

        // Create all test files and collect canonicalized paths
        let paths = file_paths
            .iter()
            .map(|path_str| {
                let file_path = temp_dir.path().join(path_str);

                // Create parent directories if they don't exist
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                fs::write(&file_path, "content")?;
                AbsolutePath::try_from(file_path)
            })
            .collect::<Result<Vec<_>>>()?;

        // Create working directory path
        let cwd = AbsolutePath::try_from(temp_dir.path())?;

        // Create marked paths from indices
        let marked_files = marked_indices.iter().map(|&i| &paths[i]).collect_vec();
        let dirty_files = dirty_indices.iter().map(|&i| &paths[i]).collect_vec();

        // Get current path
        let current_path = &paths[current_index];

        // Test the function
        let result = format_path_list(&marked_files, &dirty_files, current_path, &cwd);

        // Assert result
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_current_path_not_in_list() -> Result<()> {
        run_test_case(
            &["file1.txt", "file2.txt", "current.txt"],
            &[0, 1], // Mark first two files
            &[],     // No files dirty
            2,       // Current is third file (not in list)
            " [-] 📝 file1.txt  [-] 📝 file2.txt \u{200b} [ ] 📝 current.txt \u{200b}",
        )
    }

    #[test]
    fn test_current_path_as_first_file() -> Result<()> {
        run_test_case(
            &["first.txt", "second.txt", "third.txt"],
            &[0, 1, 2], // All files marked
            &[0],       // First file dirty
            0,          // Current is first file
            "\u{200b} [÷] 📝 first.txt \u{200b} [-] 📝 second.txt  [-] 📝 third.txt ",
        )
    }

    #[test]
    fn test_current_path_as_middle_file() -> Result<()> {
        run_test_case(
            &["first.txt", "middle.txt", "last.txt"],
            &[0, 1, 2], // All files marked
            &[],        // No files dirty
            1,          // Current is middle file
            " [-] 📝 first.txt \u{200b} [-] 📝 middle.txt \u{200b} [-] 📝 last.txt ",
        )
    }

    #[test]
    fn test_current_unmarked_saved() -> Result<()> {
        run_test_case(
            &["first.txt", "middle.txt", "last.txt"],
            &[1, 2], // First file is unmarked
            &[1],    // First marked file is dirty
            0,       // Current is middle file
            " [÷] 📝 middle.txt  [-] 📝 last.txt \u{200b} [ ] 📝 first.txt \u{200b}",
        )
    }

    #[test]
    fn test_current_multiple_unmarked_unsaved() -> Result<()> {
        run_test_case(
            &["a.rs", "b.rs", "c.rs", "d.rs"],
            &[1, 2],    // First and last file is unmarked
            &[0, 2, 3], // First marked file is dirty
            3,          // Current is middle file
            " [-] 🦀 b.rs  [÷] 🦀 c.rs \u{200b} [:] 🦀 d.rs \u{200b}",
        )
    }

    #[test]
    fn test_current_path_as_last_file() -> Result<()> {
        run_test_case(
            &["first.txt", "second.txt", "last.txt"],
            &[0, 1, 2], // All files marked
            &[2],       // Last file dirty
            2,          // Current is last file
            " [-] 📝 first.txt  [-] 📝 second.txt \u{200b} [÷] 📝 last.txt \u{200b}",
        )
    }

    #[test]
    fn test_empty_path_list() -> Result<()> {
        run_test_case(
            &["only.txt"],
            &[], // No files marked
            &[], // No files dirty
            0,   // Current is the only file
            "\u{200b} [ ] 📝 only.txt \u{200b}",
        )
    }

    #[test]
    fn test_same_basename_files() -> Result<()> {
        run_test_case(
            &["dir1/same_name.txt", "dir2/same_name.txt"],
            &[1], // Mark the second file
            &[],  // No files dirty
            0,    // Current is first file
            " [-] 📝 dir2/same_name.txt \u{200b} [ ] 📝 dir1/same_name.txt \u{200b}",
        )
    }

    #[test]
    fn test_relative_paths_stay_within_cwd() -> Result<()> {
        run_test_case(
            &["Cargo.txt", "event/Cargo.txt"],
            &[0, 1], // Both files marked
            &[],     // No files dirty
            0,       // Current is first file (root Cargo.txt)
            "\u{200b} [-] 📝 Cargo.txt \u{200b} [-] 📝 event/Cargo.txt ",
        )
    }
}
