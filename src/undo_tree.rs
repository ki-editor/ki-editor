use std::fmt::Display;

use undo::History;

use crate::components::editor::{Direction, Movement};

#[derive(Clone)]
pub struct OldNew<T: Clone> {
    pub old_to_new: T,
    pub new_to_old: T,
}

pub trait Applicable {
    type Target;
    type Output;
    fn apply(&self, target: &mut Self::Target) -> Self::Output;
}

#[derive(Clone)]
pub struct UndoTree<T: Applicable + Clone + Display> {
    history: History<OldNew<T>>,
}

impl<T: Applicable + Clone + Display> UndoTree<T> {
    pub fn edit(&mut self, target: &mut T::Target, edit: OldNew<T>) -> T::Output {
        self.history.edit(target, edit)
    }

    pub fn undo(&mut self, target: &mut T::Target) -> Option<T::Output> {
        self.history.undo(target)
    }

    pub fn redo(&mut self, target: &mut T::Target) -> Option<T::Output> {
        self.history.redo(target)
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
    ) -> Option<T::Output> {
        match movement {
            Movement::Next => self.redo(target),
            Movement::Previous => self.undo(target),
            Movement::Last => todo!(),
            Movement::Current => todo!(),
            Movement::Up => self.go_to_history_branch(target, Direction::End),
            Movement::Down => self.go_to_history_branch(target, Direction::Start),
            Movement::First => todo!(),
            Movement::Index(_) => todo!(),
            Movement::Jump(_) => todo!(),
        }
    }

    fn go_to_history_branch(
        &mut self,
        target: &mut T::Target,
        direction: Direction,
    ) -> Option<T::Output> {
        let Some(destination) = (match direction {
            Direction::Start => self.history.prev_branch_head(),
            Direction::End => self.history.next_branch_head(),
        }) else {
            return None;
        };

        self.history.go_to(target, destination).pop()
    }
}

impl<T: Applicable + Clone> undo::Edit for OldNew<T> {
    type Target = T::Target;
    type Output = T::Output;

    fn edit(&mut self, target: &mut Self::Target) -> Self::Output {
        self.old_to_new.apply(target)
    }

    fn undo(&mut self, target: &mut Self::Target) -> Self::Output {
        self.new_to_old.apply(target)
    }
}

impl<T: Clone + std::fmt::Display> std::fmt::Display for OldNew<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.old_to_new.fmt(f)
    }
}
