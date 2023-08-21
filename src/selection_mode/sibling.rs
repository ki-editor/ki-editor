use super::{ByteRange, SelectionMode};

pub struct Sibling;

impl SelectionMode for Sibling {
    fn name(&self) -> &'static str {
        "SIBLING"
    }
    fn iter<'a>(
        &'a self,
        current_selection: &'a crate::selection::Selection,
        buffer: &'a crate::buffer::Buffer,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let node = buffer.get_current_node(current_selection)?;

        if let Some(parent) = node.parent() {
            Ok(Box::new(
                (0..parent.named_child_count())
                    .filter_map(move |i| parent.named_child(i))
                    .map(|node| ByteRange::new(node.byte_range())),
            ))
        } else {
            Ok(Box::new(std::iter::empty()))
        }
    }
}

#[cfg(test)]
mod test_sibling {
    use crate::{
        buffer::Buffer,
        selection::{CharIndex, Selection},
    };

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            tree_sitter_rust::language(),
            "fn main() { let x = X {z,b,c:d} }",
        );
        Sibling.assert_all_selections(
            &buffer,
            Selection::default().set_range((CharIndex(23)..CharIndex(24)).into()),
            &[(23..24, "z"), (25..26, "b"), (27..30, "c:d")],
        );
    }
}
