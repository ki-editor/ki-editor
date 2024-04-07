use std::cmp::Reverse;

use crate::{app::Dispatches, components::editor::Movement};

use itertools::Itertools;

use shared::{canonicalized_path::CanonicalizedPath, icons::get_icon_config};

use super::suggestive_editor::Info;

#[derive(Clone, Debug, PartialEq, Eq)]
/// Note: filtering will be done on the combination of `display` and `group` (if applicable)
pub struct DropdownItem {
    pub dispatches: Dispatches,
    display: String,
    group: Option<String>,
    info: Option<Info>,
    /// Sorting will be based on `rank` if defined, otherwise sorting will be based on `display`
    rank: Option<Box<[usize]>>,
    group_and_display: String,
}

impl AsRef<str> for DropdownItem {
    fn as_ref(&self) -> &str {
        &self.group_and_display
    }
}

impl DropdownItem {
    pub fn display(&self) -> String {
        self.display.clone()
    }

    pub(crate) fn new(display: String) -> Self {
        Self {
            dispatches: Default::default(),
            group_and_display: display.clone(),
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

    pub fn set_group(self, group: Option<String>) -> Self {
        Self {
            group_and_display: format!("{} {}", group.clone().unwrap_or_default(), self.display),
            group,
            ..self
        }
    }

    pub(crate) fn set_rank(self, rank: Option<Box<[usize]>>) -> DropdownItem {
        Self { rank, ..self }
    }
}

impl From<CanonicalizedPath> for DropdownItem {
    fn from(value: CanonicalizedPath) -> Self {
        DropdownItem::new({
            let name = value
                .to_path_buf()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let icon = value.icon();
            format!("{icon} {name}")
        })
        .set_group(value.parent().ok().flatten().map(|parent| {
            format!(
                "{} {}",
                get_icon_config().folder,
                parent.try_display_relative()
            )
        }))
        .set_dispatches(Dispatches::one(crate::app::Dispatch::OpenFile(value)))
    }
}

impl From<String> for DropdownItem {
    fn from(value: String) -> Self {
        Self::new(value)
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
        use nucleo_matcher::{
            pattern::{CaseMatching, Normalization, Pattern},
            Config, Matcher,
        };
        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
        let matches = Pattern::parse(&self.filter, CaseMatching::Ignore, Normalization::Smart)
            .match_list(self.items.clone(), &mut matcher);
        println!("=========");
        self.filtered_items = matches
            .into_iter()
            .sorted_by_key(|(item, _)| item.group.clone())
            // Sort by group first
            .group_by(|(item, _)| item.group.clone())
            .into_iter()
            .map(|(group, items)| {
                let items = items.collect_vec();
                println!(
                    "group: {group:?} [{}]",
                    items
                        .clone()
                        .into_iter()
                        .map(|item| item.0.display)
                        .join(", ")
                );
                (
                    group,
                    // Then for each group, sort by fuzzy score
                    items
                        .into_iter()
                        .sorted_by_key(|(item, fuzzy_score)| {
                            (Reverse(*fuzzy_score), item.display.clone())
                        })
                        .collect_vec(),
                )
            })
            // Then sort the group by the best fuzzy score of each group
            .sorted_by_key(|(group, items)| {
                (
                    Reverse(
                        items
                            .iter()
                            .map(|(_, fuzzy_score)| (*fuzzy_score))
                            .max()
                            .unwrap_or_default(),
                    ),
                    group.clone(),
                )
            })
            .flat_map(|group| group.1.into_iter().map(|(item, _)| item).collect_vec())
            .collect_vec();
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

    pub(crate) fn set_current_item_index(&mut self, current_item_index: usize) {
        self.current_item_index = current_item_index
    }

    pub(crate) fn current_item_index(&self) -> usize {
        self.current_item_index
    }
}

#[cfg(test)]
mod test_dropdown {
    use itertools::Itertools as _;

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
                group_and_display: value.label.to_string(),
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

    #[test]
    /// 1. Group items by their group
    /// 2. Sort the items of each group, by fuzzy score desc, followed by rank asc, display asc
    /// 3. Rank each group by their highest fuzzy score item desc, followed by group name asc
    fn items_sorting() -> anyhow::Result<()> {
        let mut dropdown = Dropdown::new(DropdownConfig {
            title: "test".to_string(),
        });
        let items = [
            Item::new("test_redditor", "", "z"),
            Item::new("test_reddit", "", "c"),
            Item::new("test_editor", "", "z"),
            Item::new("patrick", "", "e"),
            Item::new("crab", "", "c"),
            Item::new("patrick", "", "d"),
        ];
        dropdown.set_items(items.clone().into_iter().map(|s| s.into()).collect());
        dropdown.set_filter("test edit");

        assert_eq!(
            dropdown
                .filtered_items
                .clone()
                .into_iter()
                .map(|item| item.display)
                .collect_vec(),
            &[
                "test_editor",
                "test_redditor",
                // "test_reddit" is ranked lower than "test_redditor" although it's fuzzy score is higher,
                // because "test_reddit" score is lowest that the fuzzy score of the highest fuzzy score of "test_redditor"'s group
                "test_reddit"
            ]
        );

        dropdown.set_filter("");

        // When fuzzy rank is the same across all items, sort by their group name, then by their display
        let expected = items
            .into_iter()
            .map(|item| -> DropdownItem { item.into() })
            .sorted_by_key(|item| (item.group.clone(), item.display.clone()))
            .collect_vec();
        assert_eq!(dropdown.filtered_items, expected);
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
impl DropdownRender {
    #[cfg(test)]
    pub(crate) fn current_line(&self) -> String {
        self.content.lines().collect_vec()[self.highlight_line_index].to_string()
    }
}
