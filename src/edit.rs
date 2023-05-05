use std::ops::Range;

use itertools::Itertools;
use ropey::Rope;

use crate::selection::{CharIndex, Selection, SelectionSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Edit {
    pub start: CharIndex,
    pub old: Rope,
    pub new: Rope,
}
impl Edit {
    fn apply_offset(self, offset: isize) -> Edit {
        Edit {
            start: CharIndex((self.start.0 as isize + offset) as usize),
            old: self.old,
            new: self.new,
        }
    }

    pub fn end(&self) -> CharIndex {
        CharIndex(self.start.0 + self.old.len_chars())
    }

    fn range(&self) -> Range<CharIndex> {
        self.start..self.end()
    }
}

trait ApplyOffset {
    fn apply_offset(self, offset: isize) -> Self;
}

impl ApplyOffset for Range<CharIndex> {
    fn apply_offset(self, offset: isize) -> Self {
        Range {
            start: self.start.apply_offset(offset),
            end: self.end.apply_offset(offset),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Select(Selection),
    Edit(Edit),
}

impl Action {
    fn edit(start: usize, old: &str, new: &str) -> Self {
        Action::Edit(Edit {
            start: CharIndex(start),
            old: Rope::from_str(old),
            new: Rope::from_str(new),
        })
    }

    fn select(range: Range<usize>) -> Self {
        Action::Select(Selection {
            range: CharIndex(range.start)..CharIndex(range.end),
            ..Selection::default()
        })
    }

    fn subset_of(&self, other: &Action) -> bool {
        let self_range = self.range();
        let other_range = other.range();
        self_range.start >= other_range.start && self_range.end <= other_range.end
    }

    fn range(&self) -> Range<CharIndex> {
        match self {
            Action::Select(selection) => selection.range.clone(),
            Action::Edit(edit) => edit.range(),
        }
    }

    fn update_start_char_index(self, start: CharIndex) -> Self {
        match self {
            Action::Select(selection) => {
                let diff = start.0 as isize - selection.range.start.0 as isize;

                Action::Select(selection.apply_offset(diff))
            }
            Action::Edit(edit) => Action::Edit(Edit { start, ..edit }),
        }
    }

    fn apply_offset(self, offset: isize) -> Self {
        match self {
            Action::Select(selection) => Action::Select(Selection {
                range: selection.range.start.apply_offset(offset)
                    ..selection.range.end.apply_offset(offset),
                ..selection
            }),
            Action::Edit(edit) => Action::Edit(edit.apply_offset(offset)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EditTransaction {
    actions: Vec<Action>,
    /// TODO: `selection_set` should not belong here, but rather belong to the UndoableAction struct.
    /// Used for restoring previous selection after undo/redo
    pub selection_set: SelectionSet,
}

impl EditTransaction {
    fn apply_to(&self, mut rope: Rope) -> (Vec<String>, Rope) {
        for edit in &self.edits() {
            rope.remove(edit.start.0..edit.end().0);
            rope.insert(edit.start.0, edit.new.to_string().as_str());
        }
        let selections = self
            .selections()
            .into_iter()
            .map(|selection| {
                rope.slice(selection.range.start.0..selection.range.end.0)
                    .to_string()
            })
            .collect_vec();
        (selections, rope)
    }

    pub fn edits(&self) -> Vec<&Edit> {
        self.actions
            .iter()
            .filter_map(|action| match action {
                Action::Edit(edit) => Some(edit),
                _ => None,
            })
            .collect_vec()
    }

    pub fn from_actions(current_selection_set: SelectionSet, actions: Vec<Action>) -> Self {
        Self {
            actions: Self::normalize_actions(actions),
            selection_set: current_selection_set,
        }
    }

    #[cfg(test)]
    pub fn from_tuples(actions: Vec<Action>) -> Self {
        Self {
            selection_set: SelectionSet::default(),
            actions: Self::normalize_actions(actions),
        }
    }

    pub fn normalize_actions(actions: Vec<Action>) -> Vec<Action> {
        log::info!("actions = {:#?}", actions);
        // 1) remove edits that are subset of other edits
        let groups = actions
            .clone()
            .into_iter()
            .filter(|action| {
                !actions.iter().any(|other| {
                    action != other
                        && matches!(action, Action::Edit(_))
                        && matches!(other, Action::Edit(_))
                        && action.subset_of(other)
                })
            })
            // 2) sort actions by start position
            .sorted_by_key(|action| action.range().start)
            // 3) Remove duplicates
            .unique()
            // 4) Group the actions by start position
            .group_by(|action| action.range().start)
            .into_iter()
            .map(|(start, group)| ActionGroup {
                start,
                actions: group.collect_vec(),
            })
            .collect::<Vec<_>>();

        let mut offset: isize = 0;
        let mut result = vec![];

        for group in groups {
            let mut group = group.apply_offset(offset);
            offset += group.get_max_offset();

            result.append(&mut group.actions);
        }

        log::info!("result = {:#?}", result);

        result
    }

    pub fn min_char_index(&self) -> CharIndex {
        self.actions
            .iter()
            .map(|edit| edit.range().start)
            .min()
            .unwrap_or(CharIndex(0))
    }

    pub fn max_char_index(&self) -> CharIndex {
        self.actions
            .iter()
            .map(|edit| edit.range().end)
            .max()
            .unwrap_or(CharIndex(0))
    }

    pub fn merge(
        selection_set: SelectionSet,
        edit_transactions: Vec<EditTransaction>,
    ) -> EditTransaction {
        EditTransaction::from_actions(
            selection_set,
            edit_transactions
                .into_iter()
                .flat_map(|edit_transaction| edit_transaction.actions)
                .collect(),
        )
    }

    pub fn selections(&self) -> Vec<&Selection> {
        self.actions
            .iter()
            .filter_map(|action| match action {
                Action::Select(selection) => Some(selection),
                _ => None,
            })
            .collect_vec()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ActionGroup {
    start: CharIndex,
    actions: Vec<Action>,
}

impl ActionGroup {
    fn get_max_offset(&self) -> isize {
        self.actions
            .iter()
            .filter_map(|action| match action {
                Action::Edit(edit) => {
                    Some(edit.new.len_chars() as isize - edit.old.len_chars() as isize)
                }
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }

    fn apply_offset(self, offset: isize) -> ActionGroup {
        ActionGroup {
            start: self.start.apply_offset(offset),
            actions: self
                .actions
                .into_iter()
                .map(|action| action.apply_offset(offset))
                .collect(),
        }
    }
}

#[cfg(test)]
mod test_normalize_actions {
    use ropey::Rope;

    use crate::{
        edit::{Action, EditTransaction},
        selection::Selection,
    };

    #[test]
    fn only_one_edit() {
        let edit_transaction = EditTransaction::from_tuples(vec![Action::edit(0, "Who", "What")]);
        let (_, result) = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));
        assert_eq!(result, Rope::from_str("What lives in a pineapple"));
    }

    #[test]
    fn selection_offsetted_positively() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            Action::edit(0, "Who", "What"),
            // Select the word pineapple
            Action::select(15..24),
        ]);

        let (selections, result) =
            edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What lives in a pineapple"));

        assert_eq!(selections, vec!["pineapple".to_string()]);
    }

    #[test]
    fn selection_offsetted_negatively() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            Action::edit(0, "Who", "He"),
            // Select the word "pineapple"
            Action::select(15..24),
        ]);

        let (selections, result) =
            edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("He lives in a pineapple"));

        assert_eq!(selections, vec!["pineapple".to_string()]);
    }

    #[test]
    fn selection_should_not_offset_others() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            // Select the wrod "Who"
            Action::select(0..3),
            // Select the word "pineapple"
            Action::select(15..24),
        ]);

        let (selections, result) =
            edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("Who lives in a pineapple"));
        assert_eq!(selections, vec!["Who".to_string(), "pineapple".to_string()])
    }

    #[test]
    fn no_intersection() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            // Replacement length > range length
            Action::edit(0, "Who", "What"),
            // Replacement length < range length
            Action::edit(4, "lives", "see"),
            Action::edit(13, "a", "two"),
        ]);

        let (_, result) = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What see in two pineapple"));
    }

    #[test]
    /// Expects the first edit is removed because it is a subset of the second edit.
    fn some_is_subset_of_other() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            Action::edit(0, "Who", "What"),
            Action::edit(0, "Who lives", "He"),
            Action::edit(13, "a", "two"),
        ]);

        let (_, result) = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("He in two pineapple"));
    }

    #[test]
    /// Expect the edits is not removed.
    fn some_edit_is_subset_of_some_selection() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            Action::edit(0, "Who", "What"),
            Action::select(0..6),
        ]);

        let (selections, result) =
            edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What lives in a pineapple"));
        assert_eq!(selections, vec!["What l".to_string()]);
    }

    #[test]
    /// Expects ranges with the same start to not be offsetted
    fn next_range_start_and_current_range_start_is_equal() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            Action::edit(4, "Jump", "Vec<Jump>"),
            Action::edit(0, "Vec<Jump>", "Jump"),
            Action::select(0..4),
        ]);

        let (selections, result) = edit_transaction.apply_to(Rope::from_str("Vec<Jump>"));
        assert_eq!(result, Rope::from_str("Jump"));
        assert_eq!(selections, vec!["Jump".to_string()])
    }

    #[test]
    /// Expect the edits to be sorted before being applied
    fn unsorted() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            Action::edit(13, "a", "two"),
            Action::edit(0, "Who", "What"),
            Action::edit(4, "lives", "see"),
        ]);

        let (_, result) = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What see in two pineapple"));
    }

    #[test]
    /// Expect duplicate edits to be removed
    fn duplicated() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            Action::edit(0, "Who", "What"),
            Action::edit(0, "Who", "What"),
            Action::edit(0, "Who", "What"),
            Action::edit(4, "lives", "see"),
            Action::edit(4, "lives", "see"),
            Action::edit(13, "a", "two"),
        ]);

        let (_, result) = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What see in two pineapple"));
    }

    #[test]
    /// Intersected edits
    /// TODO: I'm not sure what behavior should be expected here
    fn some_intersected() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            Action::edit(0, "Who", "What"),
            Action::edit(4, "lives", "see"),
            Action::edit(6, "ves ", "soap"),
        ]);

        let (_, result) = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        // The expected result here can be tweaked until we have finalized the behavior
        assert_eq!(result, Rope::from_str("What soapin a pineapple"));
    }
}
