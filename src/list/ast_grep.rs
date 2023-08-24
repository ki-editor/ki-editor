use crate::{buffer::Buffer, quickfix_list::Location, selection_mode::AstGrep};
use ignore::{WalkBuilder, WalkState};
use std::path::PathBuf;

pub fn run(pattern: &str, path: PathBuf) -> anyhow::Result<Vec<Location>> {
    let (sender, receiver) = crossbeam::channel::unbounded();

    let start_time = std::time::Instant::now();
    WalkBuilder::new(path).build_parallel().run(move || {
        let sender = sender.clone();
        let pattern = pattern.clone();

        Box::new(move |path| {
            if let Ok(result) = (|| -> anyhow::Result<Vec<Location>> {
                let path = path?;
                if !path
                    .file_type()
                    .map_or(false, |file_type| file_type.is_file())
                {
                    return Ok(Vec::new());
                }

                let path = path.path().try_into()?;
                let buffer = Buffer::from_path(&path)?;
                let ast_grep = AstGrep::new(&buffer, pattern)?;
                Ok(ast_grep
                    .find_all()
                    .flat_map(move |node_match| -> anyhow::Result<_> {
                        let range = node_match.range();
                        let range = buffer.byte_to_position(range.start)?
                            ..buffer.byte_to_position(range.end)?;

                        Ok(Location {
                            path: path.clone(),
                            range,
                        })
                    })
                    .collect::<Vec<_>>())
            })() {
                let _ = sender.send(result);
            }

            WalkState::Continue
        })
    });

    let time_taken = start_time.elapsed();
    log::info!("time_taken to search: {:?}", time_taken);
    Ok(receiver.into_iter().flatten().collect::<Vec<_>>())
}
