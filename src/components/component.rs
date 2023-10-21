use std::{any::Any, cell::RefCell, rc::Rc};

use event::event::Event;

use crate::{
    app::Dispatch, context::Context, grid::Grid, position::Position, rectangle::Rectangle,
};

use super::editor::Editor;

// dyn_clone::clone_trait_object!(Component);
//
pub struct GetGridResult {
    pub grid: Grid,
    pub cursor: Option<Cursor>,
}
impl GetGridResult {
    pub(crate) fn to_string(&self) -> String {
        match &self.cursor {
            Some(cursor) => self
                .grid
                .clone()
                .apply_cell_update(crate::grid::CellUpdate::new(cursor.position).set_symbol("█"))
                .to_string(),
            None => self.grid.to_string(),
        }
    }
}

pub struct Cursor {
    position: Position,
    style: crossterm::cursor::SetCursorStyle,
}
impl Cursor {
    pub fn style(&self) -> &crossterm::cursor::SetCursorStyle {
        &self.style
    }
    pub fn position(&self) -> &Position {
        &self.position
    }

    pub fn new(position: Position, style: crossterm::cursor::SetCursorStyle) -> Cursor {
        Cursor { position, style }
    }

    pub fn set_position(self, position: Position) -> Cursor {
        Cursor { position, ..self }
    }
}

pub trait Component: Any + AnyComponent {
    fn id(&self) -> ComponentId {
        self.editor().id()
    }
    fn editor(&self) -> &Editor;
    fn editor_mut(&mut self) -> &mut Editor;
    fn get_grid(&self, context: &mut Context) -> GetGridResult {
        self.editor().get_grid(context)
    }

    #[cfg(test)]
    /// This is for writing tests for components.
    fn handle_events(&mut self, events: &[event::KeyEvent]) -> anyhow::Result<Vec<Dispatch>> {
        let mut context = Context::default();
        Ok(events
            .iter()
            .map(|event| self.handle_key_event(&mut context, event.clone()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>())
    }

    fn handle_event(&mut self, context: &Context, event: Event) -> anyhow::Result<Vec<Dispatch>> {
        match event {
            Event::Key(event) => self.handle_key_event(context, event),
            Event::Paste(content) => self.handle_paste_event(content),
            Event::Mouse(event) => self.handle_mouse_event(event),
            _ => Ok(vec![]),
        }
    }

    fn handle_paste_event(&mut self, content: String) -> anyhow::Result<Vec<Dispatch>> {
        self.editor_mut().handle_paste_event(content)
    }

    fn handle_mouse_event(
        &mut self,
        _event: crossterm::event::MouseEvent,
    ) -> anyhow::Result<Vec<Dispatch>> {
        Ok(vec![])
    }

    fn handle_key_event(
        &mut self,
        context: &Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Vec<Dispatch>>;

    fn get_cursor_position(&self) -> anyhow::Result<Position> {
        self.editor().get_cursor_position()
    }

    fn scroll_offset(&self) -> u16 {
        self.editor().scroll_offset()
    }

    fn set_rectangle(&mut self, rectangle: Rectangle) {
        self.editor_mut().set_rectangle(rectangle)
    }

    fn rectangle(&self) -> &Rectangle {
        self.editor().rectangle()
    }

    fn set_content(&mut self, str: &str) -> anyhow::Result<()> {
        self.editor_mut().set_content(str)
    }

    fn content(&self) -> String {
        self.editor().buffer().content()
    }

    fn title(&self, context: &Context) -> String {
        self.editor().title(context)
    }

    fn set_title(&mut self, title: String) {
        self.editor_mut().set_title(title);
    }

    /// This should only return the direct children of this component.
    fn children(&self) -> Vec<Option<Rc<RefCell<dyn Component>>>>;

    /// Does not include the component itself
    fn descendants(&self) -> Vec<Rc<RefCell<dyn Component>>> {
        self.children()
            .into_iter()
            .flatten()
            .flat_map(|component| {
                std::iter::once(component.clone())
                    .chain(component.borrow().descendants().into_iter())
            })
            .collect::<Vec<_>>()
    }

    fn remove_child(&mut self, component_id: ComponentId);
}

/// Modified from https://github.com/helix-editor/helix/blob/91da0dc172dde1a972be7708188a134db70562c3/helix-term/src/compositor.rs#L212
pub trait AnyComponent {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Component> AnyComponent for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Why do I use UUID instead of a simple u64?
/// Because with UUID I don't need a global state to keep track of the next ID.
#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Clone, Copy, Hash, Default)]
pub struct ComponentId(usize);
impl ComponentId {
    pub fn new() -> ComponentId {
        // Current epoch
        ComponentId({
            use std::time::{SystemTime, UNIX_EPOCH};
            let start = SystemTime::now();
            let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
            since_the_epoch.as_millis() as usize
        })
    }
}

#[cfg(test)]
mod test_component {
    use std::{cell::RefCell, rc::Rc};

    use crate::{components::component::Component, context::Context};

    #[test]
    fn child_should_rank_lower_than_parent() {
        struct GrandChild {}
        impl Component for GrandChild {
            fn title(&self, _: &Context) -> String {
                "GrandChild".to_string()
            }
            fn editor(&self) -> &crate::components::editor::Editor {
                todo!()
            }

            fn editor_mut(&mut self) -> &mut crate::components::editor::Editor {
                todo!()
            }

            fn children(&self) -> Vec<Option<Rc<RefCell<dyn Component>>>> {
                vec![]
            }

            fn remove_child(&mut self, _component_id: crate::components::component::ComponentId) {
                todo!()
            }

            fn handle_key_event(
                &mut self,
                _context: &crate::context::Context,
                _event: event::KeyEvent,
            ) -> anyhow::Result<Vec<crate::app::Dispatch>> {
                todo!()
            }
        }
        struct Child {
            grand_child: Rc<RefCell<GrandChild>>,
        }
        impl Component for Child {
            fn title(&self, _: &Context) -> String {
                "Child".to_string()
            }
            fn editor(&self) -> &crate::components::editor::Editor {
                todo!()
            }

            fn editor_mut(&mut self) -> &mut crate::components::editor::Editor {
                todo!()
            }

            fn children(&self) -> Vec<Option<std::rc::Rc<std::cell::RefCell<dyn Component>>>> {
                vec![Some(self.grand_child.clone())]
            }

            fn remove_child(&mut self, _component_id: crate::components::component::ComponentId) {
                todo!()
            }

            fn handle_key_event(
                &mut self,
                _context: &crate::context::Context,
                _event: event::KeyEvent,
            ) -> anyhow::Result<Vec<crate::app::Dispatch>> {
                todo!()
            }
        }

        struct Parent {
            child: Rc<RefCell<Child>>,
        }
        impl Component for Parent {
            fn title(&self, _: &Context) -> String {
                "Parent".to_string()
            }
            fn editor(&self) -> &crate::components::editor::Editor {
                todo!()
            }

            fn editor_mut(&mut self) -> &mut crate::components::editor::Editor {
                todo!()
            }

            fn children(&self) -> Vec<Option<std::rc::Rc<std::cell::RefCell<dyn Component>>>> {
                vec![Some(self.child.clone())]
            }

            fn remove_child(&mut self, _component_id: crate::components::component::ComponentId) {
                todo!()
            }

            fn handle_key_event(
                &mut self,
                _context: &crate::context::Context,
                _event: event::KeyEvent,
            ) -> anyhow::Result<Vec<crate::app::Dispatch>> {
                todo!()
            }
        }

        let parent = Parent {
            child: Rc::new(RefCell::new(Child {
                grand_child: Rc::new(RefCell::new(GrandChild {})),
            })),
        };

        let descendants = parent.descendants();

        assert_eq!(descendants.len(), 2);
        let context = Context::default();

        assert_eq!(
            descendants
                .into_iter()
                .map(|d| d.borrow().title(&context))
                .collect::<Vec<_>>(),
            vec!["Child".to_string(), "GrandChild".to_string()],
        )
    }
}
