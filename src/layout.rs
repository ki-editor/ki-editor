use anyhow::anyhow;
use std::{cell::RefCell, rc::Rc};

use crate::{
    canonicalized_path::CanonicalizedPath,
    components::{
        component::{Component, ComponentId},
        editor::{Direction, Editor},
        prompt::Prompt,
        suggestive_editor::SuggestiveEditor,
    },
    quickfix_list::QuickfixLists,
    rectangle::{Border, Rectangle},
    screen::Dimension,
};

/// The layout of the app is split into multiple sections: the main panel, info panel, quickfix
/// lists and prompts.
/// The main panel is where the user edits code, and the info panel is for displaying info like
/// hover text, diagnostics, etc.
pub struct Layout {
    main_panel: Option<Rc<RefCell<SuggestiveEditor>>>,
    info_panel: Option<Rc<RefCell<Editor>>>,
    quickfix_lists: Option<Rc<RefCell<QuickfixLists>>>,
    prompts: Vec<Rc<RefCell<Prompt>>>,
    background_suggestive_editors: Vec<Rc<RefCell<SuggestiveEditor>>>,

    rectangles: Vec<Rectangle>,
    borders: Vec<Border>,

    focused_component_id: Option<ComponentId>,

    terminal_dimension: Dimension,
}

impl Layout {
    pub fn new(terminal_dimension: Dimension) -> Layout {
        let (rectangles, borders) = Rectangle::generate(1, terminal_dimension);
        Layout {
            main_panel: None,
            info_panel: None,
            quickfix_lists: None,
            prompts: vec![],
            focused_component_id: Some(ComponentId::new()),
            background_suggestive_editors: vec![],
            rectangles,
            borders,
            terminal_dimension,
        }
    }

    pub fn components(&self) -> Vec<Rc<RefCell<dyn Component>>> {
        let root_components = vec![]
            .into_iter()
            .chain(
                self.main_panel
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

    /// Return true if there's no more windows
    pub fn remove_current_component(&mut self) -> bool {
        self.focused_component_id.map(|id| {
            self.prompts.retain(|c| c.borrow().id() != id);

            self.main_panel = self.main_panel.take().filter(|c| c.borrow().id() != id);

            self.info_panel = self.info_panel.take().filter(|c| c.borrow().id() != id);

            self.quickfix_lists = self.quickfix_lists.take().filter(|c| c.borrow().id() != id);

            self.background_suggestive_editors
                .retain(|c| c.borrow().id() != id);

            self.set_main_panel(self.background_suggestive_editors.last().cloned());

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

    pub fn set_main_panel(&mut self, editor: Option<Rc<RefCell<SuggestiveEditor>>>) {
        self.focused_component_id = editor.as_ref().map(|editor| editor.borrow().id());
        self.main_panel = editor;
    }

    pub fn goto_opened_editor(&mut self, direction: Direction) {
        let editor = self
            .background_suggestive_editors
            .iter()
            .find(|editor| {
                let id = editor.borrow().id();
                if let Some(focused_component_id) = self.focused_component_id {
                    match direction {
                        Direction::Forward | Direction::Current => id > focused_component_id,
                        Direction::Backward => id < focused_component_id,
                    }
                } else {
                    true
                }
            })
            .cloned()
            .or_else(|| self.main_panel.take());
        self.set_main_panel(editor);
    }

    pub fn change_view(&mut self) {
        let components = self.components();
        if let Some(component) = components
            .iter()
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

    pub fn close_current_window(&mut self, change_focused_to: ComponentId) {
        self.remove_current_component();
        self.focused_component_id = Some(change_focused_to);
        self.recalculate_layout();
    }

    pub fn add_and_focus_prompt(&mut self, prompt: Rc<RefCell<Prompt>>) {
        self.focused_component_id = Some(prompt.borrow().id());
        self.prompts.push(prompt);
        self.recalculate_layout();
    }

    pub fn recalculate_layout(&mut self) {
        let (rectangles, borders) =
            Rectangle::generate(self.components().len(), self.terminal_dimension);
        self.rectangles = rectangles;
        self.borders = borders;

        self.components()
            .into_iter()
            .zip(self.rectangles.iter())
            .for_each(|(component, rectangle)| {
                // Leave 1 row on top for rendering the title
                let (_, rectangle) = rectangle.split_vertically_at(1);
                component.borrow_mut().set_rectangle(rectangle)
            });
    }

    pub fn open_file(&mut self, path: &CanonicalizedPath) -> Option<Rc<RefCell<SuggestiveEditor>>> {
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
            self.set_main_panel(Some(matching_editor.clone()));
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

    pub fn add_and_focus_suggestive_editor(
        &mut self,
        suggestive_editor: Rc<RefCell<SuggestiveEditor>>,
    ) {
        self.background_suggestive_editors
            .push(suggestive_editor.clone());
        self.set_main_panel(Some(suggestive_editor));
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

    pub fn show_info(&mut self, contents: Vec<String>) {
        let info = contents.join("\n===========\n");
        match &self.info_panel {
            None => {
                let info_panel = Rc::new(RefCell::new(Editor::from_text(
                    tree_sitter_md::language(),
                    &info,
                )));
                self.info_panel = Some(info_panel);
            }
            Some(info_panel) => info_panel.borrow_mut().set_content(&info),
        }
    }
}
