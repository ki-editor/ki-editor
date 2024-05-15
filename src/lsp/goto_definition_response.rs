use crate::quickfix_list::Location;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GotoDefinitionResponse {
    Single(Location),
    Multiple(Vec<Location>),
}

impl TryFrom<lsp_types::GotoDefinitionResponse> for GotoDefinitionResponse {
    type Error = anyhow::Error;
    fn try_from(value: lsp_types::GotoDefinitionResponse) -> Result<Self, Self::Error> {
        match value {
            lsp_types::GotoDefinitionResponse::Scalar(location) => {
                Ok(GotoDefinitionResponse::Single(location.try_into()?))
            }
            lsp_types::GotoDefinitionResponse::Array(locations) => match locations.split_first() {
                Some((first, [])) => Ok(GotoDefinitionResponse::Single(first.clone().try_into()?)),
                _ => Ok(GotoDefinitionResponse::Multiple(
                    locations
                        .into_iter()
                        .map(|location| location.try_into())
                        .collect::<Result<Vec<_>, _>>()?,
                )),
            },
            lsp_types::GotoDefinitionResponse::Link(_) => {
                todo!()
            }
        }
    }
}
