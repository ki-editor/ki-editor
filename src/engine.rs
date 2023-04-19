use std::ops::{Add, Range, Sub};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use itertools::Itertools;
use ropey::Rope;
use tree_sitter::{InputEdit, Node, Point, Tree};
use tree_sitter_traversal::{traverse, Order};

#[derive(PartialEq, Clone, Debug, Copy)]
pub struct Selection {
    pub start: CharIndex,
    pub end: CharIndex,
    pub node_id: Option<usize>,
}
impl Selection {
    fn to_char_index(&self, cursor_direction: &CursorDirection) -> CharIndex {
        match cursor_direction {
            CursorDirection::Start => self.start.clone(),
            CursorDirection::End => self.end.clone(),
        }
    }
}

impl Add<usize> for Selection {
    type Output = Selection;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            start: self.start + rhs,
            end: self.end + rhs,
            node_id: self.node_id,
        }
    }
}

impl Sub<usize> for Selection {
    type Output = Selection;

    fn sub(self, rhs: usize) -> Self::Output {
        Self {
            start: self.start - rhs,
            end: self.end - rhs,
            node_id: self.node_id,
        }
    }
}

impl Add<usize> for CharIndex {
    type Output = CharIndex;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0.saturating_add(rhs))
    }
}

impl Sub<usize> for CharIndex {
    type Output = CharIndex;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0.saturating_sub(rhs))
    }
}

#[derive(PartialEq, Clone, Debug, Copy, PartialOrd, Eq, Ord)]
pub struct CharIndex(pub usize);

impl CharIndex {
    pub fn to_point(&self, rope: &Rope) -> Point {
        let line = rope.char_to_line(self.0);
        Point {
            row: line,
            column: self.0.saturating_sub(rope.line_to_char(line)),
        }
    }
}

pub enum Mode {
    Normal,
    Insert,
    Extend { extended_selection: Selection },
    Jump { jumps: Vec<Jump> },
}

#[derive(Clone, Copy)]
pub struct Jump {
    pub character: char,
    pub selection: Selection,
}

pub struct State {
    pub selection: Selection,
    pub source_code: Rope,
    pub mode: Mode,
    pub selection_mode: SelectionMode,
    pub cursor_direction: CursorDirection,
    pub tree: Tree,
    pub quit: bool,
    yanked_text: Option<Rope>,
    selection_history: Vec<Selection>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SelectionMode {
    Line,
    Word,
    Character,
    Custom,

    NamedNode,
    Token,
    AncestorNode,
    SiblingNode,
}
impl SelectionMode {
    fn similar_to(&self, other: &SelectionMode) -> bool {
        self == other || self.is_node() && other.is_node()
    }

    fn is_node(&self) -> bool {
        use SelectionMode::*;
        match self {
            NamedNode | AncestorNode | SiblingNode => true,
            _ => false,
        }
    }
}

pub enum CursorDirection {
    Start,
    End,
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Forward,
    Backward,
    Current,
}

impl State {
    pub fn new(source_code: Rope, tree: Tree) -> Self {
        let mut state = Self {
            selection: to_selection(tree.root_node(), &source_code),
            source_code,
            mode: Mode::Normal,
            selection_mode: SelectionMode::Line,
            cursor_direction: CursorDirection::Start,
            tree,
            yanked_text: None,
            quit: false,
            selection_history: Vec::with_capacity(128),
        };
        state.select(SelectionMode::Character, Direction::Current);
        state
    }

    fn select_ancestor(&mut self) {
        self.select(SelectionMode::AncestorNode, Direction::Current);
    }

    fn select_kids(&mut self) {
        if let Some(node) = self.get_nearest_node_under_cursor() {
            let second_child = node.child(1);
            let second_last_child = node.child(node.child_count() - 2).or(second_child);
            match (second_child, second_last_child) {
                (Some(second_child), Some(second_last_child)) => {
                    self.update_selection(
                        Direction::Forward,
                        Selection {
                            start: CharIndex(second_child.start_byte() as usize),
                            end: CharIndex(second_last_child.end_byte() as usize),
                            node_id: None,
                        },
                    );
                    self.selection_mode = SelectionMode::Custom;
                }
                _ => {}
            }
        }
    }

    fn select_sibling(&mut self, direction: Direction) {
        self.select(SelectionMode::SiblingNode, direction);
    }

    fn select_line(&mut self, direction: Direction) {
        self.select(SelectionMode::Line, direction);
    }

    fn select_named_node(&mut self, direction: Direction) {
        self.select(SelectionMode::NamedNode, direction);
    }

    fn select_word(&mut self) {
        todo!()
    }

    fn select_charater(&mut self, direction: Direction) {
        self.select(SelectionMode::Character, direction);
    }

    fn select_backward(&mut self) {
        log::info!("select_backward");

        while let Some(selection) = self.selection_history.pop() {
            if selection != self.selection {
                self.selection = selection;
                break;
            }
        }
    }

    fn select_none(&mut self, direction: Direction) {
        self.select(SelectionMode::Custom, direction);
    }

    fn select_token(&mut self, direction: Direction) {
        self.select(SelectionMode::Token, direction);
    }

    fn update_selection(&mut self, direction: Direction, selection: Selection) {
        self.selection = selection;
        self.selection_history.push(selection);
        if let Mode::Extend { extended_selection } = self.mode {
            let f = match direction {
                Direction::Forward => usize::max,
                Direction::Backward => usize::min,
                Direction::Current => usize::max,
            };
            self.mode = Mode::Extend {
                extended_selection: Selection {
                    start: CharIndex(extended_selection.start.0),
                    end: CharIndex(f(self.get_cursor_char_index().0, extended_selection.end.0)),
                    node_id: None,
                },
            }
        }
    }

    fn select(&mut self, selection_mode: SelectionMode, direction: Direction) {
        log::info!("self.selection_mode: {:?}", self.selection_mode);
        let direction = if self.selection_mode.similar_to(&selection_mode) {
            direction
        } else {
            Direction::Current
        };
        log::info!("select: {:?} {:?}", selection_mode, direction);
        self.update_selection(direction, self.get_selection(&selection_mode, direction));
        self.selection_mode = selection_mode;
    }

    fn jump_from_selection(&mut self, direction: Direction, selection: Selection) {
        let mut current_selection = selection;
        let mut jumps = Vec::new();

        for char in ('a'..='z')
            .interleave('A'..='Z')
            .interleave('0'..='9')
            .chain(",.".chars())
            // 'j' and 'J' are reserved for subsequent jump.
            .filter(|c| c != &'j' && c != &'J')
        {
            let next_selection = Self::get_selection_(
                &self.source_code,
                &self.tree,
                &current_selection,
                &self.selection_mode,
                &direction,
                &self.cursor_direction,
            );

            if next_selection != current_selection {
                jumps.push(Jump {
                    character: char,
                    selection: next_selection,
                });
                current_selection = next_selection;
            } else {
                break;
            }
        }
        self.mode = Mode::Jump { jumps };
    }

    fn jump(&mut self, direction: Direction) {
        self.jump_from_selection(direction, self.selection);
    }

    fn get_input_edit(
        &self,
        start_char_index: CharIndex,
        old_end_char_index: CharIndex,
        new_end_char_index: CharIndex,
    ) -> InputEdit {
        InputEdit {
            start_byte: self.source_code.char_to_byte(start_char_index.0),
            old_end_byte: self.source_code.char_to_byte(old_end_char_index.0),
            new_end_byte: self.source_code.char_to_byte(new_end_char_index.0),
            start_position: start_char_index.to_point(&self.source_code),
            old_end_position: old_end_char_index.to_point(&self.source_code),
            new_end_position: new_end_char_index.to_point(&self.source_code),
        }
    }

    pub fn get_current_selection(&self) -> Selection {
        match &self.mode {
            Mode::Normal | Mode::Jump { .. } => self.selection,
            Mode::Insert => todo!(),
            Mode::Extend { extended_selection } => *extended_selection,
        }
    }

    fn delete_current_selection(&mut self) {
        let selection = self.get_current_selection();
        self.replace_with(selection.start.0..selection.end.0, Rope::new());
    }

    fn yank(&mut self) {
        let selection = self.get_current_selection();
        self.yanked_text = Some(
            self.source_code
                .slice(selection.start.0..selection.end.0)
                .into(),
        );
    }

    fn paste(&mut self) {
        if let Some(yanked_text) = &self.yanked_text {
            let cursor_position = self.get_cursor_char_index();

            self.tree.edit(&self.get_input_edit(
                cursor_position,
                cursor_position,
                cursor_position + yanked_text.len_chars(),
            ));

            let mut parser = tree_sitter::Parser::new();
            parser.set_language(self.tree.language()).unwrap();
            self.source_code
                .insert(cursor_position.0, yanked_text.to_string().as_str());
            self.tree = parser
                .parse(&self.source_code.to_string(), Some(&self.tree))
                .unwrap();

            if let CursorDirection::Start = self.cursor_direction {
                // TODO: what if we are in extend mode?
                self.selection = self.selection + yanked_text.len_chars()
            }
        }
    }

    fn replace(&mut self) {
        let yanked_text = self.yanked_text.take().unwrap_or_else(|| Rope::new());
        let selection = self.get_current_selection();
        self.replace_with(selection.start.0..selection.end.0, yanked_text);
    }

    fn replace_with(&mut self, range: Range<usize>, replacement: Rope) {
        self.tree.edit(&self.get_input_edit(
            CharIndex(range.start),
            CharIndex(range.end),
            CharIndex(range.start) + replacement.len_chars(),
        ));
        if !range.is_empty() {
            self.yanked_text = Some(self.source_code.slice(range.clone()).into());
        }
        self.source_code.remove(range.clone());
        self.source_code
            .insert(range.start, replacement.to_string().as_str());
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(self.tree.language()).unwrap();
        self.tree = parser
            .parse(&self.source_code.to_string(), Some(&self.tree))
            .unwrap();

        self.selection = Selection {
            start: CharIndex(range.start),
            end: CharIndex(range.start + replacement.len_chars()),
            node_id: None,
        };
        self.mode = Mode::Normal;
    }

    fn change_cursor_direction(&mut self) {
        self.cursor_direction = match self.cursor_direction {
            CursorDirection::Start => CursorDirection::End,
            CursorDirection::End => CursorDirection::Start,
        };
    }

    fn get_nearest_node_under_cursor(&self) -> Option<Node> {
        let cursor_pos = self
            .source_code
            .char_to_byte(self.get_cursor_char_index().0);

        get_nearest_node_after_byte(&self.tree, cursor_pos)
    }

    fn get_selection_(
        source_code: &Rope,
        tree: &Tree,
        current_selection: &Selection,
        mode: &SelectionMode,
        direction: &Direction,
        cursor_direction: &CursorDirection,
    ) -> Selection {
        let cursor_char_index = current_selection.to_char_index(cursor_direction);
        let cursor_byte = source_code.char_to_byte(cursor_char_index.0);
        let current_selection_start_byte = source_code.char_to_byte(current_selection.start.0);
        let current_selection_end_byte = source_code.char_to_byte(current_selection.end.0);
        match mode {
            SelectionMode::NamedNode => match direction {
                Direction::Forward | Direction::Current => {
                    traverse(tree.root_node().walk(), Order::Pre)
                        .find(|node| node.start_byte() > cursor_byte && node.is_named())
                }
                Direction::Backward => ReverseTreeCursor::new(tree.root_node())
                    .tuple_windows()
                    .find(|(current, next)| {
                        next.start_byte() < current.start_byte()
                            && current.start_byte() < cursor_byte
                            && current.is_named()
                    })
                    .map(|(current, _)| current),
            }
            .map(|node| to_selection(node, &source_code))
            .unwrap_or_else(|| current_selection.clone()),
            SelectionMode::Line => {
                let start = source_code.char_to_line(cursor_char_index.0);

                let start = CharIndex(
                    source_code.line_to_char(
                        match direction {
                            Direction::Forward => start.saturating_add(1),
                            Direction::Backward => start.saturating_sub(1),
                            Direction::Current => source_code.char_to_line(cursor_char_index.0),
                        }
                        .min(source_code.len_lines().saturating_sub(1)),
                    ),
                );
                let end = CharIndex(
                    start.0.saturating_add(
                        source_code
                            .line(source_code.char_to_line(start.0))
                            .len_chars(),
                    ),
                );
                Selection {
                    start: start.clone(),
                    end,
                    node_id: None,
                }
            }
            SelectionMode::Word => todo!(),
            SelectionMode::AncestorNode => {
                let current_node = get_current_node(tree, cursor_byte, current_selection);
                to_selection(current_node.parent().unwrap_or(current_node), &source_code)
            }

            SelectionMode::SiblingNode => {
                let current_node = get_current_node(tree, cursor_byte, current_selection);
                let next_node = match direction {
                    Direction::Forward => current_node.next_sibling(),
                    Direction::Backward => current_node.prev_sibling(),
                    Direction::Current => None,
                }
                .unwrap_or(current_node);
                to_selection(next_node, &source_code)
            }
            SelectionMode::Token => {
                let selection = match direction {
                    Direction::Forward => get_next_token(tree, current_selection_end_byte, false),
                    Direction::Backward => {
                        get_prev_token(tree, current_selection_start_byte, false)
                    }
                    Direction::Current => {
                        log::info!("current");
                        get_next_token(tree, cursor_byte, false)
                    }
                }
                .unwrap_or_else(|| {
                    get_next_token(tree, cursor_byte, true).unwrap_or_else(|| tree.root_node())
                });
                to_selection(selection, &source_code)
            }
            SelectionMode::Character => match direction {
                Direction::Current => Selection {
                    start: cursor_char_index,
                    end: cursor_char_index + 1,
                    node_id: None,
                },
                Direction::Forward => Selection {
                    start: cursor_char_index + 1,
                    end: cursor_char_index + 2,
                    node_id: None,
                },
                Direction::Backward => Selection {
                    start: cursor_char_index - 1,
                    end: cursor_char_index,
                    node_id: None,
                },
            },
            SelectionMode::Custom => Selection {
                start: cursor_char_index,
                end: cursor_char_index,
                node_id: None,
            },
        }
    }

    fn get_selection(&self, mode: &SelectionMode, direction: Direction) -> Selection {
        Self::get_selection_(
            &self.source_code,
            &self.tree,
            &self.selection,
            mode,
            &direction,
            &self.cursor_direction,
        )
    }

    pub fn get_cursor_point(&self) -> Point {
        self.get_cursor_char_index().to_point(&self.source_code)
    }

    fn get_cursor_char_index(&self) -> CharIndex {
        self.selection.to_char_index(&self.cursor_direction)
    }

    fn toggle_extend_mode(&mut self) {
        self.mode = match self.mode {
            Mode::Extend { .. } => Mode::Normal,
            _ => Mode::Extend {
                extended_selection: self.selection,
            },
        };
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(event) => self.handle_key_event(event),
            _ => {
                log::info!("{:?}", event)
            }
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match &self.mode {
            Mode::Normal | Mode::Extend { .. } => self.handle_normal_mode(key_event),
            Mode::Insert => self.handle_insert_mode(key_event),
            Mode::Jump { .. } => self.handle_jump_mode(key_event),
        }
    }

    // TODO: jump should work with extended selection
    // TODO: do we really need extended selection?
    fn handle_jump_mode(&mut self, key_event: KeyEvent) {
        match self.mode {
            Mode::Jump { ref jumps, .. } => match key_event.code {
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                }
                KeyCode::Char('j') => {
                    if let Some(jump) = jumps
                        .iter()
                        .max_by(|a, b| a.selection.start.cmp(&b.selection.start))
                    {
                        self.jump_from_selection(Direction::Forward, jump.selection);
                    }
                }
                KeyCode::Char('J') => {
                    if let Some(jump) = jumps
                        .iter()
                        .min_by(|a, b| a.selection.end.cmp(&b.selection.end))
                    {
                        self.jump_from_selection(Direction::Backward, jump.selection);
                    }
                }
                KeyCode::Char(c) => {
                    let matching_jump = jumps.iter().find(|jump| c == jump.character);
                    if let Some(jump) = matching_jump {
                        self.update_selection(Direction::Forward, jump.selection);
                        self.mode = Mode::Normal;
                    }
                }
                _ => {}
            },
            _ => unreachable!(),
        }
    }

    fn handle_insert_mode(&mut self, event: KeyEvent) {
        let Selection { start, end, .. } = self.selection;
        match event.code {
            KeyCode::Esc => self.enter_normal_mode(),
            KeyCode::Backspace => self.replace_with(start.0.saturating_sub(1)..end.0, "".into()),
            KeyCode::Enter => self.replace_with(start.0..end.0, "\n".into()),
            KeyCode::Tab => self.replace_with(start.0..end.0, "\t".into()),
            KeyCode::Char(c) => self.replace_with(start.0..end.0, c.to_string().into()),
            KeyCode::Left => {
                self.selection = Selection {
                    start: start - 1,
                    end: end - 1,
                    node_id: None,
                };
            }
            KeyCode::Right => {
                self.selection = Selection {
                    start: start + 1,
                    end: end + 1,
                    node_id: None,
                };
            }
            _ => {}
        }
    }

    fn handle_normal_mode(&mut self, event: KeyEvent) {
        match event.code {
            // Objects
            KeyCode::Char('a') => self.select_ancestor(),
            KeyCode::Char('b') => self.select_backward(),
            KeyCode::Char('c') if event.modifiers == KeyModifiers::CONTROL => {
                self.quit = true;
            }
            KeyCode::Char('c') => self.select_charater(Direction::Forward),
            KeyCode::Char('C') => self.select_charater(Direction::Backward),
            KeyCode::Char('d') => self.delete_current_selection(),
            KeyCode::Char('i') => self.enter_insert_mode(),
            KeyCode::Char('j') => self.jump(Direction::Forward),
            KeyCode::Char('J') => self.jump(Direction::Backward),
            KeyCode::Char('k') => self.select_kids(),
            KeyCode::Char('l') => self.select_line(Direction::Forward),
            KeyCode::Char('L') => self.select_line(Direction::Backward),
            KeyCode::Char('n') => self.select_named_node(Direction::Forward),
            KeyCode::Char('N') => self.select_named_node(Direction::Backward),
            KeyCode::Char('o') => self.change_cursor_direction(),
            KeyCode::Char('s') => self.select_sibling(Direction::Forward),
            KeyCode::Char('S') => self.select_sibling(Direction::Backward),
            KeyCode::Char('t') => self.select_token(Direction::Forward),
            KeyCode::Char('T') => self.select_token(Direction::Backward),
            KeyCode::Char('w') => self.select_word(),
            KeyCode::Char('p') => self.paste(),
            KeyCode::Char('r') => self.replace(),
            KeyCode::Char('x') => self.toggle_extend_mode(),
            KeyCode::Char('y') => self.yank(),
            KeyCode::Char('0') => self.select_none(Direction::Forward),
            _ => {
                log::info!("event: {:?}", event);
                // todo!("Back to previous selection");
                // todo!("Search by node kind")
            }
        }
    }

    fn enter_insert_mode(&mut self) {
        self.mode = Mode::Insert;
        let cursor_char_index = self.get_cursor_char_index();
        self.selection = Selection {
            start: cursor_char_index,
            end: cursor_char_index,
            node_id: None,
        };
    }

    fn enter_normal_mode(&mut self) {
        self.mode = Mode::Normal;
        self.select(SelectionMode::Character, Direction::Current);
    }

    pub fn jumps(&self) -> Vec<&Jump> {
        match self.mode {
            Mode::Jump { ref jumps } => jumps.iter().collect(),
            _ => vec![],
        }
    }
}

fn get_prev_token(tree: &Tree, byte: usize, is_named: bool) -> Option<Node> {
    for node in ReverseTreeCursor::new(tree.root_node()) {
        if node.child_count() == 0 && (!is_named || node.is_named()) && node.start_byte() < byte {
            return Some(node);
        }
    }
    None
}

fn get_next_token(tree: &Tree, byte: usize, is_named: bool) -> Option<Node> {
    for node in traverse(tree.root_node().walk(), Order::Post) {
        if node.child_count() == 0 && (!is_named || node.is_named()) && node.end_byte() > byte {
            return Some(node);
        }
    }
    None
}

fn get_nearest_node_after_byte(tree: &Tree, byte: usize) -> Option<Node> {
    // Preorder is the main key here,
    // because preorder traversal walks the parent first
    for node in traverse(tree.root_node().walk(), Order::Pre) {
        if node.start_byte() >= byte {
            return Some(node);
        }
    }
    None
}

fn get_node_by_id(tree: &Tree, node_id: usize) -> Option<Node> {
    let result = traverse(tree.walk(), Order::Pre).find(|node| node.id() == node_id);
    result
}

fn to_selection(node: Node, source_code: &Rope) -> Selection {
    Selection {
        start: CharIndex(source_code.byte_to_char(node.start_byte())),
        end: CharIndex(source_code.byte_to_char(node.end_byte())),
        node_id: Some(node.id()),
    }
}

struct ReverseTreeCursor<'a> {
    node: Node<'a>,
}

struct ParentTreeCursor<'a> {
    node: Node<'a>,
}

impl<'a> ReverseTreeCursor<'a> {
    fn new(node: Node<'a>) -> Self {
        Self {
            node: go_to_last_descendant(node),
        }
    }
}

fn go_to_last_descendant(node: Node) -> Node {
    let mut node = node;
    loop {
        if let Some(sibling) = node.next_sibling() {
            node = sibling
        } else if let Some(child) = node.child(node.child_count().saturating_sub(1)) {
            node = child;
        } else {
            return node;
        }
    }
}

impl<'a> Iterator for ReverseTreeCursor<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self
            .node
            .prev_sibling()
            .map(|node| node.child(0).map(go_to_last_descendant).unwrap_or(node))
            .or_else(|| self.node.parent());
        if let Some(next) = next {
            self.node = next;
            Some(self.node)
        } else {
            None
        }
    }
}

impl<'a> Iterator for ParentTreeCursor<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.node.parent();
        if let Some(next) = next {
            self.node = next;
            Some(self.node)
        } else {
            None
        }
    }
}

fn get_current_node<'a>(tree: &'a Tree, cursor_byte: usize, selection: &Selection) -> Node<'a> {
    if let Some(node_id) = selection.node_id {
        get_node_by_id(&tree, node_id)
    } else {
        get_nearest_node_after_byte(&tree, cursor_byte)
    }
    .unwrap_or_else(|| tree.root_node())
}
