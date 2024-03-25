use crate::{app::Dispatches, components::editor::Movement};

use itertools::Itertools;
use shared::{canonicalized_path::CanonicalizedPath, icons::get_icon_config};

use super::suggestive_editor::Info;

#[derive(Clone, Debug, PartialEq, Eq)]
/// Note: filtering will be done on the combination of `display` and `group` (if applicable)
pub struct DropdownItem {
    pub dispatches: Dispatches,
    pub display: String,
    pub group: Option<String>,
    pub info: Option<Info>,
    /// Sorting will be based on `rank` if defined, otherwise sorting will be based on `display`
    pub rank: Option<Box<[usize]>>,
}

impl DropdownItem {
    pub fn display(&self) -> String {
        self.display.clone()
    }

    pub(crate) fn new(display: String) -> Self {
        Self {
            dispatches: Default::default(),
            display,
            group: Default::default(),
            info: Default::default(),
            rank: None,
        }
    }

    pub(crate) fn set_info(self, info: Option<Info>) -> Self {
        Self { info, ..self }
    }

    pub(crate) fn set_dispatches(self, dispatches: Dispatches) -> DropdownItem {
        Self { dispatches, ..self }
    }
}

impl From<CanonicalizedPath> for DropdownItem {
    fn from(value: CanonicalizedPath) -> Self {
        Self {
            display: {
                let name = value
                    .to_path_buf()
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let icon = value.icon();
                format!("{icon} {name}")
            },
            group: value.parent().ok().flatten().map(|parent| {
                format!(
                    "{} {}",
                    get_icon_config().folder,
                    parent.try_display_relative()
                )
            }),
            dispatches: Dispatches::one(crate::app::Dispatch::OpenFile(value)),
            info: None,
            rank: None,
        }
    }
}

impl From<String> for DropdownItem {
    fn from(value: String) -> Self {
        Self {
            display: value.clone(),
            dispatches: Dispatches::default(),
            group: None,
            info: None,
            rank: None,
        }
    }
}

pub trait FromVec<T: Clone + Into<DropdownItem>> {
    fn from(value: Vec<T>) -> Vec<DropdownItem>
    where
        Self: Sized,
    {
        value.into_iter().map(|v| v.into()).collect()
    }
}

pub struct Dropdown {
    title: String,
    filter: String,
    items: Vec<DropdownItem>,
    filtered_items: Vec<DropdownItem>,
    current_item_index: usize,
}

pub struct DropdownConfig {
    pub title: String,
}

impl Dropdown {
    pub fn new(config: DropdownConfig) -> Self {
        Self {
            filter: String::new(),
            items: vec![],
            filtered_items: vec![],
            current_item_index: 0,
            title: config.title,
        }
    }

    pub fn change_index(&mut self, index: usize) {
        if !(0..self.items.len()).contains(&index) {
            return;
        }
        self.current_item_index = index;
    }

    fn highlight_line_index(&self) -> usize {
        let group_title_size = self
            .items
            .first()
            .and_then(|item| item.group.as_ref().map(|_| 1))
            .unwrap_or(0);
        let gap_size = 1;
        let group_index = self.get_current_item_group_index().unwrap_or(0);
        let group_gap = group_index * gap_size;

        self.current_item_index + group_index * group_title_size + group_gap + group_title_size
    }

    pub fn next_item(&mut self) {
        self.change_index(self.current_item_index + 1)
    }

    pub fn previous_item(&mut self) {
        self.change_index(self.current_item_index.saturating_sub(1))
    }

    fn last_item(&mut self) {
        self.change_index(self.items.len().saturating_sub(1))
    }

    fn first_item(&mut self) {
        self.change_index(0)
    }

    fn groups(&self) -> Option<Vec<String>> {
        let groups = self
            .filtered_items
            .iter()
            .flat_map(|item| item.group.clone())
            .unique()
            .sorted()
            .collect_vec();
        if groups.is_empty() {
            None
        } else {
            Some(groups)
        }
    }

    fn get_current_item_group_index(&self) -> Option<usize> {
        let current_group = self.current_item()?.group?;
        let groups = self.groups()?;
        let (current_group_index, _) = groups
            .iter()
            .find_position(|group| group == &&current_group)?;
        Some(current_group_index)
    }

    fn change_group_index(&mut self, increment: bool) -> Option<()> {
        let _groups = self.groups();
        let groups = self.groups()?;
        let current_group_index = self.get_current_item_group_index()?;
        let new_group_index = if increment {
            current_group_index.saturating_add(1)
        } else {
            current_group_index.saturating_sub(1)
        };
        let new_group = groups.get(new_group_index)?;
        let (new_item_index, _) = self
            .filtered_items
            .iter()
            .find_position(|item| item.group.as_ref() == Some(new_group))?;
        self.change_index(new_item_index);
        Some(())
    }

    fn next_group(&mut self) {
        self.change_group_index(true).unwrap_or_default()
    }

    fn previous_group(&mut self) {
        self.change_group_index(false).unwrap_or_default()
    }

    pub fn current_item(&self) -> Option<DropdownItem> {
        self.filtered_items.get(self.current_item_index).cloned()
    }

    pub fn set_items(&mut self, items: Vec<DropdownItem>) {
        self.items = items;
        self.current_item_index = 0;
        self.compute_filtered_items();
    }

    fn compute_filtered_items(&mut self) {
        self.filtered_items = self
            .items
            .iter()
            .filter(|item| {
                item.display
                    .to_lowercase()
                    .contains(&self.filter.to_lowercase())
            })
            .sorted_by(|a, b| match (&a.rank, &b.rank) {
                (Some(rank_a), Some(rank_b)) => (&a.group, rank_a).cmp(&(&b.group, rank_b)),
                _ => (&a.group, &a.display).cmp(&(&b.group, &b.display)),
            })
            .cloned()
            .collect();
    }

    pub fn set_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
        self.current_item_index = 0;
        self.compute_filtered_items();
    }

    pub fn render(&self) -> DropdownRender {
        DropdownRender {
            title: self.title.clone(),
            content: self.content(),
            highlight_line_index: self.highlight_line_index(),
            info: self.current_item().and_then(|item| item.info),
        }
    }

    fn content(&self) -> String {
        self.filtered_items
            .iter()
            .group_by(|item| &item.group)
            .into_iter()
            .map(|(group_key, items)| {
                if let Some(group_key) = group_key {
                    let items = items.collect_vec();
                    let items_len = items.len();
                    let items = items
                        .into_iter()
                        .enumerate()
                        .map(|(index, item)| {
                            let content = item.display();
                            let indicator = if index == items_len.saturating_sub(1) {
                                "└─"
                            } else {
                                "├─"
                            };
                            format!(" {} {}", indicator, content)
                        })
                        .join("\n");
                    format!("■┬ {}\n{}", group_key, items)
                } else {
                    items.into_iter().map(|item| item.display()).join("\n")
                }
            })
            .collect::<Vec<String>>()
            .join("\n\n")
    }

    pub fn apply_movement(&mut self, movement: Movement) {
        match movement {
            Movement::Next => self.next_item(),
            Movement::Current => {}
            Movement::Previous => self.previous_item(),
            Movement::Last => self.last_item(),
            Movement::First => self.first_item(),
            Movement::Up => self.previous_group(),
            Movement::Down => self.next_group(),
            _ => {}
        }
    }

    #[cfg(test)]
    fn assert_highlighted_content(&self, label: &str) {
        let render = self.render();
        let index = render.highlight_line_index;
        let highlighed_content = render.content.lines().collect_vec()[index];
        assert_eq!(highlighed_content, label);
    }

    pub(crate) fn items(&self) -> Vec<DropdownItem> {
        self.items.clone()
    }

    pub(crate) fn clear(&mut self) {
        self.set_items(Default::default())
    }
}

#[cfg(test)]
mod test_dropdown {
    use crate::components::{
        dropdown::{Dropdown, DropdownConfig, DropdownItem},
        suggestive_editor::Info,
    };
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Item {
        label: String,
        info: Info,
        group: String,
    }
    impl PartialOrd for Item {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for Item {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.label.cmp(&other.label)
        }
    }

    impl Item {
        fn new(label: &str, info: &str, group: &str) -> Self {
            Self {
                label: label.to_string(),
                info: Info::new("".to_string(), info.to_string()),
                group: group.to_string(),
            }
        }
    }
    impl From<Item> for DropdownItem {
        fn from(value: Item) -> Self {
            Self {
                info: Some(value.info.clone()),
                display: value.label.to_string(),
                group: Some(value.group.clone()),
                dispatches: Default::default(),
                rank: None,
            }
        }
    }

    #[test]
    fn test_next_prev_group() {
        let mut dropdown = Dropdown::new(DropdownConfig {
            title: "test".to_string(),
        });
        let item_a = Item::new("a", "", "1");
        dropdown.set_items(
            [
                item_a.clone(),
                Item::new("d", "", "2"),
                Item::new("c", "", "2"),
                Item::new("b", "", "3"),
            ]
            .into_iter()
            .map(|item| item.into())
            .collect(),
        );

        // Expect the items are sorted by group first, then by label

        assert_eq!(
            dropdown.render().content.trim(),
            "
■┬ 1
 └─ a

■┬ 2
 ├─ c
 └─ d

■┬ 3
 └─ b
"
            .trim()
        );
        dropdown.assert_highlighted_content(" └─ a");

        dropdown.next_group();
        dropdown.assert_highlighted_content(" ├─ c");
        dropdown.next_group();
        dropdown.assert_highlighted_content(" └─ b");

        dropdown.previous_group();
        dropdown.assert_highlighted_content(" ├─ c");
        dropdown.previous_group();
        dropdown.assert_highlighted_content(" └─ a");
    }

    #[test]
    fn test_dropdown_without_group() -> anyhow::Result<()> {
        let mut dropdown = Dropdown::new(DropdownConfig {
            title: "test".to_string(),
        });
        dropdown.set_items(
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
                .into_iter()
                .map(|s| s.into())
                .collect(),
        );
        assert_eq!(dropdown.render().content, "a\nb\nc");
        dropdown.assert_highlighted_content("a");
        dropdown.next_item();
        dropdown.assert_highlighted_content("b");
        dropdown.next_item();
        dropdown.assert_highlighted_content("c");
        dropdown.next_item();
        dropdown.assert_highlighted_content("c");

        dropdown.previous_item();
        dropdown.assert_highlighted_content("b");
        dropdown.previous_item();
        dropdown.assert_highlighted_content("a");
        dropdown.previous_item();
        dropdown.assert_highlighted_content("a");

        dropdown.set_filter("b");
        dropdown.assert_highlighted_content("b");
        dropdown.set_filter("c");
        dropdown.assert_highlighted_content("c");
        dropdown.set_filter("d");
        assert_eq!(dropdown.current_item(), None);

        dropdown.set_filter("");
        dropdown.assert_highlighted_content("a");
        dropdown.next_item();
        dropdown.assert_highlighted_content("b");

        dropdown.set_items(
            vec![
                "lorem".to_string(),
                "ipsum".to_string(),
                "dolor".to_string(),
            ]
            .into_iter()
            .map(|s| s.into())
            .collect(),
        );

        // The current item should be `dolor` because dropdown will sort the items
        dropdown.assert_highlighted_content("dolor");
        dropdown.next_item();
        dropdown.assert_highlighted_content("ipsum");

        Ok(())
    }

    #[test]
    fn filter_should_work_regardless_of_case() -> anyhow::Result<()> {
        let mut dropdown = Dropdown::new(DropdownConfig {
            title: "test".to_string(),
        });
        dropdown.set_items(
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
                .into_iter()
                .map(|s| s.into())
                .collect(),
        );
        dropdown.set_filter("A");
        assert_eq!(dropdown.current_item().unwrap().display, "a");
        Ok(())
    }

    #[test]
    fn setting_filter_should_show_info_of_the_new_first_item() -> anyhow::Result<()> {
        let mut dropdown = Dropdown::new(DropdownConfig {
            title: "test".to_string(),
        });
        dropdown.set_items(
            vec![
                Item::new("a", "info a", ""),
                Item::new("b", "info b", ""),
                Item::new("c", "info c", ""),
            ]
            .into_iter()
            .map(|s| s.into())
            .collect(),
        );

        assert_eq!(dropdown.current_item().unwrap().display, "a");
        assert_eq!(dropdown.render().info.unwrap().content(), "info a");

        dropdown.set_filter("b");

        assert_eq!(dropdown.current_item().unwrap().display, "b");
        assert_eq!(dropdown.render().info.unwrap().content(), "info b");
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropdownRender {
    pub content: String,
    pub title: String,
    pub highlight_line_index: usize,
    pub info: Option<Info>,
}
