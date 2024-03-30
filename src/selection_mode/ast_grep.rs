use ast_grep_core::{language::TSLanguage, NodeMatch, StrDoc};

use super::{ByteRange, SelectionMode};

pub struct AstGrep {
    pattern: ast_grep_core::matcher::Pattern<TSLanguage>,
    grep: ast_grep_core::AstGrep<StrDoc<TSLanguage>>,
}

impl AstGrep {
    pub fn new(buffer: &crate::buffer::Buffer, pattern: &str) -> anyhow::Result<Self> {
        let lang: TSLanguage = buffer.treesitter_language().into();
        let pattern = ast_grep_core::matcher::Pattern::try_new(pattern, lang.clone())?;
        let grep = ast_grep_core::AstGrep::new(buffer.rope().to_string(), lang);
        Ok(Self { pattern, grep })
    }
    pub fn replace(
        language: tree_sitter::Language,
        source_code: &str,
        pattern: &str,
        replacement: &str,
    ) -> anyhow::Result<Vec<ast_grep_core::source::Edit<std::string::String>>> {
        let lang: TSLanguage = language.into();
        let pattern = ast_grep_core::matcher::Pattern::try_new(pattern, lang.clone())?;
        let mut source_code = source_code.to_string();
        let grep = ast_grep_core::AstGrep::new(std::mem::take(&mut source_code), lang.clone());
        Ok(grep.root().replace_all(pattern.clone(), replacement))
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
