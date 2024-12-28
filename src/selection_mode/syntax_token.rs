pub(crate) struct SyntaxToken;

use crate::{components::editor::IfCurrentNotFound, selection_mode::SelectionMode};

use super::{ByteRange, TopNode};

impl SelectionMode for SyntaxToken {
    fn iter<'a>(
        &self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let buffer = params.buffer;
        let tree = buffer
            .tree()
            .ok_or(anyhow::anyhow!("Unable to find Treesitter language"))?;
        Ok(Box::new(
            tree_sitter_traversal2::traverse(tree.walk(), tree_sitter_traversal2::Order::Post)
                .filter(|node| node.child_count() == 0)
                .map(|node| ByteRange::new(node.byte_range())),
        ))
    }

    fn expand(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection_mode::ApplyMovementResult>> {
        Ok(TopNode
            .current(params, IfCurrentNotFound::LookForward)?
            .map(|selection| crate::selection_mode::ApplyMovementResult {
                selection,
                mode: Some(crate::selection::SelectionMode::SyntaxNode),
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
        SyntaxToken.assert_all_selections(
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
