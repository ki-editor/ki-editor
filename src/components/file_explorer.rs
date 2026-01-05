use itertools::Itertools;
use my_proc_macros::key;
use nonempty::NonEmpty;

use crate::{
    app::{Dispatch, Dispatches},
    buffer::BufferOwner,
    context::Context,
};
use shared::canonicalized_path::CanonicalizedPath;

use super::{
    component::Component,
    editor::Editor,
    editor_keymap_legend::{KeymapOverride, NormalModeOverride},
};

pub(crate) struct FileExplorer {
    editor: Editor,
    tree: Tree,
}

pub(crate) fn file_explorer_normal_mode_override() -> NormalModeOverride {
    NormalModeOverride {
        append: Some(KeymapOverride {
            description: "Add Path",
            dispatch: Dispatch::OpenAddPathPrompt,
        }),
        change: Some(KeymapOverride {
            description: "Move Paths",
            dispatch: Dispatch::OpenMovePathsPrompt,
        }),
        delete: Some(KeymapOverride {
            description: "Delete Paths",
            dispatch: Dispatch::OpenDeletePathsPrompt,
        }),
        replace: Some(KeymapOverride {
            description: "Refresh",
            dispatch: Dispatch::RefreshFileExplorer,
        }),
        paste: Some(KeymapOverride {
            description: "Dup Path",
            dispatch: Dispatch::OpenDuplicateFilePrompt,
        }),
        open: Some(KeymapOverride {
            description: "Toggle/Open Paths",
            dispatch: Dispatch::ToggleOrOpenPaths,
        }),
        ..Default::default()
    }
}
impl FileExplorer {
    pub(crate) fn new(path: &CanonicalizedPath) -> anyhow::Result<Self> {
        let tree = Tree::new(path)?;
        let text = tree.render();
        let mut editor = Editor::from_text(
            crate::config::from_extension("yaml")
                .and_then(|language| language.tree_sitter_language()),
            &format!("{text}\n"),
        );
        editor.set_title("File Explorer".to_string());
        editor.set_normal_mode_override(file_explorer_normal_mode_override());
        Ok(Self { editor, tree })
    }

    pub(crate) fn expanded_folders(&self) -> Vec<CanonicalizedPath> {
        self.tree
            .walk_visible(Vec::new(), |result, node| Continuation {
                state: if matches!(node.kind, NodeKind::Directory { open: true, .. }) {
                    result.into_iter().chain(Some(node.path.clone())).collect()
                } else {
                    result
                },
                kind: ContinuationKind::Continue,
            })
    }

    pub(crate) fn reveal(
        &mut self,
        path: &CanonicalizedPath,
        context: &Context,
    ) -> anyhow::Result<Dispatches> {
        let tree = std::mem::take(&mut self.tree);
        self.tree = tree.reveal(path)?;
        self.refresh_editor(context)?;
        if let Some(index) = self.tree.find_index(path) {
            self.editor_mut().select_line_at(index, context)
        } else {
            Ok(Dispatches::default())
        }
    }

    pub(crate) fn refresh(&mut self, context: &Context) -> anyhow::Result<()> {
        let tree = std::mem::take(&mut self.tree);
        self.tree = tree.refresh(context.current_working_directory())?;
        self.refresh_editor(context)?;
        Ok(())
    }

    fn refresh_editor(&mut self, context: &Context) -> anyhow::Result<()> {
        let text = self.tree.render();
        self.editor_mut().set_content(&text, context)
    }

    fn get_current_node(&self) -> anyhow::Result<Option<Node>> {
        let position = self.editor().get_cursor_position()?;
        Ok(self.tree.get(position.line))
    }

    fn get_selected_nodes(&self) -> anyhow::Result<Vec<Node>> {
        let line_indices = self.editor().get_selected_lines_indices()?;
        Ok(line_indices
            .into_iter()
            .filter_map(|line_index| self.tree.get(line_index))
            .collect_vec())
    }

    pub(crate) fn get_selected_paths(&self) -> anyhow::Result<Vec<CanonicalizedPath>> {
        self.get_selected_nodes()
            .map(|nodes| nodes.into_iter().map(|node| node.path).collect_vec())
    }

    pub(crate) fn get_current_path(&self) -> anyhow::Result<Option<CanonicalizedPath>> {
        self.get_current_node()
            .map(|node| node.map(|node| node.path))
    }

    pub(crate) fn toggle_or_open_paths(
        &mut self,
        context: &Context,
    ) -> Result<Dispatches, anyhow::Error> {
        let nodes = self.get_selected_nodes()?;
        let Some(nodes) = NonEmpty::from_vec(nodes) else {
            return Err(anyhow::anyhow!("No paths are selected."));
        };
        if nodes.len() == 1 {
            let node = nodes.first();
            match node.kind {
                NodeKind::File => Ok([
                    Dispatch::CloseCurrentWindow,
                    Dispatch::OpenFile {
                        path: node.path.clone(),
                        owner: BufferOwner::User,
                        focus: true,
                    },
                ]
                .to_vec()
                .into()),
                NodeKind::Directory { .. } => {
                    let tree = std::mem::take(&mut self.tree);
                    self.tree = tree.toggle(&node.path, |open| !open);
                    self.refresh_editor(context)?;
                    Ok(Vec::new().into())
                }
            }
        } else {
            let nodes = NonEmpty::from_vec(
                nodes
                    .into_iter()
                    .filter(|node| matches!(node.kind, NodeKind::File))
                    .map(|node| node.path)
                    .collect(),
            );
            match nodes {
                Some(nodes) => Ok(Dispatches::new(
                    [
                        Dispatch::CloseCurrentWindow,
                        Dispatch::OpenAndMarkFiles(nodes),
                    ]
                    .to_vec(),
                )),
                None => Ok(Default::default()),
            }
        }
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
                            format!("{head} :")
                        } else {
                            format!("{head} :\n{tail}")
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

    fn handle_key_event(
        &mut self,
        context: &crate::context::Context,
        event: event::KeyEvent,
    ) -> Result<Dispatches, anyhow::Error> {
        match event {
            key!("enter") => self.toggle_or_open_paths(context),
            _ => self.editor.handle_key_event(context, event),
        }
    }
}

#[cfg(test)]
mod test_file_explorer {
    use my_proc_macros::{key, keys};

    use crate::buffer::BufferOwner;
    use crate::components::editor::{Direction, IfCurrentNotFound};
    use crate::selection::SelectionMode;
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
   - 📘  hello.ts
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
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                App(RevealInExplorer(s.main_rs())),
                Expect(ComponentCount(1)),
                App(HandleKeyEvents(keys!("f").to_vec())),
                Expect(ComponentCount(2)),
                Expect(CurrentComponentTitle("Move paths".to_string())),
                Editor(Insert("/hello/world.rs".to_string())),
                App(HandleKeyEvent(key!("enter"))),
                Expect(ComponentCount(2)),
                Expect(OpenedFilesCount(1)),
                Expect(CurrentComponentTitle("File Explorer".to_string())),
            ])
        })
    }

    #[test]
    fn delete_multiple_paths_at_once_using_multiple_selections() -> anyhow::Result<()> {
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
   - 📘  hello.ts
   - 🦀  main.rs
"
                    .trim_matches('\n')
                    .to_string(),
                )),
                Editor(MatchLiteral("Cargo".to_owned())),
                Editor(CursorAddToAllSelections),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, SyntaxNode)),
                Expect(CurrentSelectedTexts(&["🔒  Cargo.lock", "📄  Cargo.toml"])),
                App(OpenDeletePathsPrompt),
                // Pick Yes
                App(HandleKeyEvent(key!("d"))),
                // Expect the two files (Cargo.lock and Cargo.toml) are deleted
                Expect(FileExplorerContent(
                    "
 - 📁  .git/ :
 - 🙈  .gitignore
 - 📂  src/ :
   - 🦀  foo.rs
   - 📘  hello.ts
   - 🦀  main.rs
"
                    .trim_matches('\n')
                    .to_string(),
                )),
            ])
        })
    }
    #[test]
    fn delete_multiple_paths_at_once_using_extened_selections() -> anyhow::Result<()> {
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
   - 📘  hello.ts
   - 🦀  main.rs
"
                    .trim_matches('\n')
                    .to_string(),
                )),
                Editor(MatchLiteral("Cargo.lock".to_owned())),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
                Editor(EnableSelectionExtension),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&[
                    "- 🔒  Cargo.lock\n - 📄  Cargo.toml",
                ])),
                App(OpenDeletePathsPrompt),
                // Pick Yes
                App(HandleKeyEvent(key!("d"))),
                // Expect the two files (Cargo.lock and Cargo.toml) are deleted
                Expect(FileExplorerContent(
                    "
 - 📁  .git/ :
 - 🙈  .gitignore
 - 📂  src/ :
   - 🦀  foo.rs
   - 📘  hello.ts
   - 🦀  main.rs
"
                    .trim_matches('\n')
                    .to_string(),
                )),
            ])
        })
    }

    #[test]
    fn open_multiple_paths_at_once_using_extened_selections() -> anyhow::Result<()> {
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
   - 📘  hello.ts
   - 🦀  main.rs
"
                    .trim_matches('\n')
                    .to_string(),
                )),
                Editor(MatchLiteral("Cargo.lock".to_owned())),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
                Editor(EnableSelectionExtension),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&[
                    "- 🔒  Cargo.lock\n - 📄  Cargo.toml",
                ])),
                App(HandleKeyEvent(key!("enter"))),
                // Expect the two files are opened and marked
                Expect(MarkedFiles(
                    [
                        s.new_path("Cargo.lock").try_into().unwrap(),
                        s.new_path("Cargo.toml").try_into().unwrap(),
                    ]
                    .to_vec(),
                )),
            ])
        })
    }

    #[test]
    fn rename_multiple_paths_at_once_using_extened_selections() -> anyhow::Result<()> {
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
   - 📘  hello.ts
   - 🦀  main.rs
"
                    .trim_matches('\n')
                    .to_string(),
                )),
                Editor(MatchLiteral("Cargo.lock".to_owned())),
                Editor(SetSelectionMode(IfCurrentNotFound::LookForward, Line)),
                Editor(EnableSelectionExtension),
                Editor(MoveSelection(Right)),
                Expect(CurrentSelectedTexts(&[
                    "- 🔒  Cargo.lock\n - 📄  Cargo.toml",
                ])),
                App(OpenMovePathsPrompt),
                // Expect the prompt shows the selected paths
                Expect(CurrentComponentContentMatches(lazy_regex::regex!(
                    "Cargo.lock"
                ))),
                Expect(CurrentComponentContentMatches(lazy_regex::regex!(
                    "Cargo.toml"
                ))),
                // Add ".x" to the end of the paths
                Editor(EnterNormalMode),
                Editor(SetSelectionMode(
                    IfCurrentNotFound::LookForward,
                    SelectionMode::Line,
                )),
                Editor(CursorAddToAllSelections),
                Editor(EnterInsertMode(Direction::End)),
                App(HandleKeyEvents(keys!(". x enter").to_vec())),
                // Expect the two files paths are appended with ".x"
                Expect(FileExplorerContent(
                    "
 - 📁  .git/ :
 - 🙈  .gitignore
 - 📄  Cargo.lock.x
 - 📄  Cargo.toml.x
 - 📂  src/ :
   - 🦀  foo.rs
   - 📘  hello.ts
   - 🦀  main.rs
"
                    .trim_matches('\n')
                    .to_string(),
                )),
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
