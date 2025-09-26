use itertools::Itertools;
use nonempty::NonEmpty;
use scraper::{Html, Selector};

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

    #[allow(dead_code)]
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

    fn to_clipboard_format(&self) -> String {
        if self.texts.tail().is_empty() {
            // Only one element, return it as-is
            self.texts.head.clone()
        } else {
            // Multiple elements (multi-cursor), wrap in HTML format
            let mut html = String::from(r#"<div source="ki-editor">"#);
            html.push('\n');

            for text in &self.texts {
                html.push_str("<div>");
                html.push_str(text);
                html.push_str("</div>");
                html.push('\n');
            }

            html.push_str("</div>");
            html
        }
    }
}

impl From<&str> for CopiedTexts {
    fn from(text: &str) -> Self {
        let html_doc = Html::parse_document(text);
        let ki_selector = Selector::parse("div[source='ki-editor'] div");

        match ki_selector {
            Ok(ki_selector) => {
                let texts: Vec<String> = html_doc
                    .select(&ki_selector)
                    .filter_map(|element| {
                        let text = element.text().collect::<String>();
                        if text.is_empty() {
                            None
                        } else {
                            Some(text.to_string())
                        }
                    })
                    .collect();

                CopiedTexts::new(
                    NonEmpty::from_vec(texts)
                        .unwrap_or_else(|| NonEmpty::singleton(text.to_string())),
                )
            }
            Err(_) => CopiedTexts::new(NonEmpty::singleton(text.to_string())),
        }
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

    pub(crate) fn get_from_system_clipboard(&self) -> anyhow::Result<CopiedTexts> {
        Ok(CopiedTexts::from(
            arboard::Clipboard::new()?.get_text()?.as_str(),
        ))
    }

    pub(crate) fn set(&mut self, copied_texts: CopiedTexts) -> anyhow::Result<()> {
        self.history.add(copied_texts.clone());
        arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.set_text(copied_texts.to_clipboard_format()))
            .or_else(|_| osc52::copy_to_clipboard(&copied_texts.to_clipboard_format()))?;
        Ok(())
    }

    pub(crate) fn add_clipboard_history(&mut self, copied_texts: CopiedTexts) {
        self.history.add(copied_texts.clone());
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
