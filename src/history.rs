use crate::undo_tree::OldNew;

pub struct History<T> {
    backward_history: Vec<OldNew<T>>,
    forward_history: Vec<OldNew<T>>,
}

impl<T: Eq + Clone + std::fmt::Debug> History<T> {
    pub fn push(&mut self, item: OldNew<T>) {
        if self.backward_history.last() == Some(&item) {
            return;
        }
        self.backward_history.push(item);

        self.forward_history.clear();
    }

    pub fn undo(&mut self) -> Option<T> {
        let item = self.backward_history.pop();
        if let Some(item) = &item {
            self.forward_history.push(item.clone());
        }
        item.map(|item| item.new_to_old)
    }

    pub fn redo(&mut self) -> Option<T> {
        let item = self.forward_history.pop();
        if let Some(item) = &item {
            self.backward_history.push(item.clone());
        }
        item.map(|item| item.old_to_new)
    }

    pub fn new() -> Self {
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
        history.push(OldNew {
            new_to_old: 0,
            old_to_new: 1,
        });
        history.push(OldNew {
            new_to_old: 1,
            old_to_new: 2,
        });

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
        history.push(OldNew {
            new_to_old: 1,
            old_to_new: 2,
        });
        assert_eq!(history.undo(), Some(1));
        history.push(OldNew {
            new_to_old: 1,
            old_to_new: 3,
        });
        assert_eq!(history.redo(), None);
        assert_eq!(history.undo(), Some(1));
    }

    #[test]
    fn push_should_not_allow_consecutive_duplicates() {
        let mut history = History::new();
        history.push(OldNew {
            new_to_old: 1,
            old_to_new: 2,
        });
        history.push(OldNew {
            new_to_old: 1,
            old_to_new: 2,
        });
        history.push(OldNew {
            new_to_old: 2,
            old_to_new: 3,
        });
        assert_eq!(history.undo(), Some(2));
        assert_eq!(history.undo(), Some(1));
        assert_eq!(history.undo(), None);
    }

    #[test]
    fn push_should_allow_non_consecutive_duplicates() {
        let mut history = History::new();
        history.push(OldNew {
            new_to_old: 1,
            old_to_new: 2,
        });
        history.push(OldNew {
            new_to_old: 2,
            old_to_new: 1,
        });
        history.push(OldNew {
            new_to_old: 1,
            old_to_new: 2,
        });
        assert_eq!(history.undo(), Some(1));
        assert_eq!(history.undo(), Some(2));
        assert_eq!(history.undo(), Some(1));
        assert_eq!(history.undo(), None);
    }
}
