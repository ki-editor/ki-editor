use std::{
    sync::{mpsc::Sender, Arc},
    time::Instant,
};

use itertools::Itertools;

use crate::thread::SendResult;

enum BufferingSignal<M: Ord> {
    MatchReceived { path_index: usize, match_: M },
    FileFinishedSearching { index: usize },
}

pub struct BufferingThread<M: Ord> {
    sender: Sender<BufferingSignal<M>>,
}

impl<M: Ord> BufferingThread<M> {
    pub fn send_match(&self, path_index: usize, match_: M) -> SendResult {
        SendResult::from(
            self.sender
                .send(BufferingSignal::MatchReceived { path_index, match_ }),
        )
    }

    pub fn notify_file_finished_searching(&self, path_index: usize) -> SendResult {
        SendResult::from(
            self.sender
                .send(BufferingSignal::FileFinishedSearching { index: path_index }),
        )
    }
}

/// Create a thread to buffer non-first entries, until the first sorted entry is found.
/// This is ensure the first entry sent to the main UI loop
/// is also the first entry that in the final sorted list.
///
/// This is necessary because the entries come in random order due to parallelization.
pub fn buffer_entries_until_first_sorted_entry_found<
    M: Send + PartialOrd + Ord + Clone + 'static,
>(
    send_match: Arc<dyn Fn(M) -> SendResult + Send + Sync>,
) -> BufferingThread<M> {
    let (sender, receiver) = std::sync::mpsc::channel::<BufferingSignal<M>>();

    let started_at = Instant::now();
    std::thread::spawn(move || {
        // Store the indices of files finished searching
        let mut indices = vec![];
        let mut buffered_matches = vec![];
        let mut first_entry_sent = false;
        let mut first_entry_sent_at = None;

        while let Ok(signal) = receiver.recv() {
            match signal {
                BufferingSignal::MatchReceived { path_index, match_ } => {
                    if first_entry_sent {
                        // If the first entry is already sent, we no longer need
                        // to worry about subsequent items being sent in random order

                        match send_match(match_) {
                            SendResult::Succeeed => continue,
                            SendResult::ReceiverDisconnected => {
                                // Break the loop is the receiver of matches is killed
                                return;
                            }
                        }
                    } else {
                        buffered_matches.push((path_index, match_))
                    }
                }
                BufferingSignal::FileFinishedSearching { index } => {
                    if first_entry_sent {
                        // Ignore the index if the first entry is already sent
                    } else {
                        // If all previous files have finished searching
                        // send the first entry over.

                        // For example: if `index` is 3 and `indices` is [0, 1, 2]
                        // then we can send the results over.
                        if indices.iter().take_while(|i| *i < &index).count() == index {
                            // Send the buffered entries over in an ordered manner
                            for (_, match_) in buffered_matches
                                .drain(..)
                                .sorted_by_key(|(path_index, m)| (*path_index, m.clone()))
                            {
                                match send_match(match_) {
                                    SendResult::Succeeed => continue,
                                    SendResult::ReceiverDisconnected => {
                                        // Break the loop is the receiver of matches is killed
                                        return;
                                    }
                                }
                            }

                            first_entry_sent = true;
                            first_entry_sent_at = Some(Instant::now());
                        } else {
                            indices.push(index)
                        }
                    }
                }
            }
        }

        let finished_at = Instant::now();

        if let Some(first_entry_sent_at) = first_entry_sent_at {
            let buffering_time_taken_percentage = ((first_entry_sent_at - started_at).as_millis()
                as f32)
                / ((finished_at - started_at).as_millis() as f32)
                * 100.0;

            log::info!(
                "First entry sent after buffering for {:?} ({:.1?}%), ahead of finished time by {:?}",
                first_entry_sent_at - started_at,
                buffering_time_taken_percentage,
                finished_at - first_entry_sent_at,
            );
        }
    });
    BufferingThread { sender }
}

#[cfg(test)]
mod test_buffering_thread {
    use std::time::Duration;

    use super::*;

    fn sleep() {
        std::thread::sleep(Duration::from_millis(100));
    }

    #[test]
    fn should_not_send_entry_if_notify_file_finished_searching_is_not_called() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let buffering_thread = buffer_entries_until_first_sorted_entry_found(Arc::new(move |_| {
            SendResult::from(sender.send(()))
        }));

        buffering_thread.send_match(0, ());

        sleep();

        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn should_send_entry_after_first_sorted_entry_is_found() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let buffering_thread = buffer_entries_until_first_sorted_entry_found(Arc::new(move |_| {
            SendResult::from(sender.send(()))
        }));

        buffering_thread.send_match(0, ());

        buffering_thread.notify_file_finished_searching(0);

        sleep();

        assert!(receiver.try_recv().is_ok());
    }

    #[test]
    fn should_not_send_entry_if_first_sorted_entry_is_not_found_yet() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let buffering_thread = buffer_entries_until_first_sorted_entry_found(Arc::new(move |_| {
            SendResult::from(sender.send(()))
        }));

        buffering_thread.send_match(1, ());

        buffering_thread.notify_file_finished_searching(1);

        sleep();

        assert!(receiver.try_recv().is_err());
    }
}

#[cfg(test)]
mod buffering_thread_quickcheck_tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
    use std::sync::mpsc;

    #[derive(Clone, Debug)]
    struct FileEvent {
        file_index: usize,
        matches: Vec<usize>, // match indices within the file
    }

    impl Arbitrary for FileEvent {
        fn arbitrary(g: &mut Gen) -> Self {
            let file_index = usize::arbitrary(g) % 10; // Limit to 10 files for practical testing
            let num_matches = usize::arbitrary(g) % 5; // 0-4 matches per file
            let matches = (0..num_matches)
                .map(|_| usize::arbitrary(g) % 100)
                .collect();
            FileEvent {
                file_index,
                matches,
            }
        }
    }

    #[derive(Clone, Debug)]
    struct TestScenario {
        events: Vec<FileEvent>,
        /// Order in which to send match events (indices into flattened match list)
        match_send_order: Vec<usize>,
        /// Order in which to send file-finished notifications
        finish_order: Vec<usize>,
    }

    impl Arbitrary for TestScenario {
        fn arbitrary(g: &mut Gen) -> Self {
            let num_files = (usize::arbitrary(g) % 8) + 1; // 1-8 files

            let mut events = Vec::new();
            for i in 0..num_files {
                let num_matches = usize::arbitrary(g) % 4; // 0-3 matches per file
                let matches = (0..num_matches).map(|_| usize::arbitrary(g) % 50).collect();
                events.push(FileEvent {
                    file_index: i,
                    matches,
                });
            }

            // Create indices for all matches
            let total_matches: usize = events.iter().map(|e| e.matches.len()).sum();
            let mut match_send_order: Vec<usize> = (0..total_matches).collect();
            // Shuffle the order
            for i in (1..match_send_order.len()).rev() {
                let j = usize::arbitrary(g) % (i + 1);
                match_send_order.swap(i, j);
            }

            // Create shuffled finish order
            let mut finish_order: Vec<usize> = (0..num_files).collect();
            for i in (1..finish_order.len()).rev() {
                let j = usize::arbitrary(g) % (i + 1);
                finish_order.swap(i, j);
            }

            TestScenario {
                events,
                match_send_order,
                finish_order,
            }
        }
    }

    fn make_match_with_index(file_index: usize, match_index: usize) -> (usize, usize) {
        (file_index, match_index)
    }

    #[test]
    fn first_received_entry_is_the_first_sorted_entry() {
        fn property(scenario: TestScenario) -> TestResult {
            if scenario.events.is_empty() {
                return TestResult::discard();
            }

            let (sender, receiver) = mpsc::channel();

            let buffering_thread =
                buffer_entries_until_first_sorted_entry_found(Arc::new(move |match_| {
                    SendResult::from(sender.send(match_))
                }));

            // Flatten all matches with their file indices
            let mut all_matches: Vec<(usize, usize, (usize, usize))> = Vec::new();
            for event in &scenario.events {
                for &match_idx in &event.matches {
                    all_matches.push((
                        event.file_index,
                        match_idx,
                        make_match_with_index(event.file_index, match_idx),
                    ));
                }
            }

            // Send matches in the specified random order
            for &idx in &scenario.match_send_order {
                if idx < all_matches.len() {
                    let (file_idx, _, match_) = all_matches[idx];
                    buffering_thread.send_match(file_idx, match_);
                }
            }

            // Send file-finished notifications in the specified random order
            for &file_idx in &scenario.finish_order {
                buffering_thread.notify_file_finished_searching(file_idx);
            }

            // Drop to close the channel
            drop(buffering_thread);

            // Wait for all signals
            let received = receiver.into_iter().collect_vec();

            if received.is_empty() {
                // Skip checking is no matches
                return TestResult::passed();
            }

            // Ensure that the first received entry is the first sorted entry
            TestResult::from_bool(
                received.first().copied().unwrap() == received.into_iter().sorted().next().unwrap(),
            )
        }

        QuickCheck::new()
            .tests(100)
            .quickcheck(property as fn(TestScenario) -> TestResult);
    }
}
