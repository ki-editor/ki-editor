use itertools::Itertools;

use crate::{components::editor::IfCurrentNotFound, selection_mode::ApplyMovementResult};

use super::{ByteRange, SelectionMode, TopNode};

pub(crate) struct SyntaxNode {
    /// If this is true:
    /// - anonymous siblings node will be skipped
    /// - current takes `TopNode`
    pub coarse: bool,
}

impl SelectionMode for SyntaxNode {
    fn jumps(
        &self,
        params: super::SelectionModeParams,
        chars: Vec<char>,
        line_number_ranges: Vec<std::ops::Range<usize>>,
    ) -> anyhow::Result<Vec<crate::components::editor::Jump>> {
        // Why do we use TopNode.jumps?
        // Because I realize I only use TopNode for jumping, and I never use jump in SyntaxNode
        // With this decision, TopNode selection mode can be removed from user-space, and we get one more keymap space
        TopNode.jumps(params, chars, line_number_ranges)
    }
    fn iter<'a>(
        &'a self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let buffer = params.buffer;
        let current_selection = params.current_selection;
        let node = buffer
            .get_current_node(current_selection, false)?
            .ok_or(anyhow::anyhow!(
                "SyntaxNode::iter: Cannot find Treesitter language"
            ))?;

        if let Some(parent) = node.parent() {
            let children = if self.coarse {
                (0..parent.named_child_count())
                    .filter_map(move |i| parent.named_child(i))
                    .collect_vec()
            } else {
                (0..parent.child_count())
                    .filter_map(move |i| parent.child(i))
                    .collect_vec()
            };
            Ok(Box::new(
                children
                    .into_iter()
                    .map(|node| ByteRange::new(node.byte_range())),
            ))
        } else {
            Ok(Box::new(std::iter::empty()))
        }
    }
    fn current(
        &self,
        params: super::SelectionModeParams,
        if_current_not_found: IfCurrentNotFound,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        if self.coarse {
            super::TopNode.current(params, if_current_not_found)
        } else {
            self.current_default_impl(params, if_current_not_found)
        }
    }
    fn up(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        TopNode.up(params)
    }
    fn down(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        TopNode.down(params)
    }
    fn previous(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        TopNode.previous(params)
    }
    fn next(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        TopNode.next(params)
    }
    fn parent(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.select_vertical(params.clone(), true)
    }
    fn first_child(
        &self,
        params: super::SelectionModeParams,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.select_vertical(params, false)
    }
}

impl SyntaxNode {
    pub(crate) fn select_vertical(
        &self,
        params: super::SelectionModeParams,
        go_up: bool,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        let Some(mut node) = params
            .buffer
            .get_current_node(params.current_selection, false)?
        else {
            return Ok(None);
        };
        while let Some(some_node) = get_node(node, go_up, self.coarse) {
            // This is necessary because sometimes the parent node can have the same range as
            // the current node
            if some_node.range() != node.range() {
                return Ok(Some(ApplyMovementResult::from_selection(
                    ByteRange::new(some_node.byte_range())
                        .to_selection(params.buffer, params.current_selection)?,
                )));
            }
            node = some_node;
        }
        Ok(None)
    }
}

pub(crate) fn get_node(
    node: tree_sitter::Node,
    go_up: bool,
    coarse: bool,
) -> Option<tree_sitter::Node> {
    match (go_up, coarse) {
        (true, _) => node.parent(),
        (false, true) => node.named_child(0),
        (false, false) => node.child(0),
    }
}

#[cfg(test)]
mod test_syntax_node {
    use crate::{
        buffer::Buffer,
        selection::{CharIndex, Selection},
        selection_mode::SelectionModeParams,
    };

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            Some(tree_sitter_rust::language()),
            "fn main() { let x = X {z,b,c:d} }",
        );
        SyntaxNode { coarse: true }.assert_all_selections(
            &buffer,
            Selection::default().set_range((CharIndex(23)..CharIndex(24)).into()),
            &[(23..24, "z"), (25..26, "b"), (27..30, "c:d")],
        );
        SyntaxNode { coarse: false }.assert_all_selections(
            &buffer,
            Selection::default().set_range((CharIndex(23)..CharIndex(24)).into()),
            &[
                (22..23, "{"),
                (23..24, "z"),
                (24..25, ","),
                (25..26, "b"),
                (26..27, ","),
                (27..30, "c:d"),
                (30..31, "}"),
            ],
        );
    }

    #[test]
    fn case_2() {
        let buffer = Buffer::new(
            Some(tree_sitter_rust::language()),
            "fn main() { let x = S(a); }",
        );
        SyntaxNode { coarse: true }.assert_all_selections(
            &buffer,
            Selection::default().set_range((CharIndex(20)..CharIndex(21)).into()),
            &[(20..21, "S"), (21..24, "(a)")],
        );
    }

    #[test]
    fn parent() {
        let expected_parent: &str = "z.b";
        let buffer = Buffer::new(
            Some(tree_sitter_rust::language()),
            "fn main() { let x = z.b(); }",
        );

        let child_range = (CharIndex(20)..CharIndex(21)).into();

        let child_text = buffer.slice(&child_range).unwrap();
        assert_eq!(child_text, "z");
        let selection = SyntaxNode { coarse: false }.parent(SelectionModeParams {
            buffer: &buffer,
            current_selection: &Selection::new(child_range),
            cursor_direction: &crate::components::editor::Direction::Start,
        });

        let parent_range = selection.unwrap().unwrap().selection.range();

        let parent_text = buffer.slice(&parent_range).unwrap();
        assert_eq!(parent_text, expected_parent);
    }

    #[test]
    fn first_child() {
        fn test(coarse: bool, expected_child: &str) {
            let buffer = Buffer::new(
                Some(tree_sitter_rust::language()),
                "fn main() { let x = {z}; }",
            );

            let parent_range = (CharIndex(20)..CharIndex(23)).into();

            let parent_text = buffer.slice(&parent_range).unwrap();
            assert_eq!(parent_text, "{z}");
            let selection = SyntaxNode { coarse }.first_child(SelectionModeParams {
                buffer: &buffer,
                current_selection: &Selection::new(parent_range),
                cursor_direction: &crate::components::editor::Direction::Start,
            });

            let child_range = selection.unwrap().unwrap().selection.range();

            let child_text = buffer.slice(&child_range).unwrap();
            assert_eq!(child_text, expected_child);
        }
        test(true, "z");
        test(false, "{");
    }

    #[test]
    fn current() {
        fn test(coarse: bool, expected_selection: &str) {
            let buffer = Buffer::new(
                Some(tree_sitter_rust::language()),
                "
fn main() {
  let x = X;
}"
                .trim(),
            );

            let range = (CharIndex(14)..CharIndex(17)).into();
            assert_eq!(buffer.slice(&range).unwrap(), "let");
            let selection = SyntaxNode { coarse }.current(
                SelectionModeParams {
                    buffer: &buffer,
                    current_selection: &Selection::new(range),
                    cursor_direction: &crate::components::editor::Direction::Start,
                },
                IfCurrentNotFound::LookForward,
            );

            let actual_range = buffer.slice(&selection.unwrap().unwrap().range()).unwrap();
            assert_eq!(actual_range, expected_selection);
        }
        test(true, "let x = X;");
        test(false, "let");
    }
}
