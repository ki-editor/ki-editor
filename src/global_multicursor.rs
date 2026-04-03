use crate::components::component::Component;
use crate::components::editor::Direction;
use crate::divide_viewport::divide_viewport;
use crate::{
    app::{App, Dimension},
    buffer::BufferOwner,
    components::{
        component::{ComponentId, GetGridResult, RenderTitleMode},
        editor::{DispatchEditor, IfCurrentNotFound, Reveal},
        suggestive_editor::SuggestiveEditor,
    },
    context::GlobalMode,
    frontend::Frontend,
    grid::Grid,
    rectangle::Rectangle,
    selection::SelectionMode,
};
use itertools::Itertools;
use shared::absolute_path::AbsolutePath;
use std::{cell::RefCell, rc::Rc};

pub struct GlobalMulticursor {
    pub editors: Vec<(AbsolutePath, Rc<RefCell<SuggestiveEditor>>)>,
}

impl<T: Frontend> App<T> {
    fn activate_glolbal_multicursor(&mut self) -> anyhow::Result<()> {
        let paths = self
            .quickfix_list()
            .items()
            .iter()
            .map(|item| item.location().path.clone())
            .unique()
            .collect_vec();

        let editors = paths
            .into_iter()
            .map(|path| -> anyhow::Result<_> {
                let editor = self.open_file(&path, BufferOwner::User, true, false)?;
                Ok((path, editor))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        if !editors.is_empty() {
            self.global_multicursor = Some(GlobalMulticursor { editors });

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
        if self.global_multicursor.is_some() {
            self.global_multicursor = None;
            Ok(())
        } else {
            self.handle_dispatch_editor(DispatchEditor::CursorKeepPrimaryOnly)
        }
    }

    pub fn cycle_primary_cursor(&mut self, direction: Direction) -> anyhow::Result<()> {
        todo!()
    }

    #[cfg(test)]
    pub(crate) fn glolbal_multicursor_activated(&self) -> bool {
        self.global_multicursor.is_some()
    }

    pub fn render_global_multicursor(
        &self,
        global_multicursor: &GlobalMulticursor,
        rectangle: &Rectangle,
        focused_component_id: &ComponentId,
    ) -> Option<GetGridResult> {
        let selections = global_multicursor
            .editors
            .iter()
            .flat_map(|(path, editor)| {
                editor
                    .borrow()
                    .editor()
                    .selection_set
                    .selections
                    .iter()
                    .map(|selection| (path.clone(), selection.clone()))
                    .collect_vec()
            })
            .collect_vec();
        let first_item = selections.first()?;
        let viewport_sections = divide_viewport(&selections, rectangle.height, first_item.clone());

        let editor_heights = viewport_sections
            .into_iter()
            // Sort and group by path
            .sorted_by_key(|section| section.item().0.clone())
            .chunk_by(|section| section.item().0.clone())
            .into_iter()
            .filter_map(|(path, sections)| {
                let editor = global_multicursor
                    .editors
                    .iter()
                    .find(|(p, _)| p == &path)?;
                Some((
                    editor,
                    sections
                        .into_iter()
                        .map(|section| section.height())
                        .sum::<usize>(),
                ))
            })
            .collect_vec();

        let get_grid_results = editor_heights
            .into_iter()
            .enumerate()
            .map(|(index, ((_, editor), height))| {
                let is_primary_buffer = &editor.borrow().id() == focused_component_id;
                editor
                    .borrow_mut()
                    .editor_mut()
                    .get_grid_with_custom_dimension(
                        &self.context,
                        index == 0,
                        Dimension {
                            height,
                            width: rectangle.width,
                        },
                        &Some(Reveal::Cursor),
                        &RenderTitleMode::Filename,
                        is_primary_buffer,
                    )
            })
            .collect_vec();

        let cursor = get_grid_results
            .first()
            .and_then(|result| result.cursor.clone());

        let grid = get_grid_results
            .into_iter()
            .fold(Grid::new(Dimension::default()), |grid, result| {
                grid.merge_vertical(result.grid)
            });

        Some(GetGridResult { grid, cursor })
    }
}
