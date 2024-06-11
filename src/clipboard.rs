#[derive(Clone)]
pub(crate) struct Clipboard {
    history: RingHistory<Vec<String>>,
}

impl Clipboard {
    pub(crate) fn new() -> Clipboard {
        Clipboard {
            history: Default::default(),
        }
    }

    pub(crate) fn get(&self, history_offset: isize) -> Option<Vec<String>> {
        self.history.get(history_offset)
    }

    pub(crate) fn get_from_system_clipboard(&self) -> anyhow::Result<String> {
        Ok(arboard::Clipboard::new()?.get_text()?)
    }

    pub(crate) fn set(
        &mut self,
        content: Vec<String>,
        to_system_clipboard: bool,
    ) -> anyhow::Result<()> {
        self.history.add(content.clone());
        if to_system_clipboard {
            arboard::Clipboard::new()?.set_text(content.join("\n"))?
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
