use ast_grep_core::{language::TSLanguage, StrDoc};

use super::{ByteRange, SelectionMode};

pub struct AstGrep {
    pattern: ast_grep_core::matcher::Pattern<TSLanguage>,
    grep: ast_grep_core::AstGrep<StrDoc<TSLanguage>>,
}

impl AstGrep {
    pub fn new(buffer: &crate::buffer::Buffer, pattern: &str) -> anyhow::Result<Self> {
        let lang = ast_grep_core::language::TSLanguage::from(buffer.treesitter_language());
        let pattern = ast_grep_core::matcher::Pattern::try_new(pattern, lang.clone())?;
        let grep = ast_grep_core::AstGrep::new(buffer.rope().to_string(), lang);
        Ok(Self { pattern, grep })
    }
}

impl SelectionMode for AstGrep {
    fn iter<'a>(
        &'a self,
        _current_selection: &'a crate::selection::Selection,
        _buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(
            self.grep
                .root()
                .find_all(self.pattern.clone())
                .map(|node| ByteRange::new(node.range())),
        ))
    }
}
