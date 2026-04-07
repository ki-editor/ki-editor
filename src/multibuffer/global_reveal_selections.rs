#[cfg(test)]
use crate::components::suggestive_editor::SuggestiveEditor;
use crate::multibuffer::MultibufferFile;
#[cfg(test)]
use itertools::Itertools;
#[cfg(test)]
use std::{cell::RefCell, rc::Rc};

pub struct GlobalRevealSelections {
    pub files: Vec<MultibufferFile>,
    pub focused_file_index: usize,
}

impl GlobalRevealSelections {
    #[cfg(test)]
    pub fn editors(&self) -> Vec<Rc<RefCell<SuggestiveEditor>>> {
        self.files
            .iter()
            .map(|file| file.editor.clone())
            .collect_vec()
    }
}
