use ast_grep_core::{language::TSLanguage, NodeMatch, StrDoc};

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
    pub fn find_all(&self) -> impl Iterator<Item = NodeMatch<StrDoc<TSLanguage>>> {
        self.grep.root().find_all(self.pattern.clone())
    }
}

impl SelectionMode for AstGrep {
    fn name(&self) -> &'static str {
        "AST GREP"
    }
    fn iter<'a>(
        &'a self,
        _params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(
            self.find_all().map(|node| ByteRange::new(node.range())),
        ))
    }
}

#[cfg(test)]
mod test_ast_grep {
    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            tree_sitter_rust::language(),
            "fn main(x: usize) { let x = f(f(x)); }",
        );
        AstGrep::new(&buffer, "f($Y)")
            .unwrap()
            .assert_all_selections(
                &buffer,
                Selection::default(),
                &[(28..35, "f(f(x))"), (30..34, "f(x)")],
            );
    }
}
