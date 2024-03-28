use super::{ByteRange, SelectionMode};
use itertools::Itertools;

pub struct OutermostNode;

impl SelectionMode for OutermostNode {
    fn name(&self) -> &'static str {
        "OUTERMOST NODE"
    }
    fn iter<'a>(
        &self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let buffer = params.buffer;
        Ok(Box::new(
            crate::tree_sitter_traversal::traverse(
                buffer.tree().walk(),
                crate::tree_sitter_traversal::Order::Pre,
            )
            .group_by(|node| node.byte_range().start)
            .into_iter()
            .map(|(_, group)| {
                ByteRange::new(
                    group
                        .into_iter()
                        .max_by_key(|node| node.byte_range().end)
                        .unwrap()
                        .byte_range(),
                )
            })
            .collect_vec()
            .into_iter(),
        ))
    }
}

#[cfg(test)]
mod test_outermost_node {
    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            tree_sitter_rust::language(),
            "fn main(x: usize) { let x = 1; }",
        );
        OutermostNode.assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..32, "fn main(x: usize) { let x = 1; }"),
                (3..7, "main"),
                (7..17, "(x: usize)"),
                (8..16, "x: usize"),
                (9..10, ":"),
                (11..16, "usize"),
                (16..17, ")"),
                (18..32, "{ let x = 1; }"),
                (20..30, "let x = 1;"),
                (24..25, "x"),
                (26..27, "="),
                (28..29, "1"),
                (29..30, ";"),
                (31..32, "}"),
            ],
        );
    }
}
