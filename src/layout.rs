use crate::quickfix_list::QuickfixList;
use crate::ui_tree::{ComponentKind, KindedComponent, UiTree};
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
use nary_tree::NodeId;
use shared::canonicalized_path::CanonicalizedPath;
use std::{cell::RefCell, rc::Rc};

/// The layout of the app is split into multiple sections: the main panel, info panel, quickfix
/// lists, prompts, and etc.
/// The main panel is where the user edits code, and the info panel is for displaying info like
/// hover text, diagnostics, etc.
pub struct Layout {
    background_suggestive_editors: IndexMap<CanonicalizedPath, Rc<RefCell<SuggestiveEditor>>>,
    background_file_explorer: Rc<RefCell<FileExplorer>>,
    background_quickfix_list: Option<Rc<RefCell<Editor>>>,

    rectangles: Vec<Rectangle>,
    borders: Vec<Border>,

    terminal_dimension: Dimension,
    working_directory: CanonicalizedPath,

    tree: UiTree,
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
        let tree = UiTree::new();
        Ok(Layout {
            background_quickfix_list: None,
            background_suggestive_editors: IndexMap::new(),
            background_file_explorer: Rc::new(RefCell::new(FileExplorer::new(working_directory)?)),
            rectangles,
            borders,
            terminal_dimension,
            working_directory: working_directory.clone(),
            tree,
        })
    }

    pub fn components(&self) -> Vec<KindedComponent> {
        self.tree.components()
    }

    pub fn get_current_component(&self) -> Rc<RefCell<dyn Component>> {
        self.get_component(self.tree.focused_component_id())
    }

    fn get_component(&self, id: NodeId) -> Rc<RefCell<dyn Component>> {
        self.tree
            .get(id)
            .map(|node| node.data().component())
            .unwrap_or_else(|| self.tree.root().data().component().clone())
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

    pub fn remove_current_component(&mut self) {
        let node = self.tree.get_current_node();
        if let Some(path) = node.data().component().borrow().path() {
            self.background_suggestive_editors.shift_remove(&path);
            if let Some((_, editor)) = self
                .background_suggestive_editors
                .iter()
                .skip_while(|(p, _)| p != &&path)
                .nth(1)
                .or_else(|| self.background_suggestive_editors.first())
            {
                self.replace_and_focus_current_suggestive_editor(editor.clone())
            } else {
                self.tree.remove(node.node_id());
                self.cycle_window()
            }
        } else {
            self.tree.remove(node.node_id());
            self.cycle_window()
        };

        self.recalculate_layout();
    }

    pub fn cycle_window(&mut self) {
        self.tree.cycle_component()
    }

    pub fn close_current_window(&mut self, change_focused_to: Option<ComponentId>) {
        self.remove_current_component();
    }

    pub fn add_and_focus_prompt(&mut self, kind: ComponentKind, component: Rc<RefCell<Prompt>>) {
        self.tree
            .append_component_to_current(KindedComponent::new(kind, component), true);
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
                component
                    .component()
                    .borrow_mut()
                    .set_rectangle(rectangle.clone())
            });
    }

    pub fn get_existing_editor(
        &self,
        path: &CanonicalizedPath,
    ) -> Option<Rc<RefCell<SuggestiveEditor>>> {
        self.background_suggestive_editors.get(path).cloned()
    }

    pub fn open_file(
        &mut self,
        path: &CanonicalizedPath,
        focus_editor: bool,
    ) -> Option<Rc<RefCell<SuggestiveEditor>>> {
        if let Some(matching_editor) = self.get_existing_editor(path) {
            if focus_editor {
                self.replace_and_focus_current_suggestive_editor(matching_editor.clone());
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

    pub fn focused_component_id(&self) -> ComponentId {
        self.tree.current_component().borrow().id()
    }

    pub fn borders(&self) -> Vec<Border> {
        self.borders.clone()
    }

    pub fn add_suggestive_editor(&mut self, suggestive_editor: Rc<RefCell<SuggestiveEditor>>) {
        let path = suggestive_editor.borrow().path();
        if let Some(path) = path {
            self.background_suggestive_editors
                .insert(path, suggestive_editor);
        }
    }

    pub fn get_suggestive_editor(
        &self,
        component_id: ComponentId,
    ) -> Result<Rc<RefCell<SuggestiveEditor>>, anyhow::Error> {
        self.background_suggestive_editors
            .iter()
            .find(|(_, editor)| editor.borrow().id() == component_id)
            .map(|(_, editor)| editor.clone())
            .ok_or_else(|| anyhow!("Couldn't find component with id {:?}", component_id))
    }

    fn show_info_on(
        &mut self,
        node_id: NodeId,
        info: Info,
        kind: ComponentKind,
    ) -> anyhow::Result<()> {
        let info_panel = Rc::new(RefCell::new(Editor::from_text(
            tree_sitter_md::language(),
            "",
        )));
        info_panel.borrow_mut().show_info(info)?;
        self.tree
            .replace_node_child(node_id, kind, info_panel, false);
        Ok(())
    }

    pub fn show_global_info(&mut self, info: Info) -> anyhow::Result<()> {
        self.show_info_on(self.tree.root_id(), info, ComponentKind::GlobalInfo)
    }

    pub fn show_keymap_legend(&mut self, keymap_legend_config: KeymapLegendConfig) {
        self.tree.append_component_to_current(
            KindedComponent::new(
                ComponentKind::KeymapLegend,
                Rc::new(RefCell::new(KeymapLegend::new(keymap_legend_config))),
            ),
            true,
        )
    }

    pub fn close_all_except_main_panel(&mut self) {
        self.tree.remove_all_except_current()
    }

    pub fn get_opened_files(&self) -> Vec<CanonicalizedPath> {
        self.background_suggestive_editors
            .iter()
            .map(|(path, _)| path.clone())
            .collect()
    }

    pub fn save_all(&self) -> Result<(), anyhow::Error> {
        self.background_suggestive_editors
            .iter()
            .map(|(_, editor)| editor.borrow_mut().editor_mut().save())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }

    pub fn reveal_path_in_explorer(
        &mut self,
        path: &CanonicalizedPath,
    ) -> anyhow::Result<Dispatches> {
        let dispatches = self.background_file_explorer.borrow_mut().reveal(path)?;
        self.open_file_explorer();

        Ok(dispatches)
    }

    pub fn remove_suggestive_editor(&mut self, path: &CanonicalizedPath) {
        self.background_suggestive_editors.shift_remove(path);
    }

    pub fn refresh_file_explorer(
        &self,
        working_directory: &CanonicalizedPath,
    ) -> anyhow::Result<()> {
        self.background_file_explorer
            .borrow_mut()
            .refresh(working_directory)
    }

    pub fn open_file_explorer(&mut self) {
        self.tree.append_component_to_root(
            KindedComponent::new(
                ComponentKind::FileExplorer,
                self.background_file_explorer.clone(),
            ),
            true,
        );
    }

    pub fn update_highlighted_spans(
        &self,
        component_id: ComponentId,
        highlighted_spans: crate::syntax_highlight::HighlighedSpans,
    ) -> Result<(), anyhow::Error> {
        let component = self
            .background_suggestive_editors
            .iter()
            .find(|(_, component)| component.borrow().id() == component_id)
            .map(|(_, component)| component)
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
            .map(|(_, editor)| editor.borrow().editor().buffer_rc())
            .collect_vec()
    }

    pub fn open_file_with_selection(
        &mut self,
        path: &CanonicalizedPath,
        selection_set: SelectionSet,
    ) -> anyhow::Result<()> {
        if let Some(editor) = self.open_file(path, true) {
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
    pub(crate) fn current_completion_dropdown(&self) -> Option<Rc<RefCell<dyn Component>>> {
        self.get_current_node_child_id(ComponentKind::Dropdown)
            .and_then(|mut node_id| Some(self.tree.get(node_id)?.data().component().clone()))
    }

    pub(crate) fn open_dropdown(&mut self, owner_id: ComponentId) -> Option<Rc<RefCell<Editor>>> {
        let dropdown = Rc::new(RefCell::new(Editor::from_text(
            tree_sitter_md::language(),
            "",
        )));
        // Dropdown can only be rendered if the current node is SuggestiveEditor or Prompt
        if !matches!(
            self.tree.get_current_node().data().kind(),
            ComponentKind::SuggestiveEditor | ComponentKind::Prompt
        ) {
            return None;
        }
        self.tree
            .replace_current_node_child(ComponentKind::Dropdown, dropdown.clone(), false);
        self.recalculate_layout(); // This is important to give Dropdown the render area, otherwise during render, height 0 is assume, causing weird behavior when scrolling
        Some(dropdown)
    }

    /// `id` is either the `id` of the dropdown or of its owner
    pub(crate) fn close_dropdown(&mut self, id: ComponentId) {
        self.tree.remove_current_child(ComponentKind::Dropdown);
    }

    pub(crate) fn close_editor_info(&mut self) {
        self.tree.remove_current_child(ComponentKind::EditorInfo);
    }

    fn get_current_node_child_id(&self, kind: ComponentKind) -> Option<NodeId> {
        self.tree.get_current_node_child_id(kind)
    }

    fn remove_node_child(
        &mut self,
        node_id: NodeId,
        kind: ComponentKind,
    ) -> Option<KindedComponent> {
        self.tree.remove_node_child(node_id, kind)
    }

    pub(crate) fn show_dropdown_info(
        &mut self,
        owner_id: ComponentId,
        info: Info,
    ) -> anyhow::Result<()> {
        if let Some(node_id) = self.tree.get_current_node_child_id(ComponentKind::Dropdown) {
            self.show_info_on(node_id, info, ComponentKind::DropdownInfo)?;
        }

        Ok(())
    }

    pub(crate) fn hide_dropdown_info(&mut self, owner_id: ComponentId) {
        if let Some(node_id) = self.get_current_node_child_id(ComponentKind::Dropdown) {
            self.remove_node_child(node_id, ComponentKind::DropdownInfo);
        }
    }

    pub(crate) fn get_component_by_id(
        &self,
        id: &ComponentId,
    ) -> Option<Rc<RefCell<dyn Component>>> {
        Some(
            self.components()
                .into_iter()
                .find(|c| &c.component().borrow().id() == id)?
                .component(),
        )
    }

    pub(crate) fn show_quickfix_list(
        &mut self,
        quickfix_list: QuickfixList,
    ) -> anyhow::Result<Dispatches> {
        let render = quickfix_list.render();
        let editor = self.background_quickfix_list.get_or_insert_with(|| {
            Rc::new(RefCell::new(Editor::from_text(
                tree_sitter_md::language(),
                "",
            )))
        });
        let node_id =
            self.tree
                .replace_root_node_child(ComponentKind::QuickfixList, editor.clone(), false);

        let dispatches = {
            let mut editor = editor.borrow_mut();
            editor.set_content(&render.content)?;
            editor.set_decorations(&render.decorations);
            editor.set_title("Quickfix list".to_string());
            editor.select_line_at(render.highlight_line_index)?
        };
        if let Some(info) = render.info {
            self.show_info_on(node_id, info, ComponentKind::QuickfixListInfo)?;
        }
        Ok(dispatches)
    }

    #[cfg(test)]
    pub(crate) fn get_dropdown_infos_count(&self) -> usize {
        self.tree.count_by_kind(ComponentKind::DropdownInfo)
    }

    pub(crate) fn show_editor_info(
        &mut self,
        owner_id: ComponentId,
        info: Info,
    ) -> anyhow::Result<()> {
        self.show_info_on(
            self.tree.focused_component_id(),
            info,
            ComponentKind::EditorInfo,
        )
    }

    fn replace_node_child(
        &mut self,
        id: NodeId,
        kind: ComponentKind,
        component: Rc<RefCell<dyn Component>>,
        focus: bool,
    ) {
        self.tree.replace_node_child(id, kind, component, focus);
    }

    #[cfg(test)]
    pub(crate) fn editor_info_open(&self) -> bool {
        self.tree.count_by_kind(ComponentKind::EditorInfo) > 0
    }

    #[cfg(test)]
    pub(crate) fn editor_info_content(&self) -> Option<String> {
        Some(
            self.tree
                .root()
                .traverse_pre_order()
                .find(|node| node.data().kind() == ComponentKind::EditorInfo)?
                .data()
                .component()
                .borrow()
                .content(),
        )
    }

    #[cfg(test)]
    pub(crate) fn file_explorer_content(&self) -> String {
        self.background_file_explorer.borrow().content()
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

    pub fn replace_and_focus_current_suggestive_editor(
        &mut self,
        editor: Rc<RefCell<SuggestiveEditor>>,
    ) {
        self.add_suggestive_editor(editor.clone());
        self.replace_node_child(
            self.tree.root_id(),
            ComponentKind::SuggestiveEditor,
            editor,
            true,
        );
    }

    pub(crate) fn close_current_window_and_focus_parent(&mut self) {
        self.tree.close_current_and_focus_parent()
    }

    pub(crate) fn quickfix_list_info(&self) -> Option<String> {
        Some(
            self.tree
                .get_component_by_kind(ComponentKind::QuickfixListInfo)?
                .borrow()
                .content(),
        )
    }

    pub(crate) fn get_component_by_kind(
        &self,
        kind: ComponentKind,
    ) -> Option<Rc<RefCell<dyn Component>>> {
        self.tree.get_component_by_kind(kind)
    }

    pub(crate) fn hide_editor_info(&mut self) {
        self.tree.remove_current_child(ComponentKind::EditorInfo);
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
