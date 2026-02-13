use anyhow::Context;
use indexmap::IndexSet;
use itertools::Itertools;
use nonempty::NonEmpty;
use scraper::{Html, Selector};

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

    pub fn to_html(&self) -> String {
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

impl Texts {
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

        Ok(Texts::new(NonEmpty::from_vec(texts).ok_or(
            anyhow::anyhow!("CopiedTexts::from_html: texts is empty"),
        )?))
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
        // Try to parse the HTML as a Ki-injected HTML
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard
            .get()
            .html()
            .map_err(|err| anyhow::anyhow!("{err}"))
            .and_then(|html| Texts::from_html(html.as_str()))
            .or_else(|_| {
                Ok(Texts::new(NonEmpty::new(
                    clipboard.get().text().context("arboard::Get::text")?,
                )))
            })
    }

    pub fn set(&mut self, copied_texts: Texts) -> anyhow::Result<()> {
        self.history.add(copied_texts.clone());
        arboard::Clipboard::new().and_then(|mut clipboard| {
            clipboard.set_html(copied_texts.to_html(), Some(copied_texts.to_text()))
        })?;
        Ok(())
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

    #[test]
    fn add_should_not_add_duplicated_entries() {
        let mut history = RingHistory::default();
        assert_eq!(history.get(0), None);
        history.add("a".to_string());
        history.add("b".to_string());
        history.add("b".to_string());

        assert_eq!(history.items.len(), 2)
    }

    #[test]
    fn duplicated_entries_should_reorder_entry() {
        let mut history = RingHistory::default();
        assert_eq!(history.get(0), None);
        history.add("b".to_string());
        history.add("a".to_string());
        history.add("b".to_string());

        assert_eq!(history.items.into_iter().collect_vec(), vec!["a", "b"])
    }
}
