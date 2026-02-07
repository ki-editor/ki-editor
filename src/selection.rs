use itertools::Itertools;
use nonempty::NonEmpty;
use std::ops::{Add, Sub};

use crate::{
    alternator::Alternator,
    buffer::Buffer,
    char_index_range::CharIndexRange,
    components::{
        editor::{Direction, MovementApplicandum},
        suggestive_editor::Info,
    },
    context::{Context, LocalSearchConfigMode, Search},
    edit::ApplyOffset,
    non_empty_extensions::{NonEmptyTryCollectOption, NonEmptyTryCollectResult},
    position::Position,
    quickfix_list::{DiagnosticSeverityRange, QuickfixListItem},
    selection_mode::{self, ApplyMovementResult, IterBased, PositionBased, SelectionModeParams},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionSet {
    /// 0 means the cursor is at the first selection
    pub cursor_index: usize,
    pub selections: NonEmpty<Selection>,
    pub mode: Alternator<SelectionMode>,
    /// This will be set when a vertical movement is executed.
    /// Once set, its value will not changed.
    /// A non-vertical movement will reset its value to None.
    sticky_column_index: Option<usize>,
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self {
            cursor_index: 0,
            selections: NonEmpty::singleton(Selection::default()),
            mode: Alternator::new(SelectionMode::Line),
            sticky_column_index: None,
        }
    }
}

impl SelectionSet {
    pub fn map<F, A>(&self, f: F) -> NonEmpty<A>
    where
        F: Fn(&Selection) -> A,
    {
        self.selections.clone().map(|selection| f(&selection))
    }

    pub fn map_with_index<F, A>(&self, f: F) -> NonEmpty<A>
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

    pub fn only(&mut self) {
        self.selections.head = self.primary_selection().clone().set_initial_range(None);
        self.selections.tail.clear();
        self.cursor_index = 0;
    }

    pub fn apply<F>(&self, mode: Alternator<SelectionMode>, f: F) -> anyhow::Result<SelectionSet>
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
            sticky_column_index: self.sticky_column_index,
        })
    }

    pub fn move_left(&mut self, cursor_direction: &Direction) {
        self.apply_mut(|selection| {
            let cursor_char_index = selection.to_char_index(cursor_direction);
            selection.range = (cursor_char_index - 1..cursor_char_index - 1).into()
        });
    }

    pub fn move_right(&mut self, cursor_direction: &Direction, len_chars: usize) {
        self.apply_mut(|selection| {
            let cursor_char_index = selection.to_char_index(cursor_direction);
            let next = (cursor_char_index + 1).min(CharIndex(len_chars));
            selection.range = (next..next).into()
        });
    }

    pub fn apply_mut<F, A>(&mut self, f: F) -> NonEmpty<A>
    where
        F: Fn(&mut Selection) -> A,
    {
        let head = f(&mut self.selections.head);
        let tail = self.selections.tail.iter_mut().map(f).collect_vec();

        NonEmpty { head, tail }
    }

    pub fn generate(
        &self,
        buffer: &Buffer,
        mode: &SelectionMode,
        movement: &MovementApplicandum,
        cursor_direction: &Direction,
        context: &Context,
    ) -> anyhow::Result<Option<SelectionSet>> {
        let Some(selections) = self
            .map(|selection| {
                Selection::get_selection_(
                    buffer,
                    selection,
                    mode,
                    movement,
                    cursor_direction,
                    context,
                )
            })
            .try_collect()?
            .try_collect()
        else {
            return Ok(None);
        };

        Ok(Some(SelectionSet {
            cursor_index: self.cursor_index,
            selections: selections.clone().map(|selection| selection.selection),
            // The following is how `mode` and `sticky_column_index` got stored
            mode: self.mode.clone().replace_primary(mode.clone()),
            sticky_column_index: selections.head.sticky_column_index,
        }))
    }

    /// Returns `false` if no new selection is added
    pub fn add_selection(
        &mut self,
        buffer: &Buffer,
        movement: &MovementApplicandum,
        cursor_direction: &Direction,
        context: &Context,
    ) -> anyhow::Result<bool> {
        let initial_selections_length = self.selections.len();
        let last_selection = &self
            .selections
            .get(self.cursor_index)
            .unwrap_or(self.selections.last());

        if let Some(new_selection) = Selection::get_selection_(
            buffer,
            last_selection,
            self.mode.primary(),
            movement,
            cursor_direction,
            context,
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

        Ok(self.selections.len() > initial_selections_length)
    }

    pub fn mode(&self) -> &SelectionMode {
        self.mode.primary()
    }

    pub fn all_selections(
        &self,
        buffer: &Buffer,
        cursor_direction: &Direction,
        context: &Context,
    ) -> anyhow::Result<Option<NonEmpty<Selection>>> {
        if let Some((head, tail)) = self
            .map(|selection| {
                let object = self
                    .mode()
                    .to_selection_mode_trait_object(
                        buffer,
                        selection,
                        cursor_direction,
                        context.current_working_directory(),
                        context.quickfix_list_items(),
                        &context.get_marks(buffer.path()),
                    )
                    .ok()?;

                let iter = object
                    .all_selections(&SelectionModeParams {
                        buffer,
                        current_selection: selection,
                        cursor_direction,
                    })
                    .ok()?;
                let result = iter
                    .into_iter()
                    .filter_map(|range| -> Option<Selection> {
                        range.to_selection(buffer, &self.selections.head).ok()
                    })
                    .collect_vec();
                Some(result)
            })
            .into_iter()
            .flatten()
            .flatten()
            .map(|selection| selection.set_initial_range(None))
            .unique_by(|selection| selection.extended_range())
            .collect_vec()
            .split_first()
        {
            Ok(Some(NonEmpty {
                head: (*head).clone(),
                tail: tail.to_vec(),
            }))
        } else {
            Ok(None)
        }
    }
    pub fn add_all(
        &mut self,
        buffer: &Buffer,
        cursor_direction: &Direction,
        context: &Context,
    ) -> anyhow::Result<()> {
        if let Some(selections) = self.all_selections(buffer, cursor_direction, context)? {
            self.selections = selections;
            self.cursor_index = 0;
        };
        Ok(())
    }
    #[cfg(test)]
    pub fn escape_highlight_mode(&mut self) {
        self.apply_mut(|selection| selection.disable_extension());
    }

    pub fn enable_selection_extension(&mut self) {
        self.mode.copy_primary_to_secondary();

        self.apply_mut(|selection| selection.enable_selection_extension());
    }

    pub fn swap_anchor(&mut self) {
        self.mode.cycle();
        self.apply_mut(|selection| selection.swap_initial_range_direction());
    }

    pub fn clamp(&self, max_char_index: CharIndex) -> anyhow::Result<SelectionSet> {
        self.apply(self.mode.clone(), |selection| {
            Ok(selection.clamp(max_char_index))
        })
    }

    pub fn len(&self) -> usize {
        self.selections.len()
    }

    pub fn unset_initial_range(&mut self) {
        self.apply_mut(|selection| selection.initial_range = None);
    }

    pub fn new(selections: NonEmpty<Selection>) -> Self {
        Self {
            selections,
            ..Default::default()
        }
    }

    pub fn primary_selection(&self) -> &Selection {
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

    pub fn set_selections(self, selections: NonEmpty<Selection>) -> SelectionSet {
        Self {
            cursor_index: self.cursor_index.min(selections.len() - 1),
            selections,
            ..self
        }
    }

    pub fn set_mode(self, mode: SelectionMode) -> SelectionSet {
        Self {
            mode: self.mode.replace_primary(mode),
            ..self
        }
    }

    pub fn secondary_selections(&self) -> Vec<&Selection> {
        self.selections
            .iter()
            .enumerate()
            .filter(|(index, _)| index != &self.cursor_index)
            .map(|(_, selection)| selection)
            .collect_vec()
    }

    pub fn apply_edit(self, edit: &crate::edit::Edit, max_char_index: CharIndex) -> Self {
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

    pub fn cycle_primary_selection(&mut self, direction: Direction) {
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

    pub fn is_extended(&self) -> bool {
        self.primary_selection().initial_range.is_some()
    }

    pub fn selections(&self) -> &NonEmpty<Selection> {
        &self.selections
    }

    pub fn delete_current_selection(&mut self, direction: Direction) {
        let index = self.cursor_index;
        match self
            .selections
            .iter()
            .enumerate()
            .filter(|(index, _)| index != &self.cursor_index)
            .map(|(_, selection)| selection.clone())
            .collect_vec()
            .split_first()
        {
            Some((head, tail)) => {
                self.selections = NonEmpty {
                    head: head.clone(),
                    tail: tail.to_vec(),
                }
            }
            _ => return,
        }
        self.cursor_index = match direction {
            Direction::Start => index.saturating_sub(1),
            Direction::End => index.min(self.selections.len() - 1),
        }
    }

    pub fn sticky_column_index(&self) -> &Option<usize> {
        &self.sticky_column_index
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SelectionMode {
    // Regex
    Subword,
    Word,
    Line,
    Character,
    Custom,
    Find { search: Search },
    // Syntax-tree
    SyntaxNode,
    SyntaxNodeFine,

    // LSP
    Diagnostic(DiagnosticSeverityRange),

    // Git
    GitHunk(crate::git::DiffMode),

    // Local quickfix
    LocalQuickfix { title: String },

    // Mark
    Mark,
    LineFull,
    BigWord,
}
impl SelectionMode {
    pub fn is_node(&self) -> bool {
        use SelectionMode::*;
        matches!(self, SyntaxNode | SyntaxNodeFine)
    }

    pub fn display(&self) -> String {
        match self {
            SelectionMode::Line => "LINE".to_string(),
            SelectionMode::LineFull => "LINE*".to_string(),
            SelectionMode::Character => "CHAR".to_string(),
            SelectionMode::Custom => "CUSTM".to_string(),
            SelectionMode::SyntaxNode => "NODE".to_string(),
            SelectionMode::SyntaxNodeFine => "NODE*".to_string(),
            SelectionMode::Find { .. } => "FIND".to_string(),
            SelectionMode::Diagnostic(severity) => match severity {
                DiagnosticSeverityRange::All => "ALL",
                DiagnosticSeverityRange::Error => "ERROR",
                DiagnosticSeverityRange::Warning => "WARN",
                DiagnosticSeverityRange::Information => "INFO",
                DiagnosticSeverityRange::Hint => "HINT",
            }
            .to_string(),
            SelectionMode::GitHunk(diff_mode) => format!("HUNK{}", diff_mode.display()).to_string(),
            SelectionMode::Mark => "MARK".to_string(),
            SelectionMode::LocalQuickfix { title } => title.to_string(),
            SelectionMode::Subword => "SUBWORD".to_string(),
            SelectionMode::Word => "WORD".to_string(),
            SelectionMode::BigWord => "WORD*".to_string(),
        }
    }

    pub fn to_selection_mode_trait_object(
        &self,
        buffer: &Buffer,
        current_selection: &Selection,
        cursor_direction: &Direction,
        working_directory: &shared::canonicalized_path::CanonicalizedPath,
        quickfix_list_items: &[QuickfixListItem],
        marks: &[CharIndexRange],
    ) -> anyhow::Result<Box<dyn selection_mode::SelectionModeTrait>> {
        let params = SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
        };
        Ok(match self {
            SelectionMode::Subword => Box::new(PositionBased(selection_mode::Subword)),
            SelectionMode::Word => Box::new(PositionBased(selection_mode::Word)),
            SelectionMode::BigWord => Box::new(PositionBased(selection_mode::BigWord)),
            SelectionMode::Line => Box::new(PositionBased(selection_mode::LineTrimmed)),
            SelectionMode::LineFull => Box::new(PositionBased(selection_mode::LineFull)),
            SelectionMode::Character => Box::new(PositionBased(selection_mode::Character)),
            SelectionMode::Custom => Box::new(IterBased(selection_mode::Custom::new(
                current_selection.clone(),
            ))),
            SelectionMode::Find { search } => match search.mode {
                LocalSearchConfigMode::Regex(regex) => Box::new(IterBased(
                    selection_mode::Regex::from_config(buffer, &search.search, regex)?,
                )),
                LocalSearchConfigMode::AstGrep => Box::new(IterBased(
                    selection_mode::AstGrep::new(buffer, &search.search)?,
                )),
                LocalSearchConfigMode::NamingConventionAgnostic => Box::new(IterBased(
                    selection_mode::NamingConventionAgnostic::new(search.search.clone()),
                )),
            },
            SelectionMode::SyntaxNode => {
                Box::new(IterBased(selection_mode::SyntaxNode { coarse: true }))
            }
            SelectionMode::SyntaxNodeFine => {
                Box::new(IterBased(selection_mode::SyntaxNode { coarse: false }))
            }
            SelectionMode::Diagnostic(severity) => Box::new(IterBased(
                selection_mode::Diagnostic::new(*severity, params),
            )),
            SelectionMode::GitHunk(diff_mode) => Box::new(IterBased(selection_mode::GitHunk::new(
                diff_mode,
                buffer,
                working_directory,
            )?)),
            SelectionMode::Mark => Box::new(IterBased(selection_mode::Mark {
                marks: marks.iter().copied().collect_vec(),
            })),
            SelectionMode::LocalQuickfix { .. } => Box::new(IterBased(
                selection_mode::LocalQuickfix::new(params, quickfix_list_items),
            )),
        })
    }

    pub fn is_contiguous(&self) -> bool {
        matches!(
            self,
            SelectionMode::Subword
                | SelectionMode::Word
                | SelectionMode::BigWord
                | SelectionMode::Line
                | SelectionMode::LineFull
                | SelectionMode::Character
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
pub struct Selection {
    pub range: CharIndexRange,

    /// Used for extended selection.
    /// Some = the selection is being extended
    /// None = the selection is not being extended
    pub initial_range: Option<CharIndexRange>,

    /// For example, used for Diagnostic and Git Hunk
    pub info: Option<Info>,
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

    #[cfg(test)]
    pub fn default() -> Selection {
        Selection {
            range: (CharIndex(0)..CharIndex(0)).into(),
            initial_range: None,
            info: None,
        }
    }

    pub fn get_selection_(
        buffer: &Buffer,
        current_selection: &Selection,
        mode: &SelectionMode,
        movement: &MovementApplicandum,
        cursor_direction: &Direction,
        context: &Context,
    ) -> anyhow::Result<Option<ApplyMovementResult>> {
        let selection_mode = mode.to_selection_mode_trait_object(
            buffer,
            current_selection,
            cursor_direction,
            context.current_working_directory(),
            context.quickfix_list_items(),
            &context.get_marks(buffer.path()),
        )?;

        let params = SelectionModeParams {
            buffer,
            current_selection,
            cursor_direction,
        };

        selection_mode.apply_movement(&params, *movement)
    }
    #[cfg(test)]
    pub fn disable_extension(&mut self) {
        log::info!("escape highlight mode");
        self.initial_range = None
    }

    pub fn enable_selection_extension(&mut self) {
        if self.initial_range.is_none() {
            self.enable_extension();
        }
    }

    pub fn swap_initial_range_direction(&mut self) {
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

    pub fn info(&self) -> Option<Info> {
        self.info.clone()
    }

    pub fn set_range(self, range: CharIndexRange) -> Selection {
        Selection { range, ..self }
    }

    /// WARNING: You should always use `extended_range` unless you know what you are doing
    /// This always represent the non-extended range
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

    pub fn get_anchor(&self, cursor_direction: &Direction) -> CharIndex {
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

    pub fn collapsed_to_anchor_range(self, direction: &Direction) -> Self {
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

    pub fn update_with_byte_range(
        self,
        buffer: &Buffer,
        byte_range: selection_mode::ByteRange,
    ) -> anyhow::Result<Selection> {
        Ok(self
            .set_info(byte_range.info())
            .set_range(buffer.byte_range_to_char_index_range(byte_range.range())?))
    }

    pub fn apply_offset(self, offset: isize) -> Selection {
        let new_range = self.range.apply_offset(offset);
        let new_initial_range = self.initial_range.map(|range| range.apply_offset(offset));
        self.set_range(new_range)
            .set_initial_range(new_initial_range)
    }
}

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

#[derive(
    PartialEq,
    Clone,
    Debug,
    Copy,
    PartialOrd,
    Eq,
    Ord,
    Hash,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
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
