pub(crate) struct SyntaxToken;

use std::rc::Rc;

use crate::{
    buffer::Buffer, components::editor::IfCurrentNotFound,
    selection_mode::PositionBasedSelectionMode,
};

use super::{
    get_current_selection_by_cursor_via_iter, ByteRange, PositionBased, SelectionMode, TopNode,
};

impl PositionBasedSelectionMode for SyntaxToken {
    fn expand_impl(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection_mode::ApplyMovementResult>> {
        Ok(PositionBased(TopNode)
            .current(params, IfCurrentNotFound::LookForward)?
            .map(|selection| crate::selection_mode::ApplyMovementResult {
                selection,
                mode: Some(crate::selection::SelectionMode::SyntaxNode),
            }))
    }

    fn get_current_selection_by_cursor(
        &self,
        buffer: &crate::buffer::Buffer,
        cursor_char_index: crate::selection::CharIndex,
        if_current_not_found: crate::components::editor::IfCurrentNotFound,
    ) -> anyhow::Result<Option<super::ByteRange>> {
        let cursor_byte = buffer.char_to_byte(cursor_char_index)?;
        let tree = buffer
            .tree()
            .ok_or(anyhow::anyhow!("Unable to find Treesitter language"))?;
        get_current_selection_by_cursor_via_iter(
            buffer,
            cursor_char_index,
            if_current_not_found,
            Rc::new(
                tree_sitter_traversal2::traverse(tree.walk(), tree_sitter_traversal2::Order::Post)
                    .filter(|node| node.child_count() == 0)
                    .map(|node| ByteRange::new(node.byte_range()))
                    .collect(),
            ),
        )
    }
}

#[cfg(test)]
mod test_token {
    use crate::{buffer::Buffer, selection::Selection, selection_mode::SelectionMode};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(Some(tree_sitter_rust::LANGUAGE.into()), "fn main() {}");
        PositionBased(SyntaxToken).assert_all_selections(
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
