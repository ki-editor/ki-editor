use std::{any::Any, cell::RefCell, rc::Rc};

use crossterm::event::Event;

use crate::{
    context::Context, grid::Grid, lsp::diagnostic::Diagnostic, position::Position,
    rectangle::Rectangle, screen::Dispatch,
};

use super::editor::Editor;

// dyn_clone::clone_trait_object!(Component);

pub trait Component: Any + AnyComponent {
    fn id(&self) -> ComponentId {
        self.editor().id()
    }
    fn editor(&self) -> &Editor;
    fn editor_mut(&mut self) -> &mut Editor;
    fn get_grid(&self, diagnostics: &[Diagnostic]) -> Grid {
        self.editor().get_grid(diagnostics)
    }

    #[cfg(test)]
    /// This is for writing tests for components.
    fn handle_events(&mut self, events: &str) -> anyhow::Result<Vec<Dispatch>> {
        use crate::key_event_parser::parse_key_events;

        let mut context = Context::default();
        Ok(parse_key_events(events)?
            .into_iter()
            .map(|event| self.handle_event(&mut context, Event::Key(event)))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>())
    }

    fn handle_event(
        &mut self,
        context: &mut Context,
        event: Event,
    ) -> anyhow::Result<Vec<Dispatch>>;

    fn get_cursor_position(&self) -> Position {
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

    fn set_content(&mut self, str: &str) {
        self.editor_mut().set_content(str);
    }

    fn title(&self) -> String {
        self.editor().title()
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
pub struct ComponentId(uuid::Uuid);
impl ComponentId {
    pub fn new() -> ComponentId {
        ComponentId(uuid::Uuid::new_v4())
    }
}

#[cfg(test)]
mod test_component {
    use std::{cell::RefCell, rc::Rc};

    use crate::components::component::Component;

    #[test]
    fn child_should_rank_lower_than_parent() {
        struct GrandChild {}
        impl Component for GrandChild {
            fn title(&self) -> String {
                "GrandChild".to_string()
            }
            fn editor(&self) -> &crate::components::editor::Editor {
                todo!()
            }

            fn editor_mut(&mut self) -> &mut crate::components::editor::Editor {
                todo!()
            }

            fn handle_event(
                &mut self,
                _context: &mut crate::context::Context,
                _event: crossterm::event::Event,
            ) -> anyhow::Result<Vec<crate::screen::Dispatch>> {
                todo!()
            }

            fn children(&self) -> Vec<Option<Rc<RefCell<dyn Component>>>> {
                vec![]
            }

            fn remove_child(&mut self, _component_id: crate::components::component::ComponentId) {
                todo!()
            }
        }
        struct Child {
            grand_child: Rc<RefCell<GrandChild>>,
        }
        impl Component for Child {
            fn title(&self) -> String {
                "Child".to_string()
            }
            fn editor(&self) -> &crate::components::editor::Editor {
                todo!()
            }

            fn editor_mut(&mut self) -> &mut crate::components::editor::Editor {
                todo!()
            }

            fn handle_event(
                &mut self,
                _context: &mut crate::context::Context,
                _event: crossterm::event::Event,
            ) -> anyhow::Result<Vec<crate::screen::Dispatch>> {
                todo!()
            }

            fn children(&self) -> Vec<Option<std::rc::Rc<std::cell::RefCell<dyn Component>>>> {
                vec![Some(self.grand_child.clone())]
            }

            fn remove_child(&mut self, _component_id: crate::components::component::ComponentId) {
                todo!()
            }
        }

        struct Parent {
            child: Rc<RefCell<Child>>,
        }
        impl Component for Parent {
            fn title(&self) -> String {
                "Parent".to_string()
            }
            fn editor(&self) -> &crate::components::editor::Editor {
                todo!()
            }

            fn editor_mut(&mut self) -> &mut crate::components::editor::Editor {
                todo!()
            }

            fn handle_event(
                &mut self,
                _context: &mut crate::context::Context,
                _event: crossterm::event::Event,
            ) -> anyhow::Result<Vec<crate::screen::Dispatch>> {
                todo!()
            }

            fn children(&self) -> Vec<Option<std::rc::Rc<std::cell::RefCell<dyn Component>>>> {
                vec![Some(self.child.clone())]
            }

            fn remove_child(&mut self, _component_id: crate::components::component::ComponentId) {
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

        assert_eq!(
            descendants
                .into_iter()
                .map(|d| d.borrow().title())
                .collect::<Vec<_>>(),
            vec!["Child".to_string(), "GrandChild".to_string()],
        )
    }
}
