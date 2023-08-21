use super::{ByteRange, SelectionMode};

pub struct SyntaxHierarchy;

impl SelectionMode for SyntaxHierarchy {
    fn name(&self) -> &'static str {
        "SYNTAX HIERARCHY"
    }
    fn iter<'a>(
        &'a self,
        current_selection: &'a crate::selection::Selection,
        buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let current_node = buffer.get_current_node(current_selection)?;

        Ok(Box::new(
            get_nodes(current_node, true)
                .into_iter()
                .chain(std::iter::once(current_node))
                .chain(get_nodes(current_node, false))
                .map(|node| {
                    ByteRange::new({
                        let range = node.range();
                        range.start_byte..range.end_byte
                    })
                }),
        ))
    }
}

fn get_node(node: tree_sitter::Node, go_up: bool) -> Option<tree_sitter::Node> {
    match go_up {
        true => node.parent(),
        false => node.named_child(0),
    }
}

fn get_nodes(node: tree_sitter::Node, go_up: bool) -> Vec<tree_sitter::Node> {
    let mut nodes = vec![];
    let mut node = node;
    while let Some(some_node) = get_node(node, go_up) {
        // This is necessary because sometimes the parent node can have the same range as
        // the current node
        if some_node.range() != node.range() {
            nodes.push(some_node);
        }
        node = some_node;
    }

    if go_up {
        nodes.reverse();
    }

    nodes
}

#[cfg(test)]
mod test_syntax_hierarchy {
    use crate::{
        buffer::Buffer,
        selection::{CharIndex, Selection},
    };

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            tree_sitter_rust::language(),
            "fn main(x: usize) { let x = 1; }",
        );
        SyntaxHierarchy.assert_all_selections(
            &buffer,
            Selection::default().set_range((CharIndex(20)..CharIndex(29)).into()),
            &[
                (0..32, "fn main(x: usize) { let x = 1; }"),
                (18..32, "{ let x = 1; }"),
                (20..30, "let x = 1;"),
                (24..25, "x"),
            ],
        );
    }
}
