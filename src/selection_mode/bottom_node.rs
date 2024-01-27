pub struct BottomNode;

use crate::selection_mode::SelectionMode;

use super::ByteRange;

impl SelectionMode for BottomNode {
    fn name(&self) -> &'static str {
        "TOKEN"
    }
    fn iter<'a>(
        &self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let buffer = params.buffer;
        Ok(Box::new(
            tree_sitter_traversal::traverse(
                buffer.tree().walk(),
                tree_sitter_traversal::Order::Post,
            )
            .filter(|node| node.child_count() == 0)
            .map(|node| ByteRange::new(node.byte_range())),
        ))
    }
}

#[cfg(test)]
mod test_token {
    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(tree_sitter_rust::language(), "fn main() {}");
        BottomNode.assert_all_selections(
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
