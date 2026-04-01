use std::{cell::RefCell, rc::Rc};

use itertools::Itertools;
use nonempty::NonEmpty;

use crate::{
    app::Dispatches,
    components::{component::Component, editor::Editor, suggestive_editor::SuggestiveEditor},
};

pub struct Multibuffer {
    primary_editor: SuggestiveEditor,
    other_editors: Vec<SuggestiveEditor>,
}

impl Multibuffer {
    fn editors(&mut self) -> Vec<&mut SuggestiveEditor> {
        Some(&mut self.primary_editor)
            .into_iter()
            .chain(self.other_editors.iter_mut())
            .collect_vec()
    }

    pub(crate) fn new(
        primary_editor: SuggestiveEditor,
        other_editors: Vec<SuggestiveEditor>,
    ) -> Self {
        Self {
            primary_editor,
            other_editors,
        }
    }
}

impl Component for Multibuffer {
    fn editor(&self) -> &super::editor::Editor {
        self.primary_editor.editor()
    }

    fn editor_mut(&mut self) -> &mut super::editor::Editor {
        self.primary_editor.editor_mut()
    }

    fn handle_dispatch_editor(
        &mut self,
        context: &mut crate::context::Context,
        dispatch: super::editor::DispatchEditor,
    ) -> anyhow::Result<crate::app::Dispatches> {
        self.editor_mut().handle_dispatch_editor(context, dispatch)
    }

    fn handle_key_event(
        &mut self,
        context: &crate::context::Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<crate::app::Dispatches> {
        Ok(self
            .editors()
            .iter_mut()
            .map(|editor| editor.handle_key_event(context, event.clone()))
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .fold(Dispatches::default(), |a, b| a.chain(b)))
    }

    fn handle_event(
        &mut self,
        context: &crate::context::Context,
        event: event::event::Event,
    ) -> anyhow::Result<crate::app::Dispatches> {
        Ok(self
            .editors()
            .iter_mut()
            .map(|editor| editor.handle_event(context, event.clone()))
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .fold(Dispatches::default(), |a, b| a.chain(b)))
    }
}
