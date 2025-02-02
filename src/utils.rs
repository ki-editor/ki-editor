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

/// Distributes a total number of items into n_parts, with any remainder going to the first parts.  
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

    (0..n_parts)
        .map(|index| quotient + usize::from(index < remainder))
        .collect()
}
use itertools::Itertools;

pub(crate) fn get_non_consecutive_nums(nums: &[usize]) -> Vec<usize> {
    if nums.is_empty() {
        return vec![];
    }
    std::iter::once(&nums[0])
        .chain(
            nums.iter()
                .tuple_windows()
                .filter(|(&a, &b)| b - a > 1)
                .map(|(_, b)| b),
        )
        .copied()
        .collect()
}

#[cfg(test)]
mod test_utils {
    use super::*;
    mod test_get_non_consecutive_nums {
        use super::*;

        #[test]
        fn test_basic_case() {
            let nums = vec![1, 2, 3, 5, 6, 8, 9];
            assert_eq!(get_non_consecutive_nums(&nums), vec![1, 5, 8]);
        }

        #[test]
        fn test_all_consecutive() {
            let nums = vec![1, 2, 3, 4, 5];
            assert_eq!(get_non_consecutive_nums(&nums), vec![1]);
        }

        #[test]
        fn test_no_consecutive() {
            let nums = vec![2, 4, 6, 8, 10];
            assert_eq!(get_non_consecutive_nums(&nums), vec![2, 4, 6, 8, 10]);
        }

        #[test]
        fn test_single_element() {
            let nums = vec![1];
            assert_eq!(get_non_consecutive_nums(&nums), vec![1]);
        }

        #[test]
        fn test_large_gaps() {
            let nums = vec![1, 10, 20, 30];
            assert_eq!(get_non_consecutive_nums(&nums), vec![1, 10, 20, 30]);
        }

        #[test]
        fn test_alternating_consecutive() {
            let nums = vec![1, 2, 4, 5, 7, 8];
            assert_eq!(get_non_consecutive_nums(&nums), vec![1, 4, 7]);
        }

        #[test]
        fn test_empty_slice() {
            let nums: Vec<usize> = vec![];
            assert_eq!(get_non_consecutive_nums(&nums).len(), 0);
        }
    }
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
