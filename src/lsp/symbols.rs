use crate::{
    app::{Dispatch, Dispatches},
    components::dropdown::DropdownItem,
    position::Position,
    quickfix_list::Location,
};
use lsp_types::{DocumentSymbolResponse, SymbolKind};
use shared::{canonicalized_path::CanonicalizedPath, icons::get_icon_config};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Symbols {
    pub(crate) symbols: Vec<Symbol>,
}

impl TryFrom<DocumentSymbolResponse> for Symbols {
    type Error = anyhow::Error;

    fn try_from(value: DocumentSymbolResponse) -> Result<Self, Self::Error> {
        let symbols = match value {
            DocumentSymbolResponse::Flat(flat_symbols) => flat_symbols
                .into_iter()
                .map(|symbol| symbol.try_into())
                .collect::<Result<Vec<_>, _>>()?,
            DocumentSymbolResponse::Nested(nested_symbols) => nested_symbols
                .into_iter()
                .map(|symbol| symbol.try_into())
                .collect::<Result<Vec<_>, _>>()?,
        };
        Ok(Self { symbols })
    }
}

impl TryFrom<lsp_types::SymbolInformation> for Symbol {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::SymbolInformation) -> Result<Self, Self::Error> {
        let name = value.name;
        let location: Location = value.location.try_into()?;
        Ok(Self {
            name,
            kind: value.kind,
            file_path: Some(location.path),
            range: location.range,
            container_name: value.container_name,
        })
    }
}

impl TryFrom<lsp_types::DocumentSymbol> for Symbol {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::DocumentSymbol) -> Result<Self, Self::Error> {
        let name = value.name;
        let start_position = value.range.start.try_into()?;
        let end_position = value.range.end.try_into()?;
        Ok(Self {
            name,
            kind: value.kind,
            file_path: None,
            range: start_position..end_position,
            container_name: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Symbol {
    pub(crate) name: String,
    pub(crate) kind: SymbolKind,
    pub(crate) file_path: Option<CanonicalizedPath>,
    pub(crate) range: Range<Position>,
    pub(crate) container_name: Option<String>,
}
impl Symbol {
    pub(crate) fn display(&self) -> String {
        let icon = get_icon_config()
            .completion
            .get(&format!("{:?}", self.kind))
            .cloned()
            .unwrap_or_default();
        format!("{} {}", icon, self.name)
    }
}

impl From<Symbol> for Dispatches {
    fn from(symbol: Symbol) -> Self {
        let range = symbol.range.clone();

        match symbol.file_path.clone() {
            Some(file_path) => {
                let location = Location {
                    path: file_path,
                    range,
                };
                Dispatches::one(Dispatch::GotoLocation(location.to_owned()))
            }
            None => Dispatches::one(Dispatch::GoToCurrentComponentRange(range)),
        }
    }
}

impl From<Symbol> for DropdownItem {
    fn from(symbol: Symbol) -> Self {
        DropdownItem::new(symbol.display())
            .set_group(Some(
                symbol
                    .container_name
                    .clone()
                    .unwrap_or("[TOP LEVEL]".to_string()),
            ))
            .set_dispatches(symbol.into())
    }
}
