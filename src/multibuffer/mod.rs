use crate::app::{App, Dimension};
mod global_multicursor;
mod global_reveal_selections;
use self::global_multicursor::GlobalMulticursor;
#[cfg(test)]
mod test_global_multicursor;
use self::global_reveal_selections::GlobalRevealSelections;
use crate::components::component::{Component, Cursor, RenderTitleMode, SetCursorStyle};
use crate::components::editor::Reveal;
use crate::components::suggestive_editor::SuggestiveEditor;
use crate::divide_viewport::divide_viewport;
use crate::grid::Grid;
use crate::{components::component::GetGridResult, frontend::Frontend, rectangle::Rectangle};
use itertools::Itertools;
use shared::absolute_path::AbsolutePath;
use std::cell::RefCell;
use std::rc::Rc;

pub enum Multibuffer {
    GlobalRevealSelections(GlobalRevealSelections),
    GlobalMulticursor(GlobalMulticursor),
}

impl Multibuffer {
    pub fn files(&self) -> Vec<MultibufferFile> {
        match self {
            Multibuffer::GlobalRevealSelections(global_reveal) => global_reveal.files.clone(),
            Multibuffer::GlobalMulticursor(global_multicursor) => global_multicursor
                .files()
                .iter()
                .map(|file| file.to_multibuffer_path())
                .collect_vec(),
        }
    }

    fn focused_file_index(&self) -> usize {
        match self {
            Multibuffer::GlobalRevealSelections(global_reveal) => global_reveal.focused_file_index,
            Multibuffer::GlobalMulticursor(global_multicursor) => {
                global_multicursor.focused_file_index()
            }
        }
    }

    fn reveal(&self) -> Reveal {
        match self {
            Multibuffer::GlobalRevealSelections(_) => Reveal::CurrentSelectionMode,
            Multibuffer::GlobalMulticursor(_) => Reveal::Cursor,
        }
    }

    #[cfg(test)]
    pub fn editors(&self) -> Vec<Rc<RefCell<SuggestiveEditor>>> {
        match self {
            Multibuffer::GlobalRevealSelections(global_reveal) => global_reveal.editors(),
            Multibuffer::GlobalMulticursor(global_multicursor) => global_multicursor.editors(),
        }
    }
}

#[derive(Clone)]
pub struct MultibufferFile {
    pub path: AbsolutePath,
    pub editor: Rc<RefCell<SuggestiveEditor>>,
}

impl<T: Frontend> App<T> {
    pub fn render_multibuffer(
        &self,
        multibuffer: &Multibuffer,
        rectangle: &Rectangle,
    ) -> Option<GetGridResult> {
        let files = multibuffer.files();
        let focused_file_index = multibuffer.focused_file_index();
        let reveal = multibuffer.reveal();
        let selections = files
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
                        let is_primary = file_index == focused_file_index
                            && selection_index == selection_set.cursor_index;
                        (is_primary, file.path.clone(), selection.clone())
                    })
                    .collect_vec()
            })
            .collect_vec();
        let focused_item = selections.iter().find(|(is_primary, _, _)| *is_primary)?;
        let viewport_sections =
            divide_viewport(&selections, rectangle.height, focused_item.clone());

        let file_with_heights = viewport_sections
            .into_iter()
            // Sort and group by path
            .sorted_by_key(|section| section.item().1.clone())
            .chunk_by(|section| section.item().1.clone())
            .into_iter()
            .filter_map(|(path, sections)| {
                let file = files.iter().find(|file| file.path == path)?;
                Some((
                    file,
                    sections
                        .into_iter()
                        .map(|section| section.height())
                        .sum::<usize>(),
                ))
            })
            .collect_vec();
        let focused_file_path = &files.get(focused_file_index)?.path;
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
                        &Some(reveal.clone()),
                        &RenderTitleMode::Filename,
                    );

                GetGridResult {
                    cursor: if is_focused { result.cursor } else { None },
                    ..result
                }
            })
            .collect_vec();

        let cursor_style = get_grid_results
            .iter()
            .find_map(|result| result.cursor.as_ref().map(|cursor| *cursor.style()));

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
