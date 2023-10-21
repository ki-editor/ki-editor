use itertools::Itertools;
use lsp_types::DiagnosticSeverity;
use std::ops::{Add, Range, Sub};

use ropey::Rope;

use crate::{
    app::Dispatch,
    buffer::Buffer,
    char_index_range::CharIndexRange,
    components::{
        editor::{Direction, Movement},
        suggestive_editor::Info,
    },
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

    pub fn only(&mut self) {
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

    pub fn move_left(&mut self, cursor_direction: &Direction) {
        self.apply_mut(|selection| {
            let cursor_char_index = selection.to_char_index(cursor_direction);
            selection.range = (cursor_char_index - 1..cursor_char_index - 1).into()
        });
    }

    pub fn move_right(&mut self, cursor_direction: &Direction) {
        self.apply_mut(|selection| {
            let cursor_char_index = selection.to_char_index(cursor_direction);
            selection.range = (cursor_char_index + 1..cursor_char_index + 1).into()
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

    pub fn copy(&mut self, buffer: &Buffer, context: &Context) -> anyhow::Result<Vec<Dispatch>> {
        if self.secondary.is_empty() {
            // Copy the primary selected text to clipboard
            let copied_text = buffer.slice(&self.primary.extended_range())?;
            self.primary = Selection {
                range: self.primary.range,
                initial_range: None,
                // `copied_text` should be `None`, so that pasting between different files can work properly
                copied_text: None,
                info: None,
            };
            Ok([Dispatch::SetClipboardContent(copied_text.to_string())].to_vec())
        } else {
            // Otherwise, don't copy to clipboard, since there's multiple selection,
            // we don't know which one to copy.
            self.apply_mut(|selection| -> anyhow::Result<()> {
                selection.copied_text = Some(buffer.slice(&selection.extended_range())?)
                    .or_else(|| context.get_clipboard_content().map(Rope::from));
                selection.initial_range = None;
                Ok(())
            });
            Ok(Vec::new())
        }
    }

    pub fn select_kids(
        &self,
        buffer: &Buffer,
        cursor_direction: &Direction,
    ) -> anyhow::Result<SelectionSet> {
        fn select_kids(
            selection: &Selection,
            buffer: &Buffer,
            cursor_direction: &Direction,
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
                            range: (CharIndex(second_child.start_byte())
                                ..CharIndex(second_last_child.end_byte()))
                                .into(),
                            copied_text: selection.copied_text.clone(),
                            initial_range: selection.initial_range,
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
        direction: &Movement,
        cursor_direction: &Direction,
        context: &Context,
    ) -> anyhow::Result<SelectionSet> {
        self.apply(mode.clone(), |selection| {
            Selection::get_selection_(
                buffer,
                selection,
                mode,
                direction,
                cursor_direction,
                context,
            )
        })
    }

    pub fn add_selection(
        &mut self,
        buffer: &Buffer,
        direction: &Movement,
        cursor_direction: &Direction,
        context: &Context,
    ) -> anyhow::Result<()> {
        let last_selection = &self.primary;

        let next_selection = Selection::get_selection_(
            buffer,
            last_selection,
            &self.mode,
            direction,
            cursor_direction,
            context,
        )?;

        let matching_index =
            self.secondary.iter().enumerate().find(|(_, selection)| {
                selection.extended_range() == next_selection.extended_range()
            });
        let previous_primary = std::mem::replace(&mut self.primary, next_selection);

        if let Some((matching_index, _)) = matching_index {
            log::info!("Remove = {}", matching_index);
            self.secondary.remove(matching_index);
        }

        log::info!("Push");
        self.secondary.push(previous_primary);

        Ok(())
    }

    pub fn add_all(
        &mut self,
        buffer: &Buffer,
        cursor_direction: &Direction,
        context: &Context,
    ) -> anyhow::Result<()> {
        match self
            .map(|selection| {
                let object = self
                    .mode
                    .to_selection_mode_trait_object(buffer, selection, cursor_direction, context)
                    .ok()?;
                let iter = object
                    .iter(SelectionModeParams {
                        buffer,
                        current_selection: selection,
                        cursor_direction,
                        context,
                    })
                    .ok()?;
                Some(
                    iter.filter_map(|range| -> Option<Selection> {
                        range.to_selection(buffer, &self.primary).ok()
                    })
                    .collect_vec(),
                )
            })
            .into_iter()
            .flatten()
            .flatten()
            .unique_by(|selection| selection.extended_range())
            .collect_vec()
            .split_first()
        {
            Some((head, tail)) => {
                self.primary = head.to_owned();
                self.secondary = tail.to_vec();
            }
            None => {}
        };
        Ok(())
    }
    pub fn escape_highlight_mode(&mut self) {
        self.apply_mut(|selection| selection.escape_highlight_mode());
    }

    pub fn toggle_highlight_mode(&mut self) {
        self.apply_mut(|selection| selection.toggle_highlight_mode());
    }

    pub fn clamp(&self, max_char_index: CharIndex) -> anyhow::Result<SelectionSet> {
        self.apply(self.mode.clone(), |selection| {
            Ok(selection.clamp(max_char_index))
        })
    }

    pub fn delete_primary_cursor(&mut self) {
        let nearest = self
            .secondary
            .iter()
            .enumerate()
            .sorted_by_key(|(_, selection)| {
                ((self.primary.extended_range().start.0 as isize)
                    - (selection.extended_range().start.0 as isize))
                    .abs()
            })
            .next();
        if let Some((index, _)) = nearest {
            self.primary = self.secondary.remove(index);
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SelectionMode {
    // Regex
    EmptyLine,
    Word,
    Line,
    Character,
    Custom,
    Find { search: Search },

    // Syntax-tree
    BottomNode,
    TopNode,
    SyntaxTree,

    // LSP
    Diagnostic(Option<DiagnosticSeverity>),

    // Git
    GitHunk,

    // Local quickfix
    LocalQuickfix { title: String },

    // Bookmark
    Bookmark,
}
impl SelectionMode {
    pub fn similar_to(&self, other: &SelectionMode) -> bool {
        self == other || self.is_node() && other.is_node()
    }

    pub fn is_node(&self) -> bool {
        use SelectionMode::*;
        matches!(self, TopNode | SyntaxTree)
    }

    pub fn display(&self) -> String {
        match self {
            SelectionMode::Word => "WORD".to_string(),
            SelectionMode::EmptyLine => "EMPTY LINE".to_string(),
            SelectionMode::Line => "LINE".to_string(),
            SelectionMode::Character => "CHAR".to_string(),
            SelectionMode::Custom => "CUSTOM".to_string(),
            SelectionMode::BottomNode => "BOTTOM NODE".to_string(),
            SelectionMode::TopNode => "TOP NODE".to_string(),
            SelectionMode::SyntaxTree => "SYNTAX TREE".to_string(),
            SelectionMode::Find { search } => {
                format!("FIND {:?} {:?}", search.kind, search.search)
            }
            SelectionMode::Diagnostic(severity) => {
                let severity = severity
                    .map(|severity| format!("{:?}", severity))
                    .unwrap_or("ANY".to_string())
                    .to_uppercase();
                format!("DIAGNOSTIC:{}", severity)
            }
            SelectionMode::GitHunk => "GIT HUNK".to_string(),
            SelectionMode::Bookmark => "BOOKMARK".to_string(),
            SelectionMode::LocalQuickfix { title } => title.to_string(),
        }
    }

    pub fn to_selection_mode_trait_object(
        &self,
        buffer: &Buffer,
        current_selection: &Selection,
        cursor_direction: &Direction,
        context: &Context,
    ) -> anyhow::Result<Box<dyn selection_mode::SelectionMode>> {
        let params = SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
            context,
        };
        Ok(match self {
            SelectionMode::Word => Box::new(selection_mode::SmallWord::new(buffer)?),
            SelectionMode::Line => Box::new(selection_mode::Line),
            SelectionMode::Character => {
                let current_column = buffer
                    .char_to_position(current_selection.to_char_index(cursor_direction))?
                    .column;
                Box::new(selection_mode::Column::new(current_column))
            }
            SelectionMode::Custom => {
                Box::new(selection_mode::Custom::new(current_selection.clone()))
            }
            SelectionMode::Find { search } => match search.kind {
                SearchKind::Literal => Box::new(selection_mode::Regex::new(
                    buffer,
                    &search.search,
                    true,
                    false,
                )?),
                SearchKind::Regex => Box::new(selection_mode::Regex::new(
                    buffer,
                    &search.search,
                    false,
                    false,
                )?),
                SearchKind::AstGrep => {
                    Box::new(selection_mode::AstGrep::new(buffer, &search.search)?)
                }
                SearchKind::LiteralIgnoreCase => Box::new(selection_mode::Regex::new(
                    buffer,
                    &search.search,
                    true,
                    true,
                )?),
            },
            SelectionMode::BottomNode => Box::new(selection_mode::Token),
            SelectionMode::TopNode => Box::new(selection_mode::OutermostNode),
            SelectionMode::SyntaxTree => Box::new(selection_mode::SyntaxTree),
            SelectionMode::Diagnostic(severity) => {
                Box::new(selection_mode::Diagnostic::new(*severity, params))
            }
            SelectionMode::GitHunk => Box::new(selection_mode::GitHunk::new(buffer)?),
            SelectionMode::Bookmark => Box::new(selection_mode::Bookmark),
            SelectionMode::EmptyLine => {
                Box::new(selection_mode::Regex::regex(buffer, r"(?m)^\s*$")?)
            }
            SelectionMode::LocalQuickfix { .. } => {
                Box::new(selection_mode::LocalQuickfix::new(params))
            }
        })
    }
}

#[derive(PartialEq, Clone, Debug, Eq, Hash, Default)]
pub struct Selection {
    range: CharIndexRange,
    copied_text: Option<Rope>,

    /// Used for extended selection.
    /// Some = the selection is being extended
    /// None = the selection is not being extended
    initial_range: Option<CharIndexRange>,

    /// For example, used for Diagnostic and Git Hunk
    info: Option<Info>,
}

impl Selection {
    pub fn to_char_index(&self, cursor_direction: &Direction) -> CharIndex {
        match cursor_direction {
            Direction::Start => self.range.start,
            Direction::End => (self.range.end - 1).max(self.range.start),
        }
    }

    pub fn new(range: CharIndexRange) -> Self {
        Selection {
            range,
            ..Selection::default()
        }
    }

    pub fn set_copied_text(self, copied_text: Option<Rope>) -> Self {
        Selection {
            copied_text,
            ..self
        }
    }

    pub fn set_initial_range(self, initial_range: Option<CharIndexRange>) -> Self {
        Selection {
            initial_range,
            ..self
        }
    }

    pub fn set_info(self, info: Option<Info>) -> Self {
        Selection { info, ..self }
    }

    pub fn extended_range(&self) -> CharIndexRange {
        match &self.initial_range {
            None => self.range,
            Some(extended_selection_anchor) => {
                (self.range.start.min(extended_selection_anchor.start)
                    ..self.range.end.max(extended_selection_anchor.end))
                    .into()
            }
        }
    }

    pub fn is_start_or_end(&self, other: &CharIndex) -> bool {
        let CharIndexRange { start, end } = self.extended_range();
        &start == other || (end > start && &(end - 1) == other)
    }

    #[cfg(test)]
    pub fn default() -> Selection {
        Selection {
            range: (CharIndex(0)..CharIndex(0)).into(),
            copied_text: None,
            initial_range: None,
            info: None,
        }
    }

    pub fn get_selection_(
        buffer: &Buffer,
        current_selection: &Selection,
        mode: &SelectionMode,
        direction: &Movement,
        cursor_direction: &Direction,
        context: &Context,
    ) -> anyhow::Result<Selection> {
        // NOTE: cursor_char_index should only be used where the Direction is Current
        let _cursor_char_index = {
            let index = current_selection.to_char_index(cursor_direction);
            match cursor_direction {
                Direction::Start => index,
                // Minus one so that selecting line backward works
                Direction::End => index - 1,
            }
        };
        let selection_mode = mode.to_selection_mode_trait_object(
            buffer,
            current_selection,
            cursor_direction,
            context,
        )?;

        let params = SelectionModeParams {
            context,
            buffer,
            current_selection,
            cursor_direction,
        };

        Ok(selection_mode
            .apply_direction(params, *direction)?
            .unwrap_or_else(|| current_selection.clone()))
    }
    pub fn escape_highlight_mode(&mut self) {
        log::info!("escape highlight mode");
        self.initial_range = None
    }

    pub fn toggle_highlight_mode(&mut self) {
        match self.initial_range.take() {
            None => {
                self.initial_range = Some(self.range);
            }
            // If highlight mode is enabled, inverse the selection
            Some(initial_range) => {
                self.initial_range = Some(std::mem::replace(&mut self.range, initial_range));
            }
        }
    }

    fn clamp(&self, max_char_index: CharIndex) -> Self {
        let range =
            (self.range.start.min(max_char_index)..self.range.end.min(max_char_index)).into();
        Selection {
            range,
            copied_text: self.copied_text.clone(),
            initial_range: self.initial_range,
            info: self.info.clone(),
        }
    }

    pub fn copied_text(&self, context: &Context) -> Option<Rope> {
        self.copied_text
            .clone()
            .or_else(|| context.get_clipboard_content().map(Rope::from))
    }

    pub fn info(&self) -> Option<Info> {
        self.info.clone()
    }

    pub fn set_range(self, range: CharIndexRange) -> Selection {
        Selection { range, ..self }
    }

    /// WARNING: You should always use `extended_range` unless you know what you are doing
    pub fn range(&self) -> CharIndexRange {
        self.range
    }

    pub fn anchors(&self) -> Vec<CharIndexRange> {
        Vec::new()
            .into_iter()
            .chain([self.range])
            .chain(self.initial_range)
            .collect_vec()
    }
}

// TODO: this works, but the result is not satisfactory,
// we will leave this function here as a reference

impl Add<usize> for Selection {
    type Output = Selection;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            range: (self.range.start + rhs..self.range.end + rhs).into(),
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
            range: (self.range.start - rhs..self.range.end - rhs).into(),
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
    pub fn to_position(self, buffer: &Buffer) -> Position {
        let line = self.to_line(buffer).unwrap_or(0);
        Position {
            line,
            column: buffer
                .rope()
                .try_line_to_char(line)
                .map(|char_index| self.0.saturating_sub(char_index))
                .unwrap_or(0),
        }
    }

    pub fn to_line(self, buffer: &Buffer) -> anyhow::Result<usize> {
        Ok(buffer.rope().try_char_to_line(self.0)?)
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

impl RangeCharIndex for CharIndexRange {
    fn to_usize_range(&self) -> Range<usize> {
        self.start.0..self.end.0
    }
}
