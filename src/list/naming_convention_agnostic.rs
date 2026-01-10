use std::sync::Arc;

use crate::{list::Match, selection_mode::NamingConventionAgnostic, thread::SendResult};

use super::WalkBuilderConfig;

pub fn run(
    pattern: String,
    walk_builder_config: WalkBuilderConfig,
    send_match: Arc<dyn Fn(Match) -> SendResult + Send + Sync>,
) -> anyhow::Result<()> {
    walk_builder_config.run_with_search(
        false,
        send_match,
        Arc::new(move |buffer| {
            let pattern = pattern.clone();
            NamingConventionAgnostic::new(pattern)
                .find_all(&buffer.content())
                .into_iter()
                .map(|(range, _)| range.range().clone())
                .collect()
        }),
    )
}
