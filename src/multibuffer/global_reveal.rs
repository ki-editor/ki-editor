use crate::components::component::Component;
use crate::components::suggestive_editor::SuggestiveEditor;
use crate::{
    app::App,
    buffer::BufferOwner,
    char_index_range::CharIndexRange,
    components::editor::{DispatchEditor, IfCurrentNotFound, Reveal},
    context::GlobalMode,
    frontend::Frontend,
    multibuffer::{Multibuffer, MultibufferFile},
    selection::SelectionMode,
};

use itertools::Itertools;
use shared::absolute_path::AbsolutePath;
use std::{cell::RefCell, rc::Rc};

use super::MultibufferRange;

pub struct GlobalReveal {
    pub files: Vec<GlobalRevealFile>,
}

pub struct GlobalRevealFile {
    pub path: AbsolutePath,
    /// Ranges of the possible selections
    pub possible_selection_ranges: Vec<CharIndexRange>,
    pub editor: Rc<RefCell<SuggestiveEditor>>,
}
impl GlobalRevealFile {
    pub(crate) fn to_multibuffer_path(&self) -> MultibufferFile {
        MultibufferFile {
            path: self.path.clone(),
            editor: self.editor.clone(),
        }
    }
}

impl GlobalReveal {
    #[cfg(test)]
    pub fn editors(&self) -> Vec<Rc<RefCell<SuggestiveEditor>>> {
        self.files
            .iter()
            .map(|file| file.editor.clone())
            .collect_vec()
    }

    pub fn ranges(&self, current_file_path: &AbsolutePath) -> Vec<MultibufferRange> {
        self.files
            .iter()
            .flat_map(|file| {
                let binding = file.editor.borrow();
                let selection_set = &binding.editor().selection_set;
                file.possible_selection_ranges
                    .iter()
                    .map(|range| {
                        let is_primary = &file.path == current_file_path
                            && range == &selection_set.primary_selection().range;
                        MultibufferRange {
                            path: file.path.clone(),
                            range: *range,
                            is_primary,
                        }
                    })
                    .collect_vec()
            })
            .collect_vec()
    }

    pub fn focused_file_index(&self, current_file_path: &AbsolutePath) -> Option<usize> {
        self.files
            .iter()
            .position(|file| &file.path == current_file_path)
    }
}
impl<T: Frontend> App<T> {
    pub fn toggle_reveal_selections(&mut self) -> anyhow::Result<()> {
        if let Some(Multibuffer::GlobalReveal(_)) = self.multibuffer {
            self.multibuffer = None;
            Ok(())
        } else if self.context.mode() == Some(GlobalMode::QuickfixListItem) {
            self.active_global_reveal_selections()
        } else {
            self.handle_dispatch_editor(DispatchEditor::ToggleReveal(Reveal::CurrentSelectionMode))
        }
    }

    fn active_global_reveal_selections(&mut self) -> anyhow::Result<()> {
        let grouped_ranges = self
            .quickfix_list()
            .items()
            .iter()
            .sorted_by_key(|item| item.location().path.clone())
            .chunk_by(|item| item.location().path.clone())
            .into_iter()
            .map(|(path, items)| {
                (
                    path,
                    items
                        .into_iter()
                        .map(|item| item.location().range)
                        .collect_vec(),
                )
            })
            .collect_vec();

        let files = grouped_ranges
            .into_iter()
            .map(|(path, ranges)| -> anyhow::Result<_> {
                let editor = self.open_file(&path, BufferOwner::User, true, false)?;

                self.handle_dispatch_editor_custom(
                    DispatchEditor::SetSelectionMode(
                        IfCurrentNotFound::LookForward,
                        SelectionMode::LocalQuickfix {
                            title: self.quickfix_list().title().to_string(),
                        },
                    ),
                    editor.clone(),
                )?;

                Ok(GlobalRevealFile {
                    path,
                    editor,
                    possible_selection_ranges: ranges,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        if !files.is_empty() {
            self.multibuffer = Some(Multibuffer::GlobalReveal(GlobalReveal { files }));

            // Close the quickfix list
            self.layout.remain_only_current_component();
        }

        // This is a hack: we need to reset the global mode because it is cleared when `self.open_file` is invoked
        self.context.set_mode(Some(GlobalMode::QuickfixListItem));

        Ok(())
    }
}
