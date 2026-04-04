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
    files: Vec<GlobalMulticursorFile>,
    focused_file_index: usize,
}
impl GlobalMulticursor {
    pub fn editors(&self) -> Vec<Rc<RefCell<SuggestiveEditor>>> {
        self.files
            .iter()
            .map(|file| file.editor.clone())
            .collect_vec()
    }

    fn files(&self) -> &Vec<GlobalMulticursorFile> {
        &self.files
    }

    fn focused_file(&self) -> anyhow::Result<&GlobalMulticursorFile> {
        self.files.get(self.focused_file_index).ok_or_else(|| {
            anyhow::anyhow!(
                "Invariant violation: attempting to get index {} of the following list:\n{:?}",
                self.focused_file_index,
                self.files
                    .iter()
                    .map(|file| file.path.try_display_relative())
                    .collect_vec()
            )
        })
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
            self.global_multicursor = Some(GlobalMulticursor {
                // We'll just assume the first file is the focused file, for simplicity purposes
                files: files,
                focused_file_index: 0,
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
            let focused_file = global_multicursor.focused_file()?;
            let current_selection_is_first_or_last_selection = {
                focused_file
                    .editor
                    .borrow()
                    .editor()
                    .current_selection_is_first_or_last_selection(&direction)
            };
            if current_selection_is_first_or_last_selection {
                let change: isize = match direction {
                    Direction::Start => -1,
                    Direction::End => 1,
                };
                let next_file_index = ((global_multicursor.focused_file_index as isize + change)
                    as usize)
                    .rem_euclid(global_multicursor.files().len());

                // Update the focused file index
                global_multicursor.focused_file_index = next_file_index;

                // Ensure that the primary selection is either the first or last selection in the focused file
                {
                    let mut editor_ref = global_multicursor.focused_file()?.editor.borrow_mut();
                    let editor = editor_ref.editor_mut();

                    match direction {
                        Direction::Start => editor.cycle_primary_selection_to_last(),
                        Direction::End => editor.cycle_primary_selection_to_first(),
                    }
                }

                #[cfg(test)]
                {
                    let selection_set = global_multicursor
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
        let focused_file_path = &global_multicursor.focused_file().ok()?.path;
        let get_grid_results = file_with_heights
            .into_iter()
            .map(|(file, height)| {
                let is_focused = &file.path == focused_file_path;
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
