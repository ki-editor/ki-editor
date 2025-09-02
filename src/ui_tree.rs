use std::{cell::RefCell, rc::Rc};

use itertools::Itertools;
use nary_tree::{NodeId, NodeMut, NodeRef, RemoveBehavior};

use crate::components::{component::Component, editor::Editor};

pub(crate) struct UiTree {
    tree: nary_tree::Tree<KindedComponent>,
    focused_component_id: NodeId,
}

/// The difference between this and `nary_tree::Tree` is that
/// the root of this `Tree` is always defined,
/// which makes its usage more pleasing.
impl UiTree {
    pub(crate) fn new() -> UiTree {
        let mut tree = nary_tree::Tree::new();
        let mut editor = Editor::from_text(Some(tree_sitter_md::LANGUAGE.into()), "");
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

    pub(crate) fn root(&self) -> NodeRef<'_, KindedComponent> {
        self.tree.root().unwrap()
    }

    pub(crate) fn get(&self, id: NodeId) -> Option<NodeRef<'_, KindedComponent>> {
        self.tree.get(id)
    }

    /// The root will never be removed to ensure that this tree always contain one component
    pub(crate) fn remove(
        &mut self,
        node_id: NodeId,
        change_focus: bool,
    ) -> Option<KindedComponent> {
        if node_id == self.root_id() {
            return None;
        }
        let parent_id = self
            .get(node_id)
            .and_then(|node| Some(node.parent()?.node_id()));
        let removed = self.tree.remove(node_id, RemoveBehavior::DropChildren);
        if change_focus {
            if let Some(parent_id) = parent_id {
                self.set_focus_component_id(parent_id);
            } else {
                self.cycle_component()
            }
            debug_assert!(self.get(self.focused_component_id).is_some());
        }
        removed
    }

    fn get_mut(&mut self, id: NodeId) -> Option<NodeMut<'_, KindedComponent>> {
        self.tree.get_mut(id)
    }

    pub(crate) fn remain_only_current_component(&mut self) {
        if !self
            .root()
            .children()
            .any(|child| child.node_id() == self.focused_component_id())
        {
            return;
        }
        let current_component_id = self.focused_component_id();
        let root = self.root();
        let root_id = root.node_id();
        for node_id in root
            .traverse_pre_order()
            .filter(|node| node.node_id() != root_id && node.node_id() != self.focused_component_id)
            .map(|node| node.node_id())
            .collect_vec()
        {
            self.remove(node_id, false);
        }
        debug_assert_eq!(current_component_id, self.focused_component_id())
    }

    /// Append `component` to the Node of given `node_id`
    pub(crate) fn append_component(
        &mut self,
        node_id: NodeId,
        component: KindedComponent,
        focus: bool,
    ) -> NodeId {
        let id = if let Some(mut node) = self.get_mut(node_id) {
            node.append(component).node_id()
        } else {
            self.root_mut().append(component).node_id()
        };
        if focus {
            self.set_focus_component_id(id)
        }
        id
    }

    pub(crate) fn append_component_to_current(&mut self, component: KindedComponent, focus: bool) {
        self.append_component(self.focused_component_id, component, focus);
    }

    fn root_mut(&mut self) -> NodeMut<'_, KindedComponent> {
        self.tree.root_mut().unwrap()
    }

    /// This return everything except the root, but if only root exists, then the root will be returned.
    /// This behaviour ensures that the tree always contain a component.
    pub(crate) fn components(&self) -> Vec<KindedComponent> {
        if self.root().children().count() == 0 {
            Some(self.root().data().clone()).into_iter().collect_vec()
        } else {
            let root_id = self.root().node_id();
            self.root()
                .traverse_pre_order()
                .filter(|node| node.node_id() != root_id)
                .sorted_by_key(|node| node.data().kind)
                .map(|node| node.data().clone())
                .collect_vec()
        }
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
            self.remove(node_id, false)
        } else {
            None
        }
    }

    pub(crate) fn get_current_node_child_id(&self, kind: ComponentKind) -> Option<NodeId> {
        let node_id = self.focused_component_id;
        self.get_node_child_id(node_id, kind)
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

    pub(crate) fn get_current_node(&self) -> NodeRef<'_, KindedComponent> {
        self.get(self.focused_component_id)
            .unwrap_or_else(|| self.root())
    }

    pub(crate) fn replace_node_child(
        &mut self,
        id: NodeId,
        kind: ComponentKind,
        component: Rc<RefCell<dyn Component>>,
        focus: bool,
    ) -> NodeId {
        self.remove_node_child(id, kind);
        self.append_component(id, KindedComponent::new(kind, component), focus)
    }

    pub(crate) fn replace_current_node_child(
        &mut self,
        kind: ComponentKind,
        component: Rc<RefCell<dyn Component>>,
        focus: bool,
    ) -> NodeId {
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

    pub(crate) fn replace_root_node_child(
        &mut self,
        kind: ComponentKind,
        component: Rc<RefCell<dyn Component>>,
        focus: bool,
    ) -> NodeId {
        self.replace_node_child(self.root_id(), kind, component, focus)
    }

    pub(crate) fn remove_all_root_children(&mut self) {
        let children_ids = self
            .root()
            .children()
            .map(|node| node.node_id())
            .collect_vec();
        for child_id in children_ids {
            self.tree.remove(child_id, RemoveBehavior::DropChildren);
        }
        debug_assert_eq!(self.root().children().count(), 0);
    }

    pub(crate) fn get_component_by_kind(
        &self,
        kind: ComponentKind,
    ) -> Option<Rc<RefCell<dyn Component>>> {
        Some(
            self.root()
                .traverse_pre_order()
                .find(|node| node.data().kind() == kind)?
                .data()
                .component(),
        )
    }

    fn get_node_child_id(&self, node_id: NodeId, kind: ComponentKind) -> Option<NodeId> {
        Some(
            self.get(node_id)?
                .traverse_pre_order()
                .filter(|node| node.node_id() != node_id)
                .find(|node| node.data().kind == kind)?
                .node_id(),
        )
    }
}

impl Default for UiTree {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub(crate) struct KindedComponent {
    component: Rc<RefCell<dyn Component>>,
    kind: ComponentKind,
}

impl KindedComponent {
    pub(crate) fn new(
        kind: ComponentKind,
        component: Rc<RefCell<dyn Component>>,
    ) -> KindedComponent {
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

#[derive(PartialEq, Eq, Clone, Copy, Debug, PartialOrd, Ord)]
/// The order of variants in this enum is significant
/// Higher-rank variant will be rendered before lower-rank variant
pub(crate) enum ComponentKind {
    SuggestiveEditor,
    FileExplorer,
    QuickfixList,
    GlobalInfo,
    Prompt,
    Dropdown,
    DropdownInfo,
    EditorInfo,
    KeymapLegend,
    /// The root should not be rendered
    Root,
}
