use crate::{quickfix_list::Location, selection_mode::NamingConventionAgnostic};

use super::WalkBuilderConfig;

pub(crate) fn run(
    pattern: String,
    walk_builder_config: WalkBuilderConfig,
) -> anyhow::Result<Vec<Location>> {
    walk_builder_config.run_with_search(
        false,
        Box::new(move |buffer| {
            let pattern = pattern.clone();
            Ok(NamingConventionAgnostic::new(pattern)
                .find_all(&buffer.content())
                .into_iter()
                .map(|(range, _)| range)
                .collect())
        }),
    )
}
