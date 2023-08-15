use crate::buffer::Buffer;

use super::{ByteRange, SelectionMode};

pub struct Regex {
    regex: regex::Regex,
    content: String,
}
pub fn get_regex(pattern: &str, escape: bool, ignore_case: bool) -> anyhow::Result<regex::Regex> {
    let pattern = if escape {
        regex::escape(pattern)
    } else {
        pattern.to_string()
    };
    let pattern = if ignore_case {
        format!("(?i){}", pattern)
    } else {
        pattern
    };
    Ok(regex::Regex::new(&pattern)?)
}

impl Regex {
    pub fn new(
        buffer: &Buffer,
        pattern: &str,
        escape: bool,
        ignore_case: bool,
    ) -> anyhow::Result<Self> {
        let regex = get_regex(pattern, escape, ignore_case)?;
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
