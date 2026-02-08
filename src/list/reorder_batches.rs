use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    sync::{mpsc::Sender, Arc},
};

use crate::thread::SendResult;

struct Indexed<T> {
    index: usize,
    items: Vec<T>,
}

impl<T> PartialEq for Indexed<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for Indexed<T> {}

impl<T> PartialOrd for Indexed<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Indexed<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

/// Reorders batches of items to ensure sequential delivery based on batch indices.
///
/// Items arrive as batches with associated indices (0, 1, 2, ...). This function
/// ensures that items are delivered in index order, even if batches arrive out of order.
/// Batches are buffered internally until all preceding batches have been processed.
///
/// # Arguments
/// * `send_match` - Callback function to send individual items once their batch is ready
///
/// # Returns
/// A sender that accepts `(index, items)` tuples. The index determines the order,
/// and items are sent via `send_match` only after all lower-indexed batches complete.
///
/// # Preconditions
/// - Indices must start at 0 and be contiguous (0, 1, 2, ..., N)
/// - All indices must eventually arrive (no gaps/missing indices)
/// - Indices may arrive in any order
///
/// # Panics
/// Panics if the sender is dropped while batches with gaps remain unprocessed
/// (e.g., received batches 0, 2, 4 but never received 1 or 3).
pub fn reorder_batches<T: Send + Sync + 'static>(
    send_match: Arc<dyn Fn(T) -> SendResult + Send + Sync>,
) -> Sender<(usize, Vec<T>)> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut current_index = 0;
        let mut heap = BinaryHeap::<Reverse<Indexed<T>>>::new();

        for (index, items) in receiver {
            heap.push(Reverse(Indexed { index, items }));

            while heap.peek().map(|x| x.0.index) == Some(current_index) {
                if let Some(Reverse(batch)) = heap.pop() {
                    for item in batch.items {
                        send_match(item);
                    }
                    current_index += 1;
                }
            }
        }

        // Check for remaining unprocessed items
        if !heap.is_empty() {
            let next_index = heap.peek().unwrap().0.index;
            panic!(
                "Index gap detected: expected index {}, but next available is {}",
                current_index, next_index
            );
        }
    });

    sender
}

#[cfg(test)]
mod test_reorder_batches {
    use std::time::Duration;

    use super::*;

    fn sleep() {
        std::thread::sleep(Duration::from_millis(100));
    }

    #[test]
    fn should_send_entries_in_order() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let buffer_sender =
            reorder_batches(Arc::new(move |item| SendResult::from(sender.send(item))));

        // Send batches out of order
        buffer_sender.send((1, vec![10, 11])).unwrap();
        buffer_sender.send((0, vec![0, 1])).unwrap();
        buffer_sender.send((2, vec![20, 21])).unwrap();

        drop(buffer_sender);

        sleep();

        let received: Vec<_> = receiver.iter().collect();
        assert_eq!(received, vec![0, 1, 10, 11, 20, 21]);
    }

    #[test]
    fn should_handle_empty_batches() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let buffer_sender =
            reorder_batches(Arc::new(move |item| SendResult::from(sender.send(item))));

        buffer_sender.send((0, vec![])).unwrap();
        buffer_sender.send((1, vec![1])).unwrap();

        drop(buffer_sender);

        sleep();

        let received: Vec<_> = receiver.iter().collect();
        assert_eq!(received, vec![1]);
    }

    #[test]
    fn should_buffer_until_next_index_arrives() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let buffer_sender =
            reorder_batches(Arc::new(move |item| SendResult::from(sender.send(item))));

        // Send index 2 first
        buffer_sender.send((2, vec![20])).unwrap();

        sleep();

        // Nothing should be received yet
        assert!(receiver.try_recv().is_err());

        // Send index 1
        buffer_sender.send((1, vec![10])).unwrap();

        sleep();

        // Still nothing, waiting for index 0
        assert!(receiver.try_recv().is_err());

        // Send index 0
        buffer_sender.send((0, vec![0])).unwrap();

        drop(buffer_sender);

        sleep();

        let received: Vec<_> = receiver.iter().collect();
        assert_eq!(received, vec![0, 10, 20]);
    }
}

#[cfg(test)]
mod reorder_batches_quickcheck_tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
    use std::sync::mpsc;

    #[derive(Clone, Debug)]
    struct TestScenario {
        /// Each element is (index, items) representing a batch
        batches: Vec<(usize, Vec<usize>)>,
        /// Order in which to send the batches
        send_order: Vec<usize>,
    }

    impl Arbitrary for TestScenario {
        fn arbitrary(g: &mut Gen) -> Self {
            let num_batches = (usize::arbitrary(g) % 8) + 1; // 1-8 batches

            let mut batches = Vec::new();
            for i in 0..num_batches {
                let num_items = usize::arbitrary(g) % 5; // 0-4 items per batch
                let items = (0..num_items).map(|_| usize::arbitrary(g) % 100).collect();
                batches.push((i, items));
            }

            // Create shuffled send order
            let mut send_order: Vec<usize> = (0..num_batches).collect();
            for i in (1..send_order.len()).rev() {
                let j = usize::arbitrary(g) % (i + 1);
                send_order.swap(i, j);
            }

            TestScenario {
                batches,
                send_order,
            }
        }
    }

    #[test]
    fn entries_are_received_in_order() {
        fn property(scenario: TestScenario) -> TestResult {
            if scenario.batches.is_empty() {
                return TestResult::discard();
            }

            let (sender, receiver) = mpsc::channel();
            let buffer_sender =
                reorder_batches(Arc::new(move |item| SendResult::from(sender.send(item))));

            // Send batches in the specified random order
            for &batch_idx in &scenario.send_order {
                if batch_idx < scenario.batches.len() {
                    let (index, items) = &scenario.batches[batch_idx];
                    buffer_sender.send((*index, items.clone())).unwrap();
                }
            }

            // Drop to close the channel
            drop(buffer_sender);

            // Collect all received items
            let received: Vec<_> = receiver.iter().collect();

            // Flatten expected items in order
            let expected: Vec<_> = scenario
                .batches
                .iter()
                .flat_map(|(_, items)| items.clone())
                .collect();

            TestResult::from_bool(received == expected)
        }

        QuickCheck::new()
            .tests(100)
            .quickcheck(property as fn(TestScenario) -> TestResult);
    }

    #[test]
    fn first_received_entry_is_from_first_batch() {
        fn property(scenario: TestScenario) -> TestResult {
            if scenario.batches.is_empty() {
                return TestResult::discard();
            }

            // Skip if first batch is empty
            if scenario.batches[0].1.is_empty() {
                return TestResult::discard();
            }

            let (sender, receiver) = mpsc::channel();
            let buffer_sender =
                reorder_batches(Arc::new(move |item| SendResult::from(sender.send(item))));

            // Send batches in the specified random order
            for &batch_idx in &scenario.send_order {
                if batch_idx < scenario.batches.len() {
                    let (index, items) = &scenario.batches[batch_idx];
                    buffer_sender.send((*index, items.clone())).unwrap();
                }
            }

            drop(buffer_sender);

            let received: Vec<_> = receiver.iter().collect();

            if received.is_empty() {
                return TestResult::discard();
            }

            // First received item should be the first item from the first batch
            let expected_first = scenario.batches[0].1[0];
            TestResult::from_bool(received[0] == expected_first)
        }

        QuickCheck::new()
            .tests(100)
            .quickcheck(property as fn(TestScenario) -> TestResult);
    }
}
