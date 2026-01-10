use crate::quickfix_list::Location;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GotoDefinitionResponse {
    Single(Location),
    Multiple(Vec<Location>),
}

impl TryFrom<lsp_types::GotoDefinitionResponse> for GotoDefinitionResponse {
    type Error = anyhow::Error;
    fn try_from(value: lsp_types::GotoDefinitionResponse) -> Result<Self, Self::Error> {
        let locations = match value {
            lsp_types::GotoDefinitionResponse::Scalar(location) => vec![location],
            lsp_types::GotoDefinitionResponse::Array(locations) => locations,
            lsp_types::GotoDefinitionResponse::Link(links) => links
                .into_iter()
                .map(|link| lsp_types::Location {
                    uri: link.target_uri,
                    range: link.target_range,
                })
                .collect(),
        };
        match locations.as_slice() {
            [single] => Ok(GotoDefinitionResponse::Single(single.clone().try_into()?)),
            multiple => Ok(GotoDefinitionResponse::Multiple(
                multiple
                    .iter()
                    .cloned()
                    .map(|location| location.try_into())
                    .collect::<Result<Vec<_>, _>>()?,
            )),
        }
    }
}
