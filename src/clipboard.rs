use anyhow::Context;
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

    fn to_text(&self) -> String {
        return self.join("\n");
    }

    fn to_html(&self) -> String {
        // Multiple elements (multi-cursor), wrap in HTML format
        let html = Xml::Node {
            tag: "div",
            attributes: vec![XmlAttribute {
                key: "source",
                value: "ki-editor".to_string(),
            }],
            children: self
                .texts
                .iter()
                .map(|text| Xml::Node {
                    tag: "div",
                    attributes: vec![],
                    children: vec![Xml::Text(text.clone())],
                })
                .collect(),
        };
        html.stringify()
    }
}

enum Xml {
    Node {
        tag: &'static str,
        attributes: Vec<XmlAttribute>,
        children: Vec<Xml>,
    },
    Text(String),
}
impl Xml {
    fn stringify(&self) -> String {
        match self {
            Xml::Node {
                tag,
                attributes,
                children,
            } => {
                let attributes = attributes
                    .iter()
                    .map(|attribute| attribute.stringify())
                    .join(" ");
                let children = children.iter().map(|child| child.stringify()).join("\n");
                format!("<{tag} {attributes}>{children}</{tag}>",)
            }
            Xml::Text(text) => escape_xml_text(text),
        }
    }
}

struct XmlAttribute {
    key: &'static str,
    value: String,
}
impl XmlAttribute {
    fn stringify(&self) -> String {
        format!("{}=\"{}\"", self.key, escape_xml_attr(&self.value))
    }
}

fn escape_xml_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

impl CopiedTexts {
    fn from_html(html: &str) -> anyhow::Result<Self> {
        let html_doc = Html::parse_document(html);
        let ki_selector = Selector::parse("div[source='ki-editor'] div")
            .map_err(|err| anyhow::anyhow!("{err}"))?;

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

        Ok(CopiedTexts::new(NonEmpty::from_vec(texts).ok_or(
            anyhow::anyhow!("CopiedTexts::from_html: texts is empty"),
        )?))
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
        // Try to parse the HTML as a Ki-injected HTML
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard
            .get()
            .html()
            .map_err(|err| anyhow::anyhow!("{err}"))
            .and_then(|html| CopiedTexts::from_html(html.as_str()))
            .or_else(|_| {
                Ok(CopiedTexts::new(NonEmpty::new(
                    clipboard.get().text().context("arboard::Get::text")?,
                )))
            })
    }

    pub(crate) fn set(&mut self, copied_texts: CopiedTexts) -> anyhow::Result<()> {
        self.history.add(copied_texts.clone());
        arboard::Clipboard::new()
            .and_then(|mut clipboard| {
                clipboard.set_html(copied_texts.to_html(), Some(copied_texts.to_text()))
            })
            .or_else(|_| osc52::copy_to_clipboard(&copied_texts.to_text()))?;
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
