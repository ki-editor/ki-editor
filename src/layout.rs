use crate::{
    app::Dimension,
    buffer::Buffer,
    components::{
        component::{Component, ComponentId},
        editor::{Editor, Movement},
        file_explorer::FileExplorer,
        keymap_legend::{KeymapLegend, KeymapLegendConfig},
        prompt::Prompt,
        suggestive_editor::{Info, SuggestiveEditor},
    },
    quickfix_list::QuickfixLists,
    rectangle::{Border, LayoutKind, Rectangle},
    undo_tree::{Applicable, OldNew, UndoTree},
};
use anyhow::anyhow;
use itertools::Itertools;
use shared::canonicalized_path::CanonicalizedPath;
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

/// The layout of the app is split into multiple sections: the main panel, info panel, quickfix
/// lists, prompts, and etc.
/// The main panel is where the user edits code, and the info panel is for displaying info like
/// hover text, diagnostics, etc.
pub struct Layout {
    main_panel: MainPanel,
    undo_tree: UndoTree<MainPanel>,
    info_panel: Option<Rc<RefCell<Editor>>>,
    keymap_legend: Option<Rc<RefCell<KeymapLegend>>>,
    quickfix_lists: Option<Rc<RefCell<QuickfixLists>>>,
    prompts: Vec<Rc<RefCell<Prompt>>>,
    background_suggestive_editors: Vec<Rc<RefCell<SuggestiveEditor>>>,
    file_explorer: Rc<RefCell<FileExplorer>>,
    file_explorer_open: bool,

    rectangles: Vec<Rectangle>,
    borders: Vec<Border>,

    focused_component_id: Option<ComponentId>,

    terminal_dimension: Dimension,
    navigation_tree: undo::History<Rc<RefCell<SuggestiveEditor>>>,
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

struct Null;
impl std::fmt::Display for Null {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NULL")
    }
}

impl Applicable for MainPanel {
    type Target = MainPanel;

    type Output = Null;

    fn apply(&self, target: &mut Self::Target) -> anyhow::Result<Self::Output> {
        *target = self.clone();
        Ok(Null)
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
            info_panel: None,
            keymap_legend: None,
            quickfix_lists: None,
            prompts: vec![],
            focused_component_id: Some(ComponentId::new()),
            background_suggestive_editors: vec![],
            file_explorer: Rc::new(RefCell::new(FileExplorer::new(working_directory)?)),
            rectangles,
            borders,
            terminal_dimension,
            file_explorer_open: false,
            navigation_tree: undo::History::new(),
            undo_tree: UndoTree::new(),
            working_directory: working_directory.clone(),
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
                self.info_panel
                    .iter()
                    .map(|c| c.clone() as Rc<RefCell<dyn Component>>),
            )
            .chain(
                self.quickfix_lists
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
        self.focused_component_id.map(|id| {
            self.prompts.retain(|c| c.borrow().id() != id);

            let main_panel = self.main_panel.take();
            self.main_panel = self.new_main_panel(
                main_panel
                    .filter(|c| c.borrow().id() != id)
                    .or_else(|| self.background_suggestive_editors.last().cloned()),
            );

            self.keymap_legend = self.keymap_legend.take().filter(|c| c.borrow().id() != id);

            self.info_panel = self.info_panel.take().filter(|c| c.borrow().id() != id);

            self.quickfix_lists = self.quickfix_lists.take().filter(|c| c.borrow().id() != id);

            self.background_suggestive_editors
                .retain(|c| c.borrow().id() != id);

            if self.file_explorer.borrow().id() == id {
                self.file_explorer_open = false
            }

            self.components().into_iter().for_each(|c| {
                c.borrow_mut().remove_child(id);
            });
        });

        if let Some(component) = self.components().last() {
            self.focused_component_id = Some(component.borrow().id());
            self.recalculate_layout();
            false
        } else {
            true
        }
    }

    pub fn show_quickfix_lists(&mut self, quickfix_lists: Rc<RefCell<QuickfixLists>>) {
        self.quickfix_lists = Some(quickfix_lists);
    }

    fn set_main_panel(&mut self, new: MainPanel) -> anyhow::Result<()> {
        self.focused_component_id = new.id();
        let old = self.main_panel.take();
        self.main_panel = new.clone();
        let old_new = OldNew {
            old_to_new: new,
            new_to_old: self.new_main_panel(old),
        };
        self.undo_tree.edit(&mut self.main_panel, old_new)?;
        Ok(())
    }

    pub fn goto_opened_editor(&mut self, movement: Movement) -> anyhow::Result<()> {
        self.undo_tree
            .apply_movement(&mut self.main_panel, movement)?;
        if let Some(editor) = &self.main_panel.editor {
            self.focused_component_id = Some(editor.borrow().id());
        }
        Ok(())
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

    pub fn open_file(
        &mut self,
        path: &CanonicalizedPath,
    ) -> anyhow::Result<Option<Rc<RefCell<SuggestiveEditor>>>> {
        if let Some(matching_editor) =
            self.background_suggestive_editors
                .iter()
                .cloned()
                .find(|component| {
                    component
                        .borrow()
                        .editor()
                        .buffer()
                        .path()
                        .map(|p| &p == path)
                        .unwrap_or(false)
                })
        {
            self.set_main_panel(self.new_main_panel(Some(matching_editor.clone())))?;
            Ok(Some(matching_editor))
        } else {
            Ok(None)
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

    pub fn show_info(&mut self, title: &str, info: Info) -> anyhow::Result<()> {
        let info_panel = self.info_panel.take().unwrap_or_else(|| {
            Rc::new(RefCell::new(Editor::from_text(
                tree_sitter_md::language(),
                "",
            )))
        });
        info_panel.borrow_mut().set_title(title.to_string());
        info_panel.borrow_mut().show_info(info);
        self.info_panel = Some(info_panel);

        Ok(())
    }

    pub fn show_keymap_legend(&mut self, keymap_legend_config: KeymapLegendConfig) {
        let keymap_legend = KeymapLegend::new(keymap_legend_config);
        self.focused_component_id = Some(keymap_legend.id());
        self.keymap_legend = Some(Rc::new(RefCell::new(keymap_legend)));
    }

    pub fn close_all_except_main_panel(&mut self) {
        self.info_panel = None;
        self.keymap_legend = None;
        self.quickfix_lists = None;
        self.prompts = vec![];
        if self.focused_component_id.is_none() {
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

    pub fn reveal_path_in_explorer(&mut self, path: &CanonicalizedPath) -> anyhow::Result<()> {
        self.file_explorer.borrow_mut().reveal(path)?;

        self.focused_component_id = Some(self.file_explorer.borrow().id());
        self.file_explorer_open = true;

        Ok(())
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

    pub(crate) fn get_quickfixes(&self) -> Option<Vec<crate::quickfix_list::QuickfixListItem>> {
        if let Some(list) = self.quickfix_lists.as_ref() {
            list.borrow().get_items().cloned()
        } else {
            None
        }
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

    pub(crate) fn get_info(&self) -> Option<String> {
        self.info_panel
            .as_ref()
            .map(|info_panel| info_panel.borrow().text())
    }

    pub(crate) fn display_navigation_history(&self) -> String {
        self.undo_tree.display().to_string()
    }

    pub(crate) fn buffers(&self) -> Vec<Rc<RefCell<Buffer>>> {
        self.background_suggestive_editors
            .iter()
            .map(|editor| editor.borrow().editor().buffer_rc())
            .collect_vec()
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
