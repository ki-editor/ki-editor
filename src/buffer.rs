use crate::{
    char_index_range::CharIndexRange,
    components::{editor::Movement, suggestive_editor::Decoration},
    edit::{Action, ActionGroup, Edit, EditTransaction},
    git::hunk::Hunk,
    position::Position,
    selection::{CharIndex, Selection, SelectionSet},
    selection_mode::ByteRange,
    syntax_highlight::{HighlighedSpan, HighlighedSpans},
    themes::Theme,
    undo_tree::{Applicable, OldNew, UndoTree},
    utils::find_previous,
};
use itertools::Itertools;
use regex::Regex;
use ropey::Rope;
use shared::{
    canonicalized_path::CanonicalizedPath,
    language::{self, Language},
};
use std::{collections::HashSet, ops::Range};
use tree_sitter::{Node, Parser, Tree};
use tree_sitter_traversal::{traverse, Order};

#[derive(Clone)]
pub struct Buffer {
    rope: Rope,
    tree: Tree,
    treesitter_language: tree_sitter::Language,
    undo_tree: UndoTree<Patch>,
    language: Option<Language>,
    path: Option<CanonicalizedPath>,
    highlighted_spans: HighlighedSpans,
    theme: Theme,
    bookmarks: Vec<CharIndexRange>,
    decorations: Vec<Decoration>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Line {
    origin_position: Position,
    /// 0-based
    pub line: usize,
    pub content: String,
}

impl Buffer {
    pub fn new(language: tree_sitter::Language, text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            treesitter_language: language,
            language: None,
            tree: {
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();
                parser.parse(text, None).unwrap()
            },
            path: None,
            highlighted_spans: HighlighedSpans::default(),
            theme: Theme::default(),
            bookmarks: Vec::new(),
            decorations: Vec::new(),
            undo_tree: UndoTree::new(),
        }
    }
    pub fn content(&self) -> String {
        self.rope.to_string()
    }

    pub fn decorations(&self) -> &Vec<Decoration> {
        &self.decorations
    }
    pub fn set_decorations(&mut self, decorations: &Vec<Decoration>) {
        self.decorations = decorations.clone();
    }

    pub fn save_bookmarks(&mut self, new_ranges: Vec<CharIndexRange>) {
        let old_ranges = std::mem::replace(&mut self.bookmarks, vec![])
            .into_iter()
            .collect::<HashSet<_>>();
        let new_ranges = new_ranges.into_iter().collect::<HashSet<_>>();
        // We take the symmetric difference between the old ranges and the new ranges
        // so that user can unmark existing bookmark
        self.bookmarks = new_ranges
            .symmetric_difference(&old_ranges)
            .cloned()
            .collect_vec();
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
        let word = regex::Regex::new(r"\b\w+").unwrap();
        let str = self.rope.to_string();
        word.find_iter(&str)
            .map(|m| m.as_str().to_string())
            .filter(|m| m.to_lowercase().contains(&substring.to_lowercase()))
            .unique()
            .collect()
    }

    pub fn get_parent_lines(&self, line_number: usize) -> anyhow::Result<Vec<Line>> {
        let char_index = self.line_to_char(line_number)?;
        let node = self.get_nearest_node_after_char(char_index);
        fn get_parent_lines(
            buffer: &Buffer,
            node: Option<tree_sitter::Node>,
            lines: Vec<Line>,
        ) -> anyhow::Result<Vec<Line>> {
            let Some(node) = node else { return Ok(lines) };
            let start_position = buffer.byte_to_position(node.start_byte())?;

            let Some(line) = buffer.get_line_by_line_index(start_position.line) else {
                return Ok(lines);
            };
            let lines = lines
                .into_iter()
                .chain([Line {
                    origin_position: start_position,
                    line: start_position.line,
                    content: line.to_string(),
                }])
                .collect_vec();
            get_parent_lines(buffer, node.parent(), lines)
        }
        let parent_lines = get_parent_lines(self, node, Vec::new())?;
        Ok(parent_lines
            .into_iter()
            // Remove lines that contains no alphabet
            .filter(|line| line.content.chars().any(|c| c.is_alphanumeric()))
            .map(|line| Line {
                origin_position: line.origin_position,
                line: line.line,
                content: line.content.trim_end().to_owned(),
            })
            .unique()
            // Unique by their indentation, this assumes parent of different hieararchies has different indentations
            .unique_by(|line| line.origin_position.column)
            .unique_by(|line| line.content.trim_end().to_string())
            .collect_vec()
            .into_iter()
            .rev()
            // Remove line that is not above `line`
            .filter(|line| line.line < line_number)
            .collect_vec())
    }

    fn get_rope_and_tree(language: tree_sitter::Language, text: &str) -> (Rope, Tree) {
        let mut parser = Parser::new();
        parser.set_language(language).unwrap();
        let tree = parser.parse(text, None).unwrap();
        // let start_char_index = edit.start;
        // let old_end_char_index = edit.end();
        // let new_end_char_index = edit.start + edit.new.len_chars();

        // let start_byte = self.char_to_byte(start_char_index);
        // let old_end_byte = self.char_to_byte(old_end_char_index);
        // let start_position = self.char_to_point(start_char_index);
        // let old_end_position = self.char_to_point(old_end_char_index);

        // self.rope.remove(edit.start.0..edit.end().0);
        // self.rope
        //     .insert(edit.start.0, edit.new.to_string().as_str());

        // let new_end_byte = self.char_to_byte(new_end_char_index);
        // let new_end_position = self.char_to_point(new_end_char_index);

        // let mut parser = tree_sitter::Parser::new();
        // parser.set_language(self.tree.language()).unwrap();
        // self.tree.edit(&InputEdit {
        //     start_byte,
        //     old_end_byte,
        //     new_end_byte,
        //     start_position,
        //     old_end_position,
        //     new_end_position,
        // });

        // self.tree = parser
        //     .parse(&self.rope.to_string(), Some(&self.tree))
        //     .unwrap();

        (Rope::from_str(text), tree)
    }

    pub fn given_range_is_node(&self, range: &CharIndexRange) -> bool {
        let Some(start) = self.char_to_byte(range.start).ok() else {
            return false;
        };
        let Some(end) = self.char_to_byte(range.end).ok() else {
            return false;
        };
        let byte_range = start..end;
        self.tree
            .root_node()
            .descendant_for_byte_range(byte_range.start, byte_range.end)
            .map(|node| node.byte_range() == byte_range)
            .unwrap_or(false)
    }

    pub fn update_highlighted_spans(&mut self, spans: HighlighedSpans) {
        self.highlighted_spans = spans;
    }

    pub fn update(&mut self, text: &str) {
        (self.rope, self.tree) = Self::get_rope_and_tree(self.treesitter_language, text);
    }

    pub fn get_line_by_char_index(&self, char_index: CharIndex) -> anyhow::Result<Rope> {
        Ok(self
            .rope
            .get_line(self.char_to_line(char_index)?)
            .map(Ok)
            .unwrap_or_else(|| {
                Err(anyhow::anyhow!(
                    "get_line: char_index {:?} is out of bound",
                    char_index
                ))
            })?
            .into())
    }

    pub fn get_word_before_char_index(&self, char_index: CharIndex) -> anyhow::Result<String> {
        let cursor_byte = self.char_to_byte(char_index)?;
        let regex = Regex::new(r"\b\w+").unwrap();
        let string = self.rope.to_string();
        let mut iter = regex.find_iter(&string);

        Ok(find_previous(
            &mut iter,
            |_, _| true,
            |match_| match_.start() >= cursor_byte,
        )
        .map(|match_| match_.as_str().to_string())
        .unwrap_or_default())
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn char_to_line(&self, char_index: CharIndex) -> anyhow::Result<usize> {
        Ok(self.rope.try_char_to_line(char_index.0)?)
    }

    pub fn char_to_line_start(&self, char_index: CharIndex) -> anyhow::Result<CharIndex> {
        let line = self.char_to_line(char_index)?;
        self.line_to_char(line)
    }

    pub fn char_to_line_end(&self, char_index: CharIndex) -> anyhow::Result<CharIndex> {
        let line = self.char_to_line(char_index)?;
        self.line_to_char(line + 1).map(|char_index| char_index - 1)
    }

    pub fn line_to_char(&self, line_index: usize) -> anyhow::Result<CharIndex> {
        Ok(CharIndex(self.rope.try_line_to_char(line_index)?))
    }

    pub fn char_to_byte(&self, char_index: CharIndex) -> anyhow::Result<usize> {
        Ok(self.rope.try_char_to_byte(char_index.0)?)
    }

    pub fn char_to_position(&self, char_index: CharIndex) -> anyhow::Result<Position> {
        let line = self.char_to_line(char_index)?;
        Ok(Position {
            line,
            column: self
                .rope
                .try_line_to_char(line)
                .map(|line_start_char_index| char_index.0.saturating_sub(line_start_char_index))
                .unwrap_or(0),
        })
    }

    pub fn position_to_char(&self, position: Position) -> anyhow::Result<CharIndex> {
        let line = position.line.clamp(0, self.len_lines());
        let column = position.column.clamp(
            0,
            self.get_line_by_line_index(line)
                .map(|slice| slice.len_chars())
                .unwrap_or_default(),
        );
        Ok(CharIndex(self.rope.try_line_to_char(line)? + column))
    }

    pub fn byte_to_char(&self, byte_index: usize) -> anyhow::Result<CharIndex> {
        Ok(CharIndex(self.rope.try_byte_to_char(byte_index)?))
    }

    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn slice(&self, range: &CharIndexRange) -> anyhow::Result<Rope> {
        let slice = self.rope.get_slice(range.start.0..range.end.0);
        match slice {
            Some(slice) => Ok(slice.into()),
            None => Err(anyhow::anyhow!(
                "Unable to obtain slice for range: {:#?}",
                range
            )),
        }
    }

    pub fn get_nearest_node_after_char(&self, char_index: CharIndex) -> Option<Node> {
        let byte = self.char_to_byte(char_index).ok()?;
        // Preorder is the main key here,
        // because preorder traversal walks the parent first
        traverse(self.tree.root_node().walk(), Order::Pre).find(|&node| node.start_byte() >= byte)
    }

    pub fn get_current_node<'a>(
        &'a self,
        selection: &Selection,
        get_largest_end: bool,
    ) -> anyhow::Result<Node<'a>> {
        let range = selection.range();
        let start = self.char_to_byte(range.start)?;
        let (start, end) = if get_largest_end {
            (start, start + 1)
        } else {
            (start, self.char_to_byte(range.end)?)
        };
        let node = self
            .tree
            .root_node()
            .descendant_for_byte_range(start, end)
            .unwrap_or_else(|| self.tree.root_node());

        // Get the most ancestral node of this range
        //
        // This is because sometimes the parent of a node can have the same range as the node
        // itself.
        //
        // If we don't get the most ancestral node, then movements like "go to next sibling" will
        // not work as expected.
        let mut result = node;
        let root_node_id = self.tree.root_node().id();
        while let Some(parent) = result.parent() {
            if parent.start_byte() == node.start_byte()
                && root_node_id != parent.id()
                && (get_largest_end || node.end_byte() == parent.end_byte())
            {
                result = parent;
            } else {
                return Ok(result);
            }
        }

        Ok(node)
    }

    pub fn get_next_token(&self, char_index: CharIndex, is_named: bool) -> Option<Node> {
        let byte = self.char_to_byte(char_index).ok()?;
        self.traverse(Order::Post).find(|&node| {
            node.child_count() == 0 && (!is_named || node.is_named()) && node.end_byte() > byte
        })
    }

    pub fn get_prev_token(&self, char_index: CharIndex, is_named: bool) -> Option<Node> {
        let byte = self.char_to_byte(char_index).ok()?;
        find_previous(
            self.traverse(Order::Pre),
            |node, _| node.child_count() == 0 && (!is_named || node.is_named()),
            |node| node.start_byte() >= byte,
        )
    }

    pub fn traverse(&self, order: Order) -> impl Iterator<Item = Node> {
        traverse(self.tree.walk(), order)
    }

    /// Returns the new selection set
    pub fn apply_edit_transaction(
        &mut self,
        edit_transaction: &EditTransaction,
        current_selection_set: SelectionSet,
    ) -> Result<SelectionSet, anyhow::Error> {
        let before = self.rope.to_string();
        let new_selection_set = edit_transaction
            .selection_set(current_selection_set.mode.clone())
            .unwrap_or_else(|| current_selection_set.clone());
        let current_buffer_state = BufferState {
            selection_set: current_selection_set,
            bookmarks: self.bookmarks.clone(),
        };

        edit_transaction
            .edits()
            .into_iter()
            .fold(Ok(()), |result, edit| match result {
                Err(err) => Err(err),
                Ok(()) => self.apply_edit(edit),
            })?;

        let new_buffer_state = BufferState {
            selection_set: new_selection_set.clone(),
            bookmarks: self.bookmarks.clone(),
        };

        self.add_undo_patch(current_buffer_state, new_buffer_state.clone(), &before);
        self.reparse_tree()?;

        Ok(new_selection_set)
    }

    fn apply_edit(&mut self, edit: &Edit) -> Result<(), anyhow::Error> {
        // Update all the bookmarks
        let bookmarks = std::mem::take(&mut self.bookmarks);
        self.bookmarks = bookmarks
            .into_iter()
            .filter_map(|bookmark| bookmark.apply_edit(edit))
            .collect();
        self.rope.try_remove(edit.range.start.0..edit.end().0)?;
        self.rope
            .try_insert(edit.range.start.0, edit.new.to_string().as_str())?;
        Ok(())
    }

    /// This method assumes `self.rope` is already updated
    fn add_undo_patch(
        &mut self,
        old_buffer_state: BufferState,
        new_buffer_state: BufferState,
        before: &str,
    ) {
        let after = &self.rope.to_string();
        if before == after {
            return;
        }
        let old_new = OldNew {
            old_to_new: Patch {
                patch: diffy::create_patch(before, after).to_string(),
                state: new_buffer_state,
            },
            new_to_old: Patch {
                patch: diffy::create_patch(after, before).to_string(),
                state: old_buffer_state,
            },
        };
        self.undo_tree
            .edit(&mut before.to_owned(), old_new)
            .unwrap();
    }

    pub fn undo(&mut self) -> anyhow::Result<Option<SelectionSet>> {
        self.undo_tree_apply_movement(Movement::Previous)
    }

    pub fn display_history(&self) -> String {
        self.undo_tree.display()
    }

    pub fn undo_tree_apply_movement(
        &mut self,
        movement: Movement,
    ) -> anyhow::Result<Option<SelectionSet>> {
        let mut content = self.rope.to_string();
        let state = self.undo_tree.apply_movement(&mut content, movement)?;

        self.update(&content);
        if let Some(BufferState {
            selection_set,
            bookmarks,
        }) = state
        {
            self.bookmarks = bookmarks;

            Ok(Some(selection_set))
        } else {
            Ok(None)
        }
    }

    pub fn redo(&mut self) -> anyhow::Result<Option<SelectionSet>> {
        self.undo_tree_apply_movement(Movement::Next)
    }

    pub fn has_syntax_error_at(&self, range: CharIndexRange) -> bool {
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

    fn reparse_tree(&mut self) -> anyhow::Result<()> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(self.tree.language())?;
        if let Some(tree) = parser.parse(&self.rope.to_string(), None) {
            self.tree = tree
        }

        Ok(())
    }

    pub fn get_formatted_content(&self) -> Option<String> {
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
        log::info!("Unable to get formatted content because of syntax error");
        None
    }

    pub fn save(
        &mut self,
        current_selection_set: SelectionSet,
    ) -> anyhow::Result<Option<CanonicalizedPath>> {
        let before = self.rope.to_string();

        let content = if let Some(formatted_content) = self.get_formatted_content() {
            if let Ok(edit_transaction) = self.get_edit_transaction(&formatted_content) {
                self.apply_edit_transaction(&edit_transaction, current_selection_set)?;
                formatted_content
            } else {
                before
            }
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

    pub fn highlighted_spans(&self) -> Vec<HighlighedSpan> {
        self.highlighted_spans.0.clone()
    }

    pub fn language(&self) -> Option<Language> {
        self.language.clone()
    }

    pub fn set_language(&mut self, language: Language) -> Result<(), anyhow::Error> {
        self.language = Some(language);
        self.reparse_tree()
    }

    pub fn treesitter_language(&self) -> tree_sitter::Language {
        self.treesitter_language
    }

    pub fn get_char_at_position(&self, position: Position) -> Option<char> {
        let char_index = position.to_char_index(self).ok()?.0;
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

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    pub fn line_to_byte(&self, line_index: usize) -> anyhow::Result<usize> {
        Ok(self.rope.try_line_to_byte(line_index)?)
    }

    pub fn position_to_byte(&self, start: Position) -> anyhow::Result<usize> {
        let start = self.position_to_char(start)?;
        self.char_to_byte(start)
    }

    pub fn line_to_char_range(&self, line: usize) -> anyhow::Result<CharIndexRange> {
        let start = self.line_to_char(line)?;
        let end = self.line_to_char(line + 1)? - 1;
        Ok((start..end).into())
    }

    pub fn line_to_byte_range(&self, line: usize) -> anyhow::Result<ByteRange> {
        let start = self.line_to_byte(line)?;
        let end = self.line_to_byte(line + 1)?.saturating_sub(1);
        Ok(ByteRange::new(start..end))
    }

    pub fn bookmarks(&self) -> Vec<CharIndexRange> {
        self.bookmarks.clone()
    }

    pub fn byte_to_position(&self, byte_index: usize) -> anyhow::Result<Position> {
        let char_index = self.byte_to_char(byte_index)?;
        self.char_to_position(char_index)
    }

    /// Line is 0-indexed
    pub fn line_to_position_range(&self, line: usize) -> anyhow::Result<Range<Position>> {
        let start = self.line_to_char(line)?;
        let end = self.line_to_char(line + 1)?;
        Ok(self.char_to_position(start)?..self.char_to_position(end - 1)?)
    }

    pub fn byte_to_line(&self, byte: usize) -> anyhow::Result<usize> {
        Ok(self.rope.try_byte_to_line(byte)?)
    }

    pub fn get_line_by_line_index(&self, line_index: usize) -> Option<ropey::RopeSlice<'_>> {
        self.rope.get_line(line_index)
    }

    pub fn position_range_to_byte_range(
        &self,
        range: &Range<Position>,
    ) -> anyhow::Result<Range<usize>> {
        Ok(self.position_to_byte(range.start)?..self.position_to_byte(range.end)?)
    }

    pub fn byte_range_to_char_index_range(
        &self,
        range: &Range<usize>,
    ) -> anyhow::Result<CharIndexRange> {
        Ok((self.byte_to_char(range.start)?..self.byte_to_char(range.end)?).into())
    }
    pub fn position_range_to_char_index_range(
        &self,
        range: &Range<Position>,
    ) -> anyhow::Result<CharIndexRange> {
        Ok((self.position_to_char(range.start)?..self.position_to_char(range.end)?).into())
    }
    pub fn char_index_range_to_position_range(
        &self,
        range: CharIndexRange,
    ) -> anyhow::Result<Range<Position>> {
        Ok(self.char_to_position(range.start)?..self.char_to_position(range.end)?)
    }

    fn get_edit_transaction(&self, new: &str) -> anyhow::Result<EditTransaction> {
        let edits: Vec<Edit> = Hunk::get(&self.rope.to_string(), new)
            .into_iter()
            .map(|hunk| -> anyhow::Result<_> {
                let old_line_range = hunk.old_line_range();
                Ok(Edit {
                    range: self.position_range_to_char_index_range(
                        &(Position::new(old_line_range.start, 0)
                            ..Position::new(old_line_range.end, 0)),
                    )?,
                    new: Rope::from_str(&hunk.new_content()),
                })
            })
            .try_collect()?;
        Ok(EditTransaction::from_action_groups(
            edits
                .into_iter()
                .map(|edit| ActionGroup {
                    actions: [Action::Edit(edit)].to_vec(),
                })
                .collect_vec(),
        ))
    }
}

#[cfg(test)]
mod test_buffer {
    use itertools::Itertools;

    use crate::selection::SelectionSet;

    use super::Buffer;

    #[test]
    fn get_parent_lines_1() {
        let buffer = Buffer::new(
            shared::language::from_extension("yaml")
                .unwrap()
                .tree_sitter_language()
                .unwrap(),
            "
- spongebob
- who:
  - lives
  - in:
    - a
    - pineapple
  - under
",
        );
        let actual = buffer
            .get_parent_lines(6)
            .unwrap()
            .into_iter()
            .map(|line| line.content)
            .collect_vec()
            .join("\n");
        let expected = "
- who:
  - in:
"
        .trim()
        .to_string();
        pretty_assertions::assert_eq!(actual, expected)
    }

    #[test]
    fn get_parent_lines_2() {
        let buffer = Buffer::new(
            shared::language::from_extension("rs")
                .unwrap()
                .tree_sitter_language()
                .unwrap(),
            "
fn f(
  x: X
) -> Result<
  A,
  B
> { 
  hello
}",
        );
        let actual = buffer
            .get_parent_lines(5)
            .unwrap()
            .into_iter()
            .map(|line| line.content)
            .collect_vec()
            .join("\n");
        let expected = "
fn f(
) -> Result<"
            .trim()
            .to_string();
        pretty_assertions::assert_eq!(actual, expected)
    }

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
            selection::{CharIndex, SelectionSet},
        };
        use shared::canonicalized_path::CanonicalizedPath;
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
                buffer.undo().unwrap();

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

        #[test]
        fn should_not_update_buffer_if_formatter_returns_error() {
            let code = r#"
            let x = "1";
                "#;

            run_test(|_, mut buffer| {
                // Update the buffer to be valid Rust code
                // but unformatable
                buffer.update(code);

                // The code should be deemed as valid by Tree-sitter,
                // but not to the formatter
                assert!(!buffer.tree.root_node().has_error());

                buffer.save(SelectionSet::default()).unwrap();

                // Expect the buffer remain unchanged
                assert_eq!(buffer.rope.to_string(), code);
            })
        }
    }

    #[test]
    fn patch_edits() -> anyhow::Result<()> {
        let old = r#"
            let x = "1";
            let y = "2";
            let z = 3;
            let a = 4;
            let b = 4;
            // This line will be removed
                "#
        .trim();

        // Suppose the new content has all kinds of changes:
        // 1. Replacement (line 1)
        // 2. Insertion (line 3)
        // 3. Deletion (last line)
        let new = r#"
            let x = "this line is replaced
                     with multiline content"
            let y = "2";
            let z = 3;
            // This is a newly inserted line
            let a = 4;
            let b = 4;
                            "#
        .trim();

        let mut buffer = Buffer::new(tree_sitter_md::language(), old);

        let edit_transaction = buffer.get_edit_transaction(new)?;

        // Expect there are 3 edits
        assert_eq!(edit_transaction.edits().len(), 3);

        // Apply the edit transaction
        buffer.apply_edit_transaction(&edit_transaction, SelectionSet::default())?;

        // Expect the content to be the same as the 2nd files
        pretty_assertions::assert_eq!(buffer.content(), new);
        Ok(())
    }
}

#[derive(Clone)]
pub struct Patch {
    /// Why don't we store this is diffy::Patch? Because it requires a lifetime parameter
    pub patch: String,
    pub state: BufferState,
}

#[derive(Clone)]
pub struct BufferState {
    pub selection_set: SelectionSet,
    pub bookmarks: Vec<CharIndexRange>,
}

impl std::fmt::Display for Patch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("")
    }
}

impl std::fmt::Display for BufferState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: this should describe the action
        // For example, "kill", "exchange", "insert"
        f.write_str("")
    }
}

impl Applicable for Patch {
    type Target = String;

    type Output = BufferState;

    fn apply(&self, target: &mut Self::Target) -> anyhow::Result<Self::Output> {
        *target = diffy::apply(target, &diffy::Patch::from_str(&self.patch)?)?;

        Ok(self.state.clone())
    }
}
impl PartialEq for Patch {
    fn eq(&self, _other: &Self) -> bool {
        // Always return false, assuming that no two patches can be identical
        false
    }
}
