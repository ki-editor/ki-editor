pub struct Token;

use crate::{
    components::editor::node_to_selection,
    selection::{CharIndex, Selection},
    selection_mode::SelectionMode,
};

impl SelectionMode for Token {
    fn iter<'a>(
        buffer: &'a crate::buffer::Buffer,
        current_selection: crate::selection::Selection,
    ) -> Box<dyn Iterator<Item = crate::selection::Selection> + 'a> {
        Box::new(
            tree_sitter_traversal::traverse(
                buffer.tree().walk(),
                tree_sitter_traversal::Order::Post,
            )
            .filter(|node| node.child_count() == 0)
            .filter_map(move |node| {
                node_to_selection(
                    node,
                    buffer,
                    current_selection.copied_text.clone(),
                    current_selection.initial_range.clone(),
                )
                .ok()
            }),
        )
    }
}
