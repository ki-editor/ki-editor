use undo::History;

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
pub struct UndoTree<T: Applicable + Clone> {
    history: History<OldNew<T>>,
}

impl<T: Applicable + Clone> UndoTree<T> {
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

    pub(crate) fn prev_branch_head(&self) -> Option<undo::At> {
        self.history.prev_branch_head()
    }

    pub(crate) fn next_branch_head(&self) -> Option<undo::At> {
        self.history.next_branch_head()
    }

    pub(crate) fn go_to(
        &mut self,
        target: &mut T::Target,
        at: undo::At,
    ) -> Vec<<T as Applicable>::Output> {
        self.history.go_to(target, at)
    }

    pub(crate) fn display(&self) -> undo::history::Display<'_, OldNew<T>, ()> {
        self.history.display()
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

impl<T: Clone> std::fmt::Display for OldNew<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
