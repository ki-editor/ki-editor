use crate::history::History;
use crate::lsp::diagnostic::Diagnostic;
use crate::quickfix_list::QuickfixListItem;
use crate::selection_mode::naming_convention_agnostic::NamingConventionAgnostic;
use crate::{
    char_index_range::CharIndexRange,
    components::{editor::Movement, suggestive_editor::Decoration},
    context::{LocalSearchConfig, LocalSearchConfigMode},
    edit::{Action, ActionGroup, Edit, EditTransaction},
    position::Position,
    selection::{CharIndex, Selection, SelectionSet},
    selection_mode::{AstGrep, ByteRange},
    syntax_highlight::{HighlighedSpan, HighlighedSpans},
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
use tree_sitter_traversal2::{traverse, Order};

#[derive(Clone)]
pub(crate) struct Buffer {
    rope: Rope,
    tree: Option<Tree>,
    treesitter_language: Option<tree_sitter::Language>,
    undo_tree: UndoTree<Patch>,
    language: Option<Language>,
    path: Option<CanonicalizedPath>,
    highlighted_spans: HighlighedSpans,
    marks: Vec<CharIndexRange>,
    diagnostics: Vec<Diagnostic>,
    quickfix_list_items: Vec<QuickfixListItem>,
    decorations: Vec<Decoration>,
    selection_set_history: History<SelectionSet>,
    dirty: bool,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Line {
    origin_position: Position,
    /// 0-based
    pub(crate) line: usize,
    pub(crate) content: String,
}

impl Buffer {
    pub(crate) fn new(language: Option<tree_sitter::Language>, text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            treesitter_language: language.clone(),
            language: None,
            tree: {
                let mut parser = Parser::new();
                language.and_then(|language| {
                    parser
                        .set_language(&language)
                        .ok()
                        .and_then(|_| parser.parse(text, None))
                })
            },
            path: None,
            highlighted_spans: HighlighedSpans::default(),
            marks: Vec::new(),
            decorations: Vec::new(),
            undo_tree: UndoTree::new(),
            diagnostics: Vec::new(),
            quickfix_list_items: Vec::new(),
            selection_set_history: History::new(),
            dirty: false,
        }
    }

    pub(crate) fn clear_quickfix_list_items(&mut self) {
        self.quickfix_list_items.clear()
    }

    pub(crate) fn update_quickfix_list_items(
        &mut self,
        quickfix_list_items: Vec<QuickfixListItem>,
    ) {
        self.quickfix_list_items = quickfix_list_items
    }

    pub(crate) fn reload(&mut self) -> anyhow::Result<()> {
        if let Some(path) = self.path() {
            let updated_content = path.read()?;
            self.update_content(&updated_content, SelectionSet::default())?;
            self.dirty = false;
        }
        Ok(())
    }

    pub(crate) fn content(&self) -> String {
        self.rope.to_string()
    }

    pub(crate) fn decorations(&self) -> &Vec<Decoration> {
        &self.decorations
    }

    pub(crate) fn set_decorations(&mut self, decorations: &[Decoration]) {
        decorations.clone_into(&mut self.decorations)
    }

    pub(crate) fn save_marks(&mut self, new_ranges: Vec<CharIndexRange>) {
        let old_ranges = std::mem::take(&mut self.marks)
            .into_iter()
            .collect::<HashSet<_>>();
        let new_ranges = new_ranges.into_iter().collect::<HashSet<_>>();
        // We take the symmetric difference between the old ranges and the new ranges
        // so that user can unmark existing mark
        self.marks = new_ranges
            .symmetric_difference(&old_ranges)
            .cloned()
            .collect_vec();
    }

    pub(crate) fn path(&self) -> Option<CanonicalizedPath> {
        self.path.clone()
    }

    #[cfg(test)]
    pub(crate) fn set_path(&mut self, path: CanonicalizedPath) {
        self.path = Some(path);
    }

    pub(crate) fn set_diagnostics(&mut self, diagnostics: Vec<lsp_types::Diagnostic>) {
        self.diagnostics = diagnostics
            .into_iter()
            .filter_map(|diagnostic| Diagnostic::try_from(self, diagnostic).ok())
            .collect()
    }

    pub(crate) fn diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostics.clone()
    }

    pub(crate) fn words(&self) -> Vec<String> {
        let regex = regex::Regex::new(r"\b\w+").unwrap();
        let str = self.rope.to_string();
        regex
            .find_iter(&str)
            .map(|m| m.as_str().to_string())
            .unique()
            .collect()
    }

    pub(crate) fn get_parent_lines(&self, line_number: usize) -> anyhow::Result<Vec<Line>> {
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

    fn get_rope_and_tree(
        language: Option<tree_sitter::Language>,
        text: &str,
    ) -> (Rope, Option<Tree>) {
        let mut parser = Parser::new();
        let tree = language
            .map(|language| parser.set_language(&language))
            .and_then(|_| parser.parse(text, None));
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

    pub(crate) fn given_range_is_node(&self, range: &CharIndexRange) -> bool {
        let Some(start) = self.char_to_byte(range.start).ok() else {
            return false;
        };
        let Some(end) = self.char_to_byte(range.end).ok() else {
            return false;
        };
        let byte_range = start..end;
        self.tree
            .as_ref()
            .map(|tree| {
                tree.root_node()
                    .descendant_for_byte_range(byte_range.start, byte_range.end)
                    .map(|node| node.byte_range() == byte_range)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    pub(crate) fn update_highlighted_spans(&mut self, spans: HighlighedSpans) {
        self.highlighted_spans = spans;
    }

    pub(crate) fn update(&mut self, text: &str) {
        (self.rope, self.tree) = Self::get_rope_and_tree(self.treesitter_language.clone(), text);
        self.dirty = true;
    }

    pub(crate) fn get_line_by_char_index(&self, char_index: CharIndex) -> anyhow::Result<Rope> {
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

    pub(crate) fn get_line_range_by_char_index(
        &self,
        char_index: CharIndex,
    ) -> anyhow::Result<CharIndexRange> {
        let line = self.get_line_by_char_index(char_index)?.to_string();
        let line_start = self.line_to_char(self.char_to_line(char_index)?)?;
        let line_end = line_start + line.chars().count();
        Ok((line_start..line_end).into())
    }

    pub(crate) fn get_word_before_char_index(
        &self,
        char_index: CharIndex,
    ) -> anyhow::Result<String> {
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

    pub(crate) fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub(crate) fn char_to_line(&self, char_index: CharIndex) -> anyhow::Result<usize> {
        Ok(self.rope.try_char_to_line(char_index.0)?)
    }

    pub(crate) fn line_to_char(&self, line_index: usize) -> anyhow::Result<CharIndex> {
        Ok(CharIndex(self.rope.try_line_to_char(line_index)?))
    }

    pub(crate) fn char_to_byte(&self, char_index: CharIndex) -> anyhow::Result<usize> {
        Ok(self.rope.try_char_to_byte(char_index.0)?)
    }

    /// Note: this method is expensive, be sure not pass in an out-of-view `char_index`
    pub(crate) fn char_to_position(&self, char_index: CharIndex) -> anyhow::Result<Position> {
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

    pub(crate) fn position_to_char(&self, position: Position) -> anyhow::Result<CharIndex> {
        let line = position.line.clamp(0, self.len_lines());
        let column = position.column.clamp(
            0,
            self.get_line_by_line_index(line)
                .map(|slice| slice.len_chars())
                .unwrap_or_default(),
        );
        Ok(CharIndex(self.rope.try_line_to_char(line)? + column))
    }

    pub(crate) fn byte_to_char(&self, byte_index: usize) -> anyhow::Result<CharIndex> {
        Ok(CharIndex(self.rope.try_byte_to_char(byte_index)?))
    }

    pub(crate) fn rope(&self) -> &Rope {
        &self.rope
    }

    pub(crate) fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub(crate) fn slice(&self, range: &CharIndexRange) -> anyhow::Result<Rope> {
        let slice = self.rope.get_slice(range.start.0..range.end.0);
        match slice {
            Some(slice) => Ok(slice.into()),
            None => Err(anyhow::anyhow!(
                "Unable to obtain slice for range: {:#?}",
                range
            )),
        }
    }

    pub(crate) fn get_nearest_node_after_char(&self, char_index: CharIndex) -> Option<Node> {
        let byte = self.char_to_byte(char_index).ok()?;
        // Preorder is the main key here,
        // because preorder traversal walks the parent first
        self.tree.as_ref().and_then(|tree| {
            traverse(tree.root_node().walk(), Order::Pre).find(|&node| node.start_byte() >= byte)
        })
    }

    pub(crate) fn get_current_node<'a>(
        &'a self,
        selection: &Selection,
        get_largest_end: bool,
    ) -> anyhow::Result<Option<Node<'a>>> {
        let Some(tree) = self.tree.as_ref() else {
            return Ok(None);
        };
        let range = selection.range();
        let start = self.char_to_byte(range.start)?;
        let (start, end) = if get_largest_end {
            (start, start + 1)
        } else {
            (start, self.char_to_byte(range.end)?)
        };
        let node = tree
            .root_node()
            .descendant_for_byte_range(start, end)
            .unwrap_or_else(|| tree.root_node());

        // Get the most ancestral node of this range
        //
        // This is because sometimes the parent of a node can have the same range as the node
        // itself.
        //
        // If we don't get the most ancestral node, then movements like "go to next sibling" will
        // not work as expected.
        let mut result = node;
        let root_node_id = tree.root_node().id();
        while let Some(parent) = result.parent() {
            if parent.start_byte() == node.start_byte()
                && root_node_id != parent.id()
                && (get_largest_end || node.end_byte() == parent.end_byte())
            {
                result = parent;
            } else {
                return Ok(Some(result));
            }
        }

        Ok(Some(node))
    }

    #[cfg(test)]
    pub(crate) fn get_next_token(&self, char_index: CharIndex, is_named: bool) -> Option<Node> {
        let byte = self.char_to_byte(char_index).ok()?;
        self.traverse(Order::Post).and_then(|mut iter| {
            iter.find(|&node| {
                node.child_count() == 0 && (!is_named || node.is_named()) && node.end_byte() > byte
            })
        })
    }

    #[cfg(test)]
    pub(crate) fn traverse(&self, order: Order) -> Option<impl Iterator<Item = Node>> {
        self.tree.as_ref().map(|tree| traverse(tree.walk(), order))
    }

    /// Returns the new selection set
    pub(crate) fn apply_edit_transaction(
        &mut self,
        edit_transaction: &EditTransaction,
        current_selection_set: SelectionSet,
        reparse_tree: bool,
    ) -> Result<SelectionSet, anyhow::Error> {
        let before = self.rope.to_string();
        let new_selection_set = edit_transaction
            .non_empty_selections()
            .map(|selections| current_selection_set.clone().set_selections(selections))
            .unwrap_or_else(|| current_selection_set.clone());
        let current_buffer_state = BufferState {
            selection_set: current_selection_set,
            marks: self.marks.clone(),
        };

        edit_transaction
            .edits()
            .into_iter()
            .try_fold((), |_, edit| self.apply_edit(edit))?;

        let new_buffer_state = BufferState {
            selection_set: new_selection_set.clone(),
            marks: self.marks.clone(),
        };

        self.add_undo_patch(current_buffer_state, new_buffer_state.clone(), &before);
        if reparse_tree {
            self.reparse_tree()?;
        }

        Ok(new_selection_set)
    }

    fn apply_edit(&mut self, edit: &Edit) -> Result<(), anyhow::Error> {
        // We have to get the char index range of positional spans before updating the content
        let quickfix_list_items_with_char_index_range =
            std::mem::take(&mut self.quickfix_list_items)
                .into_iter()
                .filter_map(|item| {
                    Some((
                        self.position_range_to_char_index_range(&item.location().range)
                            .ok()?,
                        item,
                    ))
                })
                .collect_vec();

        // Update the content
        self.rope.try_remove(edit.range.start.0..edit.end().0)?;
        self.rope
            .try_insert(edit.range.start.0, edit.new.to_string().as_str())?;
        self.dirty = true;

        // Update all the positional spans (by using the char index ranges computed before the content is updated
        self.quickfix_list_items = quickfix_list_items_with_char_index_range
            .into_iter()
            .filter_map(|(char_index_range, item)| {
                let position_range = self
                    .char_index_range_to_position_range(char_index_range.apply_edit(edit)?)
                    .ok()?;
                Some(item.set_location_range(position_range))
            })
            .collect_vec();

        // Update all the non-positional spans
        self.marks = std::mem::take(&mut self.marks)
            .into_iter()
            .filter_map(|mark| mark.apply_edit(edit))
            .collect();
        self.diagnostics = std::mem::take(&mut self.diagnostics)
            .into_iter()
            .filter_map(|diagnostic| {
                Some(Diagnostic {
                    range: diagnostic.range.apply_edit(edit)?,
                    ..diagnostic
                })
            })
            .collect_vec();
        let max_char_index = CharIndex(self.len_chars());
        self.selection_set_history = std::mem::take(&mut self.selection_set_history)
            .apply(|selection_set| selection_set.apply_edit(edit, max_char_index));
        if let Ok(byte_range) = self.char_index_range_to_byte_range(edit.range()) {
            self.highlighted_spans = std::mem::take(&mut self.highlighted_spans).apply_edit(
                &byte_range,
                edit.new.len_bytes() as isize - byte_range.len() as isize,
            )
        }
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

    pub(crate) fn display_history(&self) -> String {
        self.undo_tree.display()
    }

    pub(crate) fn undo_tree_apply_movement(
        &mut self,
        movement: Movement,
    ) -> anyhow::Result<Option<SelectionSet>> {
        let mut content = self.rope.to_string();
        let state = self.undo_tree.apply_movement(&mut content, movement)?;
        self.update(&content);

        if let Some(BufferState {
            selection_set,
            marks,
        }) = state
        {
            self.marks = marks;

            Ok(Some(selection_set))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn has_syntax_error_at(&self, range: CharIndexRange) -> bool {
        let rope = &self.rope;
        if let Some(node) = self.tree.as_ref().and_then(|tree| {
            tree.root_node().descendant_for_byte_range(
                rope.try_char_to_byte(range.start.0).unwrap_or(0),
                rope.try_char_to_byte(range.end.0).unwrap_or(0),
            )
        }) {
            node.has_error()
        } else {
            false
        }
    }

    pub(crate) fn from_path(
        path: &CanonicalizedPath,
        enable_tree_sitter: bool,
    ) -> anyhow::Result<Buffer> {
        let content = path.read()?;
        let language = if enable_tree_sitter {
            language::from_path(path)
        } else {
            None
        };

        let mut buffer = Buffer::new(
            language
                .as_ref()
                .and_then(|language| language.tree_sitter_language()),
            &content,
        );

        buffer.path = Some(path.clone());
        buffer.language = language;

        Ok(buffer)
    }

    pub(crate) fn reparse_tree(&mut self) -> anyhow::Result<()> {
        let mut parser = tree_sitter::Parser::new();
        if let Some(tree) = self.tree.as_ref() {
            parser.set_language(&tree.language())?;
            self.tree = parser.parse(self.rope.to_string(), None);
        }
        Ok(())
    }

    pub(crate) fn get_formatted_content(&self) -> Option<String> {
        if let Some(content) = self.language.as_ref().and_then(|language| {
            language.formatter().map(|formatter| {
                log::info!("[FORMAT]: {}", formatter.command_string());
                formatter.format(&self.rope.to_string())
            })
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
        None
    }

    pub(crate) fn save_without_formatting(
        &mut self,
        force: bool,
    ) -> anyhow::Result<Option<CanonicalizedPath>> {
        if !force && !self.dirty {
            return Ok(None);
        }

        if let Some(path) = &self.path {
            path.write(&self.content())?;
            self.dirty = false;
            Ok(Some(path.clone()))
        } else {
            log::info!("Buffer has no path");
            Ok(None)
        }
    }

    pub(crate) fn save(
        &mut self,
        current_selection_set: SelectionSet,
        force: bool,
    ) -> anyhow::Result<Option<CanonicalizedPath>> {
        if force || self.dirty {
            if let Some(formatted_content) = self.get_formatted_content() {
                self.update_content(&formatted_content, current_selection_set)?;
            }
        }

        self.save_without_formatting(force)
    }

    fn update_content(
        &mut self,
        new_content: &str,
        current_selection_set: SelectionSet,
    ) -> anyhow::Result<SelectionSet> {
        let edit_transaction = self.get_edit_transaction(new_content)?;
        self.apply_edit_transaction(&edit_transaction, current_selection_set, true)
    }

    /// The resulting spans must be sorted by range
    pub(crate) fn highlighted_spans(&self) -> Vec<HighlighedSpan> {
        let spans = self.highlighted_spans.0.clone();
        debug_assert!(
            spans
                .iter()
                .enumerate()
                .sorted_by_key(|(_, span)| (span.byte_range.start, span.byte_range.end))
                .map(|(index, _)| index)
                .collect_vec()
                == (0..spans.len()).collect_vec(),
        );
        spans
    }

    pub(crate) fn language(&self) -> Option<Language> {
        self.language.clone()
    }

    #[cfg(test)]
    pub(crate) fn set_language(&mut self, language: Language) -> anyhow::Result<()> {
        self.language = Some(language);
        self.reparse_tree()
    }

    pub(crate) fn treesitter_language(&self) -> Option<tree_sitter::Language> {
        self.treesitter_language.clone()
    }

    pub(crate) fn get_char_at_position(&self, position: Position) -> Option<char> {
        let char_index = position.to_char_index(self).ok()?.0;
        self.rope.get_char(char_index)
    }

    pub(crate) fn tree(&self) -> Option<&Tree> {
        self.tree.as_ref()
    }

    pub(crate) fn line_to_byte(&self, line_index: usize) -> anyhow::Result<usize> {
        Ok(self.rope.try_line_to_byte(line_index)?)
    }

    pub(crate) fn position_to_byte(&self, start: Position) -> anyhow::Result<usize> {
        let start = self.position_to_char(start)?;
        self.char_to_byte(start)
    }

    pub(crate) fn line_to_byte_range(&self, line: usize) -> anyhow::Result<ByteRange> {
        let start = self.line_to_byte(line)?;
        let end = self.line_to_byte(line + 1)?.saturating_sub(1);
        Ok(ByteRange::new(start..end))
    }

    pub(crate) fn marks(&self) -> Vec<CharIndexRange> {
        self.marks.clone()
    }

    /// Has the buffer changed since its last save?
    #[cfg(test)]
    pub(crate) fn dirty(&self) -> bool {
        self.dirty
    }

    pub(crate) fn byte_to_position(&self, byte_index: usize) -> anyhow::Result<Position> {
        let char_index = self.byte_to_char(byte_index)?;
        self.char_to_position(char_index)
    }

    pub(crate) fn byte_to_line(&self, byte: usize) -> anyhow::Result<usize> {
        Ok(self.rope.try_byte_to_line(byte)?)
    }

    pub(crate) fn get_line_by_line_index(&self, line_index: usize) -> Option<ropey::RopeSlice<'_>> {
        self.rope.get_line(line_index)
    }

    pub(crate) fn position_range_to_byte_range(
        &self,
        range: &Range<Position>,
    ) -> anyhow::Result<Range<usize>> {
        Ok(self.position_to_byte(range.start)?..self.position_to_byte(range.end)?)
    }

    pub(crate) fn byte_range_to_char_index_range(
        &self,
        range: &Range<usize>,
    ) -> anyhow::Result<CharIndexRange> {
        Ok((self.byte_to_char(range.start)?..self.byte_to_char(range.end)?).into())
    }

    pub(crate) fn position_range_to_char_index_range(
        &self,
        range: &Range<Position>,
    ) -> anyhow::Result<CharIndexRange> {
        Ok((self.position_to_char(range.start)?..self.position_to_char(range.end)?).into())
    }

    pub(crate) fn char_index_range_to_position_range(
        &self,
        range: CharIndexRange,
    ) -> anyhow::Result<Range<Position>> {
        Ok(self.char_to_position(range.start)?..self.char_to_position(range.end)?)
    }

    /// Get an `EditTransaction` by getting the line diffs between the content of this buffer and the given `new` string
    fn get_edit_transaction(&self, new: &str) -> anyhow::Result<EditTransaction> {
        let old = self.rope.to_string();
        let new = new.to_string();
        let edits = {
            let diff_from_lines = similar::TextDiff::from_lines(&old, &new);
            let changes = diff_from_lines.iter_all_changes();
            let mut old_line_index = 0;
            let mut edits = vec![];
            let mut replacement = vec![];
            let mut current_range_start = None;
            let mut current_range_end = 0;

            for change in changes {
                match change.tag() {
                    similar::ChangeTag::Delete => {
                        if current_range_start.is_none() {
                            current_range_start = Some(old_line_index);
                        }
                        current_range_end = old_line_index + 1;
                        old_line_index += 1;
                    }
                    similar::ChangeTag::Equal => {
                        if let Some(start) = current_range_start {
                            let replacement = std::mem::take(&mut replacement);

                            edits.push(Edit {
                                range: self.position_range_to_char_index_range(
                                    &(Position::new(start, 0)..Position::new(current_range_end, 0)),
                                )?,
                                new: Rope::from_str(&replacement.join("")),
                            });
                            current_range_start = None;
                        }
                        old_line_index += 1;
                    }
                    similar::ChangeTag::Insert => {
                        match current_range_start {
                            Some(_) => {}
                            None => {
                                current_range_start = Some(old_line_index);
                                current_range_end = old_line_index;
                            }
                        };

                        let content = change.to_string();
                        let content = if change.missing_newline() && content.ends_with('\n') {
                            content.trim_end_matches('\n').to_owned()
                        } else {
                            content
                        };
                        replacement.push(content);
                    }
                }
            }

            if let Some(start) = current_range_start {
                let replacement = std::mem::take(&mut replacement);

                edits.push(Edit {
                    range: self.position_range_to_char_index_range(
                        &(Position::new(start, 0)..Position::new(current_range_end, 0)),
                    )?,
                    new: Rope::from_str(&replacement.join("")),
                });
            };
            edits
        };

        Ok(EditTransaction::from_action_groups(
            edits
                .into_iter()
                .map(|edit| ActionGroup {
                    actions: [Action::Edit(edit)].to_vec(),
                })
                .collect_vec(),
        ))
    }

    /// The boolean returned indicates whether the replacement causes any modification
    pub(crate) fn replace(
        &mut self,
        config: LocalSearchConfig,
        current_selection_set: SelectionSet,
    ) -> anyhow::Result<(bool, SelectionSet)> {
        let before = self.rope.to_string();
        let edit_transaction = match config.mode {
            LocalSearchConfigMode::NamingConventionAgnostic => {
                let replaced = NamingConventionAgnostic::new(config.search())
                    .replace_all(&before, config.replacement());
                self.get_edit_transaction(&replaced)?
            }
            LocalSearchConfigMode::Regex(regex_config) => {
                let regex = regex_config.to_regex(&config.search())?;
                let replaced = regex
                    // We use `try_replacen` instead of `replace_all`
                    // because the latter panics on very large file,
                    // subsequently crashing Ki.
                    .try_replacen(&before, 0, config.replacement())?
                    .to_string();
                self.get_edit_transaction(&replaced)?
            }
            LocalSearchConfigMode::AstGrep => {
                let edits = if let Some(language) = self.treesitter_language() {
                    AstGrep::replace(language, &before, &config.search(), &config.replacement())?
                } else {
                    Default::default()
                };
                EditTransaction::from_action_groups(
                    edits
                        .into_iter()
                        .map(|edit| -> anyhow::Result<ActionGroup> {
                            let start = self.byte_to_char(edit.position)?;
                            let end = start + edit.deleted_length;

                            Ok(ActionGroup::new(
                                [Action::Edit(Edit {
                                    range: (start..end).into(),
                                    new: String::from_utf8(edit.inserted_text)?.into(),
                                })]
                                .to_vec(),
                            ))
                        })
                        .try_collect()?,
                )
            }
        };
        let selection_set =
            self.apply_edit_transaction(&edit_transaction, current_selection_set, true)?;
        let after = self.content();
        let modified = before != after;
        Ok((modified, selection_set))
    }

    pub(crate) fn char_index_range_to_byte_range(
        &self,
        range: CharIndexRange,
    ) -> anyhow::Result<Range<usize>> {
        Ok(self.char_to_byte(range.start)?..self.char_to_byte(range.end)?)
    }

    pub(crate) fn quickfix_list_items(&self) -> Vec<QuickfixListItem> {
        self.quickfix_list_items.clone()
    }

    pub(crate) fn line_range_to_char_index_range(
        &self,
        range: Range<usize>,
    ) -> anyhow::Result<CharIndexRange> {
        Ok((self.line_to_char(range.start)?..self.line_to_char(range.end)?).into())
    }

    pub(crate) fn line_range_to_full_char_index_range(
        &self,
        range: Range<usize>,
    ) -> anyhow::Result<CharIndexRange> {
        let end_line_char_start = self.line_to_char(range.end)?;
        let line = self.get_line_by_line_index(range.end).ok_or_else(|| {
            anyhow::anyhow!(
                "Buffer::line_range_to_char_index_range: Unable to get line at index {}",
                range.end
            )
        })?;
        Ok((self.line_to_char(range.start)?..end_line_char_start + line.len_chars()).into())
    }

    pub(crate) fn char_index_range_to_line_range(
        &self,
        range: CharIndexRange,
    ) -> anyhow::Result<Range<usize>> {
        Ok(self.char_to_line(range.start)?..self.char_to_line(range.end)?)
    }

    pub(crate) fn push_selection_set_history(&mut self, selection_set: SelectionSet) {
        self.selection_set_history.push(selection_set.clone());
    }

    pub(crate) fn previous_selection_set(&mut self) -> Option<SelectionSet> {
        self.selection_set_history.undo()
    }

    pub(crate) fn next_selection_set(&mut self) -> Option<SelectionSet> {
        self.selection_set_history.redo()
    }

    pub(crate) fn line_range_to_byte_range(
        &self,
        visible_line_range: &Range<usize>,
    ) -> anyhow::Result<Range<usize>> {
        let start = self
            .line_to_byte_range(visible_line_range.start)?
            .range()
            .start;
        let end = self
            .line_to_byte_range(visible_line_range.end.saturating_sub(1))?
            .range()
            .end;
        debug_assert!(start <= end);
        Ok(start..end)
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
                .tree_sitter_language(),
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
                .tree_sitter_language(),
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

    mod replace {

        use crate::{
            context::{
                LocalSearchConfig,
                LocalSearchConfigMode::{AstGrep, Regex},
            },
            list::grep::RegexConfig,
        };

        use super::*;
        fn test(input: &str, config: LocalSearchConfig, expected: &str) -> anyhow::Result<()> {
            let mut buffer = Buffer::new(
                shared::language::from_extension("rs")
                    .unwrap()
                    .tree_sitter_language(),
                input,
            );
            buffer.replace(config, SelectionSet::default())?;
            assert_eq!(buffer.content(), expected);
            Ok(())
        }

        #[test]
        fn literal_1() -> anyhow::Result<()> {
            test(
                "hel. help hel.o",
                LocalSearchConfig::new(Regex(RegexConfig {
                    escaped: true,
                    case_sensitive: false,
                    match_whole_word: false,
                }))
                .set_search("hel.".to_string())
                .set_replacment("wow".to_string())
                .to_owned(),
                "wow help wowo",
            )
        }

        #[test]
        fn regex_capture_group() -> anyhow::Result<()> {
            test(
                "123x456",
                LocalSearchConfig::new(Regex(RegexConfig {
                    escaped: false,
                    case_sensitive: false,
                    match_whole_word: false,
                }))
                .set_search(r"(\d+)".to_string())
                .set_replacment(r"($1)".to_string())
                .to_owned(),
                "(123)x(456)",
            )
        }

        #[test]
        fn ast_group_1() -> anyhow::Result<()> {
            test(
                "fn main() { replace(x + 1, f(2)); replace(a,b) }",
                LocalSearchConfig::new(AstGrep)
                    .set_search(r"replace($X,$Y)".to_string())
                    .set_replacment(r"replace($Y,$X)".to_string())
                    .to_owned(),
                "fn main() { replace(f(2),x + 1); replace(b,a) }",
            )
        }
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

            let buffer = Buffer::from_path(&path, true).unwrap();

            f(path, buffer)
        }

        #[test]
        fn should_format_code() {
            run_test(|path, mut buffer| {
                // Update the buffer with unformatted code
                buffer.update(" fn main\n() {}");

                // Save the buffer
                buffer.save(SelectionSet::default(), false).unwrap();

                // Expect the output is formatted
                let saved_content = path.read().unwrap();
                let buffer_content = buffer.rope.to_string();

                assert_eq!(saved_content, "fn main() {}\n");
                assert_eq!(buffer_content, "fn main() {}\n");

                // Expect the syntax node is also updated
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

                buffer.save(SelectionSet::default(), false).unwrap();

                // Expect the buffer is formatted
                assert_ne!(buffer.rope.to_string(), original);

                // Undo the buffer
                buffer
                    .undo_tree_apply_movement(crate::components::editor::Movement::Left)
                    .unwrap();

                let content = buffer.rope.to_string();

                // Expect the content is reverted to the original
                assert_eq!(content, " fn main\n() {}");
            })
        }

        #[test]
        fn should_not_run_when_syntax_node_is_malformed() {
            run_test(|_, mut buffer| {
                // Update the buffer to be invalid Rust code
                buffer.update("fn main() {");

                // Save the buffer
                buffer.save(SelectionSet::default(), false).unwrap();

                // Expect the buffer remain unchanged,
                // because the syntax node is invalid
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
                assert!(!buffer.tree.as_ref().unwrap().root_node().has_error());

                buffer.save(SelectionSet::default(), false).unwrap();

                // Expect the buffer remain unchanged
                assert_eq!(buffer.rope.to_string(), code);
            })
        }
    }

    mod patch_edit {
        use crate::edit::EditTransaction;

        use super::*;
        fn run_test(old: &str, new: &str) -> anyhow::Result<EditTransaction> {
            let mut buffer = Buffer::new(Some(tree_sitter_md::LANGUAGE.into()), old);

            let edit_transaction = buffer.get_edit_transaction(new)?;

            // Apply the edit transaction
            buffer.apply_edit_transaction(&edit_transaction, SelectionSet::default(), true)?;

            // Expect the content to be the same as the 2nd files
            pretty_assertions::assert_eq!(buffer.content(), new);

            Ok(edit_transaction)
        }

        #[test]
        fn empty_line_removal() -> anyhow::Result<()> {
            let old = r#"
            let y = "2";
            let z = 3;

            let a = 4;
            "#
            .trim();

            let new = r#"
            let y = "2";
            let z = 3;
            let a = 4;
            "#
            .trim();

            run_test(old, new)?;
            Ok(())
        }

        #[test]
        fn all_kinds_of_edits() -> anyhow::Result<()> {
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

            let edit_transaction = run_test(old, new)?;

            // Expect there are 3 edits
            assert_eq!(edit_transaction.edits().len(), 3);

            Ok(())
        }

        #[test]
        fn empty_line_with_whitespaces() -> anyhow::Result<()> {
            // The line after `let x = x;` has multiple whitespaces in it
            let old = r#"
fn main() {
    let x = x;
    
let z = z;

    let y = y;
}
"#
            .trim();

            let new = r#"
fn main() {
    let x = x;

    let z = z;

    let y = y;
}
"#
            .trim();

            run_test(old, new)?;
            Ok(())
        }

        #[test]
        fn newline_insertion() -> anyhow::Result<()> {
            run_test("", "\n")?;
            Ok(())
        }

        #[test]
        fn newline_removal() -> anyhow::Result<()> {
            run_test("\n", "")?;
            Ok(())
        }
    }
}

#[derive(Clone)]
pub(crate) struct Patch {
    /// Why don't we store this is diffy::Patch? Because it requires a lifetime parameter
    pub(crate) patch: String,
    pub(crate) state: BufferState,
}

#[derive(Clone)]
pub(crate) struct BufferState {
    pub(crate) selection_set: SelectionSet,
    pub(crate) marks: Vec<CharIndexRange>,
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
