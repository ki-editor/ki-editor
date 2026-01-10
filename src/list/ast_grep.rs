use itertools::Itertools;

use crate::{list::Match, selection_mode::AstGrep, thread::SendResult};

use std::sync::Arc;

use super::WalkBuilderConfig;

pub fn run(
    pattern: String,
    walk_builder_config: WalkBuilderConfig,
    send_match: Arc<dyn Fn(Match) -> SendResult + Send + Sync>,
) -> anyhow::Result<()> {
    walk_builder_config.run_with_search(
        true,
        send_match,
        Arc::new(move |buffer| {
            AstGrep::new(buffer, &pattern)
                .map(|ast_grep| {
                    ast_grep
                        .find_all()
                        .map(|node_match| node_match.range())
                        .collect_vec()
                })
                .unwrap_or_default()
        }),
    )
}
