use std::ops::Range;

use itertools::Itertools;
use ropey::Rope;

use crate::engine::CharIndex;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Edit {
    pub range: Range<CharIndex>,
    pub replacement: Rope,
}
impl Edit {
    fn apply_offset(self, offset: isize) -> Edit {
        Edit {
            range: self.range.apply_offset(offset),
            replacement: self.replacement,
        }
    }

    pub fn new(range: Range<usize>, replacement: &str) -> Self {
        Edit {
            range: CharIndex(range.start)..CharIndex(range.end),
            replacement: Rope::from(replacement),
        }
    }

    fn subset_of(&self, other: &Edit) -> bool {
        self.range.start >= other.range.start && self.range.end <= other.range.end
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
    pub edits: Vec<Edit>,
}

impl EditTransaction {
    fn apply_to(&self, mut rope: Rope) -> Rope {
        for edit in &self.edits {
            rope.remove(edit.range.start.0..edit.range.end.0);
            rope.insert(edit.range.start.0, edit.replacement.to_string().as_str());
        }
        rope
    }

    pub fn new(edits: Vec<(Range<usize>, &str)>) -> Self {
        Self {
            edits: Self::normalize_edits(
                edits
                    .into_iter()
                    .map(|(range, replacement)| Edit {
                        range: CharIndex(range.start)..CharIndex(range.end),
                        replacement: Rope::from_str(replacement),
                    })
                    .collect(),
            ),
        }
    }

    /// This is used for multi-cursor edits.
    fn normalize_edits(edits: Vec<Edit>) -> Vec<Edit> {
        // 1) remove edit that are subset of other edits
        let edits = edits
            .iter()
            .filter(|edit| {
                !edits
                    .iter()
                    .any(|other| *edit != other && edit.subset_of(other))
            })
            // 2) sort edits by start position
            .sorted_by_key(|edit| edit.range.start)
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
                    assert!(current.range.start < next.range.start);

                    // 4) trim edits that intersect with each other
                    let next = Edit {
                        range: (next.range.start.max(current.range.end))..next.range.end,
                        ..next.clone()
                    };

                    offset += current.replacement.len_chars() as isize
                        - (current.range.end.0 as isize - current.range.start.0 as isize) as isize;

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
            .map(|edit| edit.range.start)
            .min()
            .unwrap_or(CharIndex(0))
    }

    pub fn max_char_index(&self) -> CharIndex {
        self.edits
            .iter()
            .map(|edit| edit.range.end)
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
        let edit_transaction = EditTransaction::new(vec![(0..3, "What")]);
        let result = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));
        assert_eq!(result, Rope::from_str("What lives in a pineapple"));
    }

    #[test]
    fn no_intersection() {
        let edit_transaction = EditTransaction::new(vec![
            // Replacement length > range length
            (0..3, "What"),
            // Replacement length < range length
            (4..9, "see"),
            (13..14, "two"),
        ]);

        let result = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What see in two pineapple"));
    }

    #[test]
    /// Expects the first edit is removed because it is a subset of the second edit.
    fn some_is_subset_of_other() {
        let edit_transaction =
            EditTransaction::new(vec![(0..3, "What"), (0..9, "He"), (13..14, "two")]);

        let result = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("He in two pineapple"));
    }

    #[test]
    /// Expect the edits to be sorted before being applied
    fn unsorted() {
        let edit_transaction =
            EditTransaction::new(vec![(13..14, "two"), (0..3, "What"), (4..9, "see")]);

        let result = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What see in two pineapple"));
    }

    #[test]
    /// Expect duplicate edits to be removed
    fn duplicated() {
        let edit_transaction = EditTransaction::new(vec![
            (0..3, "What"),
            (0..3, "What"),
            (0..3, "What"),
            (4..9, "see"),
            (4..9, "see"),
            (13..14, "two"),
        ]);

        let result = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What see in two pineapple"));
    }

    #[test]
    /// Intersected edits should be trimmed
    fn some_intersected() {
        let edit_transaction =
            EditTransaction::new(vec![(0..3, "What"), (4..9, "see"), (6..10, "soap")]);

        let result = edit_transaction.apply_to(Rope::from_str("Who lives in a pineapple"));

        assert_eq!(result, Rope::from_str("What seesoapin a pineapple"));
    }
}
