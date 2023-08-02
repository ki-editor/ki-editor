use selection_mode::SelectionMode as SelectionModeTrait;
use std::ops::{Add, Range, Sub};

use ropey::Rope;

use crate::{
    buffer::Buffer,
    components::editor::{CursorDirection, Direction},
    context::{Context, Search, SearchKind},
    position::Position,
    selection_mode::{self, SelectionModeParams},
};

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionSet {
    pub primary: Selection,
    pub secondary: Vec<Selection>,
    pub mode: SelectionMode,
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self {
            primary: Selection::default(),
            secondary: vec![],
            mode: SelectionMode::Custom,
        }
    }
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

    pub fn apply<F>(&self, mode: SelectionMode, f: F) -> anyhow::Result<SelectionSet>
    where
        F: Fn(&Selection) -> anyhow::Result<Selection>,
    {
        Ok(SelectionSet {
            primary: f(&self.primary)?,
            secondary: self
                .secondary
                .iter()
                .map(f)
                .collect::<anyhow::Result<Vec<_>>>()?,
            mode,
        })
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

    pub fn copy(&mut self, buffer: &Buffer, context: &mut Context) {
        if self.secondary.is_empty() {
            // Copy the primary selected text to clipboard
            let copied_text = buffer.slice(&self.primary.extended_range());
            context.set_clipboard_content(copied_text.to_string());
            self.primary = Selection {
                range: self.primary.range.clone(),
                initial_range: None,
                copied_text: Some(copied_text),
                info: None,
            }
        } else {
            // Otherwise, don't copy to clipboard, since there's multiple selection,
            // we don't know which one to copy.
            self.apply_mut(|selection| {
                selection.copied_text = Some(buffer.slice(&selection.extended_range()))
                    .or_else(|| context.get_clipboard_content().map(Rope::from));
                selection.initial_range = None;
            });
        }
    }

    pub fn select_kids(
        &self,
        buffer: &Buffer,
        cursor_direction: &CursorDirection,
    ) -> anyhow::Result<SelectionSet> {
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
                            copied_text: selection.copied_text.clone(),
                            initial_range: selection.initial_range.clone(),
                            info: selection.info.clone(),
                        };
                    }
                }
            }
            selection.clone()
        }
        self.apply(SelectionMode::Custom, |selection| {
            Ok(select_kids(selection, buffer, cursor_direction))
        })
    }

    pub fn generate(
        &self,
        buffer: &Buffer,
        mode: &SelectionMode,
        direction: &Direction,
        cursor_direction: &CursorDirection,
    ) -> anyhow::Result<SelectionSet> {
        self.apply(mode.clone(), |selection| {
            Selection::get_selection_(buffer, selection, mode, direction, cursor_direction)
        })
    }

    pub fn add_selection(
        &mut self,
        buffer: &Buffer,
        cursor_direction: &CursorDirection,
    ) -> anyhow::Result<()> {
        let mode = if self.mode.is_node() {
            SelectionMode::SiblingNode
        } else {
            self.mode.clone()
        };
        let last_selection = &self.primary;
        let next_selection = Selection::get_selection_(
            buffer,
            last_selection,
            &mode,
            &Direction::Right,
            cursor_direction,
        )?;

        if next_selection.range == last_selection.range {
            return Ok(());
        }

        let previous_primary = std::mem::replace(&mut self.primary, next_selection);

        self.secondary.push(previous_primary);
        Ok(())
    }

    pub fn toggle_highlight_mode(&mut self) {
        self.apply_mut(|selection| selection.toggle_highlight_mode());
    }

    pub fn clamp(&self, max_char_index: CharIndex) -> anyhow::Result<SelectionSet> {
        self.apply(self.mode.clone(), |selection| {
            Ok(selection.clamp(max_char_index))
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SelectionMode {
    // Regex
    Word,
    Line,
    Character,
    Custom,
    Match {
        search: Search,
    },

    // Syntax-tree
    Token,
    LargestNode,
    Node,

    #[deprecated(note = "Use `Node` instead, to be removed soon")]
    SiblingNode,

    // LSP
    Diagnostic,

    // Git
    GitHunk,
}
impl SelectionMode {
    pub fn similar_to(&self, other: &SelectionMode) -> bool {
        self == other || self.is_node() && other.is_node()
    }

    pub fn is_node(&self) -> bool {
        use SelectionMode::*;
        matches!(self, LargestNode | Node | SiblingNode)
    }

    pub fn display(&self) -> String {
        match self {
            SelectionMode::Word => "WORD".to_string(),
            SelectionMode::Line => "LINE".to_string(),
            SelectionMode::Character => "CHAR".to_string(),
            SelectionMode::Custom => "CUSTOM".to_string(),
            SelectionMode::Token => "TOKEN".to_string(),
            SelectionMode::LargestNode => "LARGEST NODE".to_string(),
            SelectionMode::Node => "NODE".to_string(),
            SelectionMode::SiblingNode => "SIBLING".to_string(),
            SelectionMode::Match { search } => {
                format!("MATCH({:?})={:?}", search.kind, search.search)
            }
            SelectionMode::Diagnostic => "DIAGNOSTIC".to_string(),
            SelectionMode::GitHunk => "GIT HUNK".to_string(),
        }
    }

    pub fn to_selection_mode_trait_object(
        &self,
        buffer: &Buffer,
        current_selection: &Selection,
    ) -> anyhow::Result<Box<dyn selection_mode::SelectionMode>> {
        Ok(match self {
            SelectionMode::Word => Box::new(selection_mode::Regex::new(
                buffer,
                r"[a-z]+|[A-Z]+[a-z]*|[0-9]+",
                false,
            )?),
            SelectionMode::Line => Box::new(selection_mode::Line),
            SelectionMode::Character => {
                Box::new(selection_mode::Regex::new(buffer, r"(?s).", false)?)
            }
            SelectionMode::Custom => {
                Box::new(selection_mode::Custom::new(current_selection.clone()))
            }
            SelectionMode::Match { search } => match search.kind {
                SearchKind::Literal => {
                    Box::new(selection_mode::Regex::new(buffer, &search.search, true)?)
                }
                SearchKind::Regex => {
                    Box::new(selection_mode::Regex::new(buffer, &search.search, false)?)
                }
                SearchKind::AstGrep => {
                    Box::new(selection_mode::AstGrep::new(buffer, &search.search)?)
                }
            },
            SelectionMode::Token => Box::new(selection_mode::Token),
            SelectionMode::LargestNode => Box::new(selection_mode::LargestNode),
            SelectionMode::Node | SelectionMode::SiblingNode => Box::new(selection_mode::Node),
            SelectionMode::Diagnostic => Box::new(selection_mode::Diagnostic),
            SelectionMode::GitHunk => Box::new(selection_mode::GitHunk::new(buffer)?),
        })
    }
}

#[derive(PartialEq, Clone, Debug, Eq, Hash, Default)]
pub struct Selection {
    pub range: Range<CharIndex>,
    pub copied_text: Option<Rope>,

    /// Used for extended selection.
    /// Some = the selection is being extended
    /// None = the selection is not being extended
    pub initial_range: Option<Range<CharIndex>>,

    /// For example, used for Diagnostic and Git Hunk
    pub info: Option<String>,
}
impl Selection {
    pub fn to_char_index(&self, cursor_direction: &CursorDirection) -> CharIndex {
        match cursor_direction {
            CursorDirection::Start => self.range.start,
            CursorDirection::End => (self.range.end - 1).max(self.range.start),
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

    pub fn is_start_or_end(&self, other: &CharIndex) -> bool {
        let Range { start, end } = self.extended_range();
        &start == other || (end > start && &(end - 1) == other)
    }

    #[cfg(test)]
    pub fn default() -> Selection {
        Selection {
            range: CharIndex(0)..CharIndex(0),
            copied_text: None,
            initial_range: None,
            info: None,
        }
    }

    pub fn get_selection_(
        buffer: &Buffer,
        current_selection: &Selection,
        mode: &SelectionMode,
        direction: &Direction,
        cursor_direction: &CursorDirection,
    ) -> anyhow::Result<Selection> {
        // NOTE: cursor_char_index should only be used where the Direction is Current
        let _cursor_char_index = {
            let index = current_selection.to_char_index(cursor_direction);
            match cursor_direction {
                CursorDirection::Start => index,
                // Minus one so that selecting line backward works
                CursorDirection::End => index - 1,
            }
        };
        let selection_mode = mode.to_selection_mode_trait_object(buffer, current_selection)?;

        let params = SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
        };
        Ok(match direction {
            Direction::Right => selection_mode.right(params)?,
            Direction::RightMost => selection_mode.right_most(params)?,
            Direction::Left => selection_mode.left(params)?,
            Direction::LeftMost => selection_mode.left_most(params)?,
            Direction::Up => selection_mode.up(params)?,
            Direction::Down => selection_mode.down(params)?,
            Direction::Current => selection_mode.current(params)?,
        }
        .unwrap_or_else(|| current_selection.clone()))
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

    fn clamp(&self, max_char_index: CharIndex) -> Self {
        let range = self.range.start.min(max_char_index)..self.range.end.min(max_char_index);
        Selection {
            range,
            copied_text: self.copied_text.clone(),
            initial_range: self.initial_range.clone(),
            info: self.info.clone(),
        }
    }
}

// TODO: this works, but the result is not satisfactory,
// we will leave this function here as a reference

impl Add<usize> for Selection {
    type Output = Selection;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            range: self.range.start + rhs..self.range.end + rhs,
            copied_text: self.copied_text,
            initial_range: self.initial_range,
            info: self.info,
        }
    }
}

impl Sub<usize> for Selection {
    type Output = Selection;

    fn sub(self, rhs: usize) -> Self::Output {
        Self {
            range: self.range.start - rhs..self.range.end - rhs,
            copied_text: self.copied_text,
            initial_range: self.initial_range,
            info: self.info,
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

#[derive(PartialEq, Clone, Debug, Copy, PartialOrd, Eq, Ord, Hash, Default)]
pub struct CharIndex(pub usize);

impl CharIndex {
    pub fn to_position(self, rope: &Rope) -> Position {
        let line = self.to_line(rope);
        Position {
            line,
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
            *self - change.unsigned_abs()
        }
    }
}

pub trait RangeCharIndex {
    fn to_usize_range(&self) -> Range<usize>;
}

impl RangeCharIndex for Range<CharIndex> {
    fn to_usize_range(&self) -> Range<usize> {
        self.start.0..self.end.0
    }
}
