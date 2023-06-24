use crate::{
    canonicalized_path::CanonicalizedPath,
    components::editor::Direction,
    edit::{Edit, EditTransaction},
    language::{self, Language},
    lsp::diagnostic::Diagnostic,
    position::Position,
    selection::{CharIndex, Selection, SelectionSet, ToRangeUsize},
    utils::find_previous,
};
use itertools::Itertools;
use regex::Regex;
use ropey::Rope;
use std::ops::Range;
use tree_sitter::{InputEdit, Node, Parser, Tree};
use tree_sitter_traversal::{traverse, Order};

#[derive(Clone)]
pub struct Buffer {
    rope: Rope,
    tree: Tree,
    ts_language: tree_sitter::Language,
    language: Option<Box<dyn Language>>,
    undo_patches: Vec<Patch>,
    redo_patches: Vec<Patch>,
    path: Option<CanonicalizedPath>,
    diagnostics: Vec<Diagnostic>,
}

impl Buffer {
    pub fn new(language: tree_sitter::Language, text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            ts_language: language,
            language: None,
            tree: {
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();
                parser.parse(text, None).unwrap()
            },
            undo_patches: Vec::new(),
            redo_patches: Vec::new(),
            path: None,
            diagnostics: Vec::new(),
        }
    }

    pub fn path(&self) -> Option<CanonicalizedPath> {
        self.path.clone()
    }

    pub fn set_path(&mut self, path: CanonicalizedPath) {
        self.path = Some(path);
    }

    pub fn words(&self) -> Vec<String> {
        let regex = regex::Regex::new(r"\b\w+").unwrap();
        let str = self.rope.to_string();

        regex
            .find_iter(&str)
            .map(|m| m.as_str().to_string())
            .unique()
            .collect()
    }

    pub fn find_words(&self, substring: &str) -> Vec<String> {
        let regex = regex::Regex::new(r"\b\w+").unwrap();
        let str = self.rope.to_string();

        regex
            .find_iter(&str)
            .map(|m| m.as_str().to_string())
            .filter(|m| m.to_lowercase().contains(&substring.to_lowercase()))
            .unique()
            .collect()
    }

    fn get_rope_and_tree(language: tree_sitter::Language, text: &str) -> (Rope, Tree) {
        let mut parser = Parser::new();
        parser.set_language(language).unwrap();
        let tree = parser.parse(text, None).unwrap();
        (Rope::from_str(text), tree)
    }

    pub fn given_range_is_node(&self, range: &Range<CharIndex>) -> bool {
        let byte_range = self.char_to_byte(range.start)..self.char_to_byte(range.end);
        self.tree
            .root_node()
            .descendant_for_byte_range(byte_range.start, byte_range.end)
            .map(|node| node.byte_range() == byte_range)
            .unwrap_or(false)
    }

    pub fn update(&mut self, text: &str) {
        (self.rope, self.tree) = Self::get_rope_and_tree(self.ts_language, text);
    }

    pub fn get_line(&self, char_index: CharIndex) -> Rope {
        self.rope.line(self.char_to_line(char_index)).into()
    }

    pub fn get_word_before_char_index(&self, char_index: CharIndex) -> String {
        let cursor_byte = self.char_to_byte(char_index);
        let regex = Regex::new(r"\b\w+").unwrap();
        let string = self.rope.to_string();
        let mut iter = regex.find_iter(&string);

        find_previous(
            &mut iter,
            |_, _| true,
            |match_| match_.start() >= cursor_byte,
        )
        .map(|match_| match_.as_str().to_string())
        .unwrap_or_default()
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn char_to_line(&self, char_index: CharIndex) -> usize {
        self.rope.char_to_line(char_index.0)
    }

    pub fn line_to_char(&self, line_index: usize) -> CharIndex {
        CharIndex(self.rope.line_to_char(line_index))
    }

    pub fn char_to_byte(&self, char_index: CharIndex) -> usize {
        self.rope.char_to_byte(char_index.0)
    }

    pub fn char_to_position(&self, char_index: CharIndex) -> Position {
        let line = self.char_to_line(char_index);
        Position {
            line,
            column: self
                .rope
                .try_line_to_char(line)
                .map(|line_start_char_index| char_index.0.saturating_sub(line_start_char_index))
                .unwrap_or(0),
        }
    }

    pub fn position_to_char(&self, position: Position) -> CharIndex {
        CharIndex(self.rope.line_to_char(position.line) + position.column)
    }

    pub fn byte_to_char(&self, byte_index: usize) -> CharIndex {
        CharIndex(self.rope.byte_to_char(byte_index))
    }

    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn slice(&self, range: &Range<CharIndex>) -> Rope {
        self.rope.slice(range.to_usize_range()).into()
    }

    pub fn get_nearest_node_after_char(&self, char_index: CharIndex) -> Option<Node> {
        let byte = self.char_to_byte(char_index);
        // Preorder is the main key here,
        // because preorder traversal walks the parent first
        traverse(self.tree.root_node().walk(), Order::Pre).find(|&node| node.start_byte() >= byte)
    }

    pub fn get_current_node<'a>(&'a self, selection: &Selection) -> Node<'a> {
        let node = self
            .tree
            .root_node()
            .descendant_for_byte_range(
                self.char_to_byte(selection.range.start),
                self.char_to_byte(selection.range.end),
            )
            .unwrap_or_else(|| self.tree.root_node());

        // Get the most ancestral node of this range
        //
        // This is because sometimes the parent of a node can have the same range as the node
        // itself.
        //
        // If we don't get the most ancestral node, then movements like "go to next sibling" will
        // not work as expected.
        let mut result = node;
        while let Some(parent) = result.parent() {
            if parent.start_byte() == node.start_byte() && parent.end_byte() == node.end_byte() {
                result = parent;
            } else {
                return result;
            }
        }

        node
    }

    pub fn get_next_token(&self, char_index: CharIndex, is_named: bool) -> Option<Node> {
        let byte = self.char_to_byte(char_index);
        self.traverse(Order::Post).find(|&node| {
            node.child_count() == 0 && (!is_named || node.is_named()) && node.end_byte() > byte
        })
    }

    pub fn get_prev_token(&self, char_index: CharIndex, is_named: bool) -> Option<Node> {
        let byte = self.char_to_byte(char_index);
        find_previous(
            self.traverse(Order::Pre),
            |node, _| node.child_count() == 0 && (!is_named || node.is_named()),
            |node| node.start_byte() >= byte,
        )
    }

    pub fn traverse(&self, order: Order) -> impl Iterator<Item = Node> {
        traverse(self.tree.walk(), order)
    }

    pub fn apply_edit_transaction(
        &mut self,
        edit_transaction: &EditTransaction,
        current_selection_set: SelectionSet,
    ) -> Result<(), anyhow::Error> {
        let before = self.rope.to_string();
        edit_transaction
            .edits()
            .into_iter()
            .fold(Ok(()), |result, edit| match result {
                Err(err) => Err(err),
                Ok(()) => self.apply_edit(edit),
            })?;

        self.add_undo_patch(current_selection_set, &before);

        Ok(())
    }

    /// This method assumes `self.rope` is already updated
    fn add_undo_patch(&mut self, current_selection_set: SelectionSet, before: &str) {
        let after = &self.rope.to_string();
        if before == after {
            return;
        }

        self.redo_patches.clear();
        self.undo_patches.push(Patch {
            selection_set: current_selection_set,
            patch: diffy::create_patch(after, before).to_string(),
        });
    }

    pub fn undo(&mut self, current_selection_set: SelectionSet) -> Option<SelectionSet> {
        if let Some(patch) = self.undo_patches.pop() {
            let redo_patch = self.revert_change(&patch, current_selection_set);
            self.redo_patches.push(redo_patch);
            Some(patch.selection_set)
        } else {
            log::info!("Nothing else to be undone");
            None
        }
    }

    pub fn redo(&mut self, current_selection_set: SelectionSet) -> Option<SelectionSet> {
        if let Some(patch) = self.redo_patches.pop() {
            let undo_patch = self.revert_change(&patch, current_selection_set);
            self.undo_patches.push(undo_patch);
            Some(patch.selection_set)
        } else {
            log::info!("Nothing else to be redone");
            None
        }
    }

    fn revert_change(&mut self, patch: &Patch, current_selection_set: SelectionSet) -> Patch {
        let before = self.rope.to_string();
        self.rope = diffy::apply(
            &self.rope.to_string(),
            &diffy::Patch::from_str(&patch.patch).unwrap(),
        )
        .unwrap()
        .into();

        let after = self.rope.to_string();

        let mut parser = tree_sitter::Parser::new();
        parser.set_language(self.tree.language()).unwrap();
        self.tree = parser.parse(&self.rope.to_string(), None).unwrap();

        Patch {
            selection_set: current_selection_set,
            patch: diffy::create_patch(&after, &before).to_string(),
        }
    }

    pub fn apply_edit(&mut self, edit: &Edit) -> Result<(), anyhow::Error> {
        let start_char_index = edit.start;
        let old_end_char_index = edit.end();
        let new_end_char_index = edit.start + edit.new.len_chars();

        let start_byte = self.char_to_byte(start_char_index);
        let old_end_byte = self.char_to_byte(old_end_char_index);
        let start_position = self.char_to_position(start_char_index);
        let old_end_position = self.char_to_position(old_end_char_index);

        self.rope.remove(edit.start.0..edit.end().0);
        self.rope
            .insert(edit.start.0, edit.new.to_string().as_str());

        let new_end_byte = self.char_to_byte(new_end_char_index);
        let new_end_position = self.char_to_position(new_end_char_index);

        let mut parser = tree_sitter::Parser::new();
        parser.set_language(self.tree.language()).unwrap();
        self.tree.edit(&InputEdit {
            start_byte,
            old_end_byte,
            new_end_byte,
            start_position: start_position.into(),
            old_end_position: old_end_position.into(),
            new_end_position: new_end_position.into(),
        });

        self.tree = parser.parse(&self.rope.to_string(), None).unwrap();

        Ok(())
    }

    pub fn has_syntax_error_at(&self, range: Range<CharIndex>) -> bool {
        let rope = &self.rope;
        if let Some(node) = self.tree.root_node().descendant_for_byte_range(
            rope.try_char_to_byte(range.start.0).unwrap_or(0),
            rope.try_char_to_byte(range.end.0).unwrap_or(0),
        ) {
            node.has_error()
        } else {
            false
        }
    }

    pub fn from_path(path: &CanonicalizedPath) -> anyhow::Result<Buffer> {
        let content = path.read()?;
        let language = language::from_path(path);

        let mut buffer = Buffer::new(
            language
                .as_ref()
                .and_then(|language| language.tree_sitter_language())
                .unwrap_or_else(tree_sitter_md::language),
            &content,
        );

        buffer.path = Some(path.clone());
        buffer.language = language;

        Ok(buffer)
    }

    fn get_formatted_content(&self) -> Option<String> {
        if !self.tree.root_node().has_error() {
            if let Some(content) = self.language.as_ref().and_then(|language| {
                language
                    .formatter()
                    .map(|formatter| formatter.format(&self.rope.to_string()))
            }) {
                match content {
                    Ok(content) => {
                        return Some(content);
                    }
                    Err(error) => {
                        log::info!("Error formatting: {}", error);
                    }
                }
            }
        }
        None
    }

    pub fn save(
        &mut self,
        current_selection_set: SelectionSet,
    ) -> anyhow::Result<Option<CanonicalizedPath>> {
        let before = self.rope.to_string();

        let content = if let Some(formatted_content) = self.get_formatted_content() {
            self.update(&formatted_content);
            self.add_undo_patch(current_selection_set, &before);
            formatted_content
        } else {
            before
        };

        if let Some(path) = &self.path.clone() {
            path.write(&content)?;

            Ok(Some(path.clone()))
        } else {
            log::info!("Buffer has no path");
            Ok(None)
        }
    }

    pub fn find_diagnostic(&self, range: &Range<CharIndex>) -> Option<&Diagnostic> {
        self.diagnostics.iter().find(|diagnostic| {
            let start = diagnostic.range.start.to_char_index(self);
            let end = diagnostic.range.end.to_char_index(self);

            start == range.start && end == range.end
        })
    }

    pub fn set_diagnostics(&mut self, mut diagnostics: Vec<Diagnostic>) {
        diagnostics.sort_by(|a, b| a.range.start.cmp(&b.range.start));
        self.diagnostics = diagnostics;
    }

    pub fn get_diagnostic(
        &self,
        current_range: &Range<CharIndex>,
        direction: &Direction,
    ) -> Option<Range<CharIndex>> {
        let mut iter = self.diagnostics.iter();
        match direction {
            Direction::Current => iter.find_map(|diagnostic| {
                let start = diagnostic.range.start.to_char_index(self);
                let end = diagnostic.range.end.to_char_index(self);
                if start >= current_range.start {
                    Some(start..end)
                } else {
                    None
                }
            }),
            Direction::Forward => iter.find_map(|diagnostic| {
                let start = diagnostic.range.start.to_char_index(self);
                let end = diagnostic.range.end.to_char_index(self);
                if start >= current_range.end {
                    Some(start..end)
                } else {
                    None
                }
            }),
            Direction::Backward => find_previous(
                iter,
                |_, _| true,
                |match_| match_.range.start.to_char_index(self) >= current_range.start,
            )
            .map(|item| item.range.start.to_char_index(self)..item.range.end.to_char_index(self)),
        }
    }

    pub fn language(&self) -> tree_sitter::Language {
        self.ts_language
    }

    pub fn get_char_at_position(&self, position: Position) -> Option<char> {
        let char_index = position.to_char_index(self).0;
        self.rope.get_char(char_index)
    }

    pub fn contains_position_range(&self, range: &Range<Position>) -> bool {
        self.try_position_to_char_index(range.end)
            .map(|end| end.0 < self.rope.len_chars())
            .unwrap_or(false)
    }

    pub fn try_position_to_char_index(&self, position: Position) -> Option<CharIndex> {
        let index = self.rope.try_line_to_char(position.line).ok()?;
        Some(CharIndex(index + position.column))
    }
}

#[derive(Clone, Debug)]
pub struct Patch {
    /// Used for restoring previous selection after undo/redo
    pub selection_set: SelectionSet,
    /// Unified format patch
    /// Why don't we store this is diffy::Patch? Because it requires a lifetime parameter
    pub patch: String,
}

#[cfg(test)]
mod test_buffer {
    use super::Buffer;

    #[test]
    fn find_words() {
        let buffer = Buffer::new(tree_sitter_md::language(), "foo bar baz baz");
        let words = buffer.find_words("Ba");

        // Should return unique words
        assert_eq!(words, vec!["bar", "baz"]);
    }

    mod auto_format {
        use std::fs::File;

        use tempfile::tempdir;

        use crate::{
            buffer::Buffer,
            canonicalized_path::CanonicalizedPath,
            selection::{CharIndex, SelectionSet},
        };

        /// The TempDir is returned so that the directory is not deleted
        /// when the TempDir object is dropped
        fn run_test(f: impl Fn(CanonicalizedPath, Buffer)) {
            let dir = tempdir().unwrap();

            let file_path = dir.path().join("main.rs");
            File::create(&file_path).unwrap();
            let path = CanonicalizedPath::try_from(file_path).unwrap();
            path.write("").unwrap();

            let buffer = Buffer::from_path(&path).unwrap();

            f(path, buffer)
        }

        #[test]
        fn should_format_code() {
            run_test(|path, mut buffer| {
                // Update the buffer with unformatted code
                buffer.update(" fn main\n() {}");

                // Save the buffer
                buffer.save(SelectionSet::default()).unwrap();

                // Expect the output is formatted
                let saved_content = path.read().unwrap();
                let buffer_content = buffer.rope.to_string();

                assert_eq!(saved_content, "fn main() {}\n");
                assert_eq!(buffer_content, "fn main() {}\n");

                // Expect the syntax tree is also updated
                assert_eq!(
                    buffer
                        .get_next_token(CharIndex::default(), false)
                        .unwrap()
                        .byte_range(),
                    0..2
                );
            })
        }

        #[test]
        /// The formatted output should be undoable,
        /// in case the formatter messed up the code.
        fn should_be_undoable() {
            run_test(|_, mut buffer| {
                let original = " fn main\n() {}";
                buffer.update(original);

                buffer.save(SelectionSet::default()).unwrap();

                // Expect the buffer is formatted
                assert_ne!(buffer.rope.to_string(), original);

                // Undo the buffer
                buffer.undo(SelectionSet::default()).unwrap();

                let content = buffer.rope.to_string();

                // Expect the content is reverted to the original
                assert_eq!(content, " fn main\n() {}");
            })
        }

        #[test]
        fn should_not_run_when_syntax_tree_is_malformed() {
            run_test(|_, mut buffer| {
                // Update the buffer to be invalid Rust code
                buffer.update("fn main() {");

                // Save the buffer
                buffer.save(SelectionSet::default()).unwrap();

                // Expect the buffer remain unchanged,
                // because the syntax tree is invalid
                assert_eq!(buffer.rope.to_string(), "fn main() {");
            })
        }
    }
}
