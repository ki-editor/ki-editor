use std::ops::Range;

use itertools::Itertools;
use ropey::Rope;

use crate::selection::{CharIndex, Selection};

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
    #[cfg(test)]
    fn edit(start: usize, old: &str, new: &str) -> Self {
        Action::Edit(Edit {
            start: CharIndex(start),
            old: Rope::from_str(old),
            new: Rope::from_str(new),
        })
    }

    #[cfg(test)]
    fn select(range: Range<usize>) -> Self {
        Action::Select(Selection {
            range: CharIndex(range.start)..CharIndex(range.end),
            ..Selection::default()
        })
    }

    fn range(&self) -> Range<CharIndex> {
        match self {
            Action::Select(selection) => selection.range.clone(),
            Action::Edit(edit) => edit.range(),
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
    /// This `action_group` should be always normalized.
    action_group: ActionGroup,
}

impl EditTransaction {
    #[cfg(test)]
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
        self.action_group
            .actions
            .iter()
            .filter_map(|action| match action {
                Action::Edit(edit) => Some(edit),
                _ => None,
            })
            .collect_vec()
    }

    pub fn from_action_groups(action_groups: Vec<ActionGroup>) -> Self {
        Self {
            action_group: Self::normalize_action_groups(action_groups),
        }
    }

    #[cfg(test)]
    pub fn from_tuples(action_groups: Vec<ActionGroup>) -> Self {
        Self {
            action_group: Self::normalize_action_groups(action_groups),
        }
    }

    // Normalized action groups will become one action group, as they no longer need to offset each other
    fn normalize_action_groups(action_groups: Vec<ActionGroup>) -> ActionGroup {
        // Sort the action groups by the start char index
        let action_groups = {
            let mut action_groups = action_groups;
            action_groups.sort_by_key(|action_group| action_group.range().start);
            action_groups
        };

        let mut offset: isize = 0;
        let mut result = vec![];

        for group in action_groups {
            let mut group = group.apply_offset(offset);
            offset += group.get_net_offset();

            result.append(&mut group.actions);
        }

        ActionGroup { actions: result }
    }

    pub fn min_char_index(&self) -> CharIndex {
        self.action_group
            .actions
            .iter()
            .map(|edit| edit.range().start)
            .min()
            .unwrap_or(CharIndex(0))
    }

    pub fn max_char_index(&self) -> CharIndex {
        self.action_group
            .actions
            .iter()
            .map(|edit| edit.range().end)
            .max()
            .unwrap_or(CharIndex(0))
    }

    pub fn merge(edit_transactions: Vec<EditTransaction>) -> EditTransaction {
        EditTransaction::from_action_groups(
            edit_transactions
                .into_iter()
                .map(|transaction| transaction.action_group)
                .collect(),
        )
    }

    pub fn selections(&self) -> Vec<&Selection> {
        self.action_group
            .actions
            .iter()
            .filter_map(|action| match action {
                Action::Select(selection) => Some(selection),
                _ => None,
            })
            .collect_vec()
    }

    pub fn range(&self) -> Range<CharIndex> {
        self.min_char_index()..self.max_char_index()
    }
}

/// This is for grouping actions that should not offset each other
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActionGroup {
    pub actions: Vec<Action>,
}

impl ActionGroup {
    pub fn new(actions: Vec<Action>) -> Self {
        Self { actions }
    }
    fn overlaps(&self, other: &ActionGroup) -> bool {
        is_overlapping(&self.range(), &other.range())
    }

    fn get_net_offset(&self) -> isize {
        self.actions
            .iter()
            .map(|action| match action {
                Action::Edit(edit) => edit.new.len_chars() as isize - edit.old.len_chars() as isize,
                _ => 0,
            })
            .sum()
    }

    fn apply_offset(self, offset: isize) -> ActionGroup {
        ActionGroup {
            actions: self
                .actions
                .into_iter()
                .map(|action| action.apply_offset(offset))
                .collect(),
        }
    }

    fn range(&self) -> Range<CharIndex> {
        let min = self
            .actions
            .iter()
            .map(|action| action.range().start)
            .min()
            .unwrap_or(CharIndex(0));
        let max = self
            .actions
            .iter()
            .map(|action| action.range().end)
            .max()
            .unwrap_or(CharIndex(0));
        min..max
    }

    fn subset_of(&self, other: &ActionGroup) -> bool {
        is_subset(&self.range(), &other.range())
    }
}

#[cfg(test)]
mod test_normalize_actions {
    use ropey::Rope;

    use crate::edit::{Action, ActionGroup, EditTransaction};

    #[test]
    fn only_one_edit() {
        let edit_transaction =
            EditTransaction::from_tuples(vec![ActionGroup::new(vec![Action::edit(
                0, "Who", "What",
            )])]);
        let (_, result) = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));
        assert_eq!(result, Rope::from_str("What lives in a pineapple"));
    }

    #[test]
    fn selection_offsetted_positively() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            ActionGroup::new(vec![Action::edit(0, "Who", "What")]),
            // Select the word pineapple
            ActionGroup::new(vec![Action::select(15..24)]),
        ]);

        let (selections, result) =
            edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What lives in a pineapple"));

        assert_eq!(selections, vec!["pineapple".to_string()]);
    }

    #[test]
    fn selection_offsetted_negatively() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            ActionGroup::new(vec![Action::edit(0, "Who", "He")]),
            // Select the word "pineapple"
            ActionGroup::new(vec![Action::select(15..24)]),
        ]);

        let (selections, result) =
            edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("He lives in a pineapple"));

        assert_eq!(selections, vec!["pineapple".to_string()]);
    }

    #[test]
    fn actions_in_the_same_action_group_should_not_offset_each_other() {
        let edit_transaction = EditTransaction::from_tuples(vec![ActionGroup::new(vec![
            Action::edit(0, "Who", "What"),
            Action::edit(5, "lives", "is"),
        ])]);
        let (_, result) = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));
        assert_eq!(result, Rope::from_str("What is in a pineapple"));
    }

    #[test]
    fn selection_should_not_offset_others() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            // Select the wrod "Who"
            ActionGroup::new(vec![Action::select(0..3)]),
            // Select the word "pineapple"
            ActionGroup::new(vec![Action::select(15..24)]),
        ]);

        let (selections, result) =
            edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("Who lives in a pineapple"));
        assert_eq!(selections, vec!["Who".to_string(), "pineapple".to_string()])
    }

    #[test]
    fn no_overlap() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            // Replacement length > range length
            ActionGroup::new(vec![Action::edit(0, "Who", "What")]),
            // Replacement length < range length
            ActionGroup::new(vec![Action::edit(4, "lives", "see")]),
            ActionGroup::new(vec![Action::edit(13, "a", "two")]),
        ]);

        let (_, result) = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What see in two pineapple"));
    }

    #[test]
    /// Expect the edits to be sorted before being applied
    fn unsorted() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            ActionGroup::new(vec![Action::edit(13, "a", "two")]),
            ActionGroup::new(vec![Action::edit(0, "Who", "What")]),
            ActionGroup::new(vec![Action::edit(4, "lives", "see")]),
        ]);

        let (_, result) = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What see in two pineapple"));
    }
}

/// Check if range a is a subset of range b
fn is_subset<T: Ord>(a: &Range<T>, b: &Range<T>) -> bool {
    a.start >= b.start && a.end <= b.end
}

// Test is_subset
#[cfg(test)]
mod test_is_subset {
    use crate::edit::is_subset;

    #[test]
    fn subset() {
        assert!(is_subset(&(0..5), &(0..10)));
        assert!(is_subset(&(5..10), &(0..10)));
        assert!(is_subset(&(1..2), &(0..10)));
    }

    #[test]
    fn inverted() {
        assert!(!is_subset(&(0..10), &(0..5)));
        assert!(!is_subset(&(0..10), &(5..10)));
        assert!(!is_subset(&(0..10), &(1..2)));
    }

    #[test]
    fn not_subset() {
        assert!(!is_subset(&(0..5), &(1..10)));
        assert!(!is_subset(&(0..5), &(0..4)));
    }
}

fn is_overlapping<T: Ord>(a: &Range<T>, b: &Range<T>) -> bool {
    use std::cmp::{max, min};
    max(&a.start, &b.start) < min(&a.end, &b.end)
}

// Test is_overlapping
#[cfg(test)]
mod test_is_overlapping {
    use crate::edit::is_overlapping;

    #[test]
    fn partial_overlap() {
        assert!(is_overlapping(&(0..5), &(3..10)));
        assert!(is_overlapping(&(3..10), &(0..5)));
    }

    #[test]
    fn no_overlap() {
        assert!(!is_overlapping(&(0..5), &(5..10)));
        assert!(!is_overlapping(&(5..10), &(0..5)));
    }

    #[test]
    fn no_overlap_no_touch() {
        assert!(!is_overlapping(&(0..5), &(6..10)));
        assert!(!is_overlapping(&(6..10), &(0..5)));
    }

    #[test]
    fn subset() {
        assert!(is_overlapping(&(0..10), &(3..5)));
        assert!(is_overlapping(&(3..5), &(0..10)));
    }

    #[test]
    fn same_start() {
        assert!(is_overlapping(&(0..5), &(0..10)));
        assert!(is_overlapping(&(0..10), &(0..5)));
    }

    #[test]
    fn same_end() {
        assert!(is_overlapping(&(0..10), &(5..10)));
        assert!(is_overlapping(&(5..10), &(0..10)));
    }
}
