use crate::{
    app::{Dispatch, Dispatches},
    components::dropdown::DropdownItem,
    quickfix_list::Location,
};
use lsp_types::{DocumentSymbolResponse, SymbolKind};
use shared::{canonicalized_path::CanonicalizedPath, icons::get_icon_config};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Symbols {
    pub(crate) symbols: Vec<Symbol>,
}

impl Symbols {
    fn collect_document_symbols(
        document_symbol: &lsp_types::DocumentSymbol,
        parent_name: Option<String>,
        path: &CanonicalizedPath,
    ) -> Result<Vec<Symbol>, anyhow::Error> {
        let mut symbols = Vec::new();
        let mut symbol = Symbol::try_from_document_symbol(document_symbol.clone(), path.clone())?;
        symbol.container_name = parent_name.clone(); // Set the container_name
        symbols.push(symbol);

        if let Some(children) = document_symbol.clone().children {
            for child in children {
                let mut child_symbols = Self::collect_document_symbols(
                    &child,
                    Some(document_symbol.name.clone()),
                    path,
                )?;
                symbols.append(&mut child_symbols);
            }
        };

        Ok(symbols)
    }

    pub(crate) fn try_from_document_symbol_response(
        value: DocumentSymbolResponse,
        path: CanonicalizedPath,
    ) -> anyhow::Result<Self> {
        match value {
            DocumentSymbolResponse::Flat(symbols) => {
                let symbols = symbols
                    .into_iter()
                    .map(|symbol| symbol.try_into())
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Self { symbols })
            }
            DocumentSymbolResponse::Nested(symbols) => {
                let mut collected_symbols = Vec::new();
                for symbol in symbols {
                    let mut child_symbols = Self::collect_document_symbols(&symbol, None, &path)?;
                    collected_symbols.append(&mut child_symbols);
                }
                Ok(Self {
                    symbols: collected_symbols,
                })
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

impl Symbol {
    fn try_from_document_symbol(
        value: lsp_types::DocumentSymbol,
        path: CanonicalizedPath,
    ) -> anyhow::Result<Self> {
        let name = value.name;
        let start_position = value.range.start.into();
        let end_position = value.range.end.into();
        Ok(Self {
            name,
            kind: value.kind,
            location: Location {
                path,
                range: start_position..end_position,
            },
            container_name: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Symbol {
    pub(crate) name: String,
    pub(crate) kind: SymbolKind,
    pub(crate) location: Location,
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

impl From<Symbol> for DropdownItem {
    fn from(symbol: Symbol) -> Self {
        let dispatches = Dispatches::one(Dispatch::GotoLocation(symbol.location.to_owned()));
        DropdownItem::new(symbol.display())
            .set_group(Some(
                symbol.container_name.unwrap_or("[TOP LEVEL]".to_string()),
            ))
            .set_dispatches(dispatches)
    }
}
