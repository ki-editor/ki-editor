use std::collections::HashSet;

use itertools::Itertools;

use crate::{components::editor::ViewAlignment, utils::distribute_items_by_2};

/// Divides a set of line numbers into viewport sections based on a focused line and viewport constraints.
///
/// This function takes a set of line numbers and creates viewport sections that fit within the given
/// viewport height while maintaining context around the focused line. It handles various edge cases
/// including merging adjacent sections and distributing available space.
pub(crate) fn divide_viewport(
    line_numbers: &[usize],
    focused_line_number: usize,
    viewport_height: usize,
    max_line_index: usize,
    view_alignment: ViewAlignment,
) -> Vec<ViewportSectionWithOrigin> {
    debug_assert!(line_numbers
        .iter()
        .all(|line_number| *line_number <= max_line_index));
    let line_numbers = line_numbers
        .into_iter()
        .map(|line_number| *line_number)
        .unique()
        .sorted()
        .collect_vec();
    if line_numbers.len() > viewport_height {
        return extract_centered_window(&line_numbers, focused_line_number, viewport_height)
            .into_iter()
            .map(|line_number| ViewportSectionWithOrigin {
                start: line_number,
                end: line_number,
                start_original: line_number,
                end_original: line_number,
            })
            .collect_vec();
    }

    if line_numbers.is_empty() {
        return Vec::new();
    }
    divide_viewport_impl(
        line_numbers
            .into_iter()
            .map(|line_number| ViewportSectionOnlyOrigin {
                start_original: line_number,
                end_original: line_number,
            })
            .collect_vec(),
        viewport_height,
        max_line_index,
        view_alignment,
    )
}

fn divide_viewport_impl(
    input_sections: Vec<ViewportSectionOnlyOrigin>,
    viewport_height: usize,
    max_line_index: usize,
    view_alignment: ViewAlignment,
) -> Vec<ViewportSectionWithOrigin> {
    if viewport_height <= input_sections.len() {
        return input_sections
            .into_iter()
            .map(|section| section.into_viewport_section_with_origin())
            .collect_vec();
    }

    let sections_length = input_sections.len();
    let input_lengths = input_sections
        .iter()
        .map(|section| section.len())
        .collect_vec();
    let context_lines_lengths = calculate_distribution(
        &input_lengths,
        (viewport_height as usize).saturating_sub(input_lengths.iter().sum()),
    ); //distribute_items(viewport_height as usize, sections_length);

    let result_sections = input_sections
        .iter()
        .zip(context_lines_lengths)
        .map(|(section, context_lines_length)| {
            let (lower_context_lines_length, upper_context_lines_length) = {
                let context_lines_length = context_lines_length.saturating_sub(section.len());
                match view_alignment {
                    ViewAlignment::Top => (0, context_lines_length),
                    ViewAlignment::Center => distribute_items_by_2(context_lines_length),
                    ViewAlignment::Bottom => (context_lines_length, 0),
                }
            };
            let lower_context_lines_length = lower_context_lines_length
                + upper_context_lines_length
                    .saturating_sub(max_line_index.saturating_sub(section.end_original));
            let upper_context_lines_length = upper_context_lines_length
                + lower_context_lines_length.saturating_sub(section.start_original);
            ViewportSectionWithOrigin {
                start: section
                    .start_original
                    .saturating_sub(lower_context_lines_length as usize),
                end: section
                    .end_original
                    .saturating_add(upper_context_lines_length)
                    .min(max_line_index),
                start_original: section.start_original,
                end_original: section.end_original,
            }
        })
        .collect_vec();
    // Merge overlapping sections
    let merged = result_sections.iter().fold(
        vec![],
        |accum: Vec<ViewportSectionWithOrigin>, current_section| {
            if let Some((last_section, init)) = accum.split_last() {
                let last_range_set = last_section.range_set();
                let current_range_set = current_section.range_set();
                let intersected_lines = last_range_set
                    .intersection(&current_range_set)
                    .collect_vec();
                if intersected_lines.is_empty() {
                    init.into_iter()
                        .map(|section| section.clone())
                        .chain(Some(last_section.clone()))
                        .chain(Some(current_section.clone()))
                        .collect_vec()
                } else {
                    let merged_section = ViewportSectionWithOrigin {
                        start: last_section.start_original,
                        end: current_section.end_original,
                        start_original: last_section.start_original,
                        end_original: current_section.end_original,
                    };
                    init.into_iter()
                        .map(|section| section.clone())
                        .chain(Some(merged_section))
                        .collect()
                }
            } else {
                accum
                    .into_iter()
                    .chain(Some(current_section.clone()))
                    .collect_vec()
            }
        },
    );
    if merged.len() < sections_length {
        divide_viewport_impl(
            merged
                .into_iter()
                .map(|section| section.into_viewport_section_only_origin())
                .collect_vec(),
            viewport_height,
            max_line_index,
            view_alignment,
        )
    } else {
        result_sections
    }
}

fn extract_centered_window<T: Ord + Eq + Clone + std::fmt::Debug>(
    elements: &[T],
    element: T,
    window_size: usize,
) -> Vec<T> {
    debug_assert!(elements.contains(&element));
    debug_assert!(elements.iter().is_sorted());
    let Some(index) = elements
        .iter()
        .position(|line_number| *line_number == element)
    else {
        // This should be unreachable
        // but let's say it happens we just simply trim `elements` by `window_size`.
        return elements
            .into_iter()
            .take(window_size)
            .map(|element| element.clone())
            .collect_vec();
    };

    let (go_left_by, go_right_by) = distribute_items_by_2(window_size.saturating_sub(1));

    let go_left_by =
        go_left_by + (index + go_right_by).saturating_sub(elements.len().saturating_sub(1));

    let go_right_by = go_right_by + go_left_by.saturating_sub(index);
    let start_index = index.saturating_sub(go_left_by);
    let end_index = (index + go_right_by).min(elements.len().saturating_sub(1));
    let result = elements[start_index..end_index + 1].to_vec();

    debug_assert!(result.contains(&element));
    debug_assert_eq!(result.len(), window_size);
    result
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct ViewportSection {
    /// Inclusive (line number)
    pub(crate) start: usize,
    /// Inclusive (line number)
    end: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct ViewportSectionWithOrigin {
    /// Inclusive (line number)
    start: usize,
    /// Inclusive (line number)
    end: usize,
    /// Inclusive (line number)
    start_original: usize,
    /// Inclusive (line number)
    end_original: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct ViewportSectionOnlyOrigin {
    /// Inclusive (line number)
    start_original: usize,
    /// Inclusive (line number)
    end_original: usize,
}

impl ViewportSectionOnlyOrigin {
    fn into_viewport_section_with_origin(self) -> ViewportSectionWithOrigin {
        ViewportSectionWithOrigin {
            start: self.start_original,
            end: self.end_original,
            start_original: self.start_original,
            end_original: self.end_original,
        }
    }

    fn len(&self) -> usize {
        self.end_original + 1 - self.start_original
    }
}
impl ViewportSectionWithOrigin {
    fn range_set(&self) -> HashSet<usize> {
        (self.start..self.end + 1).collect()
    }

    fn into_viewport_section_only_origin(self) -> ViewportSectionOnlyOrigin {
        ViewportSectionOnlyOrigin {
            start_original: self.start_original,
            end_original: self.end_original,
        }
    }

    fn into_viewport_section(self) -> ViewportSection {
        ViewportSection {
            start: self.start,
            end: self.end,
        }
    }

    pub(crate) fn height(&self) -> usize {
        (self.end + 1).saturating_sub(self.start)
    }

    pub(crate) fn start(&self) -> usize {
        self.start
    }

    pub(crate) fn end_original(&self) -> usize {
        self.end_original
    }
}

impl ViewportSection {
    #[cfg(test)]
    fn range_set(&self) -> HashSet<usize> {
        (self.start..self.end + 1).collect()
    }

    pub(crate) fn range_vec(&self) -> Vec<usize> {
        (self.start..self.end + 1).collect()
    }

    pub(crate) fn height(&self) -> usize {
        (self.end + 1).saturating_sub(self.start)
    }

    pub(crate) fn start(&self) -> usize {
        self.start
    }
}

#[cfg(test)]
mod test_divide_viewport {
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;
    use rand::random;

    use super::*;

    #[test]
    fn view_alignments() {
        let result = divide_viewport(&[5], 5, 3, 100, ViewAlignment::Center);
        assert_eq!(
            result,
            vec![ViewportSectionWithOrigin {
                start: 4,
                start_original: 4,
                end: 6,
                end_original: 6
            }]
        );

        let result = divide_viewport(&[5], 5, 3, 100, ViewAlignment::Bottom);
        assert_eq!(
            result,
            vec![ViewportSectionWithOrigin {
                start: 3,
                start_original: 3,
                end: 5,
                end_original: 5
            }]
        );

        let result = divide_viewport(&[5], 5, 3, 100, ViewAlignment::Top);
        assert_eq!(
            result,
            vec![ViewportSectionWithOrigin {
                start: 5,
                start_original: 5,
                end: 7,
                end_original: 7
            }]
        );
    }

    #[test]
    fn prioritize_above_over_bottom_for_uneven_split() {
        let result = divide_viewport(&[10], 10, 4, 100, ViewAlignment::Center);
        // Two above line 10
        // One belowe line 10
        assert_eq!(
            result,
            vec![ViewportSectionWithOrigin {
                start: 8,
                start_original: 8,
                end: 11,
                end_original: 11
            }]
        );
    }

    #[test]
    fn line_numbers_length_more_than_viewport_height_focus_start() {
        let result = divide_viewport(&[1, 2, 3, 4], 1, 3, 100, ViewAlignment::Center);
        assert_eq!(
            result,
            vec![
                ViewportSectionWithOrigin {
                    start: 1,
                    start_original: 1,
                    end: 1,
                    end_original: 1
                },
                ViewportSectionWithOrigin {
                    start: 2,
                    start_original: 2,
                    end: 2,
                    end_original: 2
                },
                ViewportSectionWithOrigin {
                    start: 3,
                    start_original: 3,
                    end: 3,
                    end_original: 3
                }
            ]
        );
    }

    #[test]
    fn line_numbers_length_more_than_viewport_height_focus_middle() {
        let result = divide_viewport(&[1, 2, 3, 4, 5], 3, 3, 100, ViewAlignment::Center);
        assert_eq!(
            result,
            vec![
                ViewportSectionWithOrigin {
                    start: 2,
                    start_original: 2,
                    end: 2,
                    end_original: 2
                },
                ViewportSectionWithOrigin {
                    start: 3,
                    start_original: 3,
                    end: 3,
                    end_original: 3
                },
                ViewportSectionWithOrigin {
                    start: 4,
                    start_original: 4,
                    end: 4,
                    end_original: 4
                },
            ]
        )
    }

    #[test]
    fn line_numbers_length_more_than_viewport_height_focus_end() {
        let result = divide_viewport(&[1, 2, 3, 4, 5], 5, 3, 100, ViewAlignment::Center);
        assert_eq!(
            result,
            vec![
                ViewportSectionWithOrigin {
                    start: 3,
                    start_original: 3,
                    end: 3,
                    end_original: 3
                },
                ViewportSectionWithOrigin {
                    start: 4,
                    start_original: 4,
                    end: 4,
                    end_original: 4
                },
                ViewportSectionWithOrigin {
                    start: 5,
                    start_original: 5,
                    end: 5,
                    end_original: 5
                },
            ]
        )
    }

    #[test]
    fn test_single_cursor_line() {
        let result = divide_viewport(&[10], 10, 5, 100, ViewAlignment::Center);
        assert_eq!(
            result,
            vec![ViewportSectionWithOrigin {
                start: 8,
                start_original: 8,
                end: 12,
                end_original: 12
            }]
        );
    }

    #[test]
    fn test_duplicate_lines() {
        let result = divide_viewport(&[10, 10, 10], 10, 5, 100, ViewAlignment::Center);
        assert_eq!(
            result,
            vec![ViewportSectionWithOrigin {
                start: 8,
                start_original: 8,
                end: 12,
                end_original: 12
            }]
        );
    }

    #[test]
    fn test_adjacent_lines_merged() {
        let result = divide_viewport(&[10, 11], 10, 5, 100, ViewAlignment::Center);
        assert_eq!(
            result,
            vec![ViewportSectionWithOrigin {
                start: 8,
                start_original: 8,
                end: 12,
                end_original: 12
            }]
        );
    }

    #[test]
    fn test_distant_lines_split() {
        let result = divide_viewport(&[10, 20], 10, 6, 100, ViewAlignment::Center);
        assert_eq!(
            result,
            vec![
                ViewportSectionWithOrigin {
                    start: 9,
                    start_original: 9,
                    end: 11,
                    end_original: 11
                },
                ViewportSectionWithOrigin {
                    start: 19,
                    start_original: 19,
                    end: 21,
                    end_original: 21
                }
            ]
        );
    }

    #[test]
    fn test_smaller_line_numbers_receive_larger_portions_on_uneven_division() {
        let result = divide_viewport(&[10, 11, 12, 13], 11, 6, 100, ViewAlignment::Center);
        assert_eq!(
            result,
            vec![ViewportSectionWithOrigin {
                start: 9,
                start_original: 9,
                end: 14,
                end_original: 14
            }]
        );
    }

    #[test]
    fn test_first_line_edge_case() {
        let result = divide_viewport(&[0], 0, 4, 100, ViewAlignment::Center);
        assert_eq!(
            result,
            vec![ViewportSectionWithOrigin {
                start: 0,
                start_original: 0,
                end: 3,
                end_original: 3
            }]
        );
    }

    #[test]
    fn test_last_line_edge_case() {
        let result = divide_viewport(&[99], 99, 4, 100, ViewAlignment::Center);
        assert_eq!(
            result,
            vec![ViewportSectionWithOrigin {
                start: 97,
                start_original: 97,
                end: 100,
                end_original: 100
            }]
        );
    }

    #[test]
    fn test_mixed_edge_cases_1() {
        let result = divide_viewport(&[0, 1, 98, 99], 1, 8, 100, ViewAlignment::Center);
        assert_eq!(
            result,
            vec![
                ViewportSectionWithOrigin {
                    start: 0,
                    start_original: 0,
                    end: 3,
                    end_original: 3
                },
                ViewportSectionWithOrigin {
                    start: 97,
                    start_original: 97,
                    end: 100,
                    end_original: 100
                }
            ]
        );
    }

    #[test]
    fn test_mixed_edge_cases_2() {
        let result = divide_viewport(&[0, 1, 100], 1, 8, 100, ViewAlignment::Center);
        assert_eq!(
            result,
            vec![
                ViewportSectionWithOrigin {
                    start: 0,
                    start_original: 0,
                    end: 3,
                    end_original: 3
                },
                ViewportSectionWithOrigin {
                    start: 97,
                    start_original: 97,
                    end: 100,
                    end_original: 100
                }
            ]
        );
    }

    #[test]
    fn test_even_distribution() {
        let result = divide_viewport(&[10, 20, 30], 20, 9, 100, ViewAlignment::Center);
        assert_eq!(
            result,
            vec![
                ViewportSectionWithOrigin {
                    start: 9,
                    start_original: 9,
                    end: 11,
                    end_original: 11
                },
                ViewportSectionWithOrigin {
                    start: 19,
                    start_original: 19,
                    end: 21,
                    end_original: 21
                },
                ViewportSectionWithOrigin {
                    start: 29,
                    start_original: 29,
                    end: 31,
                    end_original: 31
                }
            ]
        );
    }

    #[test]
    fn test_sections_within_viewport() {
        let viewport_height = 8;
        let lines = vec![5, 6, 15];
        let result = divide_viewport(&lines, 6, viewport_height, 100, ViewAlignment::Center);

        let total_lines: usize = result
            .iter()
            .map(|section| section.end - section.start + 1)
            .sum();
        assert!(
            total_lines <= viewport_height,
            "Total lines {} exceeds viewport height {}",
            total_lines,
            viewport_height
        );
    }

    #[derive(Debug, Clone)]
    struct Input {
        max_line_index: usize,
        viewport_height: usize,
        line_numbers: Vec<usize>,
    }

    impl Arbitrary for Input {
        fn arbitrary(g: &mut Gen) -> Self {
            let viewport_height = (usize::arbitrary(g) % 10).max(1);

            // `max_line_index` should be always larger than `viewport_height`
            // for this specific quickcheck test case
            let max_line_index = (usize::arbitrary(g) % 10).max(viewport_height);

            let line_numbers_length = (usize::arbitrary(g) % 10).max(1);
            let line_numbers = (0..line_numbers_length)
                .map(|_| usize::arbitrary(g) % (max_line_index + 1))
                .collect_vec();

            Input {
                max_line_index,
                line_numbers,
                viewport_height,
            }
        }
    }
    #[quickcheck]
    fn sum_of_viewport_sections_height_should_equal_viewport_height(input: Input) -> bool {
        let Input {
            max_line_index,
            viewport_height,
            line_numbers,
        } = input;
        let random_index = random::<usize>() % line_numbers.len();
        let focused_line_number = line_numbers.get(random_index).unwrap();
        let sections = divide_viewport(
            &line_numbers,
            *focused_line_number,
            viewport_height,
            max_line_index,
            ViewAlignment::Center,
        );

        let sum: usize = sections
            .into_iter()
            .map(|section| section.range_set().len())
            .sum();
        sum == viewport_height
    }

    #[quickcheck]
    fn no_viewport_sections_should_intersect(input: Input) -> bool {
        let Input {
            max_line_index,
            viewport_height,
            line_numbers,
        } = input;
        let random_index = random::<usize>() % line_numbers.len();
        let focused_line_number = line_numbers.get(random_index).unwrap();
        let sections = divide_viewport(
            &line_numbers,
            *focused_line_number,
            viewport_height,
            max_line_index,
            ViewAlignment::Center,
        );

        sections.iter().enumerate().all(|(index, section)| {
            !sections
                .iter()
                .enumerate()
                .filter(|(other_index, _)| other_index != &index)
                .any(|(_, other_section)| {
                    other_section
                        .range_set()
                        .intersection(&section.range_set())
                        .count()
                        > 0
                })
        })
    }
}

/// Distributes additional resources fairly by repeatedly allocating to the element with the lowest value.
/// Takes initial amounts and a total resource count, returns a new vector with resources distributed.
fn calculate_distribution(initial_amounts: &[usize], total_resource_count: usize) -> Vec<usize> {
    let mut final_amounts = initial_amounts.to_vec();
    let mut remaining = total_resource_count;

    while remaining > 0 {
        // Find index of minimum value
        let min_idx = final_amounts
            .iter()
            .enumerate()
            .min_by_key(|&(_, value)| value)
            .map(|(index, _)| index)
            .unwrap();

        final_amounts[min_idx] += 1;
        remaining -= 1;
    }

    final_amounts
}

#[cfg(test)]
mod tests_calculate_distribution {
    use super::*;

    #[test]
    fn test_calculate_distribution() {
        // Basic case
        assert_eq!(calculate_distribution(&[5, 2, 0], 3), vec![5, 3, 2]);

        // Empty resources
        assert_eq!(calculate_distribution(&[1, 1, 1], 0), vec![1, 1, 1]);

        // Equal initial values
        assert_eq!(calculate_distribution(&[2, 2, 2], 3), vec![3, 3, 3]);

        // Single element
        assert_eq!(calculate_distribution(&[0], 5), vec![5]);

        // Large gap between values
        assert_eq!(calculate_distribution(&[10, 0, 0], 6), vec![10, 3, 3]);

        // More resources than needed for perfect balance
        assert_eq!(calculate_distribution(&[3, 0], 10), vec![7, 6]);

        // All zeros initial
        assert_eq!(calculate_distribution(&[0, 0, 0, 0], 7), vec![2, 2, 2, 1]);

        // Different lengths
        assert_eq!(
            calculate_distribution(&[5, 4, 3, 2, 1], 5),
            vec![5, 4, 4, 4, 3]
        );
    }
}
