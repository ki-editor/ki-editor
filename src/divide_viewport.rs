use crate::components::editor::ViewAlignment;
use itertools::Itertools;

use crate::utils::{distribute_items, distribute_items_by_2};

pub(crate) fn divide_viewport<T: Clone + std::fmt::Debug + Eq>(
    items: &Vec<T>,
    viewport_height: usize,
    focused_item: T,
) -> Vec<ViewportSection<T>> {
    let result = if viewport_height < items.len() {
        extract_centered_window(
            items,
            |byte_range| byte_range == &focused_item,
            viewport_height,
            ViewAlignment::Center,
        )
        .into_iter()
        .map(|byte_range| ViewportSection {
            item: byte_range,
            height: 1,
        })
        .collect_vec()
    } else {
        distribute_items(viewport_height, items.len())
            .into_iter()
            .zip(items)
            .map(|(height, byte_range)| ViewportSection {
                item: byte_range.clone(),
                height,
            })
            .collect()
    };
    debug_assert_eq!(
        result.iter().map(|section| section.height).sum::<usize>(),
        viewport_height
    );
    result
}

#[derive(Debug)]
pub(crate) struct ViewportSection<T> {
    item: T,
    height: usize,
}
impl<T: Clone> ViewportSection<T> {
    pub(crate) fn height(&self) -> usize {
        self.height
    }

    pub(crate) fn item(&self) -> T {
        self.item.clone()
    }
}

pub(crate) fn extract_centered_window<T: Eq + Clone + std::fmt::Debug, F: Fn(&T) -> bool>(
    elements: &[T],
    predicate: F,
    window_size: usize,
    view_alignment: ViewAlignment,
) -> Vec<T> {
    debug_assert!(elements.iter().any(&predicate));
    let Some(index) = elements.iter().position(&predicate) else {
        // This should be unreachable
        // but let's say it happens we just simply trim `elements` by `window_size`.
        return elements.iter().take(window_size).cloned().collect_vec();
    };

    let window = calculate_window_position(index, elements.len(), window_size, view_alignment);

    let result = elements[window.start..=window.end].to_vec();

    debug_assert!(result.iter().any(&predicate));
    debug_assert_eq!(result.len(), window_size);
    result
}

#[cfg(test)]
mod test_divide_viewport {
    use super::*;
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;

    #[test]
    fn test_viewport_larger_than_sections() {
        let items = vec![1, 2, 3]; // 3 sections
        let viewport_height = 6; // Larger than number of sections
        let focused_item = 2;

        let result = divide_viewport(&items, viewport_height, focused_item);

        assert_eq!(result.len(), 3);
        // Since viewport_height (6) is distributed among 3 sections,
        // we expect heights of 2,2,2
        assert_eq!(result[0].height(), 2);
        assert_eq!(result[1].height(), 2);
        assert_eq!(result[2].height(), 2);

        // Verify items are preserved
        assert_eq!(result[0].item(), 1);
        assert_eq!(result[1].item(), 2);
        assert_eq!(result[2].item(), 3);
    }

    #[test]
    fn test_viewport_smaller_than_sections() {
        let items = vec![1, 2, 3, 4, 5]; // 5 sections
        let viewport_height = 3; // Smaller than number of sections
        let focused_item = 3; // Focus on middle section

        let result = divide_viewport(&items, viewport_height, focused_item);

        assert_eq!(result.len(), viewport_height);
        // We expect 3 sections centered around the focused item
        assert_eq!(result[0].item(), 2);
        assert_eq!(result[1].item(), 3);
        assert_eq!(result[2].item(), 4);

        // Each section should have height 1 since viewport is smaller
        assert!(result.iter().all(|section| section.height() == 1));
    }

    // Helper struct for QuickCheck testing
    #[derive(Debug, Clone)]
    struct TestInput {
        items: Vec<usize>,
        viewport_height: usize,
        focused_index: usize,
    }

    impl Arbitrary for TestInput {
        fn arbitrary(g: &mut Gen) -> Self {
            let num_items = (usize::arbitrary(g) % 10) + 1; // At least 1 item
            let items: Vec<usize> = (1..=num_items).collect();

            let viewport_height = (usize::arbitrary(g) % 10) + 1; // At least height 1
            let focused_index = usize::arbitrary(g) % items.len();

            TestInput {
                items,
                viewport_height,
                focused_index,
            }
        }
    }

    #[quickcheck]
    fn quickcheck_viewport_height_sum(input: TestInput) -> bool {
        let focused_item = input.items[input.focused_index];
        let result = divide_viewport(&input.items, input.viewport_height, focused_item);

        // Property: sum of heights equals viewport height
        let height_sum: usize = result.iter().map(|section| section.height()).sum();
        height_sum == input.viewport_height
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct WindowPosition {
    /// Start index of the window (inclusive)
    pub(crate) start: usize,
    /// End index of the window (inclusive)
    pub(crate) end: usize,
}

impl WindowPosition {
    /// Returns the size of the window
    pub(crate) fn size(&self) -> usize {
        self.end - self.start + 1
    }
}

/// Calculates the optimal window position around a target index based on the given constraints.
///
/// This function determines where to position a window of a given size within total_len items,
/// such that it contains the target index while respecting the specified view alignment.
/// When the target is near the boundaries (start or end), the window will automatically
/// adjust to maximize the visible context.
///
/// # Arguments
///
/// * `index` - The target index that the window should contain
/// * `total_len` - Total number of items available
/// * `window_size` - Size of the window to position
/// * `view_alignment` - Preferred alignment of the target within the window
///
/// # Returns
///
/// Returns a `WindowPosition` containing the start and end indices of the calculated window.
/// The window is guaranteed to:
/// - Have the specified size (unless total_len is smaller)
/// - Contain the target index
/// - Be within bounds [0, total_len)
///
/// # Examples
///
/// ```
/// // Center a window of size 5 around index 10 in a buffer of 100 items
/// let pos = calculate_window_position(10, 100, 5, ViewAlignment::Center);
/// assert_eq!(pos.size(), 5);
/// assert!(pos.start <= 10 && pos.end >= 10);
///
/// // Position a window at the start of the buffer
/// let pos = calculate_window_position(1, 100, 5, ViewAlignment::Top);
/// assert_eq!(pos.start, 0);
///
/// // Position a window at the end of the buffer
/// let pos = calculate_window_position(98, 100, 5, ViewAlignment::Bottom);
/// assert_eq!(pos.end, 99);
/// ```
pub(crate) fn calculate_window_position(
    index: usize,
    total_len: usize,
    window_size: usize,
    view_alignment: ViewAlignment,
) -> WindowPosition {
    let go_by = window_size.saturating_sub(1);
    let (go_left_by, go_right_by) = match view_alignment {
        ViewAlignment::Top => (0, go_by),
        ViewAlignment::Center => distribute_items_by_2(go_by),
        ViewAlignment::Bottom => (go_by, 0),
    };

    let (go_left_by, go_right_by) = (
        go_left_by + (index + go_right_by).saturating_sub(total_len.saturating_sub(1)),
        go_right_by + go_left_by.saturating_sub(index),
    );

    let start = index.saturating_sub(go_left_by);
    let end = (index + go_right_by).min(total_len.saturating_sub(1));

    WindowPosition { start, end }
}

#[cfg(test)]
mod test_calculate_window_position {
    use super::*;

    #[test]
    fn test_normal_center_alignment() {
        let pos = calculate_window_position(10, 100, 5, ViewAlignment::Center);
        assert_eq!(pos.size(), 5);
        assert_eq!(pos.start, 8);
        assert_eq!(pos.end, 12);
    }

    #[test]
    fn test_normal_top_alignment() {
        let pos = calculate_window_position(10, 100, 5, ViewAlignment::Top);
        assert_eq!(pos.size(), 5);
        assert_eq!(pos.start, 10);
        assert_eq!(pos.end, 14);
    }

    #[test]
    fn test_normal_bottom_alignment() {
        let pos = calculate_window_position(10, 100, 5, ViewAlignment::Bottom);
        assert_eq!(pos.size(), 5);
        assert_eq!(pos.start, 6);
        assert_eq!(pos.end, 10);
    }

    #[test]
    fn test_near_start() {
        let pos = calculate_window_position(1, 100, 5, ViewAlignment::Bottom);
        assert_eq!(pos.size(), 5);
        assert_eq!(pos.start, 0);
        assert_eq!(pos.end, 4);
    }

    #[test]
    fn test_near_end() {
        // Even with top alignment, when near end it should adjust to show until end
        let pos = calculate_window_position(98, 100, 5, ViewAlignment::Top);
        assert_eq!(pos.size(), 5);
        assert_eq!(pos.start, 95);
        assert_eq!(pos.end, 99);
    }

    #[test]
    fn test_small_total_len() {
        // When total_len is smaller than window_size
        let pos = calculate_window_position(1, 3, 5, ViewAlignment::Center);
        assert_eq!(pos.start, 0);
        assert_eq!(pos.end, 2);
        assert_eq!(pos.size(), 3); // Size is capped by total_len
    }

    #[test]
    fn test_zero_total_len() {
        let pos = calculate_window_position(0, 0, 5, ViewAlignment::Center);
        assert_eq!(pos.start, 0);
        assert_eq!(pos.end, 0);
        assert_eq!(pos.size(), 1);
    }

    #[test]
    fn test_index_out_of_bounds() {
        // When index is beyond total_len, should handle gracefully
        let pos = calculate_window_position(101, 100, 5, ViewAlignment::Center);
        assert_eq!(pos.size(), 5);
        assert!(pos.end < 100);
        assert!(pos.start <= pos.end);
    }
    #[cfg(test)]
    mod property {
        use super::*;
        use quickcheck::{Arbitrary, Gen};
        use quickcheck_macros::quickcheck;

        #[derive(Debug, Clone)]
        struct TestInput {
            index: usize,
            total_len: usize,
            window_size: usize,
            view_alignment: ViewAlignment,
        }

        impl Arbitrary for TestInput {
            fn arbitrary(g: &mut Gen) -> Self {
                // First generate total_len (at least 1)
                let total_len = (usize::arbitrary(g) % 100).max(1);

                // Generate valid index within total_len
                let index = usize::arbitrary(g) % total_len;

                // Generate window_size between 1 and total_len
                let window_size = (usize::arbitrary(g) % total_len).max(1);

                // Generate random view alignment
                let view_alignment = match usize::arbitrary(g) % 3 {
                    0 => ViewAlignment::Top,
                    1 => ViewAlignment::Center,
                    _ => ViewAlignment::Bottom,
                };

                TestInput {
                    index,
                    total_len,
                    window_size,
                    view_alignment,
                }
            }
        }

        #[quickcheck]
        fn prop_window_size_matches(input: TestInput) -> bool {
            let pos = calculate_window_position(
                input.index,
                input.total_len,
                input.window_size,
                input.view_alignment,
            );
            pos.size() == input.window_size
        }

        #[quickcheck]
        fn prop_contains_target_index(input: TestInput) -> bool {
            let pos = calculate_window_position(
                input.index,
                input.total_len,
                input.window_size,
                input.view_alignment,
            );
            input.index >= pos.start && input.index <= pos.end
        }

        #[quickcheck]
        fn prop_window_within_bounds(input: TestInput) -> bool {
            let pos = calculate_window_position(
                input.index,
                input.total_len,
                input.window_size,
                input.view_alignment,
            );
            pos.start < input.total_len && pos.end < input.total_len
        }

        #[quickcheck]
        fn prop_window_is_continuous(input: TestInput) -> bool {
            let pos = calculate_window_position(
                input.index,
                input.total_len,
                input.window_size,
                input.view_alignment,
            );
            pos.start <= pos.end
        }

        #[quickcheck]
        fn prop_respects_minimum_size(input: TestInput) -> bool {
            let pos = calculate_window_position(
                input.index,
                input.total_len,
                input.window_size,
                input.view_alignment,
            );
            // Window size should never be smaller than 1
            pos.size() >= 1
        }
    }
}
