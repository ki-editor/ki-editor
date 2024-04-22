use std::{cell::RefCell, rc::Rc};

use itertools::Itertools;
use nary_tree::{NodeId, NodeMut, NodeRef, RemoveBehavior};

use crate::components::{component::Component, editor::Editor};

pub struct UiTree {
    tree: nary_tree::Tree<KindedComponent>,
    focused_component_id: NodeId,
}

/// The difference between this and `nary_tree::Tree` is that
/// the root of this `Tree` is always defined,
/// which makes its usage more pleasing.
impl UiTree {
    pub fn new() -> UiTree {
        let mut tree = nary_tree::Tree::new();
        let mut editor = Editor::from_text(tree_sitter_md::language(), "");
        editor.set_title("[ROOT] (Cannot be saved)".to_string());
        let focused_component_id = tree.set_root(KindedComponent::new(
            ComponentKind::Root,
            Rc::new(RefCell::new(editor)).clone(),
        ));
        UiTree {
            tree,
            focused_component_id,
        }
    }

    pub fn root<'a>(&'a self) -> NodeRef<'a, KindedComponent> {
        self.tree.root().unwrap()
    }

    pub fn get<'a>(&'a self, id: NodeId) -> Option<NodeRef<'a, KindedComponent>> {
        self.tree.get(id)
    }

    /// The root will never be removed to ensure that this tree always contain one component
    pub fn remove(&mut self, node_id: NodeId) -> Option<KindedComponent> {
        if node_id == self.root_id() {
            return None;
        }
        self.tree.remove(node_id, RemoveBehavior::DropChildren)
    }

    fn get_mut<'a>(&'a mut self, id: NodeId) -> Option<NodeMut<'a, KindedComponent>> {
        self.tree.get_mut(id)
    }

    pub fn remove_all_except_current(&mut self) {
        let root = self.root();
        let root_id = root.node_id();
        for node_id in root
            .traverse_pre_order()
            .filter(|node| node.node_id() != root_id && node.node_id() != self.focused_component_id)
            .map(|node| node.node_id())
            .collect_vec()
        {
            self.remove(node_id);
        }
    }

    /// Append `component` to the Node of given `node_id`
    pub(crate) fn append_component(
        &mut self,
        node_id: NodeId,
        component: KindedComponent,
        focus: bool,
    ) {
        let id = if let Some(mut node) = self.get_mut(node_id) {
            node.append(component).node_id()
        } else {
            self.root_mut().append(component).node_id()
        };
        if focus {
            self.set_focus_component_id(id)
        }
    }

    pub(crate) fn append_component_to_current(&mut self, component: KindedComponent, focus: bool) {
        self.append_component(self.focused_component_id, component, focus)
    }

    fn root_mut<'a>(&'a mut self) -> NodeMut<'a, KindedComponent> {
        self.tree.root_mut().unwrap()
    }

    /// This return everything except the root, but if only root exists, then the root will be returned.
    /// This behaviour ensures that the tree always contain a component.
    pub(crate) fn components(&self) -> Vec<Rc<RefCell<dyn Component>>> {
        if self.root().children().count() == 0 {
            Some(self.root().data().component())
                .into_iter()
                .collect_vec()
        } else {
            let root_id = self.root().node_id();
            self.root()
                .traverse_pre_order()
                .filter(|node| node.node_id() != root_id)
                .map(|node| node.data().component())
                .collect_vec()
        }
    }

    pub(crate) fn append_component_to_root(&mut self, component: KindedComponent, focus: bool) {
        self.append_component(self.root_id(), component, focus)
    }

    pub(crate) fn root_id(&self) -> NodeId {
        self.root().node_id()
    }

    pub(crate) fn remove_current_child(&mut self, kind: ComponentKind) -> Option<KindedComponent> {
        self.remove_node_child(self.focused_component_id, kind)
    }

    pub(crate) fn remove_node_child(
        &mut self,
        node_id: NodeId,
        kind: ComponentKind,
    ) -> Option<KindedComponent> {
        if let Some(node) = self.tree.get_mut(node_id) {
            let node_id = node
                .as_ref()
                .traverse_pre_order()
                .find(|node| node.node_id() != node_id && node.data().kind == kind)?
                .node_id();
            self.remove(node_id)
        } else {
            None
        }
    }

    pub(crate) fn get_current_node_child_id(&self, kind: ComponentKind) -> Option<NodeId> {
        let node_id = self.focused_component_id;
        Some(
            self.get(node_id)?
                .traverse_pre_order()
                .filter(|node| node.node_id() != node_id)
                .find(|node| node.data().kind == kind)?
                .node_id(),
        )
    }

    #[cfg(test)]
    pub(crate) fn count_by_kind(&self, kind: ComponentKind) -> usize {
        self.root()
            .traverse_pre_order()
            .filter(|node| node.data().kind == kind)
            .count()
    }

    pub(crate) fn set_focus_component_id(&mut self, id: NodeId) {
        // This check is necessary.
        // In case of any logical error that causes `id` to be pointing to node that is removed from the tree,
        // it should always fallback to use root ID
        self.focused_component_id = if self
            .root()
            .traverse_pre_order()
            .any(|node| node.node_id() == id)
        {
            id
        } else {
            self.root_id()
        };
    }

    pub(crate) fn focused_component_id(&self) -> NodeId {
        self.focused_component_id
    }

    pub(crate) fn cycle_component(&mut self) {
        self.set_focus_component_id(
            self.root()
                .traverse_pre_order()
                .map(|node| node.node_id())
                .filter(|node_id| node_id != &self.root_id())
                .collect_vec()
                .into_iter()
                .skip_while(|node_id| node_id != &self.focused_component_id)
                .nth(1)
                .or_else(|| self.root().first_child().map(|node| node.node_id()))
                .unwrap_or_else(|| self.root_id()),
        );
    }

    pub(crate) fn get_current_node<'a>(&'a self) -> NodeRef<'a, KindedComponent> {
        self.get(self.focused_component_id)
            .unwrap_or_else(|| self.root())
    }

    pub(crate) fn replace_node_child(
        &mut self,
        id: NodeId,
        kind: ComponentKind,
        component: Rc<RefCell<dyn Component>>,
        focus: bool,
    ) {
        self.remove_node_child(id, kind);
        self.append_component(id, KindedComponent::new(kind, component), focus)
    }

    pub(crate) fn replace_current_node_child(
        &mut self,
        kind: ComponentKind,
        component: Rc<RefCell<dyn Component>>,
        focus: bool,
    ) {
        let id = self.focused_component_id;
        self.remove_node_child(id, kind);
        self.append_component(id, KindedComponent::new(kind, component), focus)
    }

    pub(crate) fn close_current_and_focus_parent(&mut self) {
        if let Some(node) = self.tree.get(self.focused_component_id) {
            let parent_id = node.parent().map(|parent| parent.node_id());
            self.tree
                .remove(node.node_id(), RemoveBehavior::DropChildren);
            self.focused_component_id = parent_id.unwrap_or_else(|| self.root_id())
        }
    }

    pub(crate) fn current_component(&self) -> Rc<RefCell<(dyn Component)>> {
        self.get_current_node().data().component()
    }
}

#[derive(Clone)]
pub struct KindedComponent {
    component: Rc<RefCell<dyn Component>>,
    kind: ComponentKind,
}

impl KindedComponent {
    pub fn new(kind: ComponentKind, component: Rc<RefCell<dyn Component>>) -> KindedComponent {
        Self { kind, component }
    }

    pub(crate) fn component(&self) -> Rc<RefCell<dyn Component>> {
        self.component.clone()
    }

    pub(crate) fn kind(&self) -> ComponentKind {
        self.kind
    }
}

impl std::fmt::Debug for KindedComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.kind)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ComponentKind {
    Dropdown,
    SuggestiveEditor,
    Info,
    DropdownInfo,
    KeymapLegend,
    FileExplorer,
    Prompt,
    QuickfixList,
    EditorInfo,
    /// The root should not be rendered
    Root,
}
