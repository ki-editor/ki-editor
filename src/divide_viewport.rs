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

fn extract_centered_window<T: Eq + Clone + std::fmt::Debug, F: Fn(&T) -> bool>(
    elements: &[T],
    predicate: F,
    window_size: usize,
) -> Vec<T> {
    debug_assert!(elements.iter().any(&predicate));
    let Some(index) = elements.iter().position(&predicate) else {
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

    debug_assert!(result.iter().any(predicate));
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
