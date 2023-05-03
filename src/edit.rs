use std::ops::Range;

use itertools::Itertools;
use ropey::Rope;

use crate::engine::{CharIndex, Selection};

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

    fn subset_of(&self, other: &Edit) -> bool {
        self.start >= other.start && self.end() <= other.end()
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

#[derive(Clone, Debug)]
pub struct EditTransaction {
    edits: Vec<Edit>,
    /// Used for restoring previous selection after undo/redo
    pub selection: Selection,
}

impl EditTransaction {
    fn apply_to(&self, mut rope: Rope) -> Rope {
        for edit in &self.edits {
            rope.remove(edit.start.0..edit.end().0);
            rope.insert(edit.start.0, edit.new.to_string().as_str());
        }
        rope
    }

    pub fn edits(&self) -> &Vec<Edit> {
        &self.edits
    }

    pub fn from_edits(current_selection: Selection, edits: Vec<Edit>) -> Self {
        Self {
            edits: Self::normalize_edits(&edits),
            selection: current_selection,
        }
    }

    pub fn from_tuples(edits: Vec<(/*start*/ usize, /*old*/ &str, /*new*/ &str)>) -> Self {
        Self {
            selection: Selection::default(),
            edits: Self::normalize_edits(
                &edits
                    .into_iter()
                    .map(|(start, old, new)| Edit {
                        start: CharIndex(start),
                        old: Rope::from_str(old),
                        new: Rope::from_str(new),
                    })
                    .collect::<Vec<_>>(),
            ),
        }
    }

    /// This is used for multi-cursor edits.
    pub fn normalize_edits(edits: &Vec<Edit>) -> Vec<Edit> {
        // 1) remove edit that are subset of other edits
        let edits = edits
            .iter()
            .filter(|edit| {
                !edits
                    .iter()
                    .any(|other| *edit != other && edit.subset_of(other))
            })
            // 2) sort edits by start position
            .sorted_by_key(|edit| edit.start)
            // 3) Remove duplicates
            .unique()
            .collect::<Vec<_>>();

        match edits.split_first() {
            None => vec![],
            Some((head, &[])) => vec![(*head).clone()],
            Some((head, _)) => {
                let mut offset: isize = 0;
                let mut result = vec![(*head).clone()];
                let tuples = edits.into_iter().tuple_windows::<(&Edit, &Edit)>();

                for (current, next) in tuples {
                    assert!(current.start < next.start);

                    // 4) trim edits that intersect with each other
                    let next = Edit {
                        start: next.start.max(current.end()),
                        ..next.clone()
                    };

                    offset += current.new.len_chars() as isize
                        - (current.end().0 as isize - current.start.0 as isize) as isize;

                    // 5) apply offset to edits
                    result.push(next.apply_offset(offset))
                }

                return result;
            }
        }
    }

    pub fn min_char_index(&self) -> CharIndex {
        self.edits
            .iter()
            .map(|edit| edit.start)
            .min()
            .unwrap_or(CharIndex(0))
    }

    pub fn max_char_index(&self) -> CharIndex {
        self.edits
            .iter()
            .map(|edit| edit.end())
            .max()
            .unwrap_or(CharIndex(0))
    }
}

// Test normalize_edits
#[cfg(test)]
mod test_normalize_edit {
    use ropey::Rope;

    use crate::edit::EditTransaction;

    #[test]
    fn only_one_edit() {
        let edit_transaction = EditTransaction::from_tuples(vec![(0, "Who", "What")]);
        let result = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));
        assert_eq!(result, Rope::from_str("What lives in a pineapple"));
    }

    #[test]
    fn no_intersection() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            // Replacement length > range length
            (0, "Who", "What"),
            // Replacement length < range length
            (4, "lives", "see"),
            (13, "a", "two"),
        ]);

        let result = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What see in two pineapple"));
    }

    #[test]
    /// Expects the first edit is removed because it is a subset of the second edit.
    fn some_is_subset_of_other() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            (0, "Who", "What"),
            (0, "Who lives", "He"),
            (13, "a", "two"),
        ]);

        let result = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("He in two pineapple"));
    }

    #[test]
    /// Expect the edits to be sorted before being applied
    fn unsorted() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            (13, "a", "two"),
            (0, "Who", "What"),
            (4, "lives", "see"),
        ]);

        let result = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What see in two pineapple"));
    }

    #[test]
    /// Expect duplicate edits to be removed
    fn duplicated() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            (0, "Who", "What"),
            (0, "Who", "What"),
            (0, "Who", "What"),
            (4, "lives", "see"),
            (4, "lives", "see"),
            (13, "a", "two"),
        ]);

        let result = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What see in two pineapple"));
    }

    #[test]
    /// Intersected edits should be trimmed
    fn some_intersected() {
        let edit_transaction = EditTransaction::from_tuples(vec![
            (0, "Who", "What"),
            (4, "lives", "see"),
            (6, "ves ", "soap"),
        ]);

        let result = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What seesoapa pineapple"));
    }
}
