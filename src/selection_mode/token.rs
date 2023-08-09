pub struct Token;

use crate::selection_mode::SelectionMode;

use super::ByteRange;

impl SelectionMode for Token {
    fn iter<'a>(
        &self,
        _current_selection: &'a crate::selection::Selection,
        buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
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
