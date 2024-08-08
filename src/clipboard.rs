use itertools::Itertools;
use nonempty::NonEmpty;

use crate::osc52;

#[derive(Clone)]
pub(crate) struct Clipboard {
    history: RingHistory<CopiedTexts>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Why is it a vector?  
/// Because it needs to support multiple cursors.
/// The first entry represent the copied text of the first cursor,
/// and so forth.
pub(crate) struct CopiedTexts {
    texts: NonEmpty<String>,
}
impl CopiedTexts {
    pub(crate) fn new(texts: NonEmpty<String>) -> Self {
        Self { texts }
    }

    fn join(&self, separator: &str) -> String {
        self.texts.clone().into_iter().join(separator)
    }

    /// Returns the first element if no element is found at the given `index`
    pub(crate) fn get(&self, index: usize) -> String {
        self.texts
            .get(index)
            .unwrap_or_else(|| self.texts.first())
            .to_string()
    }

    #[cfg(test)]
    pub(crate) fn one(string: String) -> CopiedTexts {
        CopiedTexts::new(NonEmpty::singleton(string))
    }
}

impl Clipboard {
    pub(crate) fn new() -> Clipboard {
        Clipboard {
            history: RingHistory::new(),
        }
    }

    pub(crate) fn get(&self, history_offset: isize) -> Option<CopiedTexts> {
        self.history.get(history_offset)
    }

    pub(crate) fn get_from_system_clipboard(&self) -> anyhow::Result<String> {
        Ok(arboard::Clipboard::new()?.get_text()?)
    }

    pub(crate) fn set(
        &mut self,
        copied_texts: CopiedTexts,
        use_system_clipboard: bool,
    ) -> anyhow::Result<()> {
        self.history.add(copied_texts.clone());
        if use_system_clipboard {
            arboard::Clipboard::new()
                .and_then(|mut clipboard| clipboard.set_text(copied_texts.join("\n")))
                .or_else(|_| osc52::copy_to_clipboard(&copied_texts.join("\n")))?
        }
        Ok(())
    }
}

#[derive(PartialEq, Clone, Debug, Eq, Hash, Default)]
pub(crate) struct RingHistory<T: Clone> {
    items: Vec<T>,
}
impl<T: Clone> RingHistory<T> {
    /// 0 means latest.  
    /// -1 means previous.  
    /// +1 means next.  
    pub(crate) fn get(&self, history_offset: isize) -> Option<T> {
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

    pub(crate) fn add(&mut self, item: T) {
        self.items.push(item)
    }

    fn new() -> Self {
        Self {
            items: Default::default(),
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
            assert_eq!(history.get(offset), Some(expected.to_string()))
        }
    }
}
