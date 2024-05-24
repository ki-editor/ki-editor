use itertools::Itertools;
use my_proc_macros::key;

use crate::app::{Dispatch, Dispatches, YesNoPrompt};
use shared::canonicalized_path::CanonicalizedPath;

use super::{
    component::Component,
    editor::Editor,
    keymap_legend::{Keymap, Keymaps},
};

pub(crate) struct FileExplorer {
    editor: Editor,
    tree: Tree,
}

impl FileExplorer {
    pub(crate) fn new(path: &CanonicalizedPath) -> anyhow::Result<Self> {
        let tree = Tree::new(path)?;
        let text = tree.render();
        let mut editor = Editor::from_text(
            shared::language::from_extension("yaml")
                .and_then(|language| language.tree_sitter_language()),
            &format!("{}\n", text),
        );
        editor.set_title("File Explorer".to_string());
        Ok(Self { editor, tree })
    }

    pub(crate) fn reveal(&mut self, path: &CanonicalizedPath) -> anyhow::Result<Dispatches> {
        let tree = std::mem::take(&mut self.tree);
        self.tree = tree.reveal(path)?;
        self.refresh_editor()?;
        if let Some(index) = self.tree.find_index(path) {
            self.editor_mut().select_line_at(index)
        } else {
            Ok(Dispatches::default())
        }
    }

    pub(crate) fn refresh(&mut self, working_directory: &CanonicalizedPath) -> anyhow::Result<()> {
        let tree = std::mem::take(&mut self.tree);
        self.tree = tree.refresh(working_directory)?;
        self.refresh_editor()?;
        Ok(())
    }

    fn refresh_editor(&mut self) -> anyhow::Result<()> {
        let text = self.tree.render();
        self.editor_mut().set_content(&text)
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
        .flat_map(|entry| -> anyhow::Result<Node> {
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
    fn new(working_directory: &CanonicalizedPath) -> anyhow::Result<Self> {
        let nodes = get_nodes(working_directory)?;
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
                let content = match &node.kind {
                    NodeKind::File => format!("{}  {}", node.path.icon(), node.name),
                    NodeKind::Directory { open, children } => {
                        let icon = if *open { "📂" } else { "📁" };
                        let head = format!("{}  {}{}", icon, node.name, "/");

                        let tail = if *open {
                            children
                                .as_ref()
                                .map(|tree| tree.render_with_indent(indent + 1))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };
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
                components.join(std::path::MAIN_SEPARATOR_STR).try_into()
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

    fn refresh(self, working_directory: &CanonicalizedPath) -> anyhow::Result<Self> {
        let opened_paths = self.walk_visible(Vec::new(), |result, node| Continuation {
            kind: ContinuationKind::Continue,
            state: match &node.kind {
                NodeKind::File => result,
                NodeKind::Directory { open, .. } => {
                    if *open {
                        result
                            .into_iter()
                            .chain(Some(node.path.clone()))
                            .collect_vec()
                    } else {
                        result
                    }
                }
            },
        });
        let tree = Tree::new(working_directory)?;
        log::info!("opened_paths = {:?}", opened_paths);
        let tree = opened_paths
            .into_iter()
            .fold(tree, |tree, path| tree.toggle(&path, |_| true));
        Ok(tree)
    }
}

#[derive(Clone)]
struct Node {
    name: String,
    path: CanonicalizedPath,
    kind: NodeKind,
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

    fn contextual_keymaps(&self) -> Vec<super::keymap_legend::KeymapLegendSection> {
        self.get_current_node()
            .ok()
            .flatten()
            .map(|node| super::keymap_legend::KeymapLegendSection {
                title: "File Explorer".to_string(),
                keymaps: Keymaps::new(&[
                    Keymap::new(
                        "a",
                        "Add file (or postfix with / for folder)".to_string(),
                        Dispatch::OpenAddPathPrompt(node.path.clone()),
                    ),
                    Keymap::new(
                        "d",
                        "Delete path".to_string(),
                        Dispatch::OpenYesNoPrompt(YesNoPrompt {
                            title: format!("Delete \"{}\"?", node.path.display_absolute()),
                            yes: Box::new(Dispatch::DeletePath(node.path.clone())),
                        }),
                    ),
                    Keymap::new(
                        "m",
                        "Move path".to_string(),
                        Dispatch::OpenMoveFilePrompt(node.path.clone()),
                    ),
                    Keymap::new("r", "Refresh".to_string(), Dispatch::RefreshFileExplorer),
                ]),
            })
            .into_iter()
            .collect()
    }

    fn handle_key_event(
        &mut self,
        context: &crate::context::Context,
        event: event::KeyEvent,
    ) -> Result<Dispatches, anyhow::Error> {
        match event {
            key!("enter") => {
                if let Some(node) = self.get_current_node()? {
                    match node.kind {
                        NodeKind::File => Ok([
                            Dispatch::CloseCurrentWindow,
                            Dispatch::OpenFile(node.path.clone()),
                        ]
                        .to_vec()
                        .into()),
                        NodeKind::Directory { .. } => {
                            let tree = std::mem::take(&mut self.tree);
                            self.tree = tree.toggle(&node.path, |open| !open);
                            self.refresh_editor()?;
                            Ok(Vec::new().into())
                        }
                    }
                } else {
                    Ok(Vec::new().into())
                }
            }
            _ => self.editor.handle_key_event(context, event),
        }
    }
}

#[cfg(test)]
mod test_file_explorer {
    use my_proc_macros::{key, keys};

    use crate::test_app::*;

    #[test]
    fn reveal() -> Result<(), anyhow::Error> {
        execute_test(|s| {
            Box::new([
                App(RevealInExplorer(s.main_rs())),
                Expect(FileExplorerContent(
                    "
 - 📁  .git/ :
 - 🙈  .gitignore
 - 🔒  Cargo.lock
 - 📄  Cargo.toml
 - 📂  src/ :
   - 🦀  foo.rs
   - 🦀  main.rs
"
                    .trim_matches('\n')
                    .to_string(),
                )),
                Expect(CurrentSelectedTexts(&["   - 🦀  main.rs"])),
                App(RevealInExplorer(s.foo_rs())),
                Expect(CurrentSelectedTexts(&["   - 🦀  foo.rs\n"])),
            ])
        })
    }

    #[test]
    fn move_path() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(OpenFile(s.main_rs())),
                App(RevealInExplorer(s.main_rs())),
                Expect(ComponentCount(1)),
                App(HandleKeyEvents(keys!("space m").to_vec())),
                Expect(ComponentCount(2)),
                Expect(CurrentComponentTitle("Move path")),
                Editor(Insert("/hello/world.rs".to_string())),
                App(HandleKeyEvent(key!("enter"))),
                Expect(ComponentCount(2)),
                Expect(OpenedFilesCount(1)),
                Expect(CurrentComponentTitle("File Explorer")),
            ])
        })
    }

    #[test]
    fn open_file() -> anyhow::Result<()> {
        execute_test(|s| {
            Box::new([
                App(RevealInExplorer(s.main_rs())),
                Expect(ComponentCount(1)),
                App(HandleKeyEvent(key!("enter"))),
                Expect(ComponentCount(1)),
                Expect(CurrentComponentPath(Some(s.main_rs()))),
            ])
        })
    }
}
