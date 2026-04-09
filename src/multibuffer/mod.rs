use crate::app::{App, Dimension};
use crate::char_index_range::CharIndexRange;
use crate::ui_tree::ComponentKind;
mod global_multicursor;
mod global_reveal;
use self::global_multicursor::GlobalMulticursor;
#[cfg(test)]
mod test_global_multicursor;
#[cfg(test)]
mod test_global_reveal;
use self::global_reveal::GlobalReveal;
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
    GlobalReveal(GlobalReveal),
    GlobalMulticursor(GlobalMulticursor),
}

impl Multibuffer {
    pub fn files(&self) -> Vec<MultibufferFile> {
        match self {
            Multibuffer::GlobalReveal(global_reveal) => global_reveal
                .files
                .iter()
                .map(|file| file.to_multibuffer_path())
                .collect_vec(),
            Multibuffer::GlobalMulticursor(global_multicursor) => global_multicursor
                .files()
                .iter()
                .map(|file| file.to_multibuffer_path())
                .collect_vec(),
        }
    }

    fn focused_file_index(&self, current_file_path: &AbsolutePath) -> Option<usize> {
        match self {
            Multibuffer::GlobalReveal(global_reveal) => {
                global_reveal.focused_file_index(current_file_path)
            }
            Multibuffer::GlobalMulticursor(global_multicursor) => {
                Some(global_multicursor.focused_file_index())
            }
        }
    }

    fn reveal(&self) -> Reveal {
        match self {
            Multibuffer::GlobalReveal(_) => Reveal::CurrentSelectionMode,
            Multibuffer::GlobalMulticursor(_) => Reveal::Cursor,
        }
    }

    #[cfg(test)]
    pub fn editors(&self) -> Vec<Rc<RefCell<SuggestiveEditor>>> {
        match self {
            Multibuffer::GlobalReveal(global_reveal) => global_reveal.editors(),
            Multibuffer::GlobalMulticursor(global_multicursor) => global_multicursor.editors(),
        }
    }

    fn ranges(&self, current_file_path: &AbsolutePath) -> Vec<MultibufferRange> {
        match self {
            Multibuffer::GlobalReveal(global_reveal) => global_reveal.ranges(current_file_path),
            Multibuffer::GlobalMulticursor(global_multicursor) => global_multicursor.ranges(),
        }
    }

    pub(crate) fn display_name(&self) -> &'static str {
        match self {
            Multibuffer::GlobalReveal(_) => "[GLOBAL REVEAL]",
            Multibuffer::GlobalMulticursor(_) => "[GLOBAL MULTICURSOR]",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultibufferRange {
    path: AbsolutePath,
    range: CharIndexRange,
    is_primary: bool,
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
        let current_file_path = self
            .layout
            .get_component_by_kind(ComponentKind::SuggestiveEditor)?
            .borrow()
            .path()?;
        let focused_file_index = multibuffer.focused_file_index(&current_file_path)?;
        let reveal = multibuffer.reveal();
        let ranges = multibuffer.ranges(&current_file_path);
        let focused_item = ranges.iter().find(|range| range.is_primary)?;
        let viewport_sections = divide_viewport(&ranges, rectangle.height, focused_item.clone());

        let file_with_heights = viewport_sections
            .into_iter()
            // Sort and group by path
            .sorted_by_key(|section| section.item().path.clone())
            .chunk_by(|section| section.item().path.clone())
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

                // Only show cursors of each file for Global Multicursor (where Reveal is Reveal::Cursor),
                // but don't show cursors of every file for Global Reveal
                let show_cursors = is_focused || reveal == Reveal::Cursor;
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
                        show_cursors,
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
