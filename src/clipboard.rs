use indexmap::IndexSet;
use itertools::Itertools;
use nonempty::NonEmpty;

#[derive(Clone)]
pub struct Clipboard {
    history: RingHistory<Texts>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// Why is it a vector?
/// Because it needs to support multiple cursors.
/// The first entry represent the copied text of the first cursor,
/// and so forth.
pub struct Texts {
    texts: NonEmpty<String>,
}
impl Texts {
    pub fn new(texts: NonEmpty<String>) -> Self {
        Self { texts }
    }

    fn join(&self, separator: &str) -> String {
        self.texts.clone().into_iter().join(separator)
    }

    /// Returns the first element if no element is found at the given `index`
    pub fn get(&self, index: usize) -> String {
        self.texts
            .get(index)
            .unwrap_or_else(|| self.texts.first())
            .to_string()
    }

    #[cfg(test)]
    pub fn one(string: String) -> Texts {
        Texts::new(NonEmpty::singleton(string))
    }

    pub fn to_text(&self) -> String {
        self.join("\n")
    }
}

impl Clipboard {
    pub fn new() -> Clipboard {
        Clipboard {
            history: RingHistory::new(),
        }
    }

    pub fn get(&self, history_offset: isize) -> Option<Texts> {
        self.history.get(history_offset)
    }

    pub fn get_from_system_clipboard(&self) -> anyhow::Result<Texts> {
        let arboard_result = arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.get().text())
            .ok();

        let latest_ki_entry = self.history.get(0);

        match (arboard_result, latest_ki_entry) {
            // arboard failed: fall back to ki's history
            (None, Some(ki_entry)) => Ok(ki_entry),
            // arboard failed and ki history is empty: error
            (None, None) => Err(anyhow::anyhow!("system clipboard is inaccessible")),
            // Content matches ki's entry — use ki's entry to preserve multi-cursor
            (Some(arboard_text), Some(ki_entry)) if arboard_text == ki_entry.to_text() => {
                Ok(ki_entry)
            }
            // External content or no ki history: return as single-text Texts
            (Some(arboard_text), _) => Ok(Texts::new(NonEmpty::singleton(arboard_text))),
        }
    }

    /// Adds the copied texts to ki's internal clipboard history.
    /// The caller is responsible for writing to the system clipboard (e.g. via OSC52).
    pub fn set(&mut self, copied_texts: Texts) {
        self.history.add(copied_texts);
    }
}

#[derive(Clone, Debug, Default)]
pub struct RingHistory<T: Clone> {
    items: IndexSet<T>,
}
impl<T: Clone + PartialEq + Eq + std::hash::Hash> RingHistory<T> {
    /// 0 means latest.
    /// -1 means previous.
    /// +1 means next.
    pub fn get(&self, history_offset: isize) -> Option<T> {
        let len = self.items.len();
        if len == 0 {
            return None;
        }
        if history_offset.is_positive() {
            self.items
                .iter()
                .cycle()
                .skip(len - 1)
                .nth(history_offset.unsigned_abs().rem_euclid(len))
                .cloned()
        } else {
            self.items
                .iter()
                .rev()
                .nth(history_offset.unsigned_abs().rem_euclid(len))
                .cloned()
        }
    }

    pub fn add(&mut self, item: T) {
        self.items.shift_remove(&item);
        self.items.insert(item);
    }

    pub fn new() -> Self {
        Self {
            items: IndexSet::default(),
        }
    }
}

#[cfg(test)]
mod test_ring_history {
    use super::*;
    #[test]
    fn test_get() {
        let mut history = RingHistory::default();
        assert_eq!(history.get(0), None);
        history.add("a".to_string());
        history.add("b".to_string());
        history.add("c".to_string());

        let expected = [
            (-3, "c"),
            (-2, "a"),
            (-1, "b"),
            (0, "c"),
            (1, "a"),
            (2, "b"),
            (3, "c"),
        ];
        for (offset, expected) in expected {
            assert_eq!(history.get(offset), Some(expected.to_string()));
        }
    }

    #[test]
    fn add_should_not_add_duplicated_entries() {
        let mut history = RingHistory::default();
        assert_eq!(history.get(0), None);
        history.add("a".to_string());
        history.add("b".to_string());
        history.add("b".to_string());

        assert_eq!(history.items.len(), 2);
    }

    #[test]
    fn duplicated_entries_should_reorder_entry() {
        let mut history = RingHistory::default();
        assert_eq!(history.get(0), None);
        history.add("b".to_string());
        history.add("a".to_string());
        history.add("b".to_string());

        assert_eq!(history.items.into_iter().collect_vec(), vec!["a", "b"]);
    }
}
