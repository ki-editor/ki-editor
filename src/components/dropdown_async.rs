use std::{sync::Arc, time::Duration};

use itertools::Itertools;

use nucleo::pattern::{CaseMatching, Normalization};

use crate::thread::Callback;

use super::dropdown_sync::{DropdownItem, DropdownRender, DropdownSync, DropdownSyncConfig};

pub struct DropdownAsyncConfig {
    pub title: String,
    pub on_nucleo_notify: Callback<()>,
}

pub struct DropdownAsync {
    pub inner: DropdownSync,
    matcher: BackgroundFuzzyMatcher,
}

pub const MIN_ITEMS_TO_TEST_NUCLEO: usize = 50;

impl DropdownAsync {
    pub fn new(config: DropdownAsyncConfig) -> Self {
        Self {
            inner: DropdownSync::new(DropdownSyncConfig {
                title: config.title,
            }),
            matcher: BackgroundFuzzyMatcher::new(config.on_nucleo_notify),
        }
    }

    pub fn set_filter(&mut self, filter: &str) {
        self.inner.set_filter(filter);
        self.matcher.reparse(filter);
    }

    pub fn handle_nucleo_notify(&mut self) -> DropdownRender {
        let Some(items) = self.matcher.handle_nucleo_notify() else {
            return self.inner.render();
        };

        self.inner.set_items(items);

        self.inner.render()
    }

    pub fn inject_items(&mut self, items: Vec<DropdownItem>) {
        // In tests, skip nucleo for small item sets and filter synchronously instead.
        // This avoids spawning background threads unnecessarily, which can
        // exhaust OS thread limits when many dropdowns are created (e.g. during recipe generation).
        // We don't disable nucleo entirely in tests because we still want to test
        // the background matching behaviour (see tests that explicitly pass >= MIN_ITEMS_TO_TEST_NUCLEO items).
        #[cfg(test)]
        if items.len() < MIN_ITEMS_TO_TEST_NUCLEO {
            self.inner.set_items(items);
            return;
        }

        self.matcher.clear();

        let injector = self.injector();
        for item in items {
            injector.call(item);
        }
    }

    pub fn injector(&mut self) -> Callback<DropdownItem> {
        let injector = self.matcher.injector();
        Callback::new(Arc::new(move |item| {
            injector.push(item, |item, columns| {
                let group = item.group().clone().unwrap_or_default();
                let display = item.display().clone();
                columns[0] = format!("{group} {display}").into();
            });
        }))
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.matcher.clear();
    }

    pub fn current_item(&self) -> Option<DropdownItem> {
        self.inner.current_item()
    }

    pub fn opened(&self) -> bool {
        !self.inner.items().is_empty()
    }

    pub fn no_matching_candidates(&self) -> bool {
        self.inner.no_matching_candidates()
    }

    pub fn render(&self) -> DropdownRender {
        self.inner.render()
    }

    pub fn update_current_item(&mut self, item: DropdownItem) {
        self.inner.update_current_item(item);
    }

    pub fn previous_item(&mut self) {
        self.inner.previous_item();
    }

    pub fn next_item(&mut self) {
        self.inner.next_item();
    }
}

struct BackgroundFuzzyMatcher {
    /// `nucleo` is lazily initialized because each construction spawns background threads,
    /// which can exhaust system thread limits and cause `generate_recipes.rs` to fail during `just doc-assets`.
    /// It is only constructed when `injector()` is first called.
    nucleo: Option<nucleo::Nucleo<DropdownItem>>,
    notify: Callback<()>,
}

impl BackgroundFuzzyMatcher {
    fn new(notify: Callback<()>) -> Self {
        Self {
            nucleo: None,
            notify,
        }
    }

    fn reparse(&mut self, filter: &str) {
        let Some(nucleo) = self.nucleo.as_mut() else {
            return;
        };

        nucleo.pattern.reparse(
            0,
            filter,
            CaseMatching::default(),
            Normalization::default(),
            false,
        );
    }

    /// Returns `None` if Nucleo is not initialized
    fn handle_nucleo_notify(&mut self) -> Option<Vec<DropdownItem>> {
        let nucleo = self.nucleo.as_mut()?;

        nucleo.tick(10);
        let snapshot = nucleo.snapshot();

        let scroll_offset = 0;
        let items = snapshot
            .matched_items(
                scroll_offset as u32
                    ..MIN_ITEMS_TO_TEST_NUCLEO.min(snapshot.matched_item_count() as usize) as u32,
            )
            .map(|item| item.data.clone())
            .collect_vec();

        Some(items)
    }

    fn injector(&mut self) -> nucleo::Injector<DropdownItem> {
        // We only initialize nucleo when this method (`injector`) is called,
        // so that we can avoid unnecessary thread allocations.
        // Read more at the docs of `BackgroundFuzzyMatcher::nucleo`
        let nucleo = self.nucleo.get_or_insert_with(|| {
            let debounced = crate::thread::debounce(
                self.notify.clone(),
                Duration::from_millis(1000 / 30), // 30 Hz
            );
            nucleo::Nucleo::new(
                nucleo::Config::DEFAULT,
                Arc::new(move || debounced.call(())),
                None,
                1,
            )
        });

        nucleo.injector()
    }

    fn clear(&mut self) {
        let Some(nucleo) = self.nucleo.as_mut() else {
            return;
        };

        nucleo.restart(false);
    }
}

#[cfg(test)]
mod test_dropdown_async {
    use itertools::Itertools as _;

    use crate::{
        components::{
            dropdown_async::{DropdownAsync, DropdownAsyncConfig, MIN_ITEMS_TO_TEST_NUCLEO},
            dropdown_sync::DropdownItem,
        },
        thread::Callback,
    };

    fn make_dropdown(title: &str) -> DropdownAsync {
        DropdownAsync::new(DropdownAsyncConfig {
            title: title.to_string(),
            on_nucleo_notify: Callback::no_op(),
        })
    }

    #[test]
    fn inject_items_uses_background_matching_when_items_exceed_threshold() {
        let mut dropdown = make_dropdown("test");

        assert!(dropdown.matcher.nucleo.is_none());

        let items = (0..MIN_ITEMS_TO_TEST_NUCLEO)
            .map(|i| DropdownItem::new(format!("item_{i}")))
            .collect_vec();

        dropdown.inject_items(items);

        assert!(dropdown.matcher.nucleo.is_some());
    }

    #[test]
    fn inject_items_uses_synchronous_matching_when_items_below_threshold() {
        let mut dropdown = make_dropdown("test");

        assert!(dropdown.matcher.nucleo.is_none());

        let items = (0..MIN_ITEMS_TO_TEST_NUCLEO - 1)
            .map(|i| DropdownItem::new(format!("item_{i}")))
            .collect_vec();

        dropdown.inject_items(items);

        assert!(dropdown.matcher.nucleo.is_none());
    }

    #[test]
    fn filter_works_when_background_matching_is_active() {
        let mut dropdown = make_dropdown("test");

        let items = (0..MIN_ITEMS_TO_TEST_NUCLEO)
            .map(|i| DropdownItem::new(format!("item_{i}")))
            .collect_vec();

        dropdown.inject_items(items);

        assert!(dropdown.matcher.nucleo.is_some());
        assert!(dropdown.inner.items.is_empty());

        dropdown.set_filter("item_1");
        dropdown.handle_nucleo_notify();

        let first_item = dropdown
            .inner
            .filtered_item_groups
            .iter()
            .flat_map(|group| &group.items)
            .next()
            .map(|item| item.item.display())
            .expect("Expected at least one item after filtering");

        assert_eq!(first_item, "item_1");
    }
}
