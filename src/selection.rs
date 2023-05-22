use std::ops::{Add, Range, Sub};

use regex::Regex;
use ropey::Rope;
use tree_sitter::{Node, Point};
use tree_sitter_traversal::Order;

use crate::{
    buffer::Buffer,
    components::editor::{node_to_selection, CursorDirection, Direction},
    utils::find_previous,
};

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionSet {
    pub primary: Selection,
    pub secondary: Vec<Selection>,
    pub mode: SelectionMode,
}
impl SelectionSet {
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
        self.primary.initial_range = None;
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

    pub fn copy(&mut self, buffer: &Buffer) {
        self.apply_mut(|selection| {
            selection.copied_text = Some(buffer.slice(&selection.extended_range()));
            selection.initial_range = None;
        });
    }

    pub fn select_kids(&self, buffer: &Buffer, cursor_direction: &CursorDirection) -> SelectionSet {
        fn select_kids(
            selection: &Selection,
            buffer: &Buffer,
            cursor_direction: &CursorDirection,
        ) -> Selection {
            let cursor_char_index = selection.to_char_index(cursor_direction);
            if let Some(node) = buffer.get_nearest_node_after_char(cursor_char_index) {
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
                            initial_range: selection.initial_range.clone(),
                        };
                    }
                }
            }
            selection.clone()
        }
        self.apply(SelectionMode::Custom, |selection| {
            select_kids(selection, buffer, cursor_direction)
        })
    }

    pub fn generate(
        &self,
        buffer: &Buffer,
        mode: &SelectionMode,
        direction: &Direction,
        cursor_direction: &CursorDirection,
    ) -> SelectionSet {
        self.apply(mode.clone(), |selection| {
            Selection::get_selection_(buffer, selection, mode, direction, cursor_direction)
        })
    }

    pub fn add_selection(&mut self, buffer: &Buffer, cursor_direction: &CursorDirection) {
        let last_selection = &self.primary;
        let next_selection = Selection::get_selection_(
            buffer,
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

    pub fn toggle_highlight_mode(&mut self) {
        self.apply_mut(|selection| selection.toggle_highlight_mode());
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

    pub fn display(&self) -> String {
        match self {
            SelectionMode::Word => "WORD".to_string(),
            SelectionMode::Line => "LINE".to_string(),
            SelectionMode::Character => "CHAR".to_string(),
            SelectionMode::Custom => "CUSTOM".to_string(),
            SelectionMode::Token => "TOKEN".to_string(),
            SelectionMode::NamedNode => "NODE".to_string(),
            SelectionMode::ParentNode => "PARENT".to_string(),
            SelectionMode::SiblingNode => "SIBLING".to_string(),
            SelectionMode::Match { regex } => format!("MATCH {:?}", regex),
        }
    }
}

#[derive(PartialEq, Clone, Debug, Eq, Hash)]
pub struct Selection {
    pub range: Range<CharIndex>,
    pub node_id: Option<usize>,
    pub copied_text: Option<Rope>,

    /// Used for extended selection.
    /// Some = the selection is being extended
    /// None = the selection is not being extended
    pub initial_range: Option<Range<CharIndex>>,
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

    pub fn extended_range(&self) -> Range<CharIndex> {
        match &self.initial_range {
            None => self.range.clone(),
            Some(extended_selection_anchor) => {
                self.range.start.min(extended_selection_anchor.start)
                    ..self.range.end.max(extended_selection_anchor.end)
            }
        }
    }

    #[cfg(test)]
    pub fn default() -> Selection {
        Selection {
            range: CharIndex(0)..CharIndex(0),
            node_id: None,
            copied_text: None,
            initial_range: None,
        }
    }

    pub fn get_selection_(
        buffer: &Buffer,
        current_selection: &Selection,
        mode: &SelectionMode,
        direction: &Direction,
        cursor_direction: &CursorDirection,
    ) -> Selection {
        // NOTE: cursor_char_index should only be used where the Direction is Current
        let cursor_char_index = {
            let index = current_selection.to_char_index(cursor_direction);
            match cursor_direction {
                CursorDirection::Start => index,
                // Minus one so that selecting line backward works
                CursorDirection::End => index - 1,
            }
        };
        let initial_range = current_selection.initial_range.clone();
        let cursor_byte = buffer.char_to_byte(cursor_char_index);
        let copied_text = current_selection.copied_text.clone();

        let Range {
            start: current_selection_start,
            end: current_selection_end,
        } = current_selection.extended_range();
        match mode {
            SelectionMode::NamedNode => match direction {
                Direction::Current => {
                    Some(buffer.get_current_node(cursor_char_index, current_selection))
                }
                Direction::Forward => buffer
                    .traverse(Order::Pre)
                    .find(|node| node.start_byte() > current_selection_start.0 && node.is_named()),
                Direction::Backward => {
                    find_previous(
                        buffer.traverse(Order::Pre),
                        |node, last_match| match last_match {
                            Some(last_match) => {
                                node.is_named()
                                // This predicate is so that if there's multiple node with the same
                                // start byte, we will only take the node with the largest range
                                && last_match.start_byte() < node.start_byte()
                            }
                            None => true,
                        },
                        |node| node.start_byte() >= current_selection_start.0 && node.is_named(),
                    )
                }
            }
            .map(|node| node_to_selection(node, buffer, copied_text, initial_range))
            .unwrap_or_else(|| current_selection.clone()),

            SelectionMode::Line => get_selection_via_regex(
                buffer,
                cursor_byte,
                r"(?m)^(.*)\n?",
                direction,
                current_selection,
                copied_text,
            ),
            SelectionMode::Word => get_selection_via_regex(
                buffer,
                cursor_byte,
                r"\b\w+",
                direction,
                current_selection,
                copied_text,
            ),
            SelectionMode::Character => get_selection_via_regex(
                buffer,
                cursor_byte,
                r"(?s).",
                direction,
                current_selection,
                copied_text,
            ),
            SelectionMode::Match { regex } => {
                get_selection_via_regex(
                    buffer,
                    cursor_byte,
                    &regex,
                    direction,
                    current_selection,
                    copied_text,
                )
                // get_selection_via_ast_grep(
                //     buffer,
                //     cursor_byte,
                //     search,
                //     direction,
                //     current_selection,
                //     copied_text,
                // )
            }
            SelectionMode::ParentNode => {
                let current_node = buffer.get_current_node(cursor_char_index, current_selection);

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
                node_to_selection(node, buffer, copied_text, initial_range)
            }

            SelectionMode::SiblingNode => {
                let current_node = buffer.get_current_node(cursor_char_index, current_selection);
                let next_node = match direction {
                    Direction::Current => Some(current_node),
                    Direction::Forward => buffer
                        .get_current_node(current_selection_end, current_selection)
                        .next_named_sibling(),
                    Direction::Backward => buffer
                        .get_current_node(current_selection_start, current_selection)
                        .prev_named_sibling(),
                }
                .unwrap_or(current_node);
                node_to_selection(next_node, buffer, copied_text, initial_range)
            }
            SelectionMode::Token => {
                let selection = match direction {
                    Direction::Forward => buffer.get_next_token(current_selection.range.end, false),
                    Direction::Backward => {
                        buffer.get_prev_token(current_selection.range.start, false)
                    }
                    Direction::Current => buffer.get_next_token(cursor_char_index, false),
                }
                .unwrap_or_else(|| {
                    buffer
                        .get_next_token(cursor_char_index, true)
                        .unwrap_or_else(|| buffer.tree().root_node())
                });
                node_to_selection(selection, buffer, copied_text, initial_range)
            }
            SelectionMode::Custom => Selection {
                range: cursor_char_index..cursor_char_index,
                node_id: None,
                copied_text,
                initial_range: current_selection.initial_range.clone(),
            },
        }
    }

    pub fn toggle_highlight_mode(&mut self) {
        match self.initial_range.take() {
            None => {
                self.initial_range = Some(self.range.clone());
            }
            // If highlight mode is enabled, inverse the selection
            Some(initial_range) => {
                self.initial_range = Some(std::mem::replace(&mut self.range, initial_range));
            }
        }
    }
}

// TODO: this works, but the result is not satisfactory,
// we will leave this function here as a reference
fn get_selection_via_ast_grep(
    buffer: &Buffer,
    cursor_byte: usize,
    pattern: &String,
    direction: &Direction,
    current_selection: &Selection,
    copied_text: Option<Rope>,
) -> Selection {
    let lang = ast_grep_core::language::TSLanguage::from(buffer.language());
    let pattern = ast_grep_core::matcher::Pattern::new(&pattern, lang.clone());
    let grep = ast_grep_core::AstGrep::new(buffer.rope().to_string(), lang);
    let mut matches_iter = grep.root().find_all(pattern);
    // let mut matches_iter = grep.root().find_all(ast_grep_core::matcher::MatchAll);
    let matches = match direction {
        Direction::Current => matches_iter.find(|matched| matched.range().contains(&cursor_byte)),
        Direction::Forward => matches_iter.find(|matched| matched.range().start > cursor_byte),
        Direction::Backward => find_previous(
            &mut matches_iter,
            |_, _| true,
            |match_| match_.range().start >= cursor_byte,
        ),
    };

    match matches {
        None => current_selection.clone(),
        Some(matches) => Selection {
            range: buffer.byte_to_char(matches.range().start)
                ..buffer.byte_to_char(matches.range().end),
            node_id: None,
            copied_text,
            initial_range: current_selection.initial_range.clone(),
        },
    }
}

fn get_selection_via_regex(
    buffer: &Buffer,
    cursor_byte: usize,
    regex: &str,
    direction: &Direction,
    current_selection: &Selection,
    copied_text: Option<Rope>,
) -> Selection {
    let regex = Regex::new(&regex);
    let regex = match regex {
        Err(_) => return current_selection.clone(),
        Ok(regex) => regex,
    };
    let string = buffer.rope().to_string();
    let matches = match direction {
        Direction::Current => regex.find_at(&string, cursor_byte),
        // TODO: should we rotate? i.e. if we are at the end, we should go to the beginning
        Direction::Forward => regex.find_at(&string, current_selection.extended_range().end.0),
        Direction::Backward => find_previous(
            &mut regex.find_iter(&string),
            |_, _| true,
            |match_| match_.start() >= current_selection.extended_range().start.0,
        ),
    };

    match matches {
        None => current_selection.clone(),
        Some(matches) => Selection {
            range: buffer.byte_to_char(matches.start())..buffer.byte_to_char(matches.end()),
            node_id: None,
            copied_text,
            initial_range: current_selection.initial_range.clone(),
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
            initial_range: self.initial_range,
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
            initial_range: self.initial_range,
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
