use crate::buffer::Buffer;

use super::{ByteRange, SelectionMode};

pub struct Regex {
    regex: regex::Regex,
    content: String,
}

impl Regex {
    pub fn new(buffer: &Buffer, regex: &str, escape: bool) -> anyhow::Result<Self> {
        let regex = if escape {
            regex::Regex::new(&regex::escape(regex))?
        } else {
            regex::Regex::new(regex)?
        };
        Ok(Self {
            regex,
            content: buffer.rope().to_string(),
        })
    }
}

impl SelectionMode for Regex {
    fn iter<'a>(
        &'a self,
        _current_selection: &'a crate::selection::Selection,
        _: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let matches = self.regex.find_iter(&self.content);
        Ok(Box::new(matches.filter_map(move |matches| {
            Some(ByteRange::new(matches.start()..matches.end()))
        })))
    }
}
