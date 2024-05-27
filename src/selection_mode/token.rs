pub(crate) struct Token;

use crate::selection_mode::SelectionMode;

use super::{ByteRange, TopNode};

impl SelectionMode for Token {
    fn iter<'a>(
        &self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let buffer = params.buffer;
        let tree = buffer
            .tree()
            .ok_or(anyhow::anyhow!("Unable to find Treesitter language"))?;
        Ok(Box::new(
            crate::tree_sitter_traversal::traverse(
                tree.walk(),
                crate::tree_sitter_traversal::Order::Post,
            )
            .filter(|node| node.child_count() == 0)
            .map(|node| ByteRange::new(node.byte_range())),
        ))
    }

    fn parent(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection_mode::ApplyMovementResult>> {
        Ok(TopNode
            .current(params)?
            .map(|selection| crate::selection_mode::ApplyMovementResult {
                selection,
                mode: Some(crate::selection::SelectionMode::SyntaxTreeCoarse),
            }))
    }
}

#[cfg(test)]
mod test_token {
    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(Some(tree_sitter_rust::language()), "fn main() {}");
        Token.assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..2, "fn"),
                (3..7, "main"),
                (7..8, "("),
                (8..9, ")"),
                (10..11, "{"),
                (11..12, "}"),
            ],
        );
    }
}
