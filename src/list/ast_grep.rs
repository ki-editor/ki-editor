use crate::{buffer::Buffer, quickfix_list::Location, selection_mode::AstGrep};

use super::WalkBuilderConfig;

pub fn run(
    pattern: String,
    walk_builder_config: WalkBuilderConfig,
) -> anyhow::Result<Vec<Location>> {
    walk_builder_config.run(Box::new(move |path, sender| {
        let path = path.try_into()?;
        let buffer = Buffer::from_path(&path)?;
        let pattern = pattern.clone();
        let ast_grep = AstGrep::new(&buffer, &pattern)?;
        let _ = ast_grep
            .find_all()
            .flat_map(move |node_match| -> anyhow::Result<_> {
                let range = node_match.range();
                let range =
                    buffer.byte_to_position(range.start)?..buffer.byte_to_position(range.end)?;

                let _ = sender
                    .send(Location {
                        path: path.clone(),
                        range,
                    })
                    .map_err(|error| {
                        log::error!("sender.send {:?}", error);
                    });

                Ok(())
            })
            .collect::<Vec<_>>();
        Ok(())
    }))
}
