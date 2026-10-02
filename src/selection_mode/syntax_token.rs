pub struct SyntaxToken;

use crate::components::editor::IfCurrentNotFound;

use super::{ByteRange, IterBasedSelectionMode, TopNode};

impl IterBasedSelectionMode for SyntaxToken {
    fn iter<'a>(
        &self,
        params: &super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let buffer = params.buffer;
        let ranges = buffer
            .collect_ranges_across_layers(|tree| {
                tree_sitter_traversal2::traverse(tree.walk(), tree_sitter_traversal2::Order::Post)
                    .filter(|node| node.child_count() == 0)
                    .map(|node| node.byte_range())
                    .collect()
            })?
            .ok_or(anyhow::anyhow!("Unable to find Treesitter language"))?
            .into_iter()
            .map(ByteRange::new)
            .collect::<Vec<_>>();
        Ok(Box::new(ranges.into_iter()))
    }

    fn expand(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection_mode::ApplyMovementResult>> {
        Ok(TopNode
            .current(params, IfCurrentNotFound::LookForward)?
            .map(|selection| crate::selection_mode::ApplyMovementResult {
                selection,
                sticky_column_index: None,
            }))
    }
}

#[cfg(test)]
mod test_token {
    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(Some(tree_sitter_rust::LANGUAGE.into()), "fn main() {}");
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
