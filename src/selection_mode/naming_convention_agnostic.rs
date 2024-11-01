use itertools::Itertools;

use super::{ByteRange, SelectionMode};

pub(crate) struct NamingConventionAgnostic {
    pattern: String,
}

impl NamingConventionAgnostic {
    pub(crate) fn replace(input: &str, _: &str, replace_pattern: &str) -> anyhow::Result<String> {
        let case = Self::cases()
            .into_iter()
            .find(|case| convert_case::Casing::is_case(&input, *case))
            .ok_or(anyhow::anyhow!(
                "Unable to determing the casing of {:?}",
                input
            ))?;

        Ok(convert_case::Casing::to_case(&replace_pattern, case))
    }
    pub(crate) fn new(pattern: String) -> Self {
        Self { pattern }
    }
    fn cases() -> Vec<convert_case::Case> {
        use convert_case::Case::*;
        [
            Pascal, Camel, Kebab, Snake, Title, Upper, Lower, Flat, UpperKebab, UpperSnake, Train,
        ]
        .to_vec()
    }
    fn possible_patterns(&self) -> Vec<String> {
        Self::cases()
            .into_iter()
            .map(|case| convert_case::Casing::to_case(&self.pattern, case))
            .collect()
    }

    pub(crate) fn find_all(&self, haystack: &str) -> Vec<(ByteRange, String)> {
        self.possible_patterns()
            .into_iter()
            .flat_map(move |pattern| {
                haystack
                    .match_indices(&pattern)
                    .map(|(start_index, str)| {
                        (
                            ByteRange::new(start_index..start_index + str.len()),
                            str.to_string(),
                        )
                    })
                    .collect_vec()
            })
            .collect()
    }

    pub(crate) fn replace_all(&self, haystack: &str, replace_pattern: String) -> String {
        self.find_all(haystack)
            .into_iter()
            .filter_map(move |(_, str)| {
                let replacement = Self::replace(&str, &self.pattern, &replace_pattern).ok()?;
                Some((str, replacement))
            })
            .fold(haystack.to_string(), |result, (str, replacement)| {
                result.replace(&str, &replacement)
            })
    }
}

impl SelectionMode for NamingConventionAgnostic {
    fn iter<'a>(
        &'a self,
        params: super::SelectionModeParams<'a>,
    ) -> anyhow::Result<Box<dyn Iterator<Item = super::ByteRange> + 'a>> {
        let string = params.buffer.rope().to_string();
        Ok(Box::new(
            self.find_all(&string).into_iter().map(|(range, _)| range),
        ))
    }
}

#[cfg(test)]
mod test_naming_convention_agnostic {
    use crate::{buffer::Buffer, selection::Selection};

    use super::*;

    #[test]
    fn case_1() {
        let buffer = Buffer::new(
            None,
            "AliBu aliBu ali-bu ali_bu Ali Bu ALI BU ali bu ALI-BU ALI_BU Ali-Bu",
        );
        let selection_mode = NamingConventionAgnostic::new("ali bu".to_string());
        selection_mode.assert_all_selections(
            &buffer,
            Selection::default(),
            &[
                (0..5, "AliBu"),
                (6..11, "aliBu"),
                (12..18, "ali-bu"),
                (19..25, "ali_bu"),
                (26..32, "Ali Bu"),
                (33..39, "ALI BU"),
                (40..46, "ali bu"),
                (47..53, "ALI-BU"),
                (54..60, "ALI_BU"),
                (61..67, "Ali-Bu"),
            ],
        );
        let replaced = selection_mode.replace_all(&buffer.content(), "cha dako".to_string());
        assert_eq!(replaced, "ChaDako chaDako cha-dako cha_dako Cha Dako CHA DAKO cha dako CHA-DAKO CHA_DAKO Cha-Dako")
    }
}
