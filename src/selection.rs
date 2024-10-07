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
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self {
            cursor_index: 0,
            selections: NonEmpty::singleton(Selection::default()),
            mode: SelectionMode::Line,
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
                Selection::get_selection_(buffer, selection, mode, direction, cursor_direction)
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
                    .to_selection_mode_trait_object(buffer, selection, cursor_direction)
                    .ok()?;

                let iter = object
                    .iter_filtered(SelectionModeParams {
                        buffer,
                        current_selection: selection,
                        cursor_direction,
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
        self.apply_mut(|selection| selection.disable_extension());
    }

    pub(crate) fn enable_selection_extension(&mut self) {
        self.apply_mut(|selection| selection.enable_selection_extension());
    }

    pub(crate) fn swap_initial_range_direction(&mut self) {
        self.apply_mut(|selection| selection.swap_initial_range_direction());
    }

    pub(crate) fn clamp(&self, max_char_index: CharIndex) -> anyhow::Result<SelectionSet> {
        self.apply(self.mode.clone(), |selection| {
            Ok(selection.clamp(max_char_index))
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.selections.len()
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

    pub(crate) fn cycle_primary_selection(&mut self, direction: Direction) {
        let sorted_ranges = self
            .selections
            .iter()
            .map(|selection| selection.extended_range())
            .sorted()
            .collect_vec();
        let primary_range = self.primary_selection().extended_range();
        if let Some(primary_index) = sorted_ranges
            .iter()
            .position(|range| range == &primary_range)
        {
            let last_index = sorted_ranges.len().saturating_sub(1);
            let next_primary_selection_index = match direction {
                Direction::Start => {
                    if primary_index == 0 {
                        last_index
                    } else {
                        primary_index.saturating_sub(1)
                    }
                }
                Direction::End => {
                    if primary_index == last_index {
                        0
                    } else {
                        primary_index + 1
                    }
                }
            };
            let next_range = sorted_ranges[next_primary_selection_index];
            if let Some(index) = self
                .selections
                .iter()
                .position(|selection| selection.extended_range() == next_range)
            {
                self.cursor_index = index;
            }
        }
    }

    pub(crate) fn is_extended(&self) -> bool {
        self.primary_selection().initial_range.is_some()
    }

    pub(crate) fn selections(&self) -> &NonEmpty<Selection> {
        &self.selections
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum SelectionMode {
    // Regex
    EmptyLine,
    SubWord,
    Word,
    Line,
    Column,
    Custom,
    Find {
        search: Search,
    },
    Till {
        character: char,
        direction: Direction,
    },

    // Syntax-tree
    #[cfg(test)]
    Token,
    SyntaxNode,
    SyntaxNodeFine,

    // LSP
    Diagnostic(DiagnosticSeverityRange),

    // Git
    GitHunk(crate::git::DiffMode),

    // Local quickfix
    LocalQuickfix {
        title: String,
    },

    // Mark
    Mark,
    LineFull,
}
impl SelectionMode {
    pub(crate) fn is_node(&self) -> bool {
        use SelectionMode::*;
        matches!(self, SyntaxNode | SyntaxNodeFine)
    }

    pub(crate) fn display(&self) -> String {
        match self {
            SelectionMode::SubWord => "SUB WORD".to_string(),
            SelectionMode::Word => "WORD".to_string(),
            SelectionMode::EmptyLine => "EMPTY LINE".to_string(),
            SelectionMode::Line => "LINE".to_string(),
            SelectionMode::LineFull => "FULL LINE".to_string(),
            SelectionMode::Column => "COLUMN".to_string(),
            SelectionMode::Custom => "CUSTOM".to_string(),
            #[cfg(test)]
            SelectionMode::Token => "TOKEN".to_string(),
            SelectionMode::SyntaxNode => "SYNTAX NODE".to_string(),
            SelectionMode::SyntaxNodeFine => "FINE SYNTAX NODE".to_string(),
            SelectionMode::Find { search } => {
                format!("{} {:?}", search.mode.display(), search.search)
            }
            SelectionMode::Diagnostic(severity) => {
                let severity = format!("{:?}", severity).to_uppercase();
                format!("DIAGNOSTIC:{}", severity)
            }
            SelectionMode::GitHunk(diff_mode) => {
                format!("GIT HUNK ({})", diff_mode.display()).to_string()
            }
            SelectionMode::Mark => "MARK".to_string(),
            SelectionMode::LocalQuickfix { title } => title.to_string(),
            SelectionMode::Till {
                character,
                direction,
            } => format!("{} {character:?}", direction.format_action("TILL")),
        }
    }

    pub(crate) fn to_selection_mode_trait_object(
        &self,
        buffer: &Buffer,
        current_selection: &Selection,
        cursor_direction: &Direction,
    ) -> anyhow::Result<Box<dyn selection_mode::SelectionMode>> {
        let params = SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
        };
        Ok(match self {
            SelectionMode::SubWord => Box::new(selection_mode::WordShort::as_regex(buffer)?),
            SelectionMode::Word => Box::new(selection_mode::WordLong::as_regex(buffer)?),
            SelectionMode::Line => Box::new(selection_mode::LineTrimmed),
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
            #[cfg(test)]
            SelectionMode::Token => Box::new(selection_mode::Token),
            SelectionMode::SyntaxNode => Box::new(selection_mode::SyntaxNode { coarse: true }),
            SelectionMode::SyntaxNodeFine => Box::new(selection_mode::SyntaxNode { coarse: false }),
            SelectionMode::Diagnostic(severity) => {
                Box::new(selection_mode::Diagnostic::new(*severity, params))
            }
            SelectionMode::GitHunk(diff_mode) => {
                Box::new(selection_mode::GitHunk::new(diff_mode, buffer)?)
            }
            SelectionMode::Mark => Box::new(selection_mode::Mark),
            SelectionMode::EmptyLine => Box::new(selection_mode::Regex::new(buffer, r"(?m)^\s*$")?),
            SelectionMode::LocalQuickfix { .. } => {
                Box::new(selection_mode::LocalQuickfix::new(params))
            }
            SelectionMode::Till {
                character,
                direction,
            } => Box::new(selection_mode::Till::from_config(
                buffer,
                *character,
                direction.clone(),
            )),
        })
    }

    pub(crate) fn is_contiguous(&self) -> bool {
        #[cfg(test)]
        if matches!(self, SelectionMode::Token) {
            return true;
        }
        matches!(
            self,
            SelectionMode::SubWord
                | SelectionMode::Word
                | SelectionMode::Line
                | SelectionMode::LineFull
                | SelectionMode::Column
                | SelectionMode::SyntaxNode
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
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        let selection_mode =
            mode.to_selection_mode_trait_object(buffer, current_selection, cursor_direction)?;

        let params = SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
        };

        selection_mode.apply_movement(params, *direction)
    }
    #[cfg(test)]
    pub(crate) fn disable_extension(&mut self) {
        log::info!("escape highlight mode");
        self.initial_range = None
    }

    pub(crate) fn enable_selection_extension(&mut self) {
        if self.initial_range.is_none() {
            self.enable_extension();
        }
    }

    pub(crate) fn swap_initial_range_direction(&mut self) {
        if let Some(initial_range) = self.initial_range.take() {
            self.initial_range = Some(std::mem::replace(&mut self.range, initial_range));
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
