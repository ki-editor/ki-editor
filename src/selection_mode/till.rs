use itertools::Itertools;
use num::ToPrimitive;

use crate::components::editor::Direction;

use super::{ByteRange, SelectionMode};

pub(crate) struct Till {
    content: String,
    character: char,
    direction: Direction,
}

impl SelectionMode for Till {
    fn iter<'a>(
        &'a self,
        _: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = ByteRange> + 'a>> {
        let matches = self.content.match_indices(self.character);
        Ok(Box::new(matches.map(move |(byte_start, matches)| {
            let byte_start = match self.direction {
                Direction::Start => byte_start.saturating_add(1),
                Direction::End => byte_start.saturating_sub(1),
            };
            ByteRange::new(byte_start..byte_start + matches.len())
        })))
    }
}

impl Till {
    pub(crate) fn from_config(
        buffer: &crate::buffer::Buffer,
        character: char,
        direction: Direction,
    ) -> Self {
        Self {
            character,
            content: buffer.rope().to_string(),
            direction,
        }
    }
}

#[cfg(test)]
mod test_till {
    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn forward() {
        let buffer = Buffer::new(None, "fn main() { let x = m.n; }");
        crate::selection_mode::Till::from_config(&buffer, 'n', Direction::End)
            .assert_all_selections(
                &buffer,
                Selection::default(),
                &[(0..1, "f"), (5..6, "i"), (21..22, ".")],
            );
    }

    #[test]
    fn backward() {
        let buffer = Buffer::new(None, "fn main() { let x = m.n; }");
        crate::selection_mode::Till::from_config(&buffer, 'n', Direction::Start)
            .assert_all_selections(
                &buffer,
                Selection::default(),
                &[(2..3, " "), (7..8, "("), (23..24, ";")],
            );
    }
}
