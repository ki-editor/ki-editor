use itertools::Itertools;
use std::fmt::Display;

use undo::History;

use crate::components::editor::{Direction, Movement};

#[derive(Clone, PartialEq)]
pub struct OldNew<T: Clone + PartialEq> {
    pub old_to_new: T,
    pub new_to_old: T,
}

pub trait Applicable: Clone + Display + PartialEq {
    type Target;
    type Output: Display;
    fn apply(&self, target: &mut Self::Target) -> anyhow::Result<Self::Output>;
}

#[derive(Clone)]
pub struct UndoTree<T: Applicable> {
    history: History<OldNew<T>>,
}

impl<T: Applicable> UndoTree<T> {
    pub fn edit(
        &mut self,
        target: &mut T::Target,
        edit: OldNew<T>,
    ) -> anyhow::Result<Option<T::Output>> {
        let head = self.history.head();

        let current_entry = self.history.get_entry(head.index.saturating_sub(1));

        match current_entry {
            Some(last_entry) if last_entry.get().old_to_new == edit.old_to_new => Ok(None),
            _ => Ok(Some(self.history.edit(target, edit)?)),
        }
    }

    pub fn undo(&mut self, target: &mut T::Target) -> anyhow::Result<Option<T::Output>> {
        self.history.undo(target).transpose()
    }

    pub fn redo(&mut self, target: &mut T::Target) -> anyhow::Result<Option<T::Output>> {
        self.history.redo(target).transpose()
    }

    pub(crate) fn new() -> UndoTree<T> {
        Self {
            history: History::new(),
        }
    }

    pub(crate) fn display(&self) -> String {
        self.history.display().detailed(false).to_string()
    }

    pub(crate) fn apply_movement(
        &mut self,
        target: &mut T::Target,
        movement: Movement,
    ) -> anyhow::Result<Option<T::Output>> {
        match movement {
            Movement::Next => self.redo(target),
            Movement::Previous => self.undo(target),
            Movement::Last => Err(anyhow::anyhow!(
                "UndoTree: moving to Last is not supported yet",
            )),
            Movement::Current => Err(anyhow::anyhow!(
                "UndoTree: moving to Current is not supported yet",
            )),
            Movement::Up => {
                self.go_to_history_branch(target, Direction::End)?;
                Ok(None)
            }
            Movement::Down => {
                self.go_to_history_branch(target, Direction::Start)?;
                Ok(None)
            }
            Movement::First => Err(anyhow::anyhow!(
                "UndoTree: moving to First is not supported yet",
            )),
            Movement::Index(_) => Err(anyhow::anyhow!(
                "UndoTree: moving to Index is not supported yet",
            )),
            Movement::Jump(_) => Err(anyhow::anyhow!(
                "UndoTree: moving to Jump is not supported yet",
            )),
            Movement::ToParentLine => Err(anyhow::anyhow!(
                "UndoTree: moving to ParentLine is not supported yet",
            )),
        }
    }

    fn go_to_history_branch(
        &mut self,
        target: &mut T::Target,
        direction: Direction,
    ) -> anyhow::Result<()> {
        let Some(destination) = (match direction {
            Direction::Start => self.history.prev_branch_head(),
            Direction::End => self.history.next_branch_head(),
        }) else {
            return Ok(());
        };

        // Switch to destination branch
        let mut first_outputs = self.history.go_to(target, destination);

        let _first_output = first_outputs.pop();

        // Go to the last entry of the current branch
        let entries_len = self.history.len();
        let at = undo::At {
            root: destination.root,
            index: entries_len,
        };

        let _output = self.history.go_to(target, at);

        Ok(())
    }

    pub(crate) fn previous_entries(&self) -> Vec<(usize, &undo::Entry<OldNew<T>>)> {
        self.get_entries(true)
    }

    pub(crate) fn next_entries(&self) -> Vec<(usize, &undo::Entry<OldNew<T>>)> {
        self.get_entries(false)
    }

    fn get_entries(&self, previous: bool) -> Vec<(usize, &undo::Entry<OldNew<T>>)> {
        let cmp = if previous {
            |x: usize, y: usize| x < y
        } else {
            |x, y| x > y
        };

        self.entries()
            .filter(|(index, _)| cmp(*index, self.current_entry_index()))
            .collect()
    }

    fn entries(&self) -> impl Iterator<Item = (usize, &undo::Entry<OldNew<T>>)> {
        log::info!("entries.count = {}", self.history.entries().count());
        self.history.head();
        self.history
            .entries()
            .enumerate()
            // We need to add 1 because the `undo::history::History::entries
            // method does not return the first entry with index 0
            .map(|(index, entry)| (index + 1, entry))
            .map(|(index, entry)| {
                log::info!("[{}]: {}", index, entry.get().old_to_new.to_string());
                (index, entry)
            })
    }

    fn current_entry_index(&self) -> usize {
        self.history.head().index
    }

    pub(crate) fn go_to_entry(
        &mut self,
        target: &mut T::Target,
        index: usize,
    ) -> Result<Vec<<T as Applicable>::Output>, anyhow::Error> {
        log::info!(
            "go_to_entry: index = {index} self.history.head() = {:?}",
            self.history.head()
        );
        self.history
            .go_to(
                target,
                undo::At {
                    root: self.history.head().root,
                    index,
                },
            )
            .into_iter()
            .try_collect()
    }
}

impl<T: Applicable + Clone> undo::Edit for OldNew<T> {
    type Target = T::Target;
    type Output = anyhow::Result<T::Output>;

    fn edit(&mut self, target: &mut Self::Target) -> Self::Output {
        self.old_to_new.apply(target)
    }

    fn undo(&mut self, target: &mut Self::Target) -> Self::Output {
        self.new_to_old.apply(target)
    }
}

impl<T: Clone + std::fmt::Display + PartialEq> std::fmt::Display for OldNew<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.old_to_new.fmt(f)
    }
}
