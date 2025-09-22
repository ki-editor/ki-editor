use itertools::Itertools;

use crate::{
    components::editor::Direction,
    selection_mode::{syntax_token::SyntaxToken, ApplyMovementResult},
};

use super::{ByteRange, IterBasedSelectionMode, TopNode};

pub(crate) struct SyntaxNode {
    pub(crate) coarse: bool,
}

impl IterBasedSelectionMode for SyntaxNode {
    fn iter_revealed<'a>(
        &'a self,
        params: &super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let buffer = params.buffer;
        let current_selection = params.current_selection;
        let node = buffer
            .get_current_node(current_selection, false)?
            .ok_or(anyhow::anyhow!(
                "SyntaxNode::iter.get_current_node: Cannot find Treesitter language"
            ))?;
        let Some(node) = node.parent() else {
            return Ok(Box::new(std::iter::empty()));
        };
        let mut cursor = params
            .buffer
            .tree()
            .ok_or(anyhow::anyhow!(
                "SyntaxNode::iter.tree: Cannot find Treesitter language"
            ))?
            .walk();
        let vector = if self.coarse {
            node.named_children(&mut cursor).collect_vec()
        } else {
            node.children(&mut cursor).collect_vec()
        };
        Ok(Box::new(
            vector
                .into_iter()
                .map(|node| ByteRange::new(node.byte_range())),
        ))
    }
    fn iter<'a>(
        &'a self,
        params: &super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        if self.coarse {
            TopNode.iter(params)
        } else {
            SyntaxToken.iter(params)
        }
    }
    fn expand(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.select_vertical(params, true)
    }
    fn down(
        &self,
        params: &super::SelectionModeParams,
        _: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.select_vertical(params, false)
    }

    fn up(
        &self,
        params: &super::SelectionModeParams,
        _: Option<usize>,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        self.select_vertical(params, true)
    }

    fn left(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.navigate_sibling_nodes(params, &Direction::Start, true)
    }

    fn right(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.navigate_sibling_nodes(params, &Direction::End, true)
    }

    fn previous(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.navigate_sibling_nodes(params, &Direction::Start, false)
    }

    fn next(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.navigate_sibling_nodes(params, &Direction::End, false)
    }

    fn all_meaningful_selections<'a>(
        &'a self,
        params: &super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let buffer = params.buffer;
        let current_selection = params.current_selection;
        let node = buffer
            .get_current_node(current_selection, false)?
            .ok_or(anyhow::anyhow!(
                "SyntaxNode::iter: Cannot find Treesitter language"
            ))?;

        if let Some(parent) = node.parent() {
            let children = {
                (0..parent.named_child_count())
                    .filter_map(move |i| parent.named_child(i))
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

    #[cfg(test)]
    fn all_selections<'a>(
        &'a self,
        params: &super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let buffer = params.buffer;
        let current_selection = params.current_selection;
        let node = buffer
            .get_current_node(current_selection, false)?
            .ok_or(anyhow::anyhow!(
                "SyntaxNode::iter: Cannot find Treesitter language"
            ))?;

        if let Some(parent) = node.parent() {
            let children = {
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

    fn process_paste_gap(
        &self,
        _: &super::SelectionModeParams,
        prev_gap: Option<String>,
        next_gap: Option<String>,
        _: &Direction,
    ) -> String {
        match (prev_gap, next_gap) {
            (None, None) => Default::default(),
            (None, Some(gap)) | (Some(gap), None) => gap,
            (Some(prev_gap), Some(next_gap)) => {
                if prev_gap.chars().count() > next_gap.chars().count() {
                    prev_gap
                } else {
                    next_gap
                }
            }
        }
    }

    fn delete_backward(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.left(params)
    }

    fn delete_forward(
        &self,
        params: &super::SelectionModeParams,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        self.right(params)
    }
}

impl SyntaxNode {
    fn navigate_sibling_nodes(
        &self,
        params: &super::SelectionModeParams,
        direction: &Direction,
        named: bool,
    ) -> anyhow::Result<Option<crate::selection::Selection>> {
        let buffer = params.buffer;
        let current_selection = params.current_selection;
        let node = buffer
            .get_current_node(current_selection, false)?
            .ok_or(anyhow::anyhow!(
                "SyntaxNode::iter: Cannot find Treesitter language"
            ))?;
        let node = match (named, direction) {
            (true, Direction::Start) => node.prev_named_sibling(),
            (true, Direction::End) => node.next_named_sibling(),
            (false, Direction::Start) => node.prev_sibling(),
            (false, Direction::End) => node.next_sibling(),
        };
        Ok(node.and_then(|node| {
            ByteRange::new(node.byte_range())
                .to_selection(params.buffer, params.current_selection)
                .ok()
        }))
    }
    pub(crate) fn select_vertical(
        &self,
        params: &super::SelectionModeParams,
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
    use crate::buffer::BufferOwner;
    use crate::selection::SelectionMode;
    use crate::test_app::*;
    use crate::{
        buffer::Buffer,
        components::editor::IfCurrentNotFound,
        selection::{CharIndex, Selection},
        selection_mode::SelectionModeParams,
    };

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            Some(tree_sitter_rust::LANGUAGE.into()),
            "fn main() { let x = X {z,b,c:d} }",
        );
        super::SyntaxNode { coarse: true }.assert_all_selections(
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
            Some(tree_sitter_rust::LANGUAGE.into()),
            "fn main() { let x = S(a); }",
        );
        super::SyntaxNode { coarse: true }.assert_all_selections(
            &buffer,
            Selection::default().set_range((CharIndex(20)..CharIndex(21)).into()),
            &[(20..21, "S"), (21..24, "(a)")],
        );
    }

    #[test]
    fn parent() {
        let expected_parent: &str = "z.b";
        let buffer = Buffer::new(
            Some(tree_sitter_rust::LANGUAGE.into()),
            "fn main() { let x = z.b(); }",
        );

        let child_range = (CharIndex(20)..CharIndex(21)).into();

        let child_text = buffer.slice(&child_range).unwrap();
        assert_eq!(child_text, "z");
        let selection = super::SyntaxNode { coarse: false }.expand(&SelectionModeParams {
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
                Some(tree_sitter_rust::LANGUAGE.into()),
                "fn main() { let x = {z}; }",
            );

            let parent_range = (CharIndex(20)..CharIndex(23)).into();

            let parent_text = buffer.slice(&parent_range).unwrap();
            assert_eq!(parent_text, "{z}");
            let selection = super::SyntaxNode { coarse }.down(
                &SelectionModeParams {
                    buffer: &buffer,
                    current_selection: &Selection::new(parent_range),
                    cursor_direction: &crate::components::editor::Direction::Start,
                },
                None,
            );

            let child_range = selection.unwrap().unwrap().selection.range();

            let child_text = buffer.slice(&child_range).unwrap();
            assert_eq!(child_text, expected_child);
        }
        test(true, "z");
        test(false, "{");
    }

    #[test]
    fn current_prioritize_same_line() {
        fn test(coarse: bool, expected_selection: &str) {
            let buffer = Buffer::new(
                Some(tree_sitter_rust::LANGUAGE.into()),
                "
fn main() {
  let x = X;
}"
                .trim(),
            );

            let range = (CharIndex(13)..CharIndex(17)).into();
            assert_eq!(buffer.slice(&range).unwrap(), " let");
            let selection = super::SyntaxNode { coarse }.current(
                &SelectionModeParams {
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

    #[test]
    fn paste_forward_with_gap() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("fn f(x: X, y: Y) {}".to_string())),
                Editor(MatchLiteral("x: X".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::SyntaxNode,
                )),
                Editor(MoveSelection(Right)),
                Editor(Copy),
                Editor(Paste),
                Expect(CurrentComponentContent("fn f(x: X, y: Y, y: Y) {}")),
            ])
        })
    }

    #[test]
    fn paste_backward_with_gap() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("fn f(x: X, y: Y) {}".to_string())),
                Editor(MatchLiteral("x: X".to_string())),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::SyntaxNode,
                )),
                Editor(MoveSelection(Right)),
                Editor(Copy),
                Editor(SwapCursor),
                Editor(Paste),
                Expect(CurrentComponentContent("fn f(x: X, y: Y, y: Y) {}")),
            ])
        })
    }
}
