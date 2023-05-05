use std::ops::{Add, Range, Sub};

use itertools::Itertools;
use ropey::Rope;
use tree_sitter::{Node, Point, Tree};
use tree_sitter_traversal::{traverse, Order};

use crate::{
    edit::{Action, Edit, EditTransaction},
    engine::{
        get_current_node, get_nearest_node_after_byte, get_next_token, get_prev_token,
        node_to_selection, CursorDirection, Direction, ReverseTreeCursor,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionSet {
    pub primary: Selection,
    pub secondary: Vec<Selection>,
    pub mode: SelectionMode,
}
impl SelectionSet {
    pub fn default() -> SelectionSet {
        SelectionSet {
            primary: Selection::default(),
            secondary: Vec::new(),
            mode: SelectionMode::Line,
        }
    }

    pub fn map<F, A>(&self, f: F) -> Vec<A>
    where
        F: Fn(&Selection) -> A,
    {
        vec![f(&self.primary)]
            .into_iter()
            .chain(self.secondary.iter().map(f))
            .collect()
    }

    pub fn reset(&mut self) {
        self.secondary.clear();
    }

    fn apply<F>(&self, mode: SelectionMode, f: F) -> SelectionSet
    where
        F: Fn(&Selection) -> Selection,
    {
        SelectionSet {
            primary: f(&self.primary),
            secondary: self.secondary.iter().map(f).collect(),
            mode,
        }
    }

    pub fn replace<GetOld, GetNew, GetRange>(
        &mut self,
        get_old: GetOld,
        get_new: GetNew,
        get_range: GetRange,
    ) -> EditTransaction
    where
        GetOld: Fn(&Selection) -> Rope,
        GetNew: Fn(&Selection) -> Rope,
        GetRange: Fn(&Edit) -> Range<CharIndex>,
    {
        let edit_transaction = EditTransaction::from_actions(
            self.clone(),
            self.map(|selection| {
                Action::Edit(Edit {
                    start: selection.range.start,
                    old: get_old(selection),
                    new: get_new(selection),
                })
            })
            .into_iter()
            .collect(),
        );

        match edit_transaction.edits().split_first() {
            None => {
                todo!("Not sure what to do here")
            }
            Some((head, tail)) => {
                self.primary = Selection {
                    range: get_range(head),
                    node_id: None,
                    yanked_text: None,
                };
                self.secondary = tail
                    .iter()
                    .map(|edit| Selection {
                        range: get_range(edit),
                        node_id: None,
                        yanked_text: None,
                    })
                    .collect();
            }
        }

        edit_transaction
    }

    pub fn update(&mut self, edit_transaction: &EditTransaction) {}

    pub fn move_left(&mut self, cursor_direction: &CursorDirection) {
        self.apply_mut(|selection| {
            let cursor_char_index = selection.to_char_index(cursor_direction);
            selection.range = cursor_char_index - 1..cursor_char_index - 1
        });
    }

    pub fn move_right(&mut self, cursor_direction: &CursorDirection) {
        self.apply_mut(|selection| {
            let cursor_char_index = selection.to_char_index(cursor_direction);
            selection.range = cursor_char_index + 1..cursor_char_index + 1
        });
    }

    pub fn apply_mut<F, A>(&mut self, f: F) -> Vec<A>
    where
        F: Fn(&mut Selection) -> A,
    {
        let mut result = vec![f(&mut self.primary)];
        for selection in &mut self.secondary {
            result.push(f(selection));
        }
        result
    }

    pub fn yank(&mut self, rope: &Rope) {
        self.apply_mut(|selection| {
            selection.yanked_text = rope
                .get_slice(selection.range.start.0..selection.range.end.0)
                .map(|slice| slice.into());
        });
    }

    pub fn select_kids(
        &self,
        rope: &Rope,
        tree: &Tree,
        cursor_direction: &CursorDirection,
    ) -> SelectionSet {
        fn select_kids(
            selection: &Selection,
            rope: &Rope,
            tree: &Tree,
            cursor_direction: &CursorDirection,
        ) -> Selection {
            let cursor_char_index = selection.to_char_index(cursor_direction);
            if let Some(node) =
                get_nearest_node_after_byte(tree, rope.char_to_byte(cursor_char_index.0))
            {
                if let Some(parent) = node.parent() {
                    let second_child = parent.child(1);
                    let second_last_child = parent.child(parent.child_count() - 2).or(second_child);

                    if let (Some(second_child), Some(second_last_child)) =
                        (second_child, second_last_child)
                    {
                        return Selection {
                            range: CharIndex(second_child.start_byte())
                                ..CharIndex(second_last_child.end_byte()),
                            node_id: None,
                            yanked_text: selection.yanked_text.clone(),
                        };
                    }
                }
            }
            selection.clone()
        }
        self.apply(SelectionMode::Custom, |selection| {
            select_kids(selection, rope, tree, cursor_direction)
        })
    }

    pub fn generate(
        &self,
        rope: &Rope,
        tree: &Tree,
        mode: &SelectionMode,
        direction: &Direction,
        cursor_direction: &CursorDirection,
    ) -> SelectionSet {
        self.apply(mode.clone(), |selection| {
            Selection::get_selection_(rope, tree, selection, mode, direction, cursor_direction)
        })
    }

    pub fn add_selection(&mut self, rope: &Rope, tree: &Tree, cursor_direction: &CursorDirection) {
        let last_selection = self.secondary.last().unwrap_or(&self.primary);
        let next_selection = Selection::get_selection_(
            rope,
            tree,
            last_selection,
            &self.mode,
            &Direction::Forward,
            cursor_direction,
        );
        self.secondary.push(next_selection);
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SelectionMode {
    Line,
    Character,
    Custom,

    Token,

    NamedNode,
    ParentNode,
    SiblingNode,
}
impl SelectionMode {
    pub fn similar_to(&self, other: &SelectionMode) -> bool {
        self == other
        // || self.is_node() && other.is_node()
    }

    pub fn is_node(&self) -> bool {
        use SelectionMode::*;
        matches!(self, NamedNode | ParentNode | SiblingNode)
    }
}

#[derive(PartialEq, Clone, Debug, Eq, Hash)]
pub struct Selection {
    pub range: Range<CharIndex>,
    pub node_id: Option<usize>,
    pub yanked_text: Option<Rope>,
}
impl Selection {
    pub fn to_char_index(&self, cursor_direction: &CursorDirection) -> CharIndex {
        // TODO(bug): when SelectionMode is Line and CursorDirection is End,
        // the cursor will be one line below the current selected line
        match cursor_direction {
            CursorDirection::Start => self.range.start,
            CursorDirection::End => self.range.end,
        }
    }

    pub fn from_two_char_indices(
        anchor: &CharIndex,
        get_cursor_char_index: &CharIndex,
    ) -> Selection {
        Selection {
            range: *anchor.min(get_cursor_char_index)..*anchor.max(get_cursor_char_index),
            node_id: None,
            yanked_text: None,
        }
    }

    fn len(&self) -> usize {
        self.range.end.0.saturating_sub(self.range.start.0)
    }

    pub fn apply_offset(&self, change: isize) -> Selection {
        Selection {
            range: self.range.start.apply_offset(change)..self.range.end.apply_offset(change),
            node_id: self.node_id,
            yanked_text: self.yanked_text.clone(),
        }
    }

    pub fn default() -> Selection {
        Selection {
            range: CharIndex(0)..CharIndex(0),
            node_id: None,
            yanked_text: None,
        }
    }

    pub fn get_selection_(
        text: &Rope,
        tree: &Tree,
        current_selection: &Selection,
        mode: &SelectionMode,
        direction: &Direction,
        cursor_direction: &CursorDirection,
    ) -> Selection {
        let cursor_char_index = {
            let index = current_selection.to_char_index(cursor_direction);
            match cursor_direction {
                CursorDirection::Start => index,
                // Minus one so that selecting line backward works
                CursorDirection::End => index - 1,
            }
        };
        let cursor_byte = cursor_char_index.to_byte(&text);
        let yanked_text = current_selection.yanked_text.clone();
        match mode {
            SelectionMode::NamedNode => match direction {
                Direction::Current => Some(get_current_node(tree, cursor_byte, current_selection)),
                Direction::Forward => traverse(tree.root_node().walk(), Order::Pre)
                    .find(|node| node.start_byte() > cursor_byte && node.is_named()),
                Direction::Backward => ReverseTreeCursor::new(tree.root_node())
                    .tuple_windows()
                    .find(|(current, next)| {
                        next.start_byte() < current.start_byte()
                            && current.start_byte() < cursor_byte
                            && current.is_named()
                    })
                    .map(|(current, _)| current),
            }
            .map(|node| node_to_selection(node, *mode, text, yanked_text))
            .unwrap_or_else(|| current_selection.clone()),
            SelectionMode::Line => {
                let start = cursor_char_index.to_line(text);

                let start = CharIndex(
                    text.line_to_char(
                        match direction {
                            Direction::Forward => start.saturating_add(1),
                            Direction::Backward => start.saturating_sub(1),
                            Direction::Current => cursor_char_index.to_line(text),
                        }
                        .min(text.len_lines().saturating_sub(1)),
                    ),
                );
                let end = CharIndex(
                    start
                        .0
                        .saturating_add(text.line(start.to_line(text)).len_chars()),
                );
                Selection {
                    range: start..end,
                    node_id: None,
                    yanked_text,
                }
            }
            SelectionMode::ParentNode => {
                let current_node = get_current_node(tree, cursor_byte, current_selection);

                fn get_node(node: Node, direction: Direction) -> Option<Node> {
                    match direction {
                        Direction::Current => Some(node),
                        Direction::Forward => node.parent(),

                        // Backward of ParentNode = ChildNode
                        Direction::Backward => node.named_child(0),
                    }
                }

                let node = {
                    if direction == &Direction::Current {
                        current_node
                    } else {
                        let mut node = get_node(current_node, *direction);

                        // This loop is to ensure we select the nearest parent that has a larger range than
                        // the current node
                        //
                        // This is necessary because sometimes the parent node can have the same range as
                        // the current node
                        while let Some(some_node) = node {
                            if some_node.range() != current_node.range() {
                                break;
                            }
                            node = get_node(some_node, *direction);
                        }
                        node.unwrap_or(current_node)
                    }
                };
                node_to_selection(node, *mode, text, yanked_text)
            }

            SelectionMode::SiblingNode => {
                let current_node = get_current_node(tree, cursor_byte, current_selection);
                let next_node = match direction {
                    Direction::Current => Some(current_node),
                    Direction::Forward => current_node.next_named_sibling(),
                    Direction::Backward => current_node.prev_named_sibling(),
                }
                .unwrap_or(current_node);
                node_to_selection(next_node, *mode, text, yanked_text)
            }
            SelectionMode::Token => {
                let current_selection_start_byte = current_selection.range.start.to_byte(text);
                let current_selection_end_byte = current_selection.range.end.to_byte(text);
                let selection = match direction {
                    Direction::Forward => get_next_token(tree, current_selection_end_byte, false),
                    Direction::Backward => {
                        get_prev_token(tree, current_selection_start_byte, false)
                    }
                    Direction::Current => get_next_token(tree, cursor_byte, false),
                }
                .unwrap_or_else(|| {
                    get_next_token(tree, cursor_byte, true).unwrap_or_else(|| tree.root_node())
                });
                node_to_selection(selection, *mode, text, yanked_text)
            }
            SelectionMode::Character => match direction {
                Direction::Current => Selection {
                    range: cursor_char_index..cursor_char_index + 1,
                    node_id: None,
                    yanked_text,
                },
                Direction::Forward => Selection {
                    range: cursor_char_index + 1..cursor_char_index + 2,
                    node_id: None,
                    yanked_text,
                },
                Direction::Backward => Selection {
                    range: cursor_char_index - 1..cursor_char_index,
                    node_id: None,
                    yanked_text,
                },
            },
            SelectionMode::Custom => Selection {
                range: cursor_char_index..cursor_char_index,
                node_id: None,
                yanked_text,
            },
        }
    }
}

impl Add<usize> for Selection {
    type Output = Selection;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            range: self.range.start + rhs..self.range.end + rhs,
            node_id: self.node_id,
            yanked_text: self.yanked_text,
        }
    }
}

impl Sub<usize> for Selection {
    type Output = Selection;

    fn sub(self, rhs: usize) -> Self::Output {
        Self {
            range: self.range.start - rhs..self.range.end - rhs,
            node_id: self.node_id,
            yanked_text: self.yanked_text,
        }
    }
}

impl Add<usize> for CharIndex {
    type Output = CharIndex;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0.saturating_add(rhs))
    }
}

impl Sub<usize> for CharIndex {
    type Output = CharIndex;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0.saturating_sub(rhs))
    }
}

#[derive(PartialEq, Clone, Debug, Copy, PartialOrd, Eq, Ord, Hash)]
pub struct CharIndex(pub usize);

impl CharIndex {
    pub fn to_point(self, rope: &Rope) -> Point {
        let line = self.to_line(rope);
        Point {
            row: line,
            column: rope
                .try_line_to_char(line)
                .map(|char_index| self.0.saturating_sub(char_index))
                .unwrap_or(0),
        }
    }

    pub fn to_line(self, rope: &Rope) -> usize {
        rope.try_char_to_line(self.0)
            .unwrap_or_else(|_| rope.len_lines())
    }

    pub fn to_byte(self, rope: &Rope) -> usize {
        rope.try_char_to_byte(self.0)
            .unwrap_or_else(|_| rope.len_bytes())
    }

    pub fn apply_offset(&self, change: isize) -> CharIndex {
        if change.is_positive() {
            *self + (change as usize)
        } else {
            *self - (change.abs() as usize)
        }
    }
}
