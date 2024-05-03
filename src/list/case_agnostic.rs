use crate::{quickfix_list::Location, selection_mode::CaseAgnostic};

use super::WalkBuilderConfig;

pub fn run(
    pattern: String,
    walk_builder_config: WalkBuilderConfig,
) -> anyhow::Result<Vec<Location>> {
    walk_builder_config.run_with_search(Box::new(move |buffer| {
        let pattern = pattern.clone();
        Ok(CaseAgnostic::new(pattern)
            .find_all(&buffer.content())
            .into_iter()
            .map(|(range, _)| range)
            .collect())
    }))
}
