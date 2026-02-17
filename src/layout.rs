use crate::app::Dispatch;
use crate::components::keymap_legend::ReleaseKey;
use crate::config::from_extension;
use crate::context::Context;
use crate::quickfix_list::QuickfixList;
use crate::syntax_highlight::SyntaxHighlightRequestBatchId;
use crate::ui_tree::{ComponentKind, KindedComponent, UiTree};
use crate::{
    app::{Dimension, Dispatches},
    buffer::{Buffer, BufferOwner},
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
};
use anyhow::anyhow;
use indexmap::IndexMap;
use itertools::Itertools;
use nary_tree::NodeId;
use shared::absolute_path::AbsolutePath;
use std::{cell::RefCell, rc::Rc};

#[cfg(test)]
pub type BufferContentsMap = std::collections::HashMap<String, String>;

/// The layout of the app is split into multiple sections: the main panel, info panel, quickfix
/// lists, prompts, and etc.
/// The main panel is where the user edits code, and the info panel is for displaying info like
/// hover text, diagnostics, etc.
pub struct Layout {
    background_suggestive_editors: IndexMap<AbsolutePath, Rc<RefCell<SuggestiveEditor>>>,
    background_file_explorer: Rc<RefCell<FileExplorer>>,
    background_quickfix_list: Option<Rc<RefCell<Editor>>>,

    rectangles: Vec<Rectangle>,
    borders: Vec<Border>,

    terminal_dimension: Dimension,
    tree: UiTree,
}

impl Layout {
    pub fn new(
        terminal_dimension: Dimension,
        working_directory: &AbsolutePath,
    ) -> anyhow::Result<Layout> {
        let (layout_kind, ratio) = layout_kind();
        let (rectangles, borders) = Rectangle::generate(layout_kind, 1, ratio, terminal_dimension);
        let tree = UiTree::new();
        Ok(Layout {
            background_quickfix_list: None,
            background_suggestive_editors: IndexMap::new(),
            background_file_explorer: Rc::new(RefCell::new(FileExplorer::new(working_directory)?)),
            rectangles,
            borders,
            terminal_dimension,
            tree,
        })
    }

    pub fn components(&self) -> Vec<KindedComponent> {
        self.tree.components()
    }

    pub fn get_current_component(&self) -> Rc<RefCell<dyn Component>> {
        self.get_component(self.tree.focused_component_id())
    }

    pub fn get_current_component_kind(&self) -> Option<ComponentKind> {
        self.tree
            .get(self.tree.focused_component_id())
            .map(|node| node.data().kind())
    }

    pub fn get_component(&self, id: NodeId) -> Rc<RefCell<dyn Component>> {
        self.tree
            .get(id)
            .map(|node| node.data().component())
            .unwrap_or_else(|| self.tree.root().data().component().clone())
    }

    pub fn remove_current_component(&mut self, context: &Context) -> Option<AbsolutePath> {
        let node = self.tree.get_current_node();
        let removed_path = node.data().component().borrow().path();
        if let Some(path) = &removed_path {
            self.background_suggestive_editors.shift_remove(path);
            if let Some((_, editor)) = self
                .background_suggestive_editors
                .iter()
                .find(|(_, editor)| editor.borrow().editor().buffer().owner() == BufferOwner::User)
            {
                self.replace_and_focus_current_suggestive_editor(editor.clone());
            } else {
                self.tree.remove(node.node_id(), true);
            }
        } else {
            self.tree.remove(node.node_id(), true);
        };

        self.recalculate_layout(context);
        removed_path
    }

    pub fn cycle_window(&mut self) {
        self.tree.cycle_component();
    }

    pub fn close_current_window(&mut self, context: &Context) -> Option<AbsolutePath> {
        self.remove_current_component(context)
    }

    pub fn add_and_focus_prompt(
        &mut self,
        kind: ComponentKind,
        component: Rc<RefCell<Prompt>>,
        context: &Context,
    ) {
        self.tree
            .append_component_to_current(KindedComponent::new(kind, component), true);
        self.recalculate_layout(context);
    }

    pub fn recalculate_layout(&mut self, context: &Context) {
        let (layout_kind, ratio) = layout_kind();

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
                    .set_rectangle(rectangle.clone(), context);
            });
    }

    pub fn get_existing_editor(
        &self,
        path: &AbsolutePath,
    ) -> Option<Rc<RefCell<SuggestiveEditor>>> {
        self.background_suggestive_editors.get(path).cloned()
    }

    pub fn open_file(
        &mut self,
        path: &AbsolutePath,
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

    pub fn set_terminal_dimension(&mut self, dimension: Dimension, context: &Context) {
        self.terminal_dimension = dimension;
        self.recalculate_layout(context);
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

    fn show_info_on(
        &mut self,
        node_id: NodeId,
        info: Info,
        kind: ComponentKind,
        context: &Context,
    ) -> anyhow::Result<()> {
        let info_panel = Rc::new(RefCell::new(Editor::from_text(None, "")));
        info_panel.borrow_mut().show_info(info, context)?;
        self.tree
            .replace_node_child(node_id, kind, info_panel, false);
        Ok(())
    }

    pub fn show_global_info(&mut self, info: Info, context: &Context) -> anyhow::Result<()> {
        self.show_info_on(
            self.tree.root_id(),
            info,
            ComponentKind::GlobalInfo,
            context,
        )
    }

    pub fn show_keymap_legend(
        &mut self,
        keymap_legend_config: KeymapLegendConfig,
        context: &Context,
        release_key: Option<ReleaseKey>,
    ) {
        self.tree.append_component_to_current(
            KindedComponent::new(
                ComponentKind::KeymapLegend,
                Rc::new(RefCell::new(KeymapLegend::new(
                    keymap_legend_config,
                    context,
                    release_key,
                ))),
            ),
            true,
        );
    }

    pub fn remain_only_current_component(&mut self) {
        self.tree.remain_only_current_component();
    }

    pub fn get_opened_files(&self) -> Vec<AbsolutePath> {
        self.background_suggestive_editors
            .iter()
            .filter(|(_, editor)| editor.borrow().editor().buffer().owner() == BufferOwner::User)
            .map(|(path, _)| path.clone())
            .collect()
    }

    #[cfg(test)]
    pub fn get_buffer_contents_map(&self) -> BufferContentsMap {
        self.background_suggestive_editors
            .iter()
            .map(|(path, editor)| {
                (
                    path.file_name().unwrap_or_default(),
                    editor.borrow().editor().buffer().content(),
                )
            })
            .collect()
    }

    pub fn save_all(&self, context: &Context) -> Result<(), anyhow::Error> {
        self.background_suggestive_editors
            .iter()
            .map(|(_, editor)| editor.borrow_mut().editor_mut().save(context))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }

    pub fn reveal_path_in_explorer(
        &mut self,
        path: &AbsolutePath,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        let result = self
            .background_file_explorer
            .borrow_mut()
            .reveal(path, context);

        // We will render the file explorer regardless
        // of whether the reveal succeeded or not,
        // so that the users will not see a blank file explorer
        // when the reveal failed.

        self.open_file_explorer();
        let dispatches = result.unwrap_or_else(|error| {
            Dispatches::one(Dispatch::ShowGlobalInfo(Info::new(
                "Reveal File Error".to_string(),
                format!(
                    "Unable to reveal '{}' due to the following error:\n\n{error}",
                    path.try_display_relative_to(context.current_working_directory())
                ),
            )))
        });

        Ok(dispatches)
    }

    pub fn remove_suggestive_editor(&mut self, path: &AbsolutePath) {
        self.background_suggestive_editors.shift_remove(path);
    }

    pub fn refresh_file_explorer(&self, context: &Context) -> anyhow::Result<()> {
        self.background_file_explorer.borrow_mut().refresh(context)
    }

    pub fn open_file_explorer(&mut self) {
        self.tree.remove_all_root_children();
        self.tree.replace_root_node_child(
            ComponentKind::FileExplorer,
            self.background_file_explorer.clone(),
            true,
        );
        debug_assert_eq!(self.tree.root().children().count(), 1);
    }

    pub fn update_highlighted_spans(
        &self,
        component_id: ComponentId,
        batch_id: SyntaxHighlightRequestBatchId,
        highlighted_spans: crate::syntax_highlight::HighlightedSpans,
    ) -> Result<(), anyhow::Error> {
        let component = match &self.background_quickfix_list {
            Some(component) if component.borrow().id() == component_id => {
                Box::new(component.clone() as Rc<RefCell<dyn Component>>)
            }
            _ => self
                .background_suggestive_editors
                .iter()
                .find(|(_, component)| component.borrow().id() == component_id)
                .map(|(_, component)| Box::new(component.clone() as Rc<RefCell<dyn Component>>))
                .ok_or_else(|| anyhow!("Couldn't find component with id {:?}", component_id))?,
        };

        let mut component = component.borrow_mut();
        component
            .editor_mut()
            .buffer_mut()
            .update_highlighted_spans(batch_id, highlighted_spans);

        Ok(())
    }

    pub fn buffers(&self) -> Vec<Rc<RefCell<Buffer>>> {
        self.background_suggestive_editors
            .iter()
            .map(|(_, editor)| editor.borrow().editor().buffer_rc())
            .collect_vec()
    }

    pub fn reload_buffers(&self, affected_paths: Vec<AbsolutePath>) -> anyhow::Result<Dispatches> {
        self.buffers()
            .into_iter()
            .try_fold(Dispatches::default(), |dispatches, buffer| {
                let mut buffer = buffer.borrow_mut();
                if let Some(path) = buffer.path() {
                    if affected_paths
                        .iter()
                        .any(|affected_path| affected_path == &path)
                    {
                        return Ok(dispatches.chain(buffer.reload(true)?));
                    }
                }
                Ok(dispatches)
            })
    }

    #[cfg(test)]
    pub fn completion_dropdown_is_open(&self) -> bool {
        self.current_completion_dropdown().is_some()
    }

    pub fn current_completion_dropdown(&self) -> Option<Rc<RefCell<dyn Component>>> {
        self.get_current_node_child_id(ComponentKind::Dropdown)
            .and_then(|node_id| Some(self.tree.get(node_id)?.data().component().clone()))
    }

    pub fn open_dropdown(&mut self, context: &Context) -> Option<Rc<RefCell<Editor>>> {
        let dropdown = Rc::new(RefCell::new(Editor::from_text(
            Some(tree_sitter_quickfix::language()),
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
        self.recalculate_layout(context); // This is important to give Dropdown the render area, otherwise during render, height 0 is assume, causing weird behavior when scrolling
        Some(dropdown)
    }

    pub fn close_dropdown(&mut self) {
        self.tree.remove_current_child(ComponentKind::Dropdown);
    }

    pub fn close_editor_info(&mut self) {
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

    pub fn show_dropdown_info(&mut self, info: Info, context: &Context) -> anyhow::Result<()> {
        if let Some(node_id) = self.tree.get_current_node_child_id(ComponentKind::Dropdown) {
            self.show_info_on(node_id, info, ComponentKind::DropdownInfo, context)?;
        }

        Ok(())
    }

    pub fn hide_dropdown_info(&mut self) {
        if let Some(node_id) = self.get_current_node_child_id(ComponentKind::Dropdown) {
            self.remove_node_child(node_id, ComponentKind::DropdownInfo);
        }
    }

    pub fn show_quickfix_list(
        &mut self,
        quickfix_list: &QuickfixList,
        context: &Context,
    ) -> anyhow::Result<(Rc<RefCell<Editor>>, Dispatches)> {
        let render = quickfix_list.render();
        let editor = self.background_quickfix_list.get_or_insert_with(|| {
            Rc::new(RefCell::new(Editor::from_text(
                Some(tree_sitter_quickfix::language()),
                "",
            )))
        });
        editor
            .borrow_mut()
            .buffer_mut()
            .set_language(from_extension("ki_quickfix").unwrap())?;
        let node_id =
            self.tree
                .replace_root_node_child(ComponentKind::QuickfixList, editor.clone(), false);
        let dispatches = {
            let mut editor = editor.borrow_mut();
            editor.set_content(&render.content, context)?;
            editor.set_decorations(&render.decorations);
            editor.set_title("Quickfix list".to_string());
            editor.select_line_at(render.highlight_line_index, context)?
        };

        // If the QuickfixList is the only component in the layout,
        // then it needs to be focused.
        // This can happen when, say, the user executed a global search
        // when no files have been opened yet.
        if self.tree.components().len() == 1 {
            self.tree.set_focus_component_id(node_id);
        }

        let editor = (*editor).clone();

        if let Some(info) = render.info {
            self.show_info_on(
                self.tree.root_id(),
                info,
                ComponentKind::GlobalInfo,
                context,
            )?;
        }

        Ok((editor, dispatches))
    }

    #[cfg(test)]
    pub fn get_dropdown_infos_count(&self) -> usize {
        self.tree.count_by_kind(ComponentKind::DropdownInfo)
    }

    pub fn show_editor_info(&mut self, info: Info, context: &Context) -> anyhow::Result<()> {
        self.show_info_on(
            self.tree.focused_component_id(),
            info,
            ComponentKind::EditorInfo,
            context,
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
    pub fn editor_info_contents(&self) -> Vec<String> {
        self.tree
            .root()
            .traverse_pre_order()
            .filter(|node| node.data().kind() == ComponentKind::EditorInfo)
            .map(|node| node.data().component().borrow().content())
            .collect()
    }

    #[cfg(test)]
    pub fn global_info_contents(&self) -> Vec<String> {
        self.tree
            .root()
            .traverse_pre_order()
            .filter(|node| node.data().kind() == ComponentKind::GlobalInfo)
            .map(|node| node.data().component().borrow().content())
            .collect()
    }

    #[cfg(test)]
    pub fn file_explorer_content(&self) -> String {
        self.background_file_explorer.borrow().content()
    }

    pub fn file_explorer_expanded_folders(&self) -> Vec<AbsolutePath> {
        self.background_file_explorer.borrow().expanded_folders()
    }

    pub fn get_quickfix_list_items(
        &self,
        source: &QuickfixListSource,
        context: &Context,
    ) -> Vec<QuickfixListItem> {
        match source {
            QuickfixListSource::Diagnostic(severity_range) => self
                .buffers()
                .into_iter()
                .flat_map(|buffer| {
                    let buffer = buffer.borrow();
                    buffer
                        .diagnostics()
                        .into_iter()
                        .filter_map(|diagnostic| {
                            if !severity_range.contains(diagnostic.severity) {
                                return None;
                            }

                            Some(QuickfixListItem::new(
                                Location {
                                    path: buffer.path()?,
                                    range: diagnostic.range,
                                },
                                Some(Info::new(
                                    "Diagnostics".to_string(),
                                    diagnostic.message.clone(),
                                )),
                                None,
                            ))
                        })
                        .collect_vec()
                })
                .collect_vec(),
            QuickfixListSource::Mark => context
                .marks()
                .iter()
                .flat_map(|(path, marks)| {
                    marks.iter().map(|mark| {
                        QuickfixListItem::new(
                            Location {
                                path: path.clone(),
                                range: *mark,
                            },
                            None,
                            None,
                        )
                    })
                })
                .collect_vec(),
            QuickfixListSource::Custom(items) => items.clone(),
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

    pub fn close_current_window_and_focus_parent(&mut self) {
        self.tree.close_current_and_focus_parent();
    }

    #[cfg(test)]
    pub fn global_info(&self) -> Option<String> {
        Some(
            self.tree
                .get_component_by_kind(ComponentKind::GlobalInfo)?
                .borrow()
                .content(),
        )
    }

    pub fn get_component_by_kind(&self, kind: ComponentKind) -> Option<Rc<RefCell<dyn Component>>> {
        self.tree.get_component_by_kind(kind)
    }

    pub fn hide_editor_info(&mut self) {
        self.tree.remove_current_child(ComponentKind::EditorInfo);
    }

    pub fn close_global_info(&mut self) {
        self.tree
            .remove_node_child(self.tree.root_id(), ComponentKind::GlobalInfo);
    }

    pub fn get_component_by_id(
        &self,
        component_id: ComponentId,
    ) -> Option<Rc<RefCell<dyn Component>>> {
        self.tree.get_component_by_id(component_id)
    }
}
fn layout_kind() -> (LayoutKind, f32) {
    (LayoutKind::Wide, 0.70)
}
