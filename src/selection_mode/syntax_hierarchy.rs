use crate::components::editor::{node_to_selection, CursorDirection};

use super::{ByteRange, SelectionMode};

pub struct SyntaxHierarchy;

impl SelectionMode for SyntaxHierarchy {
    fn iter<'a>(
        &'a self,
        current_selection: &'a crate::selection::Selection,
        buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(std::iter::empty()))
    }

    fn right(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.move_vertically(
            params.buffer,
            params.current_selection,
            params.cursor_direction,
            false,
        )
    }

    fn left(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.move_vertically(
            params.buffer,
            params.current_selection,
            params.cursor_direction,
            true,
        )
    }

    fn move_vertically(
        &self,
        buffer: &crate::buffer::Buffer,
        current_selection: &crate::selection::Selection,
        _cursor_direction: &CursorDirection,
        go_up: bool,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let current_node = buffer.get_current_node(current_selection)?;
        let node = {
            let mut node = get_node(current_node, go_up);

            // This loop is to ensure we select the nearest parent that has a larger range than
            // the current node
            //
            // This is necessary because sometimes the parent node can have the same range as
            // the current node
            while let Some(some_node) = node {
                if some_node.range() != current_node.range() {
                    break;
                }
                node = get_node(some_node, go_up);
            }
            node.unwrap_or(current_node)
        };
        Ok(Some(node_to_selection(node, buffer, current_selection)?))
    }
}

fn get_node(node: tree_sitter::Node, go_up: bool) -> Option<tree_sitter::Node> {
    match go_up {
        true => node.parent(),
        false => node.named_child(0),
    }
}
