use std::{cell::RefCell, rc::Rc};

use nonempty::NonEmpty;

use crate::components::{component::Component, suggestive_editor::SuggestiveEditor};

pub struct Multibuffer {
    editors: NonEmpty<Rc<RefCell<SuggestiveEditor>>>,
}

impl Component for Multibuffer {
    fn editor(&self) -> &super::editor::Editor {
        todo!()
    }

    fn editor_mut(&mut self) -> &mut super::editor::Editor {
        todo!()
    }

    fn handle_key_event(
        &mut self,
        context: &crate::context::Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<crate::app::Dispatches> {
        todo!()
    }

    fn id(&self) -> super::component::ComponentId {
        self.editor().id()
    }

    fn get_grid(
        &mut self,
        context: &crate::context::Context,
        focused: bool,
    ) -> super::component::GetGridResult {
        self.editor_mut().get_grid(context, focused)
    }

    fn path(&self) -> Option<shared::absolute_path::AbsolutePath> {
        self.editor().buffer().path()
    }

    fn handle_events(
        &mut self,
        events: &[event::KeyEvent],
    ) -> anyhow::Result<crate::app::Dispatches> {
        let context = crate::context::Context::default();
        Ok(crate::app::Dispatches::new(
            events
                .iter()
                .map(|event| -> anyhow::Result<_> {
                    Ok(self.handle_key_event(&context, event.clone())?.into_vec())
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
        ))
    }

    fn handle_event(
        &mut self,
        context: &crate::context::Context,
        event: event::event::Event,
    ) -> anyhow::Result<crate::app::Dispatches> {
        let dispatches = match event {
            event::event::Event::Key(event) => self.handle_key_event(context, event)?,
            event::event::Event::Paste(content) => self.handle_paste_event(content, context)?,
            event::event::Event::Mouse(event) => self.handle_mouse_event(event)?,
            _ => crate::app::Dispatches::default(),
        };
        self.post_handle_event(dispatches)
    }

    fn post_handle_event(
        &self,
        dispatches: crate::app::Dispatches,
    ) -> anyhow::Result<crate::app::Dispatches> {
        Ok(dispatches)
    }

    fn handle_paste_event(
        &mut self,
        content: String,
        context: &crate::context::Context,
    ) -> anyhow::Result<crate::app::Dispatches> {
        self.editor_mut().handle_paste_event(content, context)
    }

    fn handle_mouse_event(
        &mut self,
        event: crossterm::event::MouseEvent,
    ) -> anyhow::Result<crate::app::Dispatches> {
        self.editor_mut().handle_mouse_event(event)
    }

    fn get_cursor_position(&self) -> anyhow::Result<crate::position::Position> {
        self.editor().get_cursor_position()
    }

    fn set_rectangle(
        &mut self,
        rectangle: crate::rectangle::Rectangle,
        context: &crate::context::Context,
    ) {
        self.editor_mut().set_rectangle(rectangle, context);
    }

    fn rectangle(&self) -> &crate::rectangle::Rectangle {
        self.editor().rectangle()
    }

    fn set_content(
        &mut self,
        str: &str,
        context: &crate::context::Context,
    ) -> anyhow::Result<crate::app::Dispatches> {
        self.editor_mut().set_content(str, context)
    }

    fn content(&self) -> String {
        self.editor().buffer().content()
    }

    fn title(&self, context: &crate::context::Context) -> String {
        self.editor().title(context)
    }

    fn set_title(&mut self, title: String) {
        self.editor_mut().set_title(title);
    }

    fn handle_dispatch_editor(
        &mut self,
        context: &mut crate::context::Context,
        dispatch: super::editor::DispatchEditor,
    ) -> anyhow::Result<crate::app::Dispatches> {
        self.editor_mut().handle_dispatch_editor(context, dispatch)
    }
}
