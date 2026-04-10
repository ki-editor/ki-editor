use crate::components::component::Component;
use crate::components::editor::{Direction, Movement};

use crate::context::QuickfixListKind;
use crate::multibuffer::{Multibuffer, MultibufferFile, MultibufferRange};
use crate::quickfix_list::QuickfixListType;
use crate::selection::SelectionSet;
use crate::{
    app::App,
    buffer::BufferOwner,
    components::{
        editor::{DispatchEditor, IfCurrentNotFound},
        suggestive_editor::SuggestiveEditor,
    },
    context::GlobalMode,
    frontend::Frontend,
    selection::SelectionMode,
};
use itertools::Itertools;
use shared::absolute_path::AbsolutePath;
use std::any::TypeId;
use std::collections::HashSet;
use std::{cell::RefCell, rc::Rc};

pub struct GlobalMulticursor {
    files: Vec<GlobalMulticursorFile>,
    focused_file_index: usize,
}

pub struct EditorWithUpdatedSelectionSet {
    editor: Rc<RefCell<SuggestiveEditor>>,
    selection_set: SelectionSet,
}
impl GlobalMulticursor {
    pub fn editors(&self) -> Vec<Rc<RefCell<SuggestiveEditor>>> {
        self.files
            .iter()
            .map(|file| file.editor.clone())
            .collect_vec()
    }

    pub fn files(&self) -> &Vec<GlobalMulticursorFile> {
        &self.files
    }

    fn focused_file(&self) -> anyhow::Result<&GlobalMulticursorFile> {
        self.files.get(self.focused_file_index).ok_or_else(|| {
            anyhow::anyhow!(
                "Invariant violation: attempting to get index {} of the following list:\n\n{:?}",
                self.focused_file_index,
                self.files
                    .iter()
                    .map(|file| file.path.try_display_relative())
                    .collect_vec()
            )
        })
    }

    /// Returns true if the focused file is changed
    fn cycle_primary_cursor(&mut self, direction: &Direction) -> Result<bool, anyhow::Error> {
        if !self.current_selection_is_first_or_last_selection(direction)? {
            return Ok(false);
        }
        let change: isize = match direction {
            Direction::Start => -1,
            Direction::End => 1,
        };
        let next_file_index = (self.focused_file_index as isize + change)
            .rem_euclid(self.files().len() as isize) as usize;

        // Update the focused file index
        self.focused_file_index = next_file_index;

        // Ensure that the primary selection is either the first or last selection in the focused file
        {
            let mut editor_ref = self.focused_file()?.editor.borrow_mut();
            let editor = editor_ref.editor_mut();

            match direction {
                Direction::Start => editor.cycle_primary_selection_to_last(),
                Direction::End => editor.cycle_primary_selection_to_first(),
            }
        }

        #[cfg(test)]
        // Post condition assertion
        {
            let selection_set = self
                .focused_file()?
                .editor
                .borrow()
                .editor()
                .selection_set
                .clone();

            let primary_selection_range = selection_set.primary_selection().range;
            let mut ranges = selection_set
                .selections
                .iter()
                .map(|selection| selection.range)
                .sorted();

            match direction {
                // If changed to the previous file, expect the primary selection is the last selection
                Direction::Start => debug_assert_eq!(
                    Some(0),
                    ranges
                        .rev()
                        .position(|range| range == primary_selection_range)
                ),
                // If changed to the next file, expect the primary selection is the first selection
                Direction::End => {
                    debug_assert_eq!(
                        Some(0),
                        ranges.position(|range| range == primary_selection_range)
                    );
                }
            }
        }
        Ok(true)
    }

    fn current_selection_is_first_or_last_selection(
        &self,
        direction: &Direction,
    ) -> anyhow::Result<bool> {
        Ok(self
            .focused_file()?
            .editor
            .borrow()
            .editor()
            .current_selection_is_first_or_last_selection(direction))
    }

    /// Returns true if current focused file is removed
    fn delete_cursor(&mut self) -> Result<bool, anyhow::Error> {
        let focused_file = self.focused_file()?;
        if focused_file.editor.borrow().editor().selection_set.len() > 1 {
            return Ok(false);
        }

        if self.files.len() == 1 {
            return Ok(false);
        }

        self.files.remove(self.focused_file_index);

        let max_file_index = self.files.len().saturating_sub(1);

        if self.focused_file_index > max_file_index {
            self.focused_file_index = max_file_index;
        };

        Ok(true)
    }

    fn filter_cursor_matching_search(
        &mut self,
        search: String,
        maintain: bool,
    ) -> Result<Vec<EditorWithUpdatedSelectionSet>, anyhow::Error> {
        let mut result = vec![];
        let mut no_match_file_indices = vec![];

        // Removed the files that has no matching selections
        for (index, file) in self.files.iter_mut().enumerate() {
            let (no_matches, new_selection_set) = file
                .editor
                .borrow_mut()
                .editor_mut()
                .filter_selection_matching_search_impl(&search, maintain)?;

            if no_matches {
                no_match_file_indices.push(index);
            } else {
                result.push(EditorWithUpdatedSelectionSet {
                    editor: file.editor.clone(),
                    selection_set: new_selection_set,
                });
            }
        }
        for index in no_match_file_indices.into_iter().rev() {
            self.files.remove(index);
        }

        Ok(result)
    }

    pub fn focused_file_index(&self) -> usize {
        self.focused_file_index
    }

    pub fn ranges(&self) -> Vec<MultibufferRange> {
        self.files()
            .iter()
            .enumerate()
            .flat_map(|(file_index, file)| {
                let binding = file.editor.borrow();
                let selection_set = &binding.editor().selection_set;
                selection_set
                    .selections
                    .iter()
                    .enumerate()
                    .map(|(selection_index, selection)| {
                        let is_primary = file_index == self.focused_file_index
                            && selection_index == selection_set.cursor_index;
                        MultibufferRange {
                            path: file.path.clone(),
                            range: selection.extended_range(),
                            is_primary,
                        }
                    })
                    .collect_vec()
            })
            .collect_vec()
    }
}

#[derive(Clone)]
pub struct GlobalMulticursorFile {
    path: AbsolutePath,
    editor: Rc<RefCell<SuggestiveEditor>>,
}
impl GlobalMulticursorFile {
    pub(crate) fn to_multibuffer_path(&self) -> MultibufferFile {
        MultibufferFile {
            path: self.path.clone(),
            editor: self.editor.clone(),
        }
    }
}

impl<T: Frontend> App<T> {
    pub fn handle_dispatch_editor(
        &mut self,
        dispatch_editor: DispatchEditor,
    ) -> anyhow::Result<()> {
        match self.multibuffer.as_ref() {
            // Only multiplex the DispatchEditor to multiple editors
            // if the current focused component is a SuggestiveEditor
            Some(Multibuffer::GlobalMulticursor(glolbal_multicursor))
                if self.current_component().borrow().type_id()
                    == TypeId::of::<SuggestiveEditor>() =>
            {
                for component in glolbal_multicursor.editors().clone() {
                    self.handle_dispatch_editor_custom(dispatch_editor.clone(), component)?;
                }
                Ok(())
            }
            // Otherwise, just send the DispatchEditor to the current component
            _ => self.handle_dispatch_editor_custom(dispatch_editor, self.current_component()),
        }
    }

    fn activate_glolbal_multicursor(&mut self) -> anyhow::Result<()> {
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
                Ok(GlobalMulticursorFile { path, editor })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        if !files.is_empty() {
            self.multibuffer = Some(Multibuffer::GlobalMulticursor(GlobalMulticursor {
                // We'll just assume the first file is the focused file, for simplicity purposes
                files,
                focused_file_index: 0,
            }));

            self.set_global_mode(None)?;

            self.handle_dispatch_editor(DispatchEditor::SetSelectionMode(
                IfCurrentNotFound::LookForward,
                SelectionMode::LocalQuickfix {
                    title: self.quickfix_list().title(),
                },
            ))?;
            self.handle_dispatch_editor(DispatchEditor::CursorAddToAllSelections)?;

            // Close the quickfix list
            self.layout.remain_only_current_component();
        }

        Ok(())
    }

    pub fn add_cursor_to_all_selections(&mut self) -> anyhow::Result<()> {
        if self.context.mode() == Some(GlobalMode::QuickfixListItem) {
            self.activate_glolbal_multicursor()
        } else {
            self.handle_dispatch_editor(DispatchEditor::CursorAddToAllSelections)
        }
    }

    pub fn keep_primary_cursor_only(&mut self) -> anyhow::Result<()> {
        self.handle_dispatch_editor(DispatchEditor::CursorKeepPrimaryOnly)?;

        self.multibuffer.take();
        Ok(())
    }

    pub fn cycle_primary_cursor(&mut self, direction: Direction) -> anyhow::Result<()> {
        if let Some(Multibuffer::GlobalMulticursor(global_multicursor)) = self.multibuffer.as_mut()
        {
            let focused_file_changed = global_multicursor.cycle_primary_cursor(&direction)?;
            if focused_file_changed {
                return Ok(());
            }
        }

        self.handle_dispatch_editor(DispatchEditor::CyclePrimarySelection(direction))
    }

    pub fn delete_cursor(&mut self) -> anyhow::Result<()> {
        if let Some(Multibuffer::GlobalMulticursor(global_multicursor)) = self.multibuffer.as_mut()
        {
            let focused_file_removed = global_multicursor.delete_cursor()?;
            if focused_file_removed {
                return Ok(());
            }
        }

        self.handle_dispatch_editor(DispatchEditor::DeleteCurrentCursor(Direction::End))
    }

    pub fn filter_cursor_matching_search(
        &mut self,
        search: String,
        maintain: bool,
    ) -> anyhow::Result<()> {
        if self.multibuffer.is_some() {
            if let Some(Multibuffer::GlobalMulticursor(global_multicursor)) =
                self.multibuffer.as_mut()
            {
                let editors = global_multicursor.filter_cursor_matching_search(search, maintain)?;

                for EditorWithUpdatedSelectionSet {
                    editor: component,
                    selection_set: new_selection_set,
                } in editors
                {
                    let dispatches = component.borrow_mut().editor_mut().update_selection_set(
                        new_selection_set,
                        false,
                        &self.context,
                    );
                    self.handle_dispatches(dispatches)?;
                }
            }

            if let Some(Multibuffer::GlobalMulticursor(global_multicursor)) =
                self.multibuffer.as_ref()
            {
                // Focus the new first file, otherwise handle_key_event will malfunction
                let new_first_file = global_multicursor.focused_file()?.path.clone();
                self.open_file(&new_first_file, BufferOwner::User, true, true)?;
            }

            Ok(())
        } else {
            self.handle_dispatch_editor(DispatchEditor::FilterSelectionMatchingSearch {
                search,
                maintain,
            })
        }
    }

    #[cfg(test)]
    pub(crate) fn glolbal_multicursor_activated(&self) -> bool {
        self.multibuffer.is_some()
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        self.handle_dispatch_editor(DispatchEditor::Save)?;
        self.multibuffer.take();
        Ok(())
    }

    pub fn toggle_selection_mark(&mut self) -> anyhow::Result<()> {
        let paths = if let Some(Multibuffer::GlobalMulticursor(global_multicursor)) =
            self.multibuffer.as_ref()
        {
            global_multicursor
                .files()
                .iter()
                .map(|file| file.path.clone())
                .collect_vec()
        } else {
            self.get_current_file_path().into_iter().collect_vec()
        };

        for path in paths {
            self.toggle_selection_mark_by_path(&path)?;
        }

        // Refresh the quickfix list if possible
        // TODO: we need to maintain the last quickfix list index too
        match (self.context.quickfix_list().kind(), self.context.mode()) {
            (Some(QuickfixListKind::Mark), Some(GlobalMode::QuickfixListItem)) => {
                self.update_quickfix_list_item(QuickfixListType::Mark);
                self.render_quickfix_list()?;
                self.goto_quickfix_list_item(Movement::Current(IfCurrentNotFound::LookForward))?;
            }
            _ => {}
        }

        Ok(())
    }

    fn toggle_selection_mark_by_path(&mut self, path: &AbsolutePath) -> anyhow::Result<()> {
        // Mark the current file if we are actually marking some selection
        let marks: HashSet<_> = self
            .context
            .get_marks(Some(path.clone()))
            .into_iter()
            .collect();
        let component = self.open_file(path, BufferOwner::User, false, false)?;
        let current_selections: HashSet<_> = component
            .borrow()
            .editor()
            .selection_set
            .selections()
            .iter()
            .map(|selection| selection.extended_range())
            .collect();

        // If the set of current selections is the subset of the marks of this file
        // then this `toggle_selection_mark` action only consists of unmarking action,
        // and zero marking action.
        //
        // We only mark the current file if there are some marking actions
        if !current_selections.is_subset(&marks) {
            let _ = self.context.mark_file(path.clone());
        }
        let dispatches = component.borrow_mut().editor_mut().toggle_marks();

        self.handle_dispatches(dispatches)?;

        Ok(())
    }
}
