use super::{component::Component, editor::Editor};

pub struct Tree {
    editor: Editor,
}

impl Component for Tree {
    fn editor(&self) -> &Editor {
        &self.editor
    }

    fn editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    fn handle_key_event(
        &mut self,
        context: &mut crate::context::Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Vec<crate::screen::Dispatch>> {
        todo!()
    }

    fn children(&self) -> Vec<Option<std::rc::Rc<std::cell::RefCell<dyn Component>>>> {
        Vec::new()
    }

    fn remove_child(&mut self, component_id: super::component::ComponentId) {}
}
