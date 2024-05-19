use crate::app::Dispatches;
use std::any::Any;

use event::event::Event;
use shared::canonicalized_path::CanonicalizedPath;

use crate::{context::Context, grid::Grid, position::Position, rectangle::Rectangle};

use super::{
    editor::{DispatchEditor, Editor},
    keymap_legend::KeymapLegendSection,
};

// dyn_clone::clone_trait_object!(Component);
//
pub(crate) struct GetGridResult {
    pub(crate) grid: Grid,
    pub(crate) cursor: Option<Cursor>,
}
#[cfg(test)]
impl std::fmt::Display for GetGridResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = match &self.cursor {
            Some(cursor) => self
                .grid
                .clone()
                .apply_cell_update(
                    crate::grid::CellUpdate::new(cursor.position).set_symbol(Some("█".to_string())),
                )
                .to_string(),
            None => self.grid.to_string(),
        };
        write!(f, "{}", content)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Cursor {
    position: Position,
    style: SetCursorStyle,
}

/// Why is this necessary? Because `crossterm::cursor::SetCursorStyle` does not implement `Debug`.
#[derive(Debug, Clone, Copy)]
pub enum SetCursorStyle {
    DefaultUserShape,
    BlinkingBlock,
    SteadyBlock,
    BlinkingUnderScore,
    SteadyUnderScore,
    BlinkingBar,
    SteadyBar,
}

impl From<&SetCursorStyle> for crossterm::cursor::SetCursorStyle {
    fn from(style: &SetCursorStyle) -> Self {
        match style {
            SetCursorStyle::DefaultUserShape => crossterm::cursor::SetCursorStyle::DefaultUserShape,
            SetCursorStyle::BlinkingBlock => crossterm::cursor::SetCursorStyle::BlinkingBlock,
            SetCursorStyle::SteadyBlock => crossterm::cursor::SetCursorStyle::SteadyBlock,
            SetCursorStyle::BlinkingUnderScore => {
                crossterm::cursor::SetCursorStyle::BlinkingUnderScore
            }
            SetCursorStyle::SteadyUnderScore => crossterm::cursor::SetCursorStyle::SteadyUnderScore,
            SetCursorStyle::BlinkingBar => crossterm::cursor::SetCursorStyle::BlinkingBar,
            SetCursorStyle::SteadyBar => crossterm::cursor::SetCursorStyle::SteadyBar,
        }
    }
}
impl Cursor {
    pub(crate) fn style(&self) -> &SetCursorStyle {
        &self.style
    }
    pub(crate) fn position(&self) -> &Position {
        &self.position
    }

    pub(crate) fn new(position: Position, style: SetCursorStyle) -> Cursor {
        Cursor { position, style }
    }

    pub(crate) fn set_position(self, position: Position) -> Cursor {
        Cursor { position, ..self }
    }
}
pub trait Component: Any + AnyComponent {
    fn id(&self) -> ComponentId {
        self.editor().id()
    }
    fn editor(&self) -> &Editor;
    fn editor_mut(&mut self) -> &mut Editor;
    fn get_grid(&self, context: &Context) -> GetGridResult {
        self.editor().get_grid(context)
    }

    fn path(&self) -> Option<CanonicalizedPath> {
        self.editor().buffer().path()
    }

    #[cfg(test)]
    /// This is for writing tests for components.
    fn handle_events(&mut self, events: &[event::KeyEvent]) -> anyhow::Result<Dispatches> {
        let context = Context::default();
        Ok(Dispatches::new(
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

    fn handle_event(&mut self, context: &Context, event: Event) -> anyhow::Result<Dispatches> {
        match event {
            Event::Key(event) => self.handle_key_event(context, event),
            Event::Paste(content) => self.handle_paste_event(content),
            Event::Mouse(event) => self.handle_mouse_event(event),
            _ => Ok(Default::default()),
        }
    }

    fn handle_paste_event(&mut self, content: String) -> anyhow::Result<Dispatches> {
        self.editor_mut().handle_paste_event(content)
    }

    fn handle_mouse_event(
        &mut self,
        _event: crossterm::event::MouseEvent,
    ) -> anyhow::Result<Dispatches> {
        Ok(Default::default())
    }

    fn handle_key_event(
        &mut self,
        context: &Context,
        event: event::KeyEvent,
    ) -> anyhow::Result<Dispatches>;

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

    fn handle_dispatch_editor(
        &mut self,
        context: &mut Context,
        dispatch: DispatchEditor,
    ) -> anyhow::Result<Dispatches> {
        self.editor_mut().handle_dispatch_editor(context, dispatch)
    }

    /// This should be different for every component.
    /// because the default context menu is not applicable in the File Explorer.
    fn contextual_keymaps(&self) -> Vec<KeymapLegendSection> {
        Default::default()
    }
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

use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn increment_counter() -> usize {
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Debug, Clone, Copy, Hash, Default)]
pub(crate) struct ComponentId(usize);
impl ComponentId {
    pub(crate) fn new() -> ComponentId {
        ComponentId(increment_counter())
    }
}
