use crate::{
    app::{Dimension, Dispatches},
    buffer::Buffer,
    components::{
        component::{Component, ComponentId},
        editor::Editor,
        file_explorer::FileExplorer,
        keymap_legend::{KeymapLegend, KeymapLegendConfig},
        prompt::Prompt,
        suggestive_editor::{Info, SuggestiveEditor},
    },
    context::QuickfixListSource,
    quickfix_list::{Location, QuickfixListItem},
    rectangle::{Border, LayoutKind, Rectangle},
    selection::SelectionSet,
};
use anyhow::anyhow;
use indexmap::IndexMap;
use itertools::Itertools;
use shared::canonicalized_path::CanonicalizedPath;
use std::{cell::RefCell, rc::Rc};

/// The layout of the app is split into multiple sections: the main panel, info panel, quickfix
/// lists, prompts, and etc.
/// The main panel is where the user edits code, and the info panel is for displaying info like
/// hover text, diagnostics, etc.
pub struct Layout {
    main_panel: MainPanel,
    global_info: Option<Rc<RefCell<Editor>>>,
    keymap_legend: Vec<Rc<RefCell<KeymapLegend>>>,
    quickfix_list: Option<Rc<RefCell<Editor>>>,
    prompts: Vec<Rc<RefCell<Prompt>>>,
    background_suggestive_editors: Vec<Rc<RefCell<SuggestiveEditor>>>,
    file_explorer: Rc<RefCell<FileExplorer>>,
    dropdowns: IndexMap</*Owner ID*/ ComponentId, Rc<RefCell<Editor>>>,
    dropdown_infos: IndexMap</*Owner ID*/ ComponentId, Rc<RefCell<Editor>>>,
    editor_infos: IndexMap</*Owner ID*/ ComponentId, Rc<RefCell<Editor>>>,

    file_explorer_open: bool,

    rectangles: Vec<Rectangle>,
    borders: Vec<Border>,

    focused_component_id: Option<ComponentId>,

    terminal_dimension: Dimension,
    working_directory: CanonicalizedPath,
}

#[derive(Clone)]
struct MainPanel {
    editor: Option<Rc<RefCell<SuggestiveEditor>>>,
    working_directory: CanonicalizedPath,
}

impl PartialEq for MainPanel {
    fn eq(&self, other: &Self) -> bool {
        match (&self.editor, &other.editor) {
            (Some(a), Some(b)) => a.borrow().path() == b.borrow().path(),
            (None, None) => true,
            _ => false,
        }
    }
}

impl MainPanel {
    fn path(&self) -> Option<CanonicalizedPath> {
        self.editor
            .as_ref()
            .and_then(|editor| editor.borrow().path())
    }

    fn take(&mut self) -> Option<Rc<RefCell<SuggestiveEditor>>> {
        self.editor.take()
    }

    fn id(&self) -> Option<ComponentId> {
        self.editor.as_ref().map(|editor| editor.borrow().id())
    }
}

impl std::fmt::Display for MainPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = self.path() {
            f.write_str(
                &path
                    .display_relative_to(&self.working_directory)
                    .unwrap_or_else(|_| path.display_absolute()),
            )
        } else {
            f.write_str("[UNTITLED]")
        }
    }
}

impl Layout {
    pub fn new(
        terminal_dimension: Dimension,
        working_directory: &CanonicalizedPath,
    ) -> anyhow::Result<Layout> {
        let (layout_kind, ratio) = layout_kind(&terminal_dimension);
        let (rectangles, borders) = Rectangle::generate(layout_kind, 1, ratio, terminal_dimension);
        Ok(Layout {
            main_panel: MainPanel {
                editor: None,
                working_directory: working_directory.clone(),
            },
            global_info: None,
            keymap_legend: vec![],
            quickfix_list: None,
            prompts: vec![],
            focused_component_id: Some(ComponentId::new()),
            background_suggestive_editors: vec![],
            file_explorer: Rc::new(RefCell::new(FileExplorer::new(working_directory)?)),
            rectangles,
            borders,
            terminal_dimension,
            file_explorer_open: false,
            working_directory: working_directory.clone(),
            dropdowns: IndexMap::new(),
            dropdown_infos: IndexMap::new(),
            editor_infos: IndexMap::new(),
        })
    }

    pub fn components(&self) -> Vec<Rc<RefCell<dyn Component>>> {
        let root_components = vec![]
            .into_iter()
            .chain(
                self.main_panel
                    .editor
                    .iter()
                    .map(|c| c.clone() as Rc<RefCell<dyn Component>>),
            )
            .chain(
                if self.file_explorer_open {
                    Some(self.file_explorer.clone())
                } else {
                    None
                }
                .iter()
                .map(|c| c.clone() as Rc<RefCell<dyn Component>>),
            )
            .chain(
                self.keymap_legend
                    .iter()
                    .map(|c| c.clone() as Rc<RefCell<dyn Component>>),
            )
            .chain(
                self.prompts
                    .iter()
                    .map(|c| c.clone() as Rc<RefCell<dyn Component>>),
            )
            .chain(
                self.dropdowns
                    .iter()
                    .map(|(_, c)| c.clone() as Rc<RefCell<dyn Component>>),
            )
            .chain(
                self.dropdown_infos
                    .iter()
                    .map(|(_, c)| c.clone() as Rc<RefCell<dyn Component>>),
            )
            .chain(
                self.editor_infos
                    .iter()
                    .map(|(_, c)| c.clone() as Rc<RefCell<dyn Component>>),
            )
            .chain(
                self.global_info
                    .iter()
                    .map(|c| c.clone() as Rc<RefCell<dyn Component>>),
            )
            .chain(
                self.quickfix_list
                    .iter()
                    .map(|c| c.clone() as Rc<RefCell<dyn Component>>),
            )
            .collect::<Vec<_>>();

        let mut components = root_components.clone();
        for component in root_components.iter() {
            components.extend(component.borrow().descendants());
        }

        components
    }

    pub fn current_component(&self) -> Option<Rc<RefCell<dyn Component>>> {
        self.focused_component_id
            .and_then(|id| self.get_component(id))
    }

    pub(crate) fn get_current_suggestive_editor(&self) -> Option<Rc<RefCell<SuggestiveEditor>>> {
        self.focused_component_id.and_then(|id| {
            self.background_suggestive_editors
                .iter()
                .find(|editor| editor.borrow().id() == id)
                .cloned()
        })
    }

    fn get_component(&self, id: ComponentId) -> Option<Rc<RefCell<dyn Component>>> {
        self.components()
            .into_iter()
            .find(|c| c.borrow().id() == id)
    }

    fn new_main_panel(
        &self,
        suggestive_editor: Option<Rc<RefCell<SuggestiveEditor>>>,
    ) -> MainPanel {
        MainPanel {
            editor: suggestive_editor,
            working_directory: self.working_directory.clone(),
        }
    }

    /// Return true if there's no more windows
    pub fn remove_current_component(&mut self) -> bool {
        if let Some(id) = self.focused_component_id {
            self.prompts.retain(|c| c.borrow().id() != id);

            let main_panel = self.main_panel.take();
            self.main_panel = self.new_main_panel(
                main_panel
                    .filter(|c| c.borrow().id() != id)
                    .or_else(|| self.background_suggestive_editors.last().cloned()),
            );

            self.keymap_legend.retain(|c| c.borrow().id() != id);

            self.global_info = self.global_info.take().filter(|c| c.borrow().id() != id);

            self.quickfix_list = self.quickfix_list.take().filter(|c| c.borrow().id() != id);

            self.close_editor_info(id);
            self.background_suggestive_editors
                .retain(|c| c.borrow().id() != id);

            self.close_dropdown(id);

            if self.file_explorer.borrow().id() == id {
                self.file_explorer_open = false
            }

            self.components().into_iter().for_each(|c| {
                c.borrow_mut().remove_child(id);
            });
        };

        if let Some(component) = self.components().last() {
            self.focused_component_id = Some(component.borrow().id());
            self.recalculate_layout();
            false
        } else {
            true
        }
    }

    fn set_main_panel(&mut self, new: MainPanel) {
        self.focused_component_id = new.id();
        self.main_panel = new;
    }

    pub fn change_view(&mut self) {
        let components = self.components();
        if let Some(component) = components
            .iter()
            .sorted_by_key(|component| component.borrow().id())
            .find(|component| {
                if let Some(id) = self.focused_component_id {
                    component.borrow().id() > id
                } else {
                    true
                }
            })
            .map_or_else(
                || {
                    components
                        .iter()
                        .min_by(|x, y| x.borrow().id().cmp(&y.borrow().id()))
                },
                Some,
            )
        {
            self.focused_component_id = Some(component.borrow().id())
        }
    }

    pub fn close_current_window(&mut self, change_focused_to: Option<ComponentId>) {
        self.remove_current_component();
        self.focused_component_id = change_focused_to;
    }

    pub fn add_and_focus_prompt(&mut self, prompt: Rc<RefCell<Prompt>>) {
        self.focused_component_id = Some(prompt.borrow().id());
        self.prompts.push(prompt);
        self.recalculate_layout();
    }

    pub fn recalculate_layout(&mut self) {
        let (layout_kind, ratio) = layout_kind(&self.terminal_dimension);

        let (rectangles, borders) = Rectangle::generate(
            layout_kind,
            self.components().len(),
            ratio,
            self.terminal_dimension,
        );
        self.rectangles = rectangles;
        self.borders = borders;

        self.components()
            .into_iter()
            .zip(self.rectangles.iter())
            .for_each(|(component, rectangle)| {
                component.borrow_mut().set_rectangle(rectangle.clone())
            });
    }

    pub fn get_existing_editor(
        &self,
        path: &CanonicalizedPath,
    ) -> Option<Rc<RefCell<SuggestiveEditor>>> {
        self.background_suggestive_editors
            .iter()
            .find(|&component| {
                component
                    .borrow()
                    .editor()
                    .buffer()
                    .path()
                    .map(|p| &p == path)
                    .unwrap_or(false)
            })
            .cloned()
    }

    pub fn open_file(
        &mut self,
        path: &CanonicalizedPath,
        focus_editor: bool,
    ) -> Option<Rc<RefCell<SuggestiveEditor>>> {
        if let Some(matching_editor) = self.get_existing_editor(path) {
            if focus_editor {
                self.set_main_panel(self.new_main_panel(Some(matching_editor.clone())));
            }
            Some(matching_editor)
        } else {
            None
        }
    }

    pub fn set_terminal_dimension(&mut self, dimension: Dimension) {
        self.terminal_dimension = dimension;
        self.recalculate_layout()
    }

    pub fn terminal_dimension(&self) -> Dimension {
        self.terminal_dimension
    }

    pub fn focused_component_id(&self) -> Option<ComponentId> {
        self.focused_component_id
    }

    pub fn borders(&self) -> Vec<Border> {
        self.borders.clone()
    }

    pub fn add_suggestive_editor(&mut self, suggestive_editor: Rc<RefCell<SuggestiveEditor>>) {
        self.background_suggestive_editors.push(suggestive_editor);
    }

    pub fn add_and_focus_suggestive_editor(
        &mut self,
        suggestive_editor: Rc<RefCell<SuggestiveEditor>>,
    ) {
        self.add_suggestive_editor(suggestive_editor.clone());

        self.set_main_panel(self.new_main_panel(Some(suggestive_editor)));
    }

    pub fn get_suggestive_editor(
        &self,
        component_id: ComponentId,
    ) -> Result<Rc<RefCell<SuggestiveEditor>>, anyhow::Error> {
        self.background_suggestive_editors
            .iter()
            .find(|editor| editor.borrow().id() == component_id)
            .cloned()
            .ok_or_else(|| anyhow!("Couldn't find component with id {:?}", component_id))
    }

    pub fn show_info(&mut self, info: Info) -> anyhow::Result<()> {
        let info_panel = self.global_info.take().unwrap_or_else(|| {
            Rc::new(RefCell::new(Editor::from_text(
                tree_sitter_md::language(),
                "",
            )))
        });
        info_panel.borrow_mut().show_info(info)?;
        self.global_info = Some(info_panel);

        Ok(())
    }

    pub fn show_keymap_legend(&mut self, keymap_legend_config: KeymapLegendConfig) {
        let keymap_legend = KeymapLegend::new(keymap_legend_config);
        self.focused_component_id = Some(keymap_legend.id());
        self.keymap_legend
            .push(Rc::new(RefCell::new(keymap_legend)));
    }

    pub fn close_all_except_main_panel(&mut self) {
        self.global_info = None;
        self.keymap_legend = vec![];
        self.quickfix_list = None;
        self.prompts = vec![];
        if let Some(id) = self.focused_component_id {
            self.close_dropdown(id);
            self.close_editor_info(id);
        } else {
            self.focused_component_id = self
                .background_suggestive_editors
                .last()
                .map(|editor| editor.borrow().id())
        }
    }

    pub fn get_opened_files(&self) -> Vec<CanonicalizedPath> {
        self.background_suggestive_editors
            .iter()
            .filter_map(|editor| editor.borrow().editor().buffer().path())
            .collect()
    }

    pub fn save_all(&self) -> Result<(), anyhow::Error> {
        self.background_suggestive_editors
            .iter()
            .map(|editor| editor.borrow_mut().editor_mut().save())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }

    pub fn reveal_path_in_explorer(
        &mut self,
        path: &CanonicalizedPath,
    ) -> anyhow::Result<Dispatches> {
        let dispatches = self.file_explorer.borrow_mut().reveal(path)?;

        self.focused_component_id = Some(self.file_explorer.borrow().id());
        self.file_explorer_open = true;

        Ok(dispatches)
    }

    pub fn remove_suggestive_editor(&mut self, path: &CanonicalizedPath) {
        self.background_suggestive_editors
            .retain(|suggestive_editor| {
                suggestive_editor.borrow().editor().buffer().path().as_ref() != Some(path)
            })
    }

    pub fn refresh_file_explorer(
        &self,
        working_directory: &CanonicalizedPath,
    ) -> anyhow::Result<()> {
        self.file_explorer.borrow_mut().refresh(working_directory)
    }

    pub fn open_file_explorer(&mut self) {
        self.file_explorer_open = true
    }

    pub fn update_highlighted_spans(
        &self,
        component_id: ComponentId,
        highlighted_spans: crate::syntax_highlight::HighlighedSpans,
    ) -> Result<(), anyhow::Error> {
        let component = self
            .background_suggestive_editors
            .iter()
            .find(|component| component.borrow().id() == component_id)
            .ok_or_else(|| anyhow!("Couldn't find component with id {:?}", component_id))?;

        let mut component = component.borrow_mut();
        component
            .editor_mut()
            .buffer_mut()
            .update_highlighted_spans(highlighted_spans);

        Ok(())
    }

    pub(crate) fn buffers(&self) -> Vec<Rc<RefCell<Buffer>>> {
        self.background_suggestive_editors
            .iter()
            .map(|editor| editor.borrow().editor().buffer_rc())
            .collect_vec()
    }

    pub fn open_file_with_selection(
        &mut self,
        path: &CanonicalizedPath,
        selection_set: SelectionSet,
    ) -> anyhow::Result<()> {
        self.open_file(path, true);
        if let Some(editor) = self.main_panel.editor.clone() {
            editor
                .borrow_mut()
                .editor_mut()
                .__update_selection_set_for_real(selection_set);
        }
        Ok(())
    }

    pub(crate) fn reload_buffers(
        &self,
        affected_paths: Vec<CanonicalizedPath>,
    ) -> anyhow::Result<()> {
        for buffer in self.buffers() {
            let mut buffer = buffer.borrow_mut();
            if let Some(path) = buffer.path() {
                if affected_paths
                    .iter()
                    .any(|affected_path| affected_path == &path)
                {
                    buffer.reload()?;
                }
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn completion_dropdown_is_open(&self) -> bool {
        self.current_completion_dropdown().is_some()
    }

    #[cfg(test)]
    pub(crate) fn current_completion_dropdown(&self) -> Option<Rc<RefCell<Editor>>> {
        self.current_component()
            .and_then(|c| self.dropdowns.get(&c.borrow().id()).cloned())
    }

    pub(crate) fn open_dropdown(&mut self, owner_id: ComponentId) -> Rc<RefCell<Editor>> {
        let dropdown = Rc::new(RefCell::new(Editor::from_text(
            tree_sitter_md::language(),
            "",
        )));
        self.dropdowns.insert(owner_id, dropdown.clone());
        self.recalculate_layout(); // This is important to give Dropdown the render area, otherwise during render, height 0 is assume, causing weird behavior when scrolling
        dropdown
    }

    /// `id` is either the `id` of the dropdown or of its owner
    pub(crate) fn close_dropdown(&mut self, id: ComponentId) {
        self.dropdowns
            .retain(|owner_id, c| owner_id != &id && c.borrow().id() != id);
        self.dropdown_infos
            .retain(|owner_id, c| owner_id != &id && c.borrow().id() != id);
    }

    pub(crate) fn show_dropdown_info(
        &mut self,
        owner_id: ComponentId,
        info: Info,
    ) -> anyhow::Result<()> {
        let editor = self
            .dropdown_infos
            .shift_remove(&owner_id)
            .unwrap_or_else(|| {
                Rc::new(RefCell::new(Editor::from_text(
                    tree_sitter_md::language(),
                    "",
                )))
            });
        editor.borrow_mut().show_info(info)?;
        self.dropdown_infos.insert(owner_id, editor);
        Ok(())
    }

    pub(crate) fn hide_dropdown_info(&mut self, owner_id: ComponentId) {
        self.dropdown_infos.shift_remove(&owner_id);
    }

    pub(crate) fn get_component_by_id(
        &self,
        id: &ComponentId,
    ) -> Option<Rc<RefCell<dyn Component>>> {
        self.components()
            .into_iter()
            .find(|c| &c.borrow().id() == id)
    }

    pub(crate) fn open_component_with_selection(
        &mut self,
        id: &ComponentId,
        selection_set: SelectionSet,
    ) {
        if let Some(component) = self.get_component_by_id(id) {
            component
                .borrow_mut()
                .editor_mut()
                .__update_selection_set_for_real(selection_set);
            self.focused_component_id = Some(component.borrow().id())
        }
    }

    pub(crate) fn show_quickfix_list(&mut self) -> Rc<RefCell<Editor>> {
        if let Some(quickfix_list) = self.quickfix_list.clone() {
            quickfix_list.clone()
        } else {
            let editor = Rc::new(RefCell::new(Editor::from_text(
                tree_sitter_md::language(),
                "",
            )));
            self.quickfix_list = Some(editor.clone());
            editor.clone()
        }
    }

    #[cfg(test)]
    pub(crate) fn get_dropdown_infos_count(&self) -> usize {
        self.dropdown_infos.len()
    }

    pub(crate) fn show_editor_info(
        &mut self,
        owner_id: ComponentId,
        info: Info,
    ) -> anyhow::Result<()> {
        let mut editor = Editor::from_text(tree_sitter_md::language(), info.content());
        editor.show_info(info)?;
        self.editor_infos
            .insert(owner_id, Rc::new(RefCell::new(editor)));
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn editor_info_open(&self) -> bool {
        !self.editor_infos.is_empty()
    }

    pub(crate) fn close_editor_info(&mut self, id: ComponentId) {
        self.editor_infos
            .retain(|owner_id, c| owner_id != &id && c.borrow().id() != id);
    }

    #[cfg(test)]
    pub(crate) fn editor_info_content(&self) -> Option<String> {
        Some(self.editor_infos.first()?.1.borrow().content())
    }

    #[cfg(test)]
    pub(crate) fn file_explorer_content(&self) -> String {
        self.file_explorer.borrow().content()
    }

    pub(crate) fn get_quickfix_list_items(
        &self,
        source: &QuickfixListSource,
    ) -> Vec<QuickfixListItem> {
        self.buffers()
            .into_iter()
            .flat_map(|buffer| {
                let buffer = buffer.borrow();
                match source {
                    QuickfixListSource::Diagnostic(severity_range) => buffer
                        .diagnostics()
                        .into_iter()
                        .filter_map(|diagnostic| {
                            if !severity_range.contains(diagnostic.severity) {
                                return None;
                            }

                            let position_range = buffer
                                .char_index_range_to_position_range(diagnostic.range)
                                .ok()?;
                            Some(QuickfixListItem::new(
                                Location {
                                    path: buffer.path()?,
                                    range: position_range,
                                },
                                Some(Info::new(
                                    "Diagnostics".to_string(),
                                    diagnostic.message.clone(),
                                )),
                            ))
                        })
                        .collect_vec(),
                    QuickfixListSource::Bookmark => buffer
                        .bookmarks()
                        .into_iter()
                        .filter_map(|bookmark| {
                            let position_range =
                                buffer.char_index_range_to_position_range(bookmark).ok()?;
                            Some(QuickfixListItem::new(
                                Location {
                                    path: buffer.path()?,
                                    range: position_range,
                                },
                                None,
                            ))
                        })
                        .collect_vec(),
                    QuickfixListSource::Custom => buffer.quickfix_list_items(),
                }
            })
            .collect_vec()
    }

    pub(crate) fn clear_quickfix_list_items(&mut self) {
        for buffer in self.buffers() {
            buffer.borrow_mut().clear_quickfix_list_items()
        }
    }
}
fn layout_kind(terminal_dimension: &Dimension) -> (LayoutKind, f32) {
    const MAIN_PANEL_MIN_WIDTH: u16 = 100;
    const RIGHT_PANEL_MIN_WIDTH: u16 = 60;
    if terminal_dimension.width > MAIN_PANEL_MIN_WIDTH + RIGHT_PANEL_MIN_WIDTH {
        (LayoutKind::Tall, 0.55)
    } else {
        (LayoutKind::Wide, 0.65)
    }
}
