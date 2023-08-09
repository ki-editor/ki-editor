use crate::quickfix_list::Location;
use lsp_types::{DocumentSymbolResponse, SymbolKind};

#[derive(Debug)]
pub struct Symbols {
    pub symbols: Vec<Symbol>,
}

impl TryFrom<DocumentSymbolResponse> for Symbols {
    type Error = anyhow::Error;

    fn try_from(value: DocumentSymbolResponse) -> Result<Self, Self::Error> {
        match value {
            DocumentSymbolResponse::Flat(symbols) => {
                let symbols = symbols
                    .into_iter()
                    .map(|symbol| symbol.try_into())
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Self { symbols })
            }
            DocumentSymbolResponse::Nested(nested) => {
                todo!()
            }
        }
    }
}

impl TryFrom<lsp_types::SymbolInformation> for Symbol {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::SymbolInformation) -> Result<Self, Self::Error> {
        let name = value.name;
        let location = value.location.try_into()?;
        Ok(Self {
            name,
            kind: value.kind,
            location,
        })
    }
}

#[derive(Debug)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
}
