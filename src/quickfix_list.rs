use std::{cell::RefCell, collections::HashMap, ops::Range, rc::Rc};

use itertools::Itertools;
use lsp_types::DiagnosticSeverity;

use crate::{
    app::Dispatches,
    buffer::Buffer,
    char_index_range::CharIndexRange,
    components::{
        dropdown::{Dropdown, DropdownConfig, DropdownItem},
        editor::Movement,
        suggestive_editor::Info,
    },
    position::Position,
};
use shared::absolute_path::AbsolutePath;

impl QuickfixListItem {
    fn into_dropdown_item(
        self: &QuickfixListItem,
        buffers: &HashMap<AbsolutePath, Rc<RefCell<Buffer>>>,
        position_range: &Range<Position>,
        current_working_directory: &AbsolutePath,
        show_line_number: bool,
        max_line_number_digits_count: usize,
        max_column_number_digits_count: usize,
    ) -> DropdownItem {
        let Position { line, column } = position_range.start;

        let line_number = if show_line_number {
            format!("{:>width$}", line + 1, width = max_line_number_digits_count)
        } else {
            " ".repeat(max_line_number_digits_count)
        };

        let prefix = format!(
            "{line_number}:{:>width$}  ",
            column + 1,
            width = max_column_number_digits_count
        );

        let highlight_column_range = if position_range.end.line == position_range.start.line {
            let left_padding = prefix.chars().count();
            Some(
                position_range.start.column + left_padding
                    ..position_range.end.column + left_padding,
            )
        } else {
            None
        };
        DropdownItem::new({
            let content = self
                .line
                .clone()
                .map(|line| line.trim_end_matches(['\n', '\r']).to_string())
                .unwrap_or_else(|| {
                    self.location
                        .read_from_buffers(buffers)
                        .unwrap_or_else(|| "[Failed to read file]".to_string())
                        .trim_matches(['\n', '\r'])
                        .to_string()
                });
            format!("{prefix}{content}")
        })
        .set_info(self.info.clone())
        .set_group({
            let path = self.location.path.clone();
            Some(
                path.display_relative_to(current_working_directory)
                    .unwrap_or_else(|_| path.display_absolute()),
            )
        })
        .set_dispatches(Dispatches::one(crate::app::Dispatch::GotoLocation(
            self.location.to_owned(),
        )))
        .set_rank(Some(Box::new([line, column])))
        .set_highlight_column_range(highlight_column_range)
        .set_is_significant(Some(show_line_number))
    }

    pub fn apply_edit(self, edit: &crate::edit::Edit) -> Option<Self> {
        Some(Self {
            location: self.location.apply_edit(edit)?,
            ..self
        })
    }
}

pub struct QuickfixList {
    dropdown: Dropdown,
    items: Vec<QuickfixListItem>,
    title: String,
    buffers: HashMap<AbsolutePath, Rc<RefCell<Buffer>>>,
}

impl QuickfixList {
    pub fn items(&self) -> &[QuickfixListItem] {
        &self.items
    }

    pub fn items_count(&self) -> usize {
        self.dropdown.items_count()
    }

    pub fn render(&self) -> crate::components::dropdown::DropdownRender {
        self.dropdown.render()
    }

    /// Returns the current item index after `movement` is applied
    pub fn get_item(&mut self, movement: Movement) -> Option<(usize, Dispatches)> {
        self.dropdown.apply_movement(movement);
        Some((
            self.dropdown.current_item_index(),
            self.dropdown.current_item()?.dispatches,
        ))
    }

    pub fn set_current_item_index(&mut self, item_index: usize) {
        self.dropdown.set_current_item_index(item_index);
    }

    pub(crate) fn default() -> QuickfixList {
        QuickfixList {
            dropdown: Dropdown::new(DropdownConfig {
                title: Default::default(),
            }),
            items: Default::default(),
            title: Default::default(),
            buffers: Default::default(),
        }
    }

    pub(crate) fn extend_items(
        &mut self,
        items: Vec<QuickfixListItem>,
        current_working_directory: &AbsolutePath,
    ) {
        self.extend_buffers(&items, current_working_directory);

        let items = {
            self.items.extend(items);
            self.items.drain(..).collect()
        };
        let (items, dropdown_items) = self.convert_items(items, current_working_directory);
        self.dropdown.set_items(dropdown_items);
        self.items = items;
    }

    pub(crate) fn set_items(
        &mut self,
        items: Vec<QuickfixListItem>,
        current_working_directory: &AbsolutePath,
    ) {
        // Clear buffers cache
        self.buffers.clear();
        self.extend_buffers(&items, current_working_directory);

        let (items, dropdown_items) = self.convert_items(items, current_working_directory);
        self.dropdown.set_items(dropdown_items);
        self.items = items;
    }

    fn convert_items(
        &mut self,
        items: Vec<QuickfixListItem>,
        current_working_directory: &AbsolutePath,
    ) -> (Vec<QuickfixListItem>, Vec<DropdownItem>) {
        let items = items
            .into_iter()
            // Sort the items by location
            .sorted_by_key(|item| item.location.clone())
            .chunk_by(|item| (item.location.clone(), item.line.clone()))
            .into_iter()
            .map(|((location, line), items)| QuickfixListItem {
                location,
                line,
                info: items
                    .into_iter()
                    .flat_map(|item| item.info.clone())
                    .reduce(Info::join),
            })
            .collect_vec();
        let items_with_position_range = items
            .iter()
            .filter_map(|item| {
                let buffer = self.buffers.get(&item.location.path)?;
                let position_range = buffer
                    .borrow()
                    .char_index_range_to_position_range(item.location.range)
                    .ok()?;
                Some((position_range, item))
            })
            .collect_vec();

        let max_line_number_digits_count = (items_with_position_range
            .iter()
            .map(|(position_range, _)| position_range.start.line)
            .max()
            .unwrap_or(0)
            + 1)
        .to_string()
        .chars()
        .count();

        let max_column_number_digits_count = (items_with_position_range
            .iter()
            .map(|(position_range, _)| position_range.start.column)
            .max()
            .unwrap_or(0)
            + 1)
        .to_string()
        .chars()
        .count();
        let dropdown_items = items_with_position_range
            .iter()
            .chunk_by(|(position_range, item)| {
                (position_range.start.line, item.location.path.clone())
            })
            .into_iter()
            .flat_map(|(_, items)| {
                items
                    .into_iter()
                    .enumerate()
                    .map(|(index, (position_range, item))| {
                        item.to_owned().into_dropdown_item(
                            &self.buffers,
                            position_range,
                            current_working_directory,
                            index == 0,
                            max_line_number_digits_count,
                            max_column_number_digits_count,
                        )
                    })
            })
            .collect_vec();

        (items, dropdown_items)
    }

    pub(crate) fn handle_applied_edits(
        &mut self,
        path: &AbsolutePath,
        edits: &[crate::edit::Edit],
        current_working_directory: &AbsolutePath,
    ) {
        let items = self
            .items
            .drain(..)
            .filter_map(|item| {
                if &item.location().path == path {
                    edits
                        .iter()
                        .try_fold(item, |item, edit| item.apply_edit(edit))
                } else {
                    Some(item)
                }
            })
            .collect_vec();
        self.set_items(items, current_working_directory);
    }

    pub(crate) fn title(&self) -> String {
        self.title.clone()
    }

    pub(crate) fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    fn extend_buffers(
        &mut self,
        items: &[QuickfixListItem],
        current_working_directory: &AbsolutePath,
    ) {
        // Extend the buffers cache with new paths
        for path in items
            .iter()
            .map(|item| &item.location().path)
            .unique_by(|path| path.try_display_relative_to(current_working_directory))
        {
            self.buffers.entry(path.clone()).or_insert_with(|| {
                Buffer::from_path(path, false)
                    .ok()
                    .map(|buffer| Rc::new(RefCell::new(buffer)))
                    .unwrap_or_else(|| Rc::new(RefCell::new(Buffer::new(None, ""))))
            });
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickfixListItem {
    /// This field is for performance optimization,
    /// if it exists, then we do not need to query the filesystem
    /// for the contain of this line (specified by `self.location.range.start.line`).
    ///
    /// This is actually not merely for performance optimization,
    /// it also avoid an issues where sometimes the location of a file   
    /// cannot be query, which happens frequently when we made global search async.   
    line: Option<String>,
    location: Location,
    info: Option<Info>,
}

impl PartialOrd for QuickfixListItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QuickfixListItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.location.cmp(&other.location)
    }
}

impl From<Location> for QuickfixListItem {
    fn from(value: Location) -> Self {
        QuickfixListItem {
            location: value,
            info: None,
            line: None,
        }
    }
}

impl QuickfixListItem {
    pub fn new(location: Location, info: Option<Info>, line: Option<String>) -> QuickfixListItem {
        QuickfixListItem {
            location,
            info,
            line,
        }
    }

    pub fn location(&self) -> &Location {
        &self.location
    }

    pub fn info(&self) -> &Option<Info> {
        &self.info
    }

    #[cfg(test)]
    pub fn set_info(self, info: Option<Info>) -> Self {
        Self { info, ..self }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Location {
    pub path: AbsolutePath,
    pub range: CharIndexRange,
}

impl Location {
    fn read_from_buffers(
        &self,
        buffers: &HashMap<AbsolutePath, Rc<RefCell<Buffer>>>,
    ) -> Option<String> {
        buffers.get(&self.path).and_then(|buffer| {
            Some(
                buffer
                    .borrow()
                    .get_line_by_char_index(self.range.start)
                    .ok()?
                    .to_string(),
            )
        })
    }

    fn apply_edit(self, edit: &crate::edit::Edit) -> Option<Location> {
        Some(Self {
            range: self.range.apply_edit(edit)?,
            ..self
        })
    }
}

impl TryFrom<lsp_types::Location> for Location {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::Location) -> Result<Self, Self::Error> {
        let path = value
            .uri
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("Failed to convert uri to file path"))?
            .try_into()?;
        let buffer = Buffer::from_path(&path, false)?;
        let range = buffer.position_range_to_char_index_range(
            &(value.range.start.into()..value.range.end.into()),
        )?;
        Ok(Location { path, range })
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&self.path, self.range.start, self.range.start).cmp(&(
            &other.path,
            other.range.start,
            other.range.start,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickfixListType {
    Diagnostic(DiagnosticSeverityRange),
    Items(Vec<QuickfixListItem>),
    Mark,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum DiagnosticSeverityRange {
    All,
    Error,
    Warning,
    Information,
    Hint,
}
impl DiagnosticSeverityRange {
    pub fn contains(&self, severity: Option<DiagnosticSeverity>) -> bool {
        matches!(
            (self, severity),
            (DiagnosticSeverityRange::All, _)
                | (
                    DiagnosticSeverityRange::Error,
                    Some(DiagnosticSeverity::ERROR)
                )
                | (
                    DiagnosticSeverityRange::Warning,
                    Some(DiagnosticSeverity::WARNING)
                )
                | (
                    DiagnosticSeverityRange::Information,
                    Some(DiagnosticSeverity::INFORMATION)
                )
                | (
                    DiagnosticSeverityRange::Hint,
                    Some(DiagnosticSeverity::HINT)
                )
        )
    }
}

#[cfg(test)]
mod test_quickfix_list {

    use crate::{
        app::{
            Dispatch::{self, *},
            LocalSearchConfigUpdate, Scope,
        },
        buffer::BufferOwner,
        components::{
            editor::{DispatchEditor::*, IfCurrentNotFound, Movement::*},
            suggestive_editor::Info,
        },
        selection::CharIndex,
        test_app::{execute_test, ExpectKind::*, Step::*},
    };

    use super::{Location, QuickfixList, QuickfixListItem};
    use pretty_assertions::assert_eq;
    use shared::absolute_path::AbsolutePath;

    #[test]
    fn should_sort_items() {
        let git_ignore_path: AbsolutePath = ".gitignore".try_into().unwrap();
        let readme_path: AbsolutePath = "README.md".try_into().unwrap();
        let foo = QuickfixListItem {
            location: Location {
                path: git_ignore_path.clone(),
                range: (CharIndex(2)..CharIndex(3)).into(),
            },
            info: None,
            line: None,
        };
        let bar = QuickfixListItem {
            location: Location {
                path: readme_path.clone(),
                range: (CharIndex(1)..CharIndex(2)).into(),
            },
            info: None,
            line: None,
        };
        let spam = QuickfixListItem {
            location: Location {
                path: git_ignore_path.clone(),
                range: (CharIndex(1)..CharIndex(2)).into(),
            },
            info: None,
            line: None,
        };
        let mut quickfix_list = QuickfixList::default();
        quickfix_list.set_items(
            vec![foo.clone(), bar.clone(), spam.clone()],
            &std::env::current_dir().unwrap().try_into().unwrap(),
        );
        assert_eq!(quickfix_list.items(), &vec![spam, foo, bar]);
    }

    #[test]
    fn should_merge_items_of_same_location() {
        let readme_path: AbsolutePath = "README.md".try_into().unwrap();
        let items = [
            QuickfixListItem {
                location: Location {
                    path: readme_path.clone(),
                    range: (CharIndex(1)..CharIndex(2)).into(),
                },
                info: Some(Info::new("Title 1".to_string(), "spongebob".to_string())),
                line: None,
            },
            QuickfixListItem {
                location: Location {
                    path: readme_path.clone(),
                    range: (CharIndex(1)..CharIndex(2)).into(),
                },
                info: Some(Info::new("Title 2".to_string(), "squarepants".to_string())),
                line: None,
            },
        ]
        .to_vec();

        let mut quickfix_list = QuickfixList::default();
        quickfix_list.set_items(items, &std::env::current_dir().unwrap().try_into().unwrap());

        assert_eq!(
            quickfix_list.items(),
            &vec![QuickfixListItem {
                location: Location {
                    path: "README.md".try_into().unwrap(),
                    range: (CharIndex(1)..CharIndex(2)).into(),
                },
                info: Some(Info::new(
                    "Title 1".to_string(),
                    ["spongebob", "squarepants"].join("\n==========\n")
                )),
                line: None
            }]
        );
    }
    #[test]
    fn should_hide_line_number_of_non_first_same_line_entries() -> anyhow::Result<()> {
        test_display_quickfix_list(
            "alohax alohax
bar
alohax third line",
            "alohax",
            "
src/foo.rs
    1:1  alohax alohax
     :8  alohax alohax
    3:1  alohax third line",
        )
    }

    #[test]
    fn line_number_should_align_right() -> anyhow::Result<()> {
        test_display_quickfix_list(
            "
one
2
3
4
5
6
7
8
9
one",
            "one",
            "
src/foo.rs
     1:1  one
    10:1  one",
        )
    }

    #[test]
    fn column_number_should_align_right() -> anyhow::Result<()> {
        test_display_quickfix_list(
            "hax                    hax",
            "hax",
            "
src/foo.rs
    1: 1  hax                    hax
     :24  hax                    hax",
        )
    }

    fn test_display_quickfix_list(
        file_content: &'static str,
        search: &'static str,
        expected: &'static str,
    ) -> anyhow::Result<()> {
        execute_test(|s| {
            let new_dispatch = |update: LocalSearchConfigUpdate| -> Dispatch {
                UpdateLocalSearchConfig {
                    update,
                    scope: Scope::Global,
                    if_current_not_found: IfCurrentNotFound::LookForward,
                    run_search_after_config_updated: true,
                    component_id: None,
                }
            };
            Box::new([
                App(OpenFile {
                    path: s.foo_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(file_content.trim().to_string())),
                Editor(Save),
                App(new_dispatch(LocalSearchConfigUpdate::Search(
                    search.to_string(),
                ))),
                WaitForAppMessage(lazy_regex::regex!("GlobalSearchFinished")),
                Expect(QuickfixListContent(expected.to_string().trim().to_string())),
            ])
        })
    }

    #[test]
    fn left_right_movement_skips_same_line_entries() -> anyhow::Result<()> {
        execute_test(|s| {
            let new_dispatch = |update: LocalSearchConfigUpdate| -> Dispatch {
                UpdateLocalSearchConfig {
                    update,
                    scope: Scope::Global,
                    if_current_not_found: IfCurrentNotFound::LookForward,
                    run_search_after_config_updated: true,
                    component_id: None,
                }
            };
            Box::new([
                App(OpenFile {
                    path: s.foo_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent(
                    "
aslmlkm world aslmlkm
bar bar
aslmlkm kitty aslmlkm
spam spam
aslmlkm ki aslmlkm"
                        .trim()
                        .to_string(),
                )),
                Editor(Save),
                App(new_dispatch(LocalSearchConfigUpdate::Search(
                    "aslmlkm".to_string(),
                ))),
                WaitForAppMessage(lazy_regex::regex!("GlobalSearchFinished")),
                Expect(QuickfixListContent(
                    "
src/foo.rs
    1: 1  aslmlkm world aslmlkm
     :15  aslmlkm world aslmlkm
    3: 1  aslmlkm kitty aslmlkm
     :15  aslmlkm kitty aslmlkm
    5: 1  aslmlkm ki aslmlkm
     :12  aslmlkm ki aslmlkm"
                        .to_string()
                        .trim()
                        .to_string(),
                )),
                Expect(QuickfixListCurrentLine("    1: 1  aslmlkm world aslmlkm")),
                Editor(MoveSelection(Right)),
                Expect(QuickfixListCurrentLine("    3: 1  aslmlkm kitty aslmlkm")),
                Editor(MoveSelection(Right)),
                Expect(QuickfixListCurrentLine("    5: 1  aslmlkm ki aslmlkm")),
                Editor(MoveSelection(Left)),
                Expect(QuickfixListCurrentLine("    3: 1  aslmlkm kitty aslmlkm")),
                Editor(MoveSelection(Left)),
                Expect(QuickfixListCurrentLine("    1: 1  aslmlkm world aslmlkm")),
            ])
        })
    }

    #[test]
    fn line_number_of_next_file_entries_should_not_be_elided() -> anyhow::Result<()> {
        execute_test(|s| {
            let new_dispatch = |update: LocalSearchConfigUpdate| -> Dispatch {
                UpdateLocalSearchConfig {
                    update,
                    scope: Scope::Global,
                    if_current_not_found: IfCurrentNotFound::LookForward,
                    run_search_after_config_updated: true,
                    component_id: None,
                }
            };
            Box::new([
                App(OpenFile {
                    path: s.foo_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("foo bar foo\nfoo spam".trim().to_string())),
                Editor(Save),
                App(OpenFile {
                    path: s.main_rs(),
                    owner: BufferOwner::User,
                    focus: true,
                }),
                Editor(SetContent("main main\nmain foo spam".trim().to_string())),
                Editor(Save),
                App(new_dispatch(LocalSearchConfigUpdate::Search(
                    "foo".to_string(),
                ))),
                WaitForAppMessage(lazy_regex::regex!("GlobalSearchFinished")),
                Expect(QuickfixListContent(
                    "
src/foo.rs
    1:1  foo bar foo
     :9  foo bar foo
    2:1  foo spam

src/main.rs
    2:6  main foo spam
"
                    .to_string()
                    .trim()
                    .to_string(),
                )),
            ])
        })
    }
}
