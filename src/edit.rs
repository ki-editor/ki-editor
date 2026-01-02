use itertools::Itertools;
use nonempty::NonEmpty;
use ropey::Rope;

use crate::{
    buffer::Buffer,
    char_index_range::CharIndexRange,
    selection::{CharIndex, Selection},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Edit {
    pub(crate) range: CharIndexRange,
    pub(crate) new: Rope,
    pub(crate) old: Rope,
}
impl Edit {
    pub(crate) fn new(rope: &Rope, range: CharIndexRange, new: Rope) -> Self {
        Self {
            range,
            old: rope.slice(range.as_usize_range()).into(),
            new,
        }
    }
    fn apply_offset(self, offset: isize) -> Edit {
        Edit {
            range: self.range.apply_offset(offset),
            new: self.new,
            old: self.old,
        }
    }

    pub(crate) fn end(&self) -> CharIndex {
        self.range.end
    }

    pub(crate) fn range(&self) -> CharIndexRange {
        self.range
    }

    pub(crate) fn chars_offset(&self) -> isize {
        self.new.len_chars() as isize - self.range.len() as isize
    }

    fn intersects_with(&self, other: &Edit) -> bool {
        self.range().intersects_with(&other.range())
    }

    pub(crate) fn to_vscode_diff_edit(
        &self,
        buffer: &Buffer,
    ) -> anyhow::Result<ki_protocol_types::DiffEdit> {
        let start = buffer.char_to_vscode_position(self.range.start)?;
        let end = buffer.char_to_vscode_position(self.range.end)?;
        let edit = ki_protocol_types::DiffEdit {
            range: ki_protocol_types::Range { start, end },
            new_text: self.new.to_string(),
        };
        Ok(edit)
    }

    fn inverse(&self) -> Self {
        let range = (self.range.start..self.range.start + self.new.len_chars()).into();
        Edit {
            range,
            new: self.old.clone(),
            old: self.new.clone(),
        }
    }
}

pub trait ApplyOffset {
    fn apply_offset(self, offset: isize) -> Self;
}

impl ApplyOffset for CharIndexRange {
    fn apply_offset(self, offset: isize) -> Self {
        (self.start.apply_offset(offset)..self.end.apply_offset(offset)).into()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Action {
    Select(Selection),
    Edit(Edit),
}

impl Action {
    #[cfg(test)]
    fn edit(start: usize, old: &str, new: &str) -> Self {
        Action::Edit(Edit {
            range: (CharIndex(start)..CharIndex(start + old.len())).into(),
            new: Rope::from_str(new),
            old: Rope::from_str(old),
        })
    }

    #[cfg(test)]
    fn select(range: std::ops::Range<usize>) -> Self {
        Action::Select(Selection::new(
            (CharIndex(range.start)..CharIndex(range.end)).into(),
        ))
    }

    fn range(&self) -> CharIndexRange {
        match self {
            Action::Select(selection) => selection.extended_range(),
            Action::Edit(edit) => edit.range(),
        }
    }

    fn apply_offset(self, offset: isize) -> Self {
        match self {
            Action::Select(selection) => Action::Select(selection.apply_offset(offset)),
            Action::Edit(edit) => Action::Edit(edit.apply_offset(offset)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EditTransaction {
    /// This `action_group` should be always normalized.
    action_group: ActionGroup,

    /// This is required by VS Code because VS Code will offset the edits on their end.
    unnormalized_edits: Vec<Edit>,
}

impl EditTransaction {
    #[cfg(test)]
    fn apply_to(&self, mut rope: Rope) -> (Vec<String>, Rope) {
        for edit in &self.edits() {
            rope.remove(edit.range.start.0..edit.end().0);
            rope.insert(edit.range.start.0, edit.new.to_string().as_str());
        }
        let selections = self
            .selections()
            .into_iter()
            .map(|selection| {
                let range = selection.extended_range();
                rope.slice(range.start.0..range.end.0).to_string()
            })
            .collect_vec();
        (selections, rope)
    }

    pub(crate) fn edits(&self) -> Vec<&Edit> {
        self.action_group
            .actions
            .iter()
            .filter_map(|action| match action {
                Action::Edit(edit) => Some(edit),
                _ => None,
            })
            .collect_vec()
    }

    pub(crate) fn from_action_groups(action_groups: Vec<ActionGroup>) -> Self {
        let unnormalized_edits = action_groups
            .iter()
            .flat_map(|action_group| {
                action_group
                    .actions
                    .iter()
                    .filter_map(|action| match action {
                        Action::Select(_) => None,
                        Action::Edit(edit) => Some(edit.clone()),
                    })
                    .collect_vec()
            })
            .collect_vec();
        Self {
            unnormalized_edits,
            action_group: Self::normalize_action_groups(action_groups),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_tuples(action_groups: Vec<ActionGroup>) -> Self {
        Self {
            action_group: Self::normalize_action_groups(action_groups),
            unnormalized_edits: Default::default(),
        }
    }

    /// Normalized action groups will become one action group, as they no longer need to offset each other
    fn normalize_action_groups(action_groups: Vec<ActionGroup>) -> ActionGroup {
        // Sort the action groups by the start char index
        let action_groups = action_groups
            .into_iter()
            .sorted_by_key(|action_group| action_group.range().start)
            .collect_vec();

        let mut offset: isize = 0;
        let mut result = vec![];

        for (index, group) in action_groups.iter().enumerate() {
            if index == 0 || !group.intersects_with(&action_groups[index - 1]) {
                let mut group = group.to_owned().apply_offset(offset);
                offset += group.get_net_offset();

                result.append(&mut group.actions);
            }
        }

        ActionGroup { actions: result }
    }

    pub(crate) fn min_char_index(&self) -> CharIndex {
        self.action_group
            .actions
            .iter()
            .map(|edit| edit.range().start)
            .min()
            .unwrap_or(CharIndex(0))
    }

    pub(crate) fn max_char_index(&self) -> CharIndex {
        self.action_group
            .actions
            .iter()
            .map(|edit| edit.range().end)
            .max()
            .unwrap_or(CharIndex(0))
    }

    pub(crate) fn merge(edit_transactions: Vec<EditTransaction>) -> EditTransaction {
        let unnormalized_edits = edit_transactions
            .iter()
            .flat_map(|edit_transaction| edit_transaction.unnormalized_edits.clone())
            .collect_vec();

        Self {
            unnormalized_edits,
            action_group: Self::normalize_action_groups(
                edit_transactions
                    .into_iter()
                    .map(|edit_transaction| edit_transaction.action_group)
                    .collect_vec(),
            ),
        }
    }

    pub(crate) fn selections(&self) -> Vec<&Selection> {
        self.action_group
            .actions
            .iter()
            .filter_map(|action| match action {
                Action::Select(selection) => Some(selection),
                _ => None,
            })
            .collect_vec()
    }

    pub(crate) fn range(&self) -> CharIndexRange {
        (self.min_char_index()..self.max_char_index()).into()
    }

    pub(crate) fn non_empty_selections(&self) -> Option<NonEmpty<Selection>> {
        if let Some((head, tail)) = self.selections().split_first() {
            Some(NonEmpty {
                head: (*head).to_owned(),
                tail: tail.iter().map(|selection| (*selection).clone()).collect(),
            })
        } else {
            None
        }
    }

    pub(crate) fn inverse(&self) -> EditTransaction {
        let action_group = ActionGroup::new(
            self.action_group
                .actions
                .iter()
                .rev()
                .map(|action| match action {
                    Action::Select(selection) => Action::Select(selection.clone()),
                    Action::Edit(edit) => Action::Edit(edit.inverse()),
                })
                .collect(),
        );
        EditTransaction {
            // NOTE: for reasons that I still don't understand,
            //       it seems like we don't need to unnormalize
            //       the inverted normalized edits, but it will
            //       allow undo/redo to be mapped to VS Code correctly.
            unnormalized_edits: action_group
                .actions
                .iter()
                .filter_map(|action| match action {
                    Action::Select(_) => None,
                    Action::Edit(edit) => Some(edit.clone()),
                })
                .collect(),
            action_group,
        }
    }

    pub(crate) fn unnormalized_edits(&self) -> Vec<Edit> {
        self.unnormalized_edits.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// This is for grouping actions that should not offset each other
pub(crate) struct ActionGroup {
    pub(crate) actions: Vec<Action>,
}

impl ActionGroup {
    pub(crate) fn new(actions: Vec<Action>) -> Self {
        Self { actions }
    }
    fn get_net_offset(&self) -> isize {
        self.actions
            .iter()
            .map(|action| match action {
                Action::Edit(edit) => edit.chars_offset(),
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

    fn range(&self) -> CharIndexRange {
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
        (min..max).into()
    }

    fn intersects_with(&self, action_group: &ActionGroup) -> bool {
        self.actions.iter().any(|a| {
            action_group.actions.iter().any(|b| match (a, b) {
                (Action::Edit(a), Action::Edit(b)) => a.intersects_with(b),
                _ => false,
            })
        })
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

    #[test]
    fn intersected_edits_removed_1() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            ActionGroup::new(vec![
                Action::edit(4, "quick", "speedy"),
                Action::select(4..10),
            ]),
            ActionGroup::new(vec![Action::edit(5, "brown ", "")]), // This will be ignored since it intersects with the first edit
        ]);

        let (_, result) = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("Who speedy in a pineapple"));
    }

    #[test]
    fn intersected_edits_removed_2() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            ActionGroup::new(vec![
                Action::edit(4, "quick", "speedy"),
                Action::select(4..10),
            ]),
            ActionGroup::new(vec![Action::edit(0, "Who", "What")]),
            ActionGroup::new(vec![Action::edit(5, "brown ", "")]), // This will be ignored since it intersects with the first edit
        ]);

        let (_, result) = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What speedy in a pineapple"));
    }
}

// Test is_subset
#[cfg(test)]
mod test_is_subset {

    use std::ops::Range;
    /// Check if range a is a subset of range b
    fn is_subset<T: Ord>(a: &Range<T>, b: &Range<T>) -> bool {
        a.start >= b.start && a.end <= b.end
    }

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

#[cfg(test)]
mod test_inverse_edit_transaction {
    use super::*;

    fn apply_and_verify(original_rope: &Rope, edit_transaction: &EditTransaction) {
        // Apply the original transaction
        let (_, modified_rope) = edit_transaction.apply_to(original_rope.clone());

        // Create and apply the inverse transaction
        let inverse_transaction = edit_transaction.inverse();
        let (_, restored_rope) = inverse_transaction.apply_to(modified_rope);

        // Verify the result
        assert_eq!(restored_rope.to_string(), original_rope.to_string());
    }

    #[test]
    fn test_simple_replacement() {
        let original_rope = Rope::from_str("Hello World");
        let edit_transaction =
            EditTransaction::from_tuples(vec![ActionGroup::new(vec![Action::edit(
                6, "World", "Universe",
            )])]);

        apply_and_verify(&original_rope, &edit_transaction);
    }

    #[test]
    fn test_two_edits() {
        let original_rope = Rope::from_str("Hello World Yo");
        let edit_transaction = EditTransaction::from_tuples(
            [ActionGroup::new(
                [
                    Action::edit(6, "World", "Universe"),
                    Action::edit(12, "Yo", "Nay"),
                ]
                .to_vec(),
            )]
            .to_vec(),
        );

        apply_and_verify(&original_rope, &edit_transaction);
    }

    #[test]
    fn test_three_edits() {
        let original_rope = Rope::from_str("Hello World Yo");
        let edit_transaction = EditTransaction::from_tuples(
            [
                ActionGroup::new([Action::edit(0, "Hello", "Bye")].to_vec()),
                ActionGroup::new([Action::edit(6, "World", "Universe")].to_vec()),
                ActionGroup::new([Action::edit(12, "Yo", "Nay")].to_vec()),
            ]
            .to_vec(),
        );

        apply_and_verify(&original_rope, &edit_transaction);
    }

    #[test]
    fn test_positive_offset() {
        let original_rope = Rope::from_str("short text");
        let edit_transaction =
            EditTransaction::from_tuples(vec![ActionGroup::new(vec![Action::edit(
                0,
                "short",
                "much longer",
            )])]);

        apply_and_verify(&original_rope, &edit_transaction);
    }

    #[test]
    fn test_negative_offset() {
        let original_rope = Rope::from_str("longer phrase here");
        let edit_transaction =
            EditTransaction::from_tuples(vec![ActionGroup::new(vec![Action::edit(
                0, "longer ", "",
            )])]);

        apply_and_verify(&original_rope, &edit_transaction);
    }

    #[test]
    fn test_with_selections() {
        let original_rope = Rope::from_str("Select this text and that text");
        let edit_transaction = EditTransaction::from_tuples(vec![ActionGroup::new(vec![
            Action::edit(7, "this", "the"),
            Action::select(7..10),
        ])]);

        apply_and_verify(&original_rope, &edit_transaction);
    }

    #[test]
    fn test_sequential_action_groups() {
        let original_rope = Rope::from_str("ABC DEF GHI");
        let edit_transaction = EditTransaction::from_tuples(vec![
            ActionGroup::new(vec![Action::edit(0, "ABC", "XYZ")]),
            ActionGroup::new(vec![Action::edit(8, "GHI", "JKL")]),
        ]);

        apply_and_verify(&original_rope, &edit_transaction);
    }

    #[test]
    fn test_insertions_and_deletions() {
        let original_rope = Rope::from_str("Hello world");
        let edit_transaction = EditTransaction::from_tuples(vec![
            ActionGroup::new(vec![Action::edit(5, " ", " beautiful ")]),
            ActionGroup::new(vec![Action::edit(0, "Hello", "Hi")]),
        ]);

        apply_and_verify(&original_rope, &edit_transaction);
    }

    #[test]
    fn test_complex_with_selection() {
        let original_rope = Rope::from_str("The quick brown fox jumps over the lazy dog");
        let edit_transaction = EditTransaction::from_tuples(vec![
            ActionGroup::new(vec![
                Action::edit(4, "quick", "speedy"),
                Action::select(4..10),
            ]),
            ActionGroup::new(vec![Action::edit(10, "brown ", "")]),
            ActionGroup::new(vec![Action::edit(20, "jumps", "leapt")]),
        ]);

        apply_and_verify(&original_rope, &edit_transaction);
    }
}
