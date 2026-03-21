use itertools::Itertools;

#[derive(Clone, Debug)]
pub struct History<T: Clone + std::fmt::Debug> {
    backward_history: Vec<T>,
    forward_history: Vec<T>,
}

impl<T: Eq + Clone + std::fmt::Debug> History<T> {
    pub fn new() -> Self {
        Self {
            backward_history: Vec::default(),
            forward_history: Vec::default(),
        }
    }
    pub fn push(&mut self, item: T) {
        if self.backward_history.last() == Some(&item) {
            return;
        }
        self.backward_history.push(item);

        self.forward_history.clear();
    }

    /// Jumps to the very first state in the history.
    pub fn go_to_first(&self) -> Option<T> {
        self.backward_history.first().cloned()
    }

    /// Jumps to the very last (most recent) state in the forward history.
    pub fn go_to_last(&self) -> Option<T> {
        if self.forward_history.is_empty() {
            // We are already at the "present," so the last state is the current one
            self.backward_history.last().cloned()
        } else {
            // The most "recent" future state is at the end of the forward stack
            self.forward_history.last().cloned()
        }
    }

    pub fn undo(&mut self) -> Option<T> {
        let item = self.backward_history.pop();
        if let Some(item) = &item {
            self.forward_history.push(item.clone());
        }
        self.backward_history.last().cloned()
    }

    pub fn redo(&mut self) -> Option<T> {
        let item = self.forward_history.pop();
        if let Some(item) = &item {
            self.backward_history.push(item.clone());
        }
        item
    }

    pub fn apply(mut self, f: impl Fn(T) -> T) -> History<T> {
        self.forward_history = std::mem::take(&mut self.forward_history)
            .into_iter()
            .map(&f)
            .collect_vec();
        self.backward_history = std::mem::take(&mut self.backward_history)
            .into_iter()
            .map(f)
            .collect_vec();
        self
    }
}

impl<T: Clone + std::fmt::Debug> Default for History<T> {
    fn default() -> Self {
        Self {
            backward_history: Vec::default(),
            forward_history: Vec::default(),
        }
    }
}

#[cfg(test)]
mod test_history {
    use super::*;
    #[test]
    fn basic_undo_redo() {
        let mut history = History::new();
        history.push(0);
        history.push(1);
        history.push(2);

        assert_eq!(history.undo(), Some(1));
        assert_eq!(history.undo(), Some(0));
        assert_eq!(history.redo(), Some(1));
        assert_eq!(history.redo(), Some(2));
        assert_eq!(history.redo(), None);
        assert_eq!(history.undo(), Some(1));
    }

    #[test]
    fn push_should_clear_redo_stack() {
        let mut history = History::new();
        history.push(1);
        history.push(2);
        assert_eq!(history.undo(), Some(1));
        history.push(3);
        assert_eq!(history.redo(), None);
        assert_eq!(history.undo(), Some(1));
    }

    #[test]
    fn push_should_not_allow_consecutive_duplicates() {
        let mut history = History::new();
        history.push(1);
        history.push(2);
        history.push(2);
        history.push(3);
        assert_eq!(history.undo(), Some(2));
        assert_eq!(history.undo(), Some(1));
        assert_eq!(history.undo(), None);
    }

    #[test]
    fn push_should_allow_non_consecutive_duplicates() {
        let mut history = History::new();
        history.push(1);
        history.push(2);
        history.push(1);
        history.push(2);
        assert_eq!(history.undo(), Some(1));
        assert_eq!(history.undo(), Some(2));
        assert_eq!(history.undo(), Some(1));
        assert_eq!(history.undo(), None);
    }
}
