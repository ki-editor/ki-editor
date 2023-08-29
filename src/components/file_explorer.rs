use itertools::Itertools;
use my_proc_macros::key;

use crate::{
    canonicalized_path::CanonicalizedPath,
    screen::{Dispatch, YesNoPrompt},
};

use super::{component::Component, editor::Editor, keymap_legend::Keymap};

pub struct FileExplorer {
    editor: Editor,
    tree: Tree,
}

impl FileExplorer {
    pub fn new(path: &CanonicalizedPath) -> anyhow::Result<Self> {
        let tree = Tree::new(path)?;
        let text = tree.render();
        let mut editor = Editor::from_text(
            crate::language::from_extension("yaml")
                .and_then(|language| language.tree_sitter_language())
                .unwrap_or(tree_sitter_md::language()),
            &text,
        );
        editor.set_title("File Explorer".to_string());
        Ok(Self { editor, tree })
    }

    pub fn reveal(&mut self, path: &CanonicalizedPath) -> anyhow::Result<()> {
        let tree = std::mem::replace(&mut self.tree, Tree::default());
        self.tree = tree.reveal(path)?;
        self.refresh_editor();
        if let Some(index) = self.tree.find_index(path) {
            self.editor_mut().select_line_at(index)?;
        }
        Ok(())
    }

    fn refresh_editor(&mut self) {
        let text = self.tree.render();
        self.editor_mut().set_content(&text);
    }

    fn get_current_node(&self) -> anyhow::Result<Option<Node>> {
        let position = self.editor().get_cursor_position()?;
        Ok(self.tree.get(position.line))
    }
}

fn get_nodes(path: &CanonicalizedPath) -> anyhow::Result<Vec<Node>> {
    let directory = std::fs::read_dir(path)?;
    Ok(directory
        .flatten()
        .map(|entry| -> anyhow::Result<Node> {
            let path: CanonicalizedPath = entry.path().try_into()?;
            let kind = if entry.file_type()?.is_dir() {
                NodeKind::Directory {
                    open: false,
                    children: None,
                }
            } else {
                NodeKind::File
            };
            Ok(Node {
                name: entry.file_name().to_string_lossy().to_string(),
                path,
                kind,
            })
        })
        .flatten()
        .sorted_by(|a, b| a.name.cmp(&b.name))
        .collect())
}

#[derive(Clone, Default)]
struct Tree {
    nodes: Vec<Node>,
}

struct Continuation<T> {
    state: T,
    kind: ContinuationKind,
}

enum ContinuationKind {
    Continue,
    Stop,
}

impl Tree {
    fn new(path: &CanonicalizedPath) -> anyhow::Result<Self> {
        let nodes = get_nodes(path)?;
        Ok(Self { nodes })
    }

    fn map<F>(self, f: F) -> Self
    where
        F: Fn(Node) -> Node + Clone,
    {
        Tree {
            nodes: self.nodes.into_iter().map(f).collect(),
        }
    }

    fn toggle<F>(self, path: &CanonicalizedPath, change_open: F) -> Self
    where
        F: Fn(bool) -> bool + Clone,
    {
        self.map(|node| {
            let kind = match node.kind {
                NodeKind::File => node.kind,
                NodeKind::Directory { open, children } => NodeKind::Directory {
                    open: if node.path == *path {
                        change_open(open)
                    } else {
                        open
                    },
                    children: children.or_else(|| Tree::new(&node.path).ok()).map(|tree| {
                        if open {
                            tree.toggle(path, change_open.clone())
                        } else {
                            tree
                        }
                    }),
                },
            };
            Node { kind, ..node }
        })
    }

    fn walk_visible<T: Clone, F>(&self, result: T, f: F) -> T
    where
        F: Fn(T, &Node) -> Continuation<T> + Clone,
    {
        self.nodes
            .iter()
            .fold(
                Continuation {
                    state: result,
                    kind: ContinuationKind::Continue,
                },
                |continuation, node| match continuation.kind {
                    ContinuationKind::Continue => match &node.kind {
                        NodeKind::File => {
                            let result = f(continuation.state, node);
                            Continuation {
                                state: result.state,
                                kind: ContinuationKind::Continue,
                            }
                        }
                        NodeKind::Directory { open, children } => {
                            let result = f(continuation.state, node);
                            if *open {
                                Continuation {
                                    state: children
                                        .as_ref()
                                        .map(|tree| {
                                            tree.walk_visible(result.state.clone(), f.clone())
                                        })
                                        .unwrap_or(result.state),
                                    kind: ContinuationKind::Continue,
                                }
                            } else {
                                Continuation {
                                    state: result.state,
                                    kind: ContinuationKind::Continue,
                                }
                            }
                        }
                    },
                    ContinuationKind::Stop => continuation,
                },
            )
            .state
    }

    fn find_map<T: Clone, F>(&self, f: F) -> Option<T>
    where
        F: Fn(&Node, usize) -> Option<T> + Clone,
    {
        // Walk the tree
        let (result, _) = self.walk_visible((None, 0), |(result, current_index), node| {
            if let Some(result) = f(node, current_index) {
                Continuation {
                    state: (Some(result), current_index + 1),
                    kind: ContinuationKind::Stop,
                }
            } else {
                Continuation {
                    state: (result, current_index + 1),
                    kind: ContinuationKind::Continue,
                }
            }
        });
        result
    }

    fn get(&self, index: usize) -> Option<Node> {
        self.find_map(|node, current_index| {
            if current_index == index {
                Some(node.clone())
            } else {
                None
            }
        })
    }

    fn render_with_indent(&self, indent: usize) -> String {
        self.nodes
            .iter()
            .map(|node| {
                let node_name = if is_valid_lisp_atom(&node.name) {
                    node.name.clone()
                } else {
                    format!("\"{}\"", node.name)
                };
                let content = match &node.kind {
                    NodeKind::File => node_name,
                    NodeKind::Directory { open, children } => {
                        let head = format!("{}{}", node_name, "/");
                        let tail = if *open {
                            children
                                .as_ref()
                                .map(|tree| tree.render_with_indent(indent + 1))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };

                        let indicator = if *open { "📂" } else { "📁" };
                        let head = format!("{}  {}", indicator, head);
                        if tail.is_empty() {
                            format!("{} :", head)
                        } else {
                            format!("{} :\n{}", head, tail)
                        }
                    }
                };
                format!("{} - {}", "  ".repeat(indent), content)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render(&self) -> String {
        self.render_with_indent(0)
    }

    fn reveal(self, path: &CanonicalizedPath) -> anyhow::Result<Self> {
        let components = path.components();

        let paths = (1..=components.len())
            .map(|i| components[..i].to_vec())
            .map(|components| -> Result<CanonicalizedPath, _> {
                components
                    .join(&std::path::MAIN_SEPARATOR.to_string())
                    .try_into()
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(paths
            .into_iter()
            .fold(self, |tree, path| tree.toggle(&path, |_| true)))
    }

    fn find_index(&self, path: &CanonicalizedPath) -> Option<usize> {
        self.find_map(|node, current_index| {
            if node.path == *path {
                Some(current_index)
            } else {
                None
            }
        })
    }
}

#[derive(Clone)]
struct Node {
    name: String,
    path: CanonicalizedPath,
    kind: NodeKind,
}
impl Node {
    fn toggle(&self) -> Result<Vec<crate::screen::Dispatch>, anyhow::Error> {
        // Err(anyhow::anyhow!("Not implemented"))
        match &self.kind {
            NodeKind::File => Ok([Dispatch::OpenFile {
                path: self.path.clone(),
            }]
            .to_vec()),
            NodeKind::Directory { open, children } => {
                todo!()
            }
        }
    }
}

#[derive(Clone)]
enum NodeKind {
    File,
    Directory {
        open: bool,
        /// Should be populated lazily
        children: Option<Tree>,
    },
}

impl Component for FileExplorer {
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
        match event {
            key!("enter") => {
                if let Some(node) = self.get_current_node()? {
                    match node.kind {
                        NodeKind::File => Ok([Dispatch::OpenFile {
                            path: node.path.clone(),
                        }]
                        .to_vec()),
                        NodeKind::Directory { .. } => {
                            let tree = std::mem::replace(&mut self.tree, Tree::default());
                            self.tree = tree.toggle(&node.path, |open| !open);
                            self.refresh_editor();
                            Ok(Vec::new())
                        }
                    }
                } else {
                    Ok(Vec::new())
                }
            }
            key!("space") => {
                let current_node = self.get_current_node()?;
                return Ok([Dispatch::ShowKeymapLegend(
                    super::keymap_legend::KeymapLegendConfig {
                        owner_id: self.id(),
                        title: "File Explorer Actions".to_string(),
                        keymaps: current_node
                            .map(|node| {
                                [
                                    Keymap::new(
                                        "a",
                                        "Add file (or postfix with / for folder)",
                                        Dispatch::OpenAddPathPrompt(node.path.clone()),
                                    ),
                                    Keymap::new(
                                        "d",
                                        "Delete file",
                                        Dispatch::OpenYesNoPrompt(YesNoPrompt {
                                            owner_id: self.id(),
                                            title: format!("Delete \"{}\"?", node.path.display()),
                                            yes: Box::new(Dispatch::DeleteFile(node.path.clone())),
                                        }),
                                    ),
                                    Keymap::new(
                                        "r",
                                        "Rename file",
                                        Dispatch::OpenRenameFilePrompt(node.path.clone()),
                                    ),
                                ]
                                .to_vec()
                            })
                            .unwrap_or_default(),
                    },
                )]
                .to_vec());
            }
            _ => self.editor.handle_key_event(context, event),
        }
    }

    fn children(&self) -> Vec<Option<std::rc::Rc<std::cell::RefCell<dyn Component>>>> {
        Vec::new()
    }

    fn remove_child(&mut self, _component_id: super::component::ComponentId) {}
}

fn is_valid_lisp_atom(s: &str) -> bool {
    !s.is_empty() && !s.contains(char::is_whitespace) && !s.contains(&['(', ')'][..])
}
