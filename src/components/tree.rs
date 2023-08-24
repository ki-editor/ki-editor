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
        _context: &mut crate::context::Context,
        _event: event::KeyEvent,
    ) -> anyhow::Result<Vec<crate::screen::Dispatch>> {
        todo!()
    }

    fn children(&self) -> Vec<Option<std::rc::Rc<std::cell::RefCell<dyn Component>>>> {
        Vec::new()
    }

    fn remove_child(&mut self, _component_id: super::component::ComponentId) {}
}
