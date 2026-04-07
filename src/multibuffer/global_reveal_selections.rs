#[cfg(test)]
use crate::components::suggestive_editor::SuggestiveEditor;
use crate::{
    app::App,
    buffer::BufferOwner,
    components::editor::{DispatchEditor, Reveal},
    context::GlobalMode,
    frontend::Frontend,
    multibuffer::{Multibuffer, MultibufferFile},
};

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
impl<T: Frontend> App<T> {
    pub fn toggle_reveal_selections(&mut self) -> anyhow::Result<()> {
        if self.context.mode() == Some(GlobalMode::QuickfixListItem) {
            if self.multibuffer.is_some() {
                self.multibuffer = None;
                Ok(())
            } else {
                self.active_global_reveal_selections()
            }
        } else {
            self.handle_dispatch_editor(DispatchEditor::ToggleReveal(Reveal::CurrentSelectionMode))
        }
    }

    fn active_global_reveal_selections(&mut self) -> anyhow::Result<()> {
        let paths = self
            .quickfix_list()
            .items()
            .iter()
            .map(|item| item.location().path.clone())
            .unique()
            .sorted()
            .collect_vec();

        let files = paths
            .into_iter()
            .map(|path| -> anyhow::Result<_> {
                let editor = self.open_file(&path, BufferOwner::User, true, false)?;
                Ok(MultibufferFile { path, editor })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        if !files.is_empty() {
            self.multibuffer = Some(Multibuffer::GlobalRevealSelections(
                GlobalRevealSelections {
                    // We'll just assume the first file is the focused file, for simplicity purposes
                    files,
                    focused_file_index: 0,
                },
            ));

            // Close the quickfix list
            self.layout.remain_only_current_component();
        }

        Ok(())
    }
}
