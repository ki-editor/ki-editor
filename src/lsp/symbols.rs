use crate::{
    app::{Dispatch, Dispatches},
    components::dropdown::DropdownItem,
    quickfix_list::Location,
};
use lsp_types::{DocumentSymbolResponse, SymbolKind};
use shared::icons::get_icon_config;

#[derive(Debug, Clone, PartialEq, Eq)]
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
            DocumentSymbolResponse::Nested(_nested) => {
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
            container_name: value.container_name,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub container_name: Option<String>,
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

impl From<Symbol> for DropdownItem {
    fn from(symbol: Symbol) -> Self {
        let dispatches = Dispatches::one(Dispatch::GotoLocation(symbol.location.to_owned()));
        Self {
            dispatches,
            display: symbol.display(),
            group: Some(symbol.container_name.unwrap_or("[TOP LEVEL]".to_string())),
            info: None,
            rank: None,
        }
    }
}
