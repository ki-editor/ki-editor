use crate::components::editor::node_to_selection;

use super::{SelectionMode, SelectionModeParams};

pub struct Node;

impl SelectionMode for Node {
    fn iter<'a>(
        &'a self,
        _buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        Ok(Box::new(std::iter::empty()))
    }

    /// For `Node`, `left` means parent node
    fn left(
        &self,
        SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction: _,
        }: SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.move_horizontally(buffer, current_selection, false)
    }

    /// For `Node`, `right` means first child node
    fn right(
        &self,
        SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction: _,
        }: SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.move_horizontally(buffer, current_selection, true)
    }

    fn up(
        &self,
        SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction: _,
        }: SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let current_node = buffer.get_current_node(current_selection)?;

        if let Some(node) = current_node.prev_named_sibling() {
            Ok(Some(node_to_selection(node, buffer, &current_selection)?))
        } else {
            Ok(None)
        }
    }

    fn down(
        &self,
        SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction: _,
        }: SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let current_node = buffer.get_current_node(current_selection)?;

        if let Some(node) = current_node.next_named_sibling() {
            Ok(Some(node_to_selection(node, buffer, &current_selection)?))
        } else {
            Ok(None)
        }
    }
}

impl Node {
    fn move_horizontally(
        &self,
        buffer: &crate::buffer::Buffer,
        current_selection: &crate::selection::Selection,
        go_right: bool,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let current_node = buffer.get_current_node(current_selection)?;
        let node = {
            let mut node = get_node(current_node, go_right);

            // This loop is to ensure we select the nearest parent that has a larger range than
            // the current node
            //
            // This is necessary because sometimes the parent node can have the same range as
            // the current node
            while let Some(some_node) = node {
                if some_node.range() != current_node.range() {
                    break;
                }
                node = get_node(some_node, go_right);
            }
            node.unwrap_or(current_node)
        };
        Ok(Some(node_to_selection(node, buffer, &current_selection)?))
    }
}

fn get_node(node: tree_sitter::Node, go_right: bool) -> Option<tree_sitter::Node> {
    match go_right {
        false => node.parent(),
        true => node.named_child(0),
    }
}
