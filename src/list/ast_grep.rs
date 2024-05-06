use crate::{
    quickfix_list::Location,
    selection_mode::{AstGrep, ByteRange},
};

use super::WalkBuilderConfig;

pub fn run(
    pattern: String,
    walk_builder_config: WalkBuilderConfig,
) -> anyhow::Result<Vec<Location>> {
    walk_builder_config.run_with_search(
        true,
        Box::new(move |buffer| {
            let pattern = pattern.clone();
            Ok(AstGrep::new(buffer, &pattern)?
                .find_all()
                .map(|node_match| ByteRange::new(node_match.range()))
                .collect())
        }),
    )
}
