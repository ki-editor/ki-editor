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
use nary_tree::{NodeId, RemoveBehavior, Tree};
use shared::canonicalized_path::CanonicalizedPath;
use std::{any::TypeId, cell::RefCell, rc::Rc};

struct Owned<T> {
    owner_id: ComponentId,
    component: T,
}

/// The layout of the app is split into multiple sections: the main panel, info panel, quickfix
/// lists, prompts, and etc.
/// The main panel is where the user edits code, and the info panel is for displaying info like
/// hover text, diagnostics, etc.
pub struct Layout {
    background_suggestive_editors: Vec<Rc<RefCell<SuggestiveEditor>>>,
    file_explorer: Rc<RefCell<FileExplorer>>,
    quickfix_list: Option<Rc<RefCell<Editor>>>,

    rectangles: Vec<Rectangle>,
    borders: Vec<Border>,

    focused_component_id: Option<NodeId>,

    terminal_dimension: Dimension,
    working_directory: CanonicalizedPath,

    tree: Tree<KindedComponent>,
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
        let mut untitled_editor = SuggestiveEditor::from_text(tree_sitter_md::language(), "");
        untitled_editor.set_title("[Untitled]".to_string());
        let mut tree = Tree::new();
        let component = Rc::new(RefCell::new(untitled_editor));
        let root_id = tree.set_root(KindedComponent::new(
            ComponentKind::SuggestiveEditor,
            component.clone(),
        ));
        Ok(Layout {
            focused_component_id: Some(root_id),
            quickfix_list: None,
            background_suggestive_editors: vec![component],
            file_explorer: Rc::new(RefCell::new(FileExplorer::new(working_directory)?)),
            rectangles,
            borders,
            terminal_dimension,
            working_directory: working_directory.clone(),
            tree,
        })
    }

    pub fn components(&self) -> Vec<Rc<RefCell<dyn Component>>> {
        self.tree
            .root()
            .map(|root| {
                root.traverse_pre_order()
                    .map(|node| node.data().component.clone())
                    .collect_vec()
            })
            .unwrap_or_default()
    }

    pub fn current_component(&self) -> Option<Rc<RefCell<dyn Component>>> {
        self.focused_component_id
            .and_then(|id| self.get_component(id))
    }

    pub(crate) fn get_current_component(&self) -> Option<Rc<RefCell<dyn Component>>> {
        self.focused_component_id
            .and_then(|id| self.get_component(id))
    }

    fn get_component(&self, id: NodeId) -> Option<Rc<RefCell<dyn Component>>> {
        self.tree.get(id).map(|node| node.data().component.clone())
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
        self.focused_component_id.map(|id| {
            if let Some(node) = self.tree.get(id) {
                self.focused_component_id = node.parent().map(|parent| parent.node_id());
                self.tree
                    .remove(node.node_id(), RemoveBehavior::DropChildren);
            }
        });

        if self.focused_component_id.is_some() {
            self.recalculate_layout();
            false
        } else {
            true
        }
    }

    pub fn change_view(&mut self) {
        self.focused_component_id = self
            .focused_component_id
            .and_then(|id| self.tree.get_mut(id))
            .and_then(|mut node| {
                node.next_sibling()
                    .map(|node| node.node_id())
                    .or_else(|| Some(node.parent()?.node_id()))
                    .or_else(|| Some(node.first_child()?.node_id()))
            })
            .or_else(|| self.focused_component_id);
    }

    pub fn close_current_window(&mut self, change_focused_to: Option<ComponentId>) {
        self.remove_current_component();
    }

    pub fn add_and_focus_component(
        &mut self,
        kind: ComponentKind,
        component: Rc<RefCell<dyn Component>>,
    ) {
        self.focused_component_id
            .or_else(|| self.tree.root_id())
            .and_then(|id| self.tree.get_mut(id))
            .map(|mut current_node| {
                let new_node = current_node.append(KindedComponent::new(kind, component));
                self.focused_component_id = Some(new_node.node_id())
            });
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
                self.focused_component_id =
                    self.replace_current_suggestive_editor(matching_editor.clone());
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
        Some(self.get_current_component()?.borrow().id())
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
        self.add_and_focus_component(ComponentKind::SuggestiveEditor, suggestive_editor);
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
        let info_panel = Rc::new(RefCell::new(Editor::from_text(
            tree_sitter_md::language(),
            "",
        )));
        info_panel.borrow_mut().show_info(info)?;
        self.tree.root_mut().map(|mut root| {
            root.append(KindedComponent::new(ComponentKind::Info, info_panel));
        });

        Ok(())
    }

    pub fn show_keymap_legend(
        &mut self,
        keymap_legend_config: KeymapLegendConfig,
    ) -> anyhow::Result<()> {
        let mut node = self
            .focused_component_id
            .and_then(|id| self.tree.get_mut(id))
            .ok_or_else(|| {
                anyhow!("Unable to show keymap legend because there is no owner can be found.")
            })?;
        let keymap_legend = KeymapLegend::new(keymap_legend_config);
        let new_node = node.append(KindedComponent::new(
            ComponentKind::KeymapLegend,
            Rc::new(RefCell::new(keymap_legend)),
        ));
        self.focused_component_id = Some(new_node.node_id());
        Ok(())
    }

    pub fn close_all_except_main_panel(&mut self) {
        if let Some(component) = self
            .focused_component_id
            .and_then(|id| self.tree.get(id).map(|node| node.data().clone()))
        {
            self.tree.set_root(component);
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
        self.open_file_explorer();

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
        self.tree.root_mut().map(|mut root| {
            let new_node = root.append(KindedComponent::new(
                ComponentKind::FileExplorer,
                self.file_explorer.clone(),
            ));
            self.focused_component_id = Some(new_node.node_id())
        });
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
            .and_then(|mut node_id| Some(self.tree.get(node_id)?.data().component.clone()))
    }

    pub(crate) fn open_dropdown(&mut self, owner_id: ComponentId) -> Rc<RefCell<Editor>> {
        let dropdown = Rc::new(RefCell::new(Editor::from_text(
            tree_sitter_md::language(),
            "",
        )));
        self.focused_component_id
            .and_then(|id| self.replace_node_child(id, ComponentKind::Dropdown, dropdown.clone()));
        self.recalculate_layout(); // This is important to give Dropdown the render area, otherwise during render, height 0 is assume, causing weird behavior when scrolling
        dropdown
    }

    /// `id` is either the `id` of the dropdown or of its owner
    pub(crate) fn close_dropdown(&mut self, id: ComponentId) {
        if let Some(node_id) = self.get_current_node_child_id(ComponentKind::Dropdown) {
            self.tree.remove(node_id, RemoveBehavior::DropChildren);
        }
    }

    fn get_current_node_child_id(&self, kind: ComponentKind) -> Option<NodeId> {
        self.get_node_child_id_by_kind(self.focused_component_id?, kind)
    }

    fn get_node_child_id_by_kind(&self, node_id: NodeId, kind: ComponentKind) -> Option<NodeId> {
        Some(
            self.tree
                .get(node_id)?
                .traverse_pre_order()
                .find(|node| node.node_id() != node_id && node.data().kind == kind)?
                .node_id(),
        )
    }

    fn remove_node_child<'a>(
        &mut self,
        node_id: NodeId,
        kind: ComponentKind,
    ) -> Option<KindedComponent> {
        self.get_node_child_id_by_kind(node_id, kind)
            .and_then(|child_id| self.tree.remove(child_id, RemoveBehavior::DropChildren))
    }

    pub(crate) fn show_dropdown_info(
        &mut self,
        owner_id: ComponentId,
        info: Info,
    ) -> anyhow::Result<()> {
        if let Some(mut node_id) = self.get_current_node_child_id(ComponentKind::Dropdown) {
            let mut editor = Editor::from_text(tree_sitter_md::language(), "");
            editor.show_info(info)?;
            self.replace_node_child(
                node_id,
                ComponentKind::DropdownInfo,
                Rc::new(RefCell::new(editor)),
            );
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
        self.components()
            .into_iter()
            .find(|c| &c.borrow().id() == id)
    }

    pub(crate) fn open_component_with_selection(
        &mut self,
        id: &ComponentId,
        selection_set: SelectionSet,
    ) {
        log::info!("This is not implemented yet!")
    }

    pub(crate) fn show_quickfix_list(&mut self) -> Rc<RefCell<Editor>> {
        let quickfix_list = if let Some(quickfix_list) = self.quickfix_list.clone() {
            quickfix_list.clone()
        } else {
            let editor = Rc::new(RefCell::new(Editor::from_text(
                tree_sitter_md::language(),
                "",
            )));
            self.quickfix_list = Some(editor.clone());
            editor.clone()
        };
        self.tree.root_mut().map(|mut root| {
            root.append(KindedComponent::new(
                ComponentKind::QuickfixList,
                quickfix_list.clone(),
            ));
        });
        quickfix_list
    }

    #[cfg(test)]
    pub(crate) fn get_dropdown_infos_count(&self) -> usize {
        self.tree
            .root()
            .map(|root| {
                root.traverse_pre_order()
                    .filter(|node| node.data().kind == ComponentKind::DropdownInfo)
                    .count()
            })
            .unwrap_or_default()
    }

    pub(crate) fn show_editor_info(
        &mut self,
        owner_id: ComponentId,
        info: Info,
    ) -> anyhow::Result<()> {
        let mut editor = Editor::from_text(tree_sitter_md::language(), info.content());
        editor.show_info(info)?;
        self.focused_component_id.map(|id| {
            self.replace_node_child(id, ComponentKind::EditorInfo, Rc::new(RefCell::new(editor)));
        });
        Ok(())
    }

    fn replace_node_child(
        &mut self,
        id: NodeId,
        kind: ComponentKind,
        component: Rc<RefCell<dyn Component>>,
    ) -> Option<NodeId> {
        self.remove_node_child(id, kind);
        self.append_node_child(id, KindedComponent::new(kind, component))
    }

    #[cfg(test)]
    pub(crate) fn editor_info_open(&self) -> bool {
        self.tree
            .root()
            .map(|root| {
                root.traverse_pre_order()
                    .filter(|node| node.data().kind == ComponentKind::EditorInfo)
                    .count()
            })
            .unwrap_or_default()
            > 0
    }

    #[cfg(test)]
    pub(crate) fn editor_info_content(&self) -> Option<String> {
        Some(
            self.tree
                .root()?
                .traverse_pre_order()
                .find(|node| node.data().kind == ComponentKind::EditorInfo)?
                .data()
                .component
                .borrow()
                .content(),
        )
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

    fn replace_current_suggestive_editor(
        &mut self,
        editor: Rc<RefCell<SuggestiveEditor>>,
    ) -> Option<NodeId> {
        self.tree.root_id().and_then(|root_id| {
            self.replace_node_child(root_id, ComponentKind::SuggestiveEditor, editor)
        })
    }

    fn append_node_child(&mut self, id: NodeId, component: KindedComponent) -> Option<NodeId> {
        self.tree
            .get_mut(id)
            .map(|mut node| node.append(component).node_id())
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

#[derive(Clone)]
struct KindedComponent {
    component: Rc<RefCell<dyn Component>>,
    kind: ComponentKind,
}
impl KindedComponent {
    fn new(kind: ComponentKind, component: Rc<RefCell<dyn Component>>) -> KindedComponent {
        Self { kind, component }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ComponentKind {
    Dropdown,
    SuggestiveEditor,
    Info,
    DropdownInfo,
    KeymapLegend,
    FileExplorer,
    Prompt,
    QuickfixList,
    EditorInfo,
}
