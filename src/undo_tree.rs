use std::fmt::Display;

use undo::History;

use crate::components::editor::{Direction, Movement};

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct OldNew<T> {
    pub(crate) old_to_new: T,
    pub(crate) new_to_old: T,
}
pub trait Applicable: Clone + Display + PartialEq {
    type Target;
    type Output: Display;
    fn apply(&self, target: &mut Self::Target) -> anyhow::Result<Self::Output>;
}

#[derive(Clone)]
pub(crate) struct UndoTree<T: Applicable> {
    history: History<OldNew<T>>,
}

impl<T: Applicable> UndoTree<T> {
    pub(crate) fn edit(
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

    pub(crate) fn undo(&mut self, target: &mut T::Target) -> anyhow::Result<Option<T::Output>> {
        self.history.undo(target).transpose()
    }

    pub(crate) fn redo(&mut self, target: &mut T::Target) -> anyhow::Result<Option<T::Output>> {
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
            Movement::Current(_) => Err(anyhow::anyhow!(
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
            Movement::Parent => Err(anyhow::anyhow!(
                "UndoTree: moving to Parent is not supported yet",
            )),
            Movement::FirstChild => Err(anyhow::anyhow!(
                "UndoTree: moving to FirstChild is not supported yet",
            )),
            Movement::RealNext => Err(anyhow::anyhow!(
                "UndoTree: moving to RealNext is not supported yet",
            )),
            Movement::RealPrevious => Err(anyhow::anyhow!(
                "UndoTree: moving to RealPrevious is not supported yet",
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
