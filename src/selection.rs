use std::ops::{Add, Range, Sub};

use regex::Regex;
use ropey::Rope;
use tree_sitter::{Node, Point, Tree};
use tree_sitter_traversal::{traverse, Order};

use crate::engine::{node_to_selection, CursorDirection, Direction};

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionSet {
    pub primary: Selection,
    pub secondary: Vec<Selection>,
    pub mode: SelectionMode,
}
impl SelectionSet {
    #[cfg(test)]
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

    pub fn copy(&mut self, rope: &Rope) {
        self.apply_mut(|selection| {
            selection.copied_text = rope
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
                            copied_text: selection.copied_text.clone(),
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
        let last_selection = &self.primary;
        let next_selection = Selection::get_selection_(
            rope,
            tree,
            last_selection,
            &self.mode,
            &Direction::Forward,
            cursor_direction,
        );

        if next_selection.range == last_selection.range {
            return;
        }

        let previous_primary = std::mem::replace(&mut self.primary, next_selection);

        self.secondary.push(previous_primary);
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SelectionMode {
    Word,
    Line,
    Character,
    Custom,

    Token,

    NamedNode,
    ParentNode,
    SiblingNode,
    Match { regex: String },
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
    pub copied_text: Option<Rope>,
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
            copied_text: None,
        }
    }

    #[cfg(test)]
    pub fn default() -> Selection {
        Selection {
            range: CharIndex(0)..CharIndex(0),
            node_id: None,
            copied_text: None,
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
        let copied_text = current_selection.copied_text.clone();
        match mode {
            SelectionMode::NamedNode => match direction {
                Direction::Current => Some(get_current_node(tree, cursor_byte, current_selection)),
                Direction::Forward => traverse(tree.root_node().walk(), Order::Pre)
                    .find(|node| node.start_byte() > cursor_byte && node.is_named()),
                Direction::Backward => {
                    find_previous(
                        traverse(tree.root_node().walk(), Order::Pre),
                        |node, last_match| match last_match {
                            Some(last_match) => {
                                node.is_named()
                                // This predicate is so that if there's multiple node with the same
                                // start byte, we will only take the node with the largest range
                                && last_match.start_byte() < node.start_byte()
                            }
                            None => true,
                        },
                        |node| node.start_byte() >= cursor_byte && node.is_named(),
                    )
                }
            }
            .map(|node| node_to_selection(node, text, copied_text))
            .unwrap_or_else(|| current_selection.clone()),

            SelectionMode::Line => get_selection_via_regex(
                text,
                cursor_byte,
                Regex::new(r"(?m)^(.*)\n").unwrap(),
                direction,
                current_selection,
                copied_text,
            ),
            SelectionMode::Word => get_selection_via_regex(
                text,
                cursor_byte,
                Regex::new(r"\b\w+").unwrap(),
                direction,
                current_selection,
                copied_text,
            ),
            SelectionMode::Character => get_selection_via_regex(
                text,
                cursor_byte,
                Regex::new(r"(?s).").unwrap(),
                direction,
                current_selection,
                copied_text,
            ),
            SelectionMode::Match { regex: search } => {
                let regex = Regex::new(search).unwrap();
                get_selection_via_regex(
                    text,
                    cursor_byte,
                    regex,
                    direction,
                    current_selection,
                    copied_text,
                )
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
                node_to_selection(node, text, copied_text)
            }

            SelectionMode::SiblingNode => {
                let current_node = get_current_node(tree, cursor_byte, current_selection);
                let next_node = match direction {
                    Direction::Current => Some(current_node),
                    Direction::Forward => current_node.next_named_sibling(),
                    Direction::Backward => current_node.prev_named_sibling(),
                }
                .unwrap_or(current_node);
                node_to_selection(next_node, text, copied_text)
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
                node_to_selection(selection, text, copied_text)
            }
            SelectionMode::Custom => Selection {
                range: cursor_char_index..cursor_char_index,
                node_id: None,
                copied_text,
            },
        }
    }
}

fn get_selection_via_regex(
    text: &Rope,
    cursor_byte: usize,
    regex: Regex,
    direction: &Direction,
    current_selection: &Selection,
    copied_text: Option<Rope>,
) -> Selection {
    let string = text.to_string();
    let matches = match direction {
        Direction::Current => regex.find_at(&string, cursor_byte),
        Direction::Forward => regex.find_at(&string, cursor_byte + 1),
        Direction::Backward => find_previous(
            &mut regex.find_iter(&string),
            |_, _| true,
            |match_| match_.start() >= cursor_byte,
        ),
    };

    match matches {
        None => current_selection.clone(),
        Some(matches) => Selection {
            range: CharIndex(text.byte_to_char(matches.start()))
                ..CharIndex(text.byte_to_char(matches.end())),
            node_id: None,
            copied_text,
        },
    }
}

impl Add<usize> for Selection {
    type Output = Selection;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            range: self.range.start + rhs..self.range.end + rhs,
            node_id: self.node_id,
            copied_text: self.copied_text,
        }
    }
}

impl Sub<usize> for Selection {
    type Output = Selection;

    fn sub(self, rhs: usize) -> Self::Output {
        Self {
            range: self.range.start - rhs..self.range.end - rhs,
            node_id: self.node_id,
            copied_text: self.copied_text,
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

pub trait ToRangeUsize {
    fn to_usize_range(&self) -> Range<usize>;
}

impl ToRangeUsize for Range<CharIndex> {
    fn to_usize_range(&self) -> Range<usize> {
        self.start.0..self.end.0
    }
}

fn find_previous<T>(
    mut iter: impl Iterator<Item = T>,
    set_last_match_predicate: impl Fn(&T, &Option<T>) -> bool,
    break_predicate: impl Fn(&T) -> bool,
) -> Option<T> {
    let mut last_match = None;
    while let Some(match_) = iter.next() {
        if break_predicate(&match_) {
            break;
        }

        if set_last_match_predicate(&match_, &last_match) {
            last_match = Some(match_);
        }
    }
    last_match
}

fn get_prev_token(tree: &Tree, byte: usize, is_named: bool) -> Option<Node> {
    find_previous(
        traverse(tree.root_node().walk(), Order::Post),
        |_, _| true,
        |node| {
            node.child_count() == 0 && (!is_named || node.is_named()) && node.start_byte() >= byte
        },
    )
}

fn get_next_token(tree: &Tree, byte: usize, is_named: bool) -> Option<Node> {
    traverse(tree.root_node().walk(), Order::Post).find(|&node| {
        node.child_count() == 0 && (!is_named || node.is_named()) && node.end_byte() > byte
    })
}

fn get_nearest_node_after_byte(tree: &Tree, byte: usize) -> Option<Node> {
    // Preorder is the main key here,
    // because preorder traversal walks the parent first
    traverse(tree.root_node().walk(), Order::Pre).find(|&node| node.start_byte() >= byte)
}

fn get_current_node<'a>(tree: &'a Tree, cursor_byte: usize, selection: &Selection) -> Node<'a> {
    if let Some(node_id) = selection.node_id {
        get_node_by_id(tree, node_id)
    } else {
        get_nearest_node_after_byte(tree, cursor_byte)
    }
    .unwrap_or_else(|| tree.root_node())
}

fn get_node_by_id(tree: &Tree, node_id: usize) -> Option<Node> {
    let result = traverse(tree.walk(), Order::Pre).find(|node| node.id() == node_id);
    result
}
