use itertools::Itertools;
use nonempty::NonEmpty;
use std::ops::{Add, Sub};

use crate::{
    buffer::Buffer,
    char_index_range::CharIndexRange,
    components::{
        editor::{Direction, Movement},
        suggestive_editor::Info,
    },
    context::{LocalSearchConfigMode, Search},
    non_empty_extensions::{NonEmptyTryCollectOption, NonEmptyTryCollectResult},
    position::Position,
    quickfix_list::DiagnosticSeverityRange,
    selection_mode::{self, ApplyMovementResult, SelectionModeParams},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectionSet {
    /// 0 means the cursor is at the first selection
    cursor_index: usize,
    selections: NonEmpty<Selection>,
    pub(crate) mode: SelectionMode,
    /// TODO: filters should be stored globally, not at SelectionSet
    pub(crate) filters: Filters,
}

/// Filters is a stack.
/// Operations on filter:
/// 1. Push new filter
/// 2. Pop latest filter
/// 3. Clear all filters
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub(crate) struct Filters(Vec<Filter>);
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
pub(crate) struct Filter {
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
pub(crate) enum FilterTarget {
    Info,
    Content,
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub(crate) enum FilterKind {
    Keep,
    Remove,
}

#[derive(Clone, Debug)]
pub(crate) enum FilterMechanism {
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
            cursor_index: 0,
            selections: NonEmpty::singleton(Selection::default()),
            mode: SelectionMode::LineTrimmed,
            filters: Filters::default(),
        }
    }
}

impl SelectionSet {
    pub(crate) fn map<F, A>(&self, f: F) -> NonEmpty<A>
    where
        F: Fn(&Selection) -> A,
    {
        self.selections.clone().map(|selection| f(&selection))
    }

    pub(crate) fn map_with_index<F, A>(&self, f: F) -> NonEmpty<A>
    where
        F: Fn(usize, &Selection) -> A,
    {
        let mut index = 0;
        self.selections.clone().map(|selection| {
            let result = f(index, &selection);
            index += 1;
            result
        })
    }

    pub(crate) fn only(&mut self) {
        self.selections.head = self.primary_selection().clone().set_initial_range(None);
        self.selections.tail.clear();
        self.cursor_index = 0;
    }

    pub(crate) fn apply<F>(&self, mode: SelectionMode, f: F) -> anyhow::Result<SelectionSet>
    where
        F: Fn(&Selection) -> anyhow::Result<Selection>,
    {
        Ok(SelectionSet {
            cursor_index: self.cursor_index,
            selections: self
                .selections
                .clone()
                .map(|selection| f(&selection))
                .try_collect()?,
            mode,
            filters: self.filters.clone(),
        })
    }

    pub(crate) fn move_left(&mut self, cursor_direction: &Direction) {
        self.apply_mut(|selection| {
            let cursor_char_index = selection.to_char_index(cursor_direction);
            selection.range = (cursor_char_index - 1..cursor_char_index - 1).into()
        });
    }

    pub(crate) fn move_right(&mut self, cursor_direction: &Direction, len_chars: usize) {
        self.apply_mut(|selection| {
            let cursor_char_index = selection.to_char_index(cursor_direction);
            let next = (cursor_char_index + 1).min(CharIndex(len_chars));
            selection.range = (next..next).into()
        });
    }

    pub(crate) fn apply_mut<F, A>(&mut self, f: F) -> NonEmpty<A>
    where
        F: Fn(&mut Selection) -> A,
    {
        let head = f(&mut self.selections.head);
        let tail = self.selections.tail.iter_mut().map(f).collect_vec();

        NonEmpty { head, tail }
    }

    pub(crate) fn generate(
        &self,
        buffer: &Buffer,
        mode: &SelectionMode,
        direction: &Movement,
        cursor_direction: &Direction,
    ) -> anyhow::Result<Option<SelectionSet>> {
        let Some(selections) = self
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
            .try_collect()?
            .try_collect()
        else {
            return Ok(None);
        };

        Ok(Some(SelectionSet {
            cursor_index: self.cursor_index,
            mode: selections.head.mode.clone().unwrap_or_else(|| mode.clone()),
            selections: selections.clone().map(|selection| selection.selection),
            filters: self.filters.clone(),
        }))
    }

    pub(crate) fn add_selection(
        &mut self,
        buffer: &Buffer,
        direction: &Movement,
        cursor_direction: &Direction,
    ) -> anyhow::Result<()> {
        let last_selection = &self
            .selections
            .get(self.cursor_index)
            .unwrap_or(self.selections.last());

        if let Some(new_selection) = Selection::get_selection_(
            buffer,
            last_selection,
            &self.mode,
            direction,
            cursor_direction,
            &self.filters,
        )? {
            let new_selection = new_selection.selection;

            let new_selection_range = new_selection.extended_range();

            // Only add this selection if it is distinct from the existing selections
            if !self
                .selections
                .iter()
                .any(|selection| selection.extended_range() == new_selection.extended_range())
            {
                self.selections.push(new_selection);
            }

            let matching_index = self
                .selections
                .iter()
                .enumerate()
                .find(|(_, selection)| selection.extended_range() == new_selection_range);

            if let Some((matching_index, _)) = matching_index {
                self.cursor_index = matching_index
            }
        }

        Ok(())
    }

    pub(crate) fn add_all(
        &mut self,
        buffer: &Buffer,
        cursor_direction: &Direction,
    ) -> anyhow::Result<()> {
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
                        range.to_selection(buffer, &self.selections.head).ok()
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
            self.selections = NonEmpty {
                head: (*head).clone(),
                tail: tail.to_vec(),
            };
            self.cursor_index = 0;
        };
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn escape_highlight_mode(&mut self) {
        self.apply_mut(|selection| selection.escape_highlight_mode());
    }

    pub(crate) fn toggle_visual_mode(&mut self) {
        self.apply_mut(|selection| selection.toggle_visual_mode());
    }

    pub(crate) fn clamp(&self, max_char_index: CharIndex) -> anyhow::Result<SelectionSet> {
        self.apply(self.mode.clone(), |selection| {
            Ok(selection.clamp(max_char_index))
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.selections.len()
    }

    pub(crate) fn filter_push(self, filter: Filter) -> SelectionSet {
        let SelectionSet {
            selections,
            mode,
            filters,
            cursor_index,
        } = self;
        Self {
            cursor_index,
            selections,
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

    pub(crate) fn new(selections: NonEmpty<Selection>) -> Self {
        Self {
            selections,
            ..Default::default()
        }
    }

    pub(crate) fn primary_selection(&self) -> &Selection {
        if let Some(selection) = self.selections.get(self.cursor_index) {
            selection
        } else {
            #[cfg(test)]
            {
                unreachable!();
            }

            // In production code, we do not want to crash the app
            // Just carry on
            #[allow(unreachable_code)]
            self.selections.first()
        }
    }

    pub(crate) fn set_selections(self, selections: NonEmpty<Selection>) -> SelectionSet {
        Self { selections, ..self }
    }

    pub(crate) fn set_mode(self, mode: SelectionMode) -> SelectionSet {
        Self { mode, ..self }
    }

    pub(crate) fn secondary_selections(&self) -> Vec<&Selection> {
        self.selections
            .iter()
            .enumerate()
            .filter(|(index, _)| index != &self.cursor_index)
            .map(|(_, selection)| selection)
            .collect_vec()
    }

    pub(crate) fn apply_edit(self, edit: &crate::edit::Edit, max_char_index: CharIndex) -> Self {
        let NonEmpty { head, tail } = self.selections;
        let head = head
            .clone()
            .apply_edit(edit)
            .unwrap_or_else(|| head.clamp(max_char_index));

        let selections = NonEmpty {
            head,
            tail: tail
                .into_iter()
                .map(|selection| {
                    selection
                        .clone()
                        .apply_edit(edit)
                        .unwrap_or_else(|| selection.clamp(max_char_index))
                })
                .collect_vec(),
        };

        Self { selections, ..self }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum SelectionMode {
    // Regex
    EmptyLine,
    WordShort,
    WordLong,
    LineTrimmed,
    Column,
    Custom,
    Find { search: Search },

    // Syntax-tree
    Token,
    SyntaxNodeCoarse,
    SyntaxNodeFine,

    // LSP
    Diagnostic(DiagnosticSeverityRange),

    // Git
    GitHunk(crate::git::DiffMode),

    // Local quickfix
    LocalQuickfix { title: String },

    // Bookmark
    Bookmark,
    LineFull,
}
impl SelectionMode {
    pub(crate) fn is_node(&self) -> bool {
        use SelectionMode::*;
        matches!(self, SyntaxNodeCoarse | SyntaxNodeFine)
    }

    pub(crate) fn display(&self) -> String {
        match self {
            SelectionMode::WordShort => "WORD (SHORT)".to_string(),
            SelectionMode::WordLong => "WORD (LONG)".to_string(),
            SelectionMode::EmptyLine => "EMPTY LINE".to_string(),
            SelectionMode::LineTrimmed => "LINE (TRIMMED)".to_string(),
            SelectionMode::LineFull => "LINE (FULL)".to_string(),
            SelectionMode::Column => "COLUMN".to_string(),
            SelectionMode::Custom => "CUSTOM".to_string(),
            SelectionMode::Token => "TOKEN".to_string(),
            SelectionMode::SyntaxNodeCoarse => "SYNTAX NODE (COARSE)".to_string(),
            SelectionMode::SyntaxNodeFine => "SYNTAX NODE (FINE)".to_string(),
            SelectionMode::Find { search } => {
                format!("FIND {} {:?}", search.mode.display(), search.search)
            }
            SelectionMode::Diagnostic(severity) => {
                let severity = format!("{:?}", severity).to_uppercase();
                format!("DIAGNOSTIC:{}", severity)
            }
            SelectionMode::GitHunk(diff_mode) => {
                format!("GIT HUNK ({})", diff_mode.display()).to_string()
            }
            SelectionMode::Bookmark => "BOOKMARK".to_string(),
            SelectionMode::LocalQuickfix { title } => title.to_string(),
        }
    }

    pub(crate) fn to_selection_mode_trait_object(
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
            SelectionMode::Column => {
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
            SelectionMode::Token => Box::new(selection_mode::Token),
            SelectionMode::SyntaxNodeCoarse => {
                Box::new(selection_mode::SyntaxNode { coarse: true })
            }
            SelectionMode::SyntaxNodeFine => Box::new(selection_mode::SyntaxNode { coarse: false }),
            SelectionMode::Diagnostic(severity) => {
                Box::new(selection_mode::Diagnostic::new(*severity, params))
            }
            SelectionMode::GitHunk(diff_mode) => {
                Box::new(selection_mode::GitHunk::new(diff_mode, buffer)?)
            }
            SelectionMode::Bookmark => Box::new(selection_mode::Bookmark),
            SelectionMode::EmptyLine => Box::new(selection_mode::Regex::new(buffer, r"(?m)^\s*$")?),
            SelectionMode::LocalQuickfix { .. } => {
                Box::new(selection_mode::LocalQuickfix::new(params))
            }
        })
    }

    pub(crate) fn is_contiguous(&self) -> bool {
        matches!(
            self,
            SelectionMode::WordShort
                | SelectionMode::WordLong
                | SelectionMode::LineTrimmed
                | SelectionMode::LineFull
                | SelectionMode::Column
                | SelectionMode::Token
                | SelectionMode::SyntaxNodeCoarse
                | SelectionMode::SyntaxNodeFine
        )
    }
}

impl From<Selection> for ApplyMovementResult {
    fn from(val: Selection) -> Self {
        ApplyMovementResult::from_selection(val)
    }
}

#[derive(PartialEq, Clone, Debug, Eq, Hash, Default)]
pub(crate) struct Selection {
    range: CharIndexRange,

    /// Used for extended selection.
    /// Some = the selection is being extended
    /// None = the selection is not being extended
    initial_range: Option<CharIndexRange>,

    /// For example, used for Diagnostic and Git Hunk
    info: Option<Info>,
}

impl Selection {
    pub(crate) fn to_char_index(&self, cursor_direction: &Direction) -> CharIndex {
        match cursor_direction {
            Direction::Start => self.range.start,
            Direction::End => (self.range.end - 1).max(self.range.start),
        }
    }

    pub(crate) fn new(range: CharIndexRange) -> Self {
        Selection {
            range,
            ..Selection::default()
        }
    }

    pub(crate) fn set_initial_range(self, initial_range: Option<CharIndexRange>) -> Self {
        Selection {
            initial_range,
            ..self
        }
    }

    pub(crate) fn set_info(self, info: Option<Info>) -> Self {
        Selection { info, ..self }
    }

    pub(crate) fn extended_range(&self) -> CharIndexRange {
        match &self.initial_range {
            None => self.range,
            Some(extended_selection_anchor) => {
                (self.range.start.min(extended_selection_anchor.start)
                    ..self.range.end.max(extended_selection_anchor.end))
                    .into()
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn default() -> Selection {
        Selection {
            range: (CharIndex(0)..CharIndex(0)).into(),
            initial_range: None,
            info: None,
        }
    }

    pub(crate) fn get_selection_(
        buffer: &Buffer,
        current_selection: &Selection,
        mode: &SelectionMode,
        direction: &Movement,
        cursor_direction: &Direction,
        filters: &Filters,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
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

        selection_mode.apply_movement(params, *direction)
    }
    #[cfg(test)]
    pub(crate) fn escape_highlight_mode(&mut self) {
        log::info!("escape highlight mode");
        self.initial_range = None
    }

    pub(crate) fn toggle_visual_mode(&mut self) {
        match self.initial_range.take() {
            None => {
                self.enable_extension();
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
            initial_range: self.initial_range,
            info: self.info.clone(),
        }
    }

    pub(crate) fn info(&self) -> Option<Info> {
        self.info.clone()
    }

    pub(crate) fn set_range(self, range: CharIndexRange) -> Selection {
        Selection { range, ..self }
    }

    /// WARNING: You should always use `extended_range` unless you know what you are doing
    pub(crate) fn range(&self) -> CharIndexRange {
        self.range
    }

    pub(crate) fn anchors(&self) -> Vec<CharIndexRange> {
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

    fn enable_extension(&mut self) {
        self.initial_range = Some(self.range);
    }

    fn apply_edit(self, edit: &crate::edit::Edit) -> Option<Self> {
        let Self {
            range,
            info,
            initial_range,
        } = self;

        Some(Self {
            range: range.apply_edit(edit)?,
            initial_range: initial_range.and_then(|range| range.apply_edit(edit)),
            info,
        })
    }

    pub(crate) fn collapsed_to_anchor_range(self, direction: &Direction) -> Self {
        let range = if let Some(initial_range) = self.initial_range {
            let (start, end) = if initial_range.start < self.range.start {
                (initial_range, self.range)
            } else {
                (self.range, initial_range)
            };
            match direction {
                Direction::Start => start,
                Direction::End => end,
            }
        } else {
            self.range
        };
        self.set_range(range).set_initial_range(None)
    }
}

// TODO: this works, but the result is not satisfactory,
// we will leave this function here as a reference

impl Add<usize> for Selection {
    type Output = Selection;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            range: (self.range.start + rhs..self.range.end + rhs).into(),
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
pub(crate) struct CharIndex(pub usize);

impl CharIndex {
    pub(crate) fn to_position(self, buffer: &Buffer) -> Position {
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

    pub(crate) fn to_line(self, buffer: &Buffer) -> anyhow::Result<usize> {
        Ok(buffer.rope().try_char_to_line(self.0)?)
    }

    pub(crate) fn apply_offset(&self, change: isize) -> CharIndex {
        if change.is_positive() {
            *self + (change as usize)
        } else {
            *self - change.unsigned_abs()
        }
    }
}
