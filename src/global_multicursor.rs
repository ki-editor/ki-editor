use crate::components::component::{Component, Cursor, SetCursorStyle};
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
    focused_file: GlobalMulticursorFile,
    other_files: Vec<GlobalMulticursorFile>,
}
impl GlobalMulticursor {
    pub fn editors(&self) -> Vec<Rc<RefCell<SuggestiveEditor>>> {
        Some(self.focused_file.editor.clone())
            .into_iter()
            .chain(self.other_files.iter().map(|file| file.editor.clone()))
            .collect_vec()
    }

    fn files(&self) -> Vec<GlobalMulticursorFile> {
        Some(self.focused_file.clone())
            .into_iter()
            .chain(self.other_files.iter().map(|file| file.clone()))
            .collect_vec()
    }
}

#[derive(Clone)]
struct GlobalMulticursorFile {
    path: AbsolutePath,
    editor: Rc<RefCell<SuggestiveEditor>>,
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
                Ok(GlobalMulticursorFile { path, editor })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        if let Some((head, tail)) = editors.split_first() {
            self.global_multicursor = Some(GlobalMulticursor {
                // We'll just assume the first file is the focused file, for simplicity purposes
                focused_file: head.clone(),
                other_files: tail.into_iter().map(|file| file.clone()).collect(),
            });

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
        if let Some(global_multicursor) = self.global_multicursor.as_mut() {
            if global_multicursor
                .focused_file
                .editor
                .borrow()
                .editor()
                .is_at_first_or_last_selection(&direction)
            {
                let next_path = {
                    let mut sorted = global_multicursor
                        .other_files
                        .iter()
                        .sorted_by_key(|file| file.path.clone());
                    match direction {
                        Direction::Start => sorted
                            .rev()
                            .find(|file| file.path < global_multicursor.focused_file.path)
                            .map(|f| f.path.clone()),
                        Direction::End => sorted
                            .find(|file| file.path > global_multicursor.focused_file.path)
                            .map(|f| f.path.clone()),
                    }
                };

                if let Some(path) = next_path {
                    if let Some(idx) = global_multicursor
                        .other_files
                        .iter()
                        .position(|f| f.path == path)
                    {
                        let new_focused_file = global_multicursor.other_files.remove(idx);
                        let old_focused_file = std::mem::replace(
                            &mut global_multicursor.focused_file,
                            new_focused_file,
                        );
                        global_multicursor.other_files.push(old_focused_file);

                        #[cfg(test)]
                        {
                            let selection_set = global_multicursor
                                .focused_file
                                .editor
                                .borrow()
                                .editor()
                                .selection_set
                                .clone();

                            let primary_selection_range = selection_set.primary_selection().range;
                            let mut ranges = selection_set
                                .selections
                                .iter()
                                .map(|selection| selection.range.clone())
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
                                    )
                                }
                            }
                        }
                        return Ok(());
                    }
                }
            }
        }

        self.handle_dispatch_editor(DispatchEditor::CyclePrimarySelection(direction))
    }

    #[cfg(test)]
    pub(crate) fn glolbal_multicursor_activated(&self) -> bool {
        self.global_multicursor.is_some()
    }

    pub fn render_global_multicursor(
        &self,
        global_multicursor: &GlobalMulticursor,
        rectangle: &Rectangle,
    ) -> Option<GetGridResult> {
        let files = global_multicursor.files();
        let selections = files
            .iter()
            .flat_map(|file| {
                file.editor
                    .borrow()
                    .editor()
                    .selection_set
                    .selections
                    .iter()
                    .map(|selection| (file.path.clone(), selection.clone()))
                    .collect_vec()
            })
            .collect_vec();
        let first_item = selections.first()?;
        let viewport_sections = divide_viewport(&selections, rectangle.height, first_item.clone());

        let file_with_heights = viewport_sections
            .into_iter()
            // Sort and group by path
            .sorted_by_key(|section| section.item().0.clone())
            .chunk_by(|section| section.item().0.clone())
            .into_iter()
            .filter_map(|(path, sections)| {
                let file = files.iter().find(|file| &file.path == &path)?;
                Some((
                    file,
                    sections
                        .into_iter()
                        .map(|section| section.height())
                        .sum::<usize>(),
                ))
            })
            .collect_vec();

        let get_grid_results = file_with_heights
            .into_iter()
            .map(|(file, height)| {
                let is_focused = &file.path == &global_multicursor.focused_file.path;
                let result = file
                    .editor
                    .borrow_mut()
                    .editor_mut()
                    .get_grid_with_custom_dimension(
                        &self.context,
                        is_focused,
                        Dimension {
                            height,
                            width: rectangle.width,
                        },
                        &Some(Reveal::Cursor),
                        &RenderTitleMode::Filename,
                        is_focused,
                    );

                GetGridResult {
                    cursor: if is_focused { result.cursor } else { None },
                    ..result
                }
            })
            .collect_vec();

        let cursor_style = get_grid_results
            .iter()
            .find_map(|result| result.cursor.as_ref().map(|cursor| cursor.style().clone()));

        let grid = get_grid_results
            .into_iter()
            .fold(Grid::new(Dimension::default()), |grid, result| {
                grid.merge_vertical(result.grid)
            });

        let cursor = grid.get_cursor_position();

        Some(GetGridResult {
            grid,
            cursor: cursor.map(|position| {
                Cursor::new(
                    position,
                    cursor_style.unwrap_or(SetCursorStyle::BlinkingBar),
                )
            }),
        })
    }
}
