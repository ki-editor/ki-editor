use itertools::Itertools;
use std::ops::{Add, Range, Sub};

use ropey::Rope;

use crate::{
    app::{Dispatch, Dispatches},
    buffer::Buffer,
    char_index_range::CharIndexRange,
    components::{
        editor::{Direction, Movement},
        suggestive_editor::Info,
    },
    context::{Context, LocalSearchConfigMode, Search},
    position::Position,
    quickfix_list::DiagnosticSeverityRange,
    selection_mode::{self, inside::InsideKind, ApplyMovementResult, SelectionModeParams},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionSet {
    pub primary: Selection,
    pub secondary: Vec<Selection>,
    pub mode: SelectionMode,
    /// TODO: filters should be stored globally, not at SelectionSet
    pub filters: Filters,
}

/// Filters is a stack.
/// Operations on filter:
/// 1. Push new filter
/// 2. Pop latest filter
/// 3. Clear all filters
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Filters(Vec<Filter>);
impl Filters {
    /// Returns `Some(item)` if it satisfy this `Filters`.
    pub(crate) fn retain(
        &self,
        buffer: &Buffer,
        item: selection_mode::ByteRange,
    ) -> Option<selection_mode::ByteRange> {
        self.0
            .iter()
            .try_fold(item, |item, filter| filter.retain(buffer, item))
    }

    fn push(self, filter: Filter) -> Filters {
        let mut result = self.0;
        result.push(filter);
        Filters(result)
    }

    pub(crate) fn display(&self) -> Option<String> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.iter().map(|filter| filter.display()).join(", "))
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filter {
    kind: FilterKind,
    mechanism: FilterMechanism,
    target: FilterTarget,
}
impl Filter {
    pub(crate) fn retain(
        &self,
        buffer: &Buffer,
        item: selection_mode::ByteRange,
    ) -> Option<selection_mode::ByteRange> {
        let target = match self.target {
            FilterTarget::Content => buffer
                .slice(&buffer.byte_range_to_char_index_range(item.range()).ok()?)
                .ok()
                .map(|rope| rope.to_string()),
            FilterTarget::Info => item.info().as_ref().map(|info| info.content().clone()),
        }?;
        let matched: bool = match &self.mechanism {
            FilterMechanism::Literal(literal) => {
                target.to_lowercase().contains(&literal.to_lowercase())
            }
            FilterMechanism::Regex(regex) => regex.is_match(&target),
        };
        match self.kind {
            FilterKind::Keep => matched,
            FilterKind::Remove => !matched,
        }
        .then_some(item)
    }

    pub(crate) fn new(kind: FilterKind, target: FilterTarget, mechanism: FilterMechanism) -> Self {
        Self {
            kind,
            target,
            mechanism,
        }
    }

    fn display(&self) -> String {
        let target = format!("{:?}", self.target);
        let kind = match self.kind {
            FilterKind::Keep => "⊇",
            FilterKind::Remove => "⊈",
        };
        let mechanism = match &self.mechanism {
            FilterMechanism::Literal(literal) => format!("\"{}\"", literal),
            FilterMechanism::Regex(regex) => format!("/{}/", regex),
        };
        format!("{}{}{}", target, kind, mechanism)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum FilterTarget {
    Info,
    Content,
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum FilterKind {
    Keep,
    Remove,
}

#[derive(Clone, Debug)]
pub enum FilterMechanism {
    Literal(String),
    Regex(regex::Regex),
    // AstGrep(ast_grep_core::Pattern),
    // TreeSitterKind(String),
}

impl PartialEq for FilterMechanism {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FilterMechanism::Literal(x), FilterMechanism::Literal(y)) => x == y,
            (FilterMechanism::Regex(a), FilterMechanism::Regex(b)) => {
                a.to_string() == b.to_string()
            }
            _ => false,
        }
    }
}

impl Eq for FilterMechanism {}

impl Default for SelectionSet {
    fn default() -> Self {
        Self {
            primary: Selection::default(),
            secondary: vec![],
            mode: SelectionMode::LineTrimmed,
            filters: Filters::default(),
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
        self.primary.copied_text = None;
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
            filters: self.filters.clone(),
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

    pub fn copy(&mut self, buffer: &Buffer, context: &Context) -> anyhow::Result<Dispatches> {
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
            Ok([Dispatch::SetClipboardContent(copied_text.to_string())]
                .to_vec()
                .into())
        } else {
            // Otherwise, don't copy to clipboard, since there's multiple selection,
            // we don't know which one to copy.
            self.apply_mut(|selection| -> anyhow::Result<()> {
                selection.copied_text = Some(buffer.slice(&selection.extended_range())?)
                    .or_else(|| context.get_clipboard_content().map(Rope::from));
                selection.initial_range = None;
                Ok(())
            });
            Ok(Vec::new().into())
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
    ) -> anyhow::Result<SelectionSet> {
        let result = self
            .map(|selection| {
                Selection::get_selection_(
                    buffer,
                    selection,
                    mode,
                    direction,
                    cursor_direction,
                    &self.filters,
                )
            })
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let (
            ApplyMovementResult {
                selection,
                mode: new_mode,
            },
            tail,
        ) = result
            .split_first()
            .expect("We should refactor `SelectionSet::map` to return NonEmpty instead of Vec.");
        Ok(SelectionSet {
            primary: selection.to_owned(),
            secondary: tail.iter().map(|it| it.selection.to_owned()).collect(),
            mode: new_mode.clone().unwrap_or_else(|| mode.clone()),
            filters: self.filters.clone(),
        })
    }

    pub fn add_selection(
        &mut self,
        buffer: &Buffer,
        direction: &Movement,
        cursor_direction: &Direction,
    ) -> anyhow::Result<()> {
        let last_selection = &self.primary;

        let next_selection = Selection::get_selection_(
            buffer,
            last_selection,
            &self.mode,
            direction,
            cursor_direction,
            &self.filters,
        )?
        .selection;

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

    pub fn add_all(&mut self, buffer: &Buffer, cursor_direction: &Direction) -> anyhow::Result<()> {
        if let Some((head, tail)) = self
            .map(|selection| {
                let object = self
                    .mode
                    .to_selection_mode_trait_object(
                        buffer,
                        selection,
                        cursor_direction,
                        &self.filters,
                    )
                    .ok()?;

                let iter = object
                    .iter_filtered(SelectionModeParams {
                        buffer,
                        current_selection: selection,
                        cursor_direction,
                        filters: &self.filters,
                    })
                    .ok()?;
                let result = iter
                    .filter_map(|range| -> Option<Selection> {
                        range.to_selection(buffer, &self.primary).ok()
                    })
                    .collect_vec();
                Some(result)
            })
            .into_iter()
            .flatten()
            .flatten()
            .unique_by(|selection| selection.extended_range())
            .collect_vec()
            .split_first()
        {
            self.primary = head.to_owned();
            self.secondary = tail.to_vec();
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

    pub(crate) fn len(&self) -> usize {
        self.secondary.len() + 1
    }

    pub(crate) fn filter_push(self, filter: Filter) -> SelectionSet {
        let SelectionSet {
            primary,
            secondary,
            mode,
            filters,
        } = self;
        Self {
            primary,
            secondary,
            mode,
            filters: filters.push(filter),
        }
    }

    pub(crate) fn filter_clear(self) -> Self {
        Self {
            filters: Filters::default(),
            ..self
        }
    }

    pub(crate) fn unset_initial_range(&mut self) {
        self.apply_mut(|selection| selection.initial_range = None);
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SelectionMode {
    // Regex
    EmptyLine,
    WordShort,
    WordLong,
    LineTrimmed,
    Character,
    Custom,
    Find { search: Search },

    // Syntax-tree
    BottomNode,
    SyntaxTree,

    // LSP
    Diagnostic(DiagnosticSeverityRange),

    // Git
    GitHunk,

    // Local quickfix
    LocalQuickfix { title: String },

    // Bookmark
    Bookmark,
    Inside(InsideKind),
    TopNode,
    LineFull,
}
impl SelectionMode {
    pub fn similar_to(&self, other: &SelectionMode) -> bool {
        self == other || self.is_node() && other.is_node()
    }

    pub fn is_node(&self) -> bool {
        use SelectionMode::*;
        matches!(self, SyntaxTree)
    }

    pub fn display(&self) -> String {
        match self {
            SelectionMode::WordShort => "WORD(SHORT)".to_string(),
            SelectionMode::WordLong => "WORD(LONG)".to_string(),
            SelectionMode::EmptyLine => "EMPTY LINE".to_string(),
            SelectionMode::LineTrimmed => "LINE(TRIMMED)".to_string(),
            SelectionMode::LineFull => "LINE(FULL)".to_string(),
            SelectionMode::Character => "CHAR".to_string(),
            SelectionMode::Custom => "CUSTOM".to_string(),
            SelectionMode::BottomNode => "BOTTOM NODE".to_string(),
            SelectionMode::SyntaxTree => "SYNTAX TREE".to_string(),
            SelectionMode::Find { search } => {
                format!("FIND {} {:?}", search.mode.display(), search.search)
            }
            SelectionMode::Diagnostic(severity) => {
                let severity = format!("{:?}", severity).to_uppercase();
                format!("DIAGNOSTIC:{}", severity)
            }
            SelectionMode::GitHunk => "GIT HUNK".to_string(),
            SelectionMode::Bookmark => "BOOKMARK".to_string(),
            SelectionMode::LocalQuickfix { title } => title.to_string(),
            SelectionMode::Inside(kind) => format!("INSIDE {}", kind),
            SelectionMode::TopNode => "TOP NODE".to_string(),
        }
    }

    pub fn to_selection_mode_trait_object(
        &self,
        buffer: &Buffer,
        current_selection: &Selection,
        cursor_direction: &Direction,
        filters: &Filters,
    ) -> anyhow::Result<Box<dyn selection_mode::SelectionMode>> {
        let params = SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
            filters,
        };
        Ok(match self {
            SelectionMode::WordShort => Box::new(selection_mode::WordShort::as_regex(buffer)?),
            SelectionMode::WordLong => Box::new(selection_mode::WordLong::as_regex(buffer)?),
            SelectionMode::LineTrimmed => Box::new(selection_mode::LineTrimmed),
            SelectionMode::LineFull => Box::new(selection_mode::LineFull),
            SelectionMode::Character => {
                let current_column = buffer
                    .char_to_position(current_selection.to_char_index(cursor_direction))?
                    .column;
                Box::new(selection_mode::Column::new(current_column))
            }
            SelectionMode::Custom => {
                Box::new(selection_mode::Custom::new(current_selection.clone()))
            }
            SelectionMode::Find { search } => match search.mode {
                LocalSearchConfigMode::Regex(regex) => Box::new(
                    selection_mode::Regex::from_config(buffer, &search.search, regex)?,
                ),
                LocalSearchConfigMode::AstGrep => {
                    Box::new(selection_mode::AstGrep::new(buffer, &search.search)?)
                }
                LocalSearchConfigMode::CaseAgnostic => {
                    Box::new(selection_mode::CaseAgnostic::new(search.search.clone()))
                }
            },
            SelectionMode::BottomNode => Box::new(selection_mode::BottomNode),
            SelectionMode::TopNode => Box::new(selection_mode::TopNode),
            SelectionMode::SyntaxTree => Box::new(selection_mode::SyntaxTree),
            SelectionMode::Diagnostic(severity) => {
                Box::new(selection_mode::Diagnostic::new(*severity, params))
            }
            SelectionMode::GitHunk => Box::new(selection_mode::GitHunk::new(buffer)?),
            SelectionMode::Bookmark => Box::new(selection_mode::Bookmark),
            SelectionMode::EmptyLine => Box::new(selection_mode::Regex::new(buffer, r"(?m)^\s*$")?),
            SelectionMode::LocalQuickfix { .. } => {
                Box::new(selection_mode::LocalQuickfix::new(params))
            }
            SelectionMode::Inside(kind) => Box::new(selection_mode::Inside::new(kind.clone())),
        })
    }

    pub(crate) fn is_contiguous(&self) -> bool {
        matches!(
            self,
            SelectionMode::WordShort
                | SelectionMode::WordLong
                | SelectionMode::LineTrimmed
                | SelectionMode::LineFull
                | SelectionMode::Character
                | SelectionMode::BottomNode
                | SelectionMode::TopNode
                | SelectionMode::SyntaxTree
        )
    }
}

impl From<Selection> for ApplyMovementResult {
    fn from(val: Selection) -> Self {
        ApplyMovementResult::from_selection(val)
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
        filters: &Filters,
    ) -> anyhow::Result<ApplyMovementResult> {
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
            filters,
        )?;

        let params = SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
            filters,
        };

        Ok(selection_mode
            .apply_movement(params, *direction)?
            .unwrap_or_else(|| ApplyMovementResult::from_selection(current_selection.clone())))
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

    pub(crate) fn get_anchor(&self, cursor_direction: &Direction) -> CharIndex {
        match cursor_direction {
            Direction::Start => self.extended_range().start,
            Direction::End => self.extended_range().end,
        }
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
