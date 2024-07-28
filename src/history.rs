use itertools::Itertools;

#[derive(Clone, Debug)]
pub(crate) struct History<T: Clone + std::fmt::Debug> {
    backward_history: Vec<T>,
    forward_history: Vec<T>,
}

impl<T: Eq + Clone + std::fmt::Debug> History<T> {
    pub(crate) fn new() -> Self {
        Self {
            backward_history: Default::default(),
            forward_history: Default::default(),
        }
    }
    pub(crate) fn push(&mut self, item: T) {
        if self.backward_history.last() == Some(&item) {
            return;
        }
        self.backward_history.push(item);

        self.forward_history.clear();
    }

    pub(crate) fn undo(&mut self) -> Option<T> {
        let item = self.backward_history.pop();
        if let Some(item) = &item {
            self.forward_history.push(item.clone());
        }
        self.backward_history.last().cloned()
    }

    pub(crate) fn redo(&mut self) -> Option<T> {
        let item = self.forward_history.pop();
        if let Some(item) = &item {
            self.backward_history.push(item.clone());
        }
        item
    }

    pub(crate) fn apply(mut self, f: impl Fn(T) -> T) -> History<T> {
        self.forward_history = std::mem::take(&mut self.forward_history)
            .into_iter()
            .map(|item| f(item))
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
            backward_history: Default::default(),
            forward_history: Default::default(),
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
