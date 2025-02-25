pub(crate) fn find_previous<T>(
    iter: impl Iterator<Item = T>,
    set_last_match_predicate: impl Fn(&T, &Option<T>) -> bool,
    break_predicate: impl Fn(&T) -> bool,
) -> Option<T> {
    let mut last_match = None;
    for match_ in iter {
        if break_predicate(&match_) {
            break;
        }

        if set_last_match_predicate(&match_, &last_match) {
            last_match = Some(match_);
        }
    }
    last_match
}

pub(crate) fn consolidate_errors<T, E: std::fmt::Debug>(
    message: &str,
    results: Vec<Result<T, E>>,
) -> anyhow::Result<()> {
    let errors = results
        .into_iter()
        .filter_map(|result| result.err())
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("{}: {:?}", message, errors))
    }
}

/// Distributes a total number of items into n_parts, with any remainder going to the leading parts.  
/// Returns empty vector if n_parts is 0.  
///
/// # Examples  
///
/// ```
/// assert_eq!(distribute_items(5, 2), vec![3, 2]);
/// assert_eq!(distribute_items(6, 3), vec![2, 2, 2]);
/// ```
pub(crate) fn distribute_items(total: usize, n_parts: usize) -> Vec<usize> {
    if n_parts == 0 {
        return vec![];
    }

    let quotient = total / n_parts;
    let remainder = total % n_parts;

    let result = (0..n_parts)
        .map(|index| quotient + usize::from(index < remainder))
        .collect_vec();
    debug_assert_eq!(result.len(), n_parts);
    debug_assert_eq!(result.iter().sum::<usize>(), total);

    // Expect variance is at maximum 1
    debug_assert!(result.iter().max().unwrap() - result.iter().min().unwrap() <= 1);

    result
}

pub(crate) fn distribute_items_by_2(total: usize) -> (usize, usize) {
    distribute_items(total, 2)
        .into_iter()
        .collect_tuple()
        .unwrap()
}
use itertools::Itertools;
use shared::{canonicalized_path::CanonicalizedPath, get_minimal_unique_paths};

#[cfg(test)]
mod test_utils {
    use super::*;
    mod test_distribute_items {
        use super::*;

        #[test]
        fn test_empty_parts() {
            assert_eq!(distribute_items(5, 0), Vec::<usize>::new());
        }

        #[test]
        fn test_even_distribution() {
            assert_eq!(distribute_items(6, 2), vec![3, 3]);
            assert_eq!(distribute_items(9, 3), vec![3, 3, 3]);
        }

        #[test]
        fn test_uneven_distribution() {
            // One extra
            assert_eq!(distribute_items(5, 2), vec![3, 2]);
            // Two extra
            assert_eq!(distribute_items(8, 3), vec![3, 3, 2]);
            // Multiple extra
            assert_eq!(distribute_items(10, 4), vec![3, 3, 2, 2]);
        }

        #[test]
        fn test_distribution_with_zero_total() {
            assert_eq!(distribute_items(0, 5), vec![0, 0, 0, 0, 0]);
        }

        #[test]
        fn test_distribution_to_one_part() {
            assert_eq!(distribute_items(5, 1), vec![5]);
        }

        #[test]
        fn test_more_parts_than_items() {
            assert_eq!(distribute_items(2, 5), vec![1, 1, 0, 0, 0]);
        }

        #[test]
        fn test_sum_equals_total() {
            let total = 17;
            let n_parts = 5;
            let result = distribute_items(total, n_parts);
            assert_eq!(result.iter().sum::<usize>(), total);
        }
    }
}

use std::ops::Range;

/// Result of a trim operation, containing the trimmed array and any remaining trim count
/// that couldn't be applied while respecting the protected range.
#[derive(Debug)]
pub struct TrimResult<T> {
    pub trimmed_array: Vec<T>,
    pub remaining_trim_count: usize,
}

/// Trims elements from an array while protecting a specified range of indices.
/// The protected range is kept as centered as possible in the resulting array.
///
/// # Arguments
///
/// * `arr` - The input slice to trim
/// * `protected_range` - Range of indices that must be preserved in the output
/// * `trim_count` - Number of elements to remove
///
/// # Returns
///
/// Returns a `TrimResult` containing:
/// * `trimmed_array`: The resulting array after trimming
/// * `remaining_trim_count`: Number of requested trims that couldn't be performed
///
/// # Example
///
/// ```
/// let arr = vec![0, 1, 2, 3, 4, 5, 6];
/// let result = trim_array(&arr, 2..5, 2);
/// assert_eq!(result.trimmed_array, vec![1, 2, 3, 4, 5]);
/// assert_eq!(result.remaining_trim_count, 0);
/// ```
///
/// # Behavior
///
/// * If the array is empty or the range is invalid, returns the original array
/// * Attempts to keep the protected range centered by trimming equally from both sides
/// * When equal trimming isn't possible, trims from the side with more available elements
/// * Never removes elements from the protected range
/// * If requested trim count exceeds available elements, trims what it can and returns the remainder
pub(crate) fn trim_array<T: Clone + std::fmt::Debug>(
    arr: &[T],
    protected_range: Range<usize>,
    trim_count: usize,
) -> TrimResult<T> {
    debug_assert!(protected_range.start <= protected_range.end);
    debug_assert!(protected_range.end <= arr.len());
    if arr.is_empty() {
        return TrimResult {
            trimmed_array: arr.to_vec(),
            remaining_trim_count: trim_count,
        };
    }

    // Calculate elements available for trimming on each side
    let left_available = protected_range.start;
    let right_available = arr.len() - protected_range.end;
    let total_available = left_available + right_available;
    // If we can't trim anything or don't need to, return early
    if total_available == 0 || trim_count == 0 {
        return TrimResult {
            trimmed_array: arr.to_vec(),
            remaining_trim_count: trim_count,
        };
    }

    // Calculate how many elements we can actually trim
    let to_trim = trim_count.min(total_available);

    // Calculate balanced trimming amounts
    // In an unbalanced situation, the right side will be trimmed more than the left side
    // That's why the tuple is unpacked as `(right_trim, left_trim)` instead of `(left_trim, right_trim)`
    // because `distribute_items_by_2` gives the left more in an unbalanced situation
    let (right_trim, left_trim) = distribute_items_by_2(to_trim);

    let (left_trim, right_trim) = (
        (left_trim + right_trim.saturating_sub(right_available)).min(left_available),
        (right_trim + left_trim.saturating_sub(left_available)).min(right_available),
    );
    // Create the result using iterator operations
    let result: Vec<T> = arr
        .iter()
        .skip(left_trim)
        .take(arr.len() - left_trim - right_trim)
        .cloned()
        .collect();

    TrimResult {
        trimmed_array: result,
        remaining_trim_count: trim_count - to_trim,
    }
}

#[cfg(test)]
mod test_trim_array {
    use super::*;

    #[test]
    fn test_basic_centering() {
        let arr = vec![0, 1, 2, 3, 4, 5, 6];
        // Protect [2, 3, 4], trim count 2
        let result = trim_array(&arr, 2..5, 2);
        assert_eq!(result.trimmed_array, vec![1, 2, 3, 4, 5]);
        assert_eq!(result.remaining_trim_count, 0);
    }

    #[test]
    fn test_with_chars() {
        let arr = vec!['0', '1', '2', '3', '4'];
        let result = trim_array(&arr, 2..3, 2);
        assert_eq!(result.trimmed_array, vec!['1', '2', '3']);
        assert_eq!(result.remaining_trim_count, 0);
    }

    #[test]
    fn test_leftmost_protected() {
        let arr = vec![0, 1, 2, 3, 4, 5];
        let result = trim_array(&arr, 0..2, 2);
        assert_eq!(result.trimmed_array, vec![0, 1, 2, 3]);
        assert_eq!(result.remaining_trim_count, 0);
    }

    #[test]
    fn test_rightmost_protected() {
        let arr = vec![0, 1, 2, 3, 4, 5];
        let result = trim_array(&arr, 4..6, 2);
        assert_eq!(result.trimmed_array, vec![2, 3, 4, 5]);
        assert_eq!(result.remaining_trim_count, 0);
    }

    #[test]
    /// Expect right-side to be trimmed more than the left-side in unbalanced situation
    fn test_cant_center_perfectly() {
        let arr = vec![0, 1, 2, 3, 4, 5, 6, 7];
        let result = trim_array(&arr, 3..5, 3);

        assert_eq!(result.trimmed_array, vec![1, 2, 3, 4, 5]);
        assert_eq!(result.remaining_trim_count, 0);
    }

    #[test]
    fn test_empty_array() {
        let arr: Vec<i32> = vec![];
        let result = trim_array(&arr, 0..0, 1);
        assert_eq!(result.trimmed_array.len(), 0);
        assert_eq!(result.remaining_trim_count, 1);
    }

    #[test]
    #[should_panic]
    fn test_invalid_range() {
        let arr = vec![0, 1, 2];
        let result = trim_array(&arr, 4..5, 1);
        assert_eq!(result.trimmed_array, vec![0, 1, 2]);
        assert_eq!(result.remaining_trim_count, 1);
    }

    #[cfg(test)]
    mod property_tests {
        use super::*;
        use quickcheck::{Arbitrary, Gen, TestResult};
        use quickcheck_macros::quickcheck;

        #[derive(Debug, Clone)]
        struct TestInput {
            array: Vec<u8>,
            range: Range<usize>,
            trim_count: usize,
        }

        impl Arbitrary for TestInput {
            fn arbitrary(g: &mut Gen) -> Self {
                // First generate the array
                let array: Vec<u8> = (0..u8::arbitrary(g) % 10).collect();
                if array.is_empty() {
                    return TestInput {
                        array: vec![0], // Ensure at least one element
                        range: 0..0,
                        trim_count: 0,
                    };
                }

                // Generate valid range indices
                let start = usize::arbitrary(g) % array.len();
                let max_end = array.len();
                let end = (start + (usize::arbitrary(g) % (max_end - start + 1))).max(start + 1);
                let range = start..end;

                // Generate trim count
                let trim_count = usize::arbitrary(g) % array.len();

                TestInput {
                    array,
                    range,
                    trim_count,
                }
            }
        }

        fn is_subsequence<T: PartialEq + std::fmt::Debug>(shorter: &[T], longer: &[T]) -> bool {
            let mut long_iter = longer.iter();
            shorter.iter().all(|x| long_iter.by_ref().any(|y| x == y))
        }

        #[quickcheck]
        fn protected_range_preserved(input: TestInput) -> TestResult {
            let result = trim_array(&input.array, input.range.clone(), input.trim_count);
            let protected = &input.array[input.range];
            let expected_subsequence = protected.to_vec();

            TestResult::from_bool(is_subsequence(&expected_subsequence, &result.trimmed_array))
        }

        #[quickcheck]
        fn correct_length_after_trim(input: TestInput) -> TestResult {
            let result = trim_array(&input.array, input.range, input.trim_count);
            let actual_trims = input.trim_count - result.remaining_trim_count;

            TestResult::from_bool(result.trimmed_array.len() == input.array.len() - actual_trims)
        }

        #[quickcheck]
        fn never_exceeds_trim_count(input: TestInput) -> bool {
            let result = trim_array(&input.array, input.range, input.trim_count);
            input.array.len() - result.trimmed_array.len() <= input.trim_count
        }

        #[quickcheck]
        fn maintains_element_order(input: TestInput) -> bool {
            let result = trim_array(&input.array, input.range, input.trim_count);
            is_subsequence(&result.trimmed_array, &input.array)
        }

        #[quickcheck]
        fn trim_count_conservation(input: TestInput) -> bool {
            let result = trim_array(&input.array, input.range, input.trim_count);
            let actual_trims = input.array.len() - result.trimmed_array.len();
            actual_trims + result.remaining_trim_count == input.trim_count
        }
    }
}

pub(crate) fn format_path_list(
    paths: &[&CanonicalizedPath],
    current_path: &CanonicalizedPath,
    current_working_directory: &CanonicalizedPath,
    dirty: bool,
) -> String {
    // Check if current path is in the list
    let current_path_index = paths.iter().position(|&p| p == current_path);
    let contains_current_path = current_path_index.is_some();

    // Create a combined list for minimal unique paths calculation
    let mut all_paths_for_minimal = paths.iter().map(|&p| p.to_path_buf().clone()).collect_vec();
    if !contains_current_path {
        all_paths_for_minimal.push(current_path.to_path_buf().clone());
    }

    // Compute minimal unique paths with the current path included
    let minimal_unique_paths =
        get_minimal_unique_paths::get_minimal_unique_paths(&all_paths_for_minimal);

    // Helper function to format a path string consistently
    let format_path_string = |path: &CanonicalizedPath| -> String {
        minimal_unique_paths
            .get(path.to_path_buf())
            .cloned()
            .unwrap_or_else(|| {
                path.display_relative_to(current_working_directory)
                    .unwrap_or_else(|_| path.display_absolute())
            })
    };

    // Add dirty indicator if needed
    let dirty_indicator = if dirty { " [*]" } else { "" };

    // Format the current path
    let current_path_string = format_path_string(current_path);
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
    use tempfile::tempdir;

    fn run_test_case(
        file_names: &[&str],
        marked_indices: &[usize],
        current_index: usize,
        dirty: bool,
        expected: &str,
    ) -> Result<()> {
        // Setup test environment
        let temp_dir = tempdir()?;

        // Create all test files and collect canonicalized paths
        let paths = file_names
            .iter()
            .map(|name| {
                let file_path = temp_dir.path().join(name);
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
}
