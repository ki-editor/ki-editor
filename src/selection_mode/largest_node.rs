use super::{ByteRange, SelectionMode};
use itertools::{Itertools, TupleWindows};

pub struct LargestNode;

impl SelectionMode for LargestNode {
    fn iter<'a>(
        &self,
        buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        Ok(Box::new(
            tree_sitter_traversal::traverse(
                buffer.tree().walk(),
                tree_sitter_traversal::Order::Pre,
            )
            .group_by(|node| node.byte_range().start)
            .into_iter()
            .map(|(_, group)| {
                ByteRange(
                    group
                        .into_iter()
                        .max_by_key(|node| node.byte_range().end)
                        .unwrap()
                        .byte_range(),
                )
            })
            .into_iter()
            .collect_vec()
            .into_iter(),
        ))
    }
}
