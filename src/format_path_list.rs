use itertools::Itertools;
use shared::{canonicalized_path::CanonicalizedPath, get_minimal_unique_paths};

pub(crate) fn format_path_list(
    paths: &[&CanonicalizedPath],
    current_path: &CanonicalizedPath,
    current_working_directory: &CanonicalizedPath,
    dirty: bool,
) -> String {
    debug_assert_eq!(paths.iter().unique().count(), paths.len());
    // Check if current path is in the list
    let current_path_index = paths.iter().position(|&p| p == current_path);
    let contains_current_path = current_path_index.is_some();

    // Create a combined list for minimal unique paths calculation, including the current path
    // even though we won't use its minimal version (we need it for context in minimization)
    let paths_for_minimal = paths
        .iter()
        .map(|&p| p.to_path_buf().clone())
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

    // Helper function to get full path display (not minimized)
    let get_full_path_display = |path: &CanonicalizedPath| -> String {
        path.display_relative_to(current_working_directory)
            .unwrap_or_else(|_| path.display_absolute())
    };

    // Display the full path for the current path (never trimmed)
    let current_path_string = get_full_path_display(current_path);

    // Helper function to format non-current paths
    let format_path_string = |path: &CanonicalizedPath| -> String {
        if path == current_path {
            // Use full path for current path
            current_path_string.clone()
        } else {
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
        }
    };

    // Add dirty indicator if needed
    let dirty_indicator = if dirty { " [*]" } else { "" };

    // Format the current path
    let current_path_display = format!(
        "\u{200B}{}{} {}{} \u{200B}",
        if contains_current_path { " # " } else { "" },
        current_path.icon(),
        current_path_string,
        dirty_indicator
    );

    // No paths in the list
    if paths.is_empty() {
        return current_path_display;
    }

    // Generate formatted strings for all paths in the list
    let formatted_paths: Vec<String> = paths
        .iter()
        .map(|&p| {
            if p == current_path {
                current_path_display.clone()
            } else {
                format!(" # {} {} ", p.icon(), format_path_string(p))
            }
        })
        .collect();

    // Join all formatted paths
    let result = formatted_paths.join("");

    // If current path is not in the list, prepend it
    if !contains_current_path {
        format!("{} {}", current_path_display, result)
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
    use std::path::Path;
    use tempfile::tempdir;

    fn run_test_case(
        file_paths: &[&str],
        marked_indices: &[usize],
        current_index: usize,
        dirty: bool,
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
                CanonicalizedPath::try_from(file_path)
            })
            .collect::<Result<Vec<_>>>()?;

        // Create working directory path
        let cwd = CanonicalizedPath::try_from(temp_dir.path())?;

        // Create marked paths from indices
        let marked_paths = marked_indices.iter().map(|&i| &paths[i]).collect_vec();

        // Get current path
        let current_path = &paths[current_index];

        // Test the function
        let result = format_path_list(&marked_paths, current_path, &cwd, dirty);

        // Assert result
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_current_path_not_in_list() -> Result<()> {
        run_test_case(
            &["file1.txt", "file2.txt", "current.txt"],
            &[0, 1], // Mark first two files
            2,       // Current is third file (not in list)
            false,   // Not dirty
            "\u{200b}📝 current.txt \u{200b}  # 📝 file1.txt  # 📝 file2.txt ",
        )
    }

    #[test]
    fn test_current_path_as_first_file() -> Result<()> {
        run_test_case(
            &["first.txt", "second.txt", "third.txt"],
            &[0, 1, 2], // All files marked
            0,          // Current is first file
            true,       // Dirty
            "\u{200B} # 📝 first.txt [*] \u{200B} # 📝 second.txt  # 📝 third.txt ",
        )
    }

    #[test]
    fn test_current_path_as_middle_file() -> Result<()> {
        run_test_case(
            &["first.txt", "middle.txt", "last.txt"],
            &[0, 1, 2], // All files marked
            1,          // Current is middle file
            false,      // Not dirty
            " # 📝 first.txt \u{200B} # 📝 middle.txt \u{200B} # 📝 last.txt ",
        )
    }

    #[test]
    fn test_current_path_as_last_file() -> Result<()> {
        run_test_case(
            &["first.txt", "second.txt", "last.txt"],
            &[0, 1, 2], // All files marked
            2,          // Current is last file
            true,       // Dirty
            " # 📝 first.txt  # 📝 second.txt \u{200B} # 📝 last.txt [*] \u{200B}",
        )
    }

    #[test]
    fn test_empty_path_list() -> Result<()> {
        run_test_case(
            &["only.txt"],
            &[],   // No files marked
            0,     // Current is the only file
            false, // Not dirty
            "\u{200B}📝 only.txt \u{200B}",
        )
    }

    #[test]
    fn test_current_path_never_minimized() -> Result<()> {
        run_test_case(
            &["dir/bar.txt", "foo.txt"],
            &[1],  // Mark the second file
            0,     // Current is first file
            false, // Not dirty
            "\u{200B}📝 dir/bar.txt \u{200B}  # 📝 foo.txt ",
        )
    }

    #[test]
    fn test_same_basename_files() -> Result<()> {
        run_test_case(
            &["dir1/same_name.txt", "dir2/same_name.txt"],
            &[1],  // Mark the second file
            0,     // Current is first file
            false, // Not dirty
            "\u{200B}📝 dir1/same_name.txt \u{200B}  # 📝 dir2/same_name.txt ",
        )
    }

    #[test]
    fn test_relative_paths_stay_within_cwd() -> Result<()> {
        run_test_case(
            &["Cargo.txt", "event/Cargo.txt"],
            &[0, 1], // Both files marked
            0,       // Current is first file (root Cargo.txt)
            false,   // Not dirty
            "\u{200B} # 📝 Cargo.txt \u{200B} # 📝 event/Cargo.txt ",
        )
    }
}
