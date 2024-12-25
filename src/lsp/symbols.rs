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

fn collect_document_symbols(
    document_symbol: &lsp_types::DocumentSymbol,
    symbols: &mut Vec<Symbol>,
    parent_name: Option<String>,
    path: &CanonicalizedPath,
) -> Result<(), anyhow::Error> {
    let mut symbol = Symbol::try_from_document_symbol(document_symbol.clone(), path.clone())?;
    symbol.container_name = parent_name.clone(); // Set the container_name
    symbols.push(symbol);

    if let Some(children) = document_symbol.clone().children {
        for child in children {
            collect_document_symbols(&child, symbols, Some(document_symbol.name.clone()), path)?;
        }
    };

    Ok(())
}

impl Symbols {
    pub(crate) fn try_from_document_symbol_response(
        value: DocumentSymbolResponse,
        path: CanonicalizedPath,
    ) -> anyhow::Result<Self> {
        let mut symbols = Vec::new();
        match value {
            DocumentSymbolResponse::Flat(flat_symbols) => {
                for symbol in flat_symbols {
                    symbols.push(Symbol::try_from_symbol_information(symbol)?);
                }
            }
            DocumentSymbolResponse::Nested(nested_symbols) => {
                for symbol in nested_symbols {
                    collect_document_symbols(&symbol, &mut symbols, None, &path)?;
                }
            }
        }

        Ok(Self { symbols })
    }
}

impl Symbol {
    fn try_from_symbol_information(value: lsp_types::SymbolInformation) -> anyhow::Result<Self> {
        let name = value.name;
        let location = Location::try_from(value.location)?;
        Ok(Self {
            name,
            kind: value.kind,
            location,
            container_name: value.container_name,
        })
    }

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
        DropdownItem::new(symbol.display())
            .set_group(Some(
                symbol
                    .container_name
                    .clone()
                    .unwrap_or("[TOP LEVEL]".to_string()),
            ))
            .set_dispatches(Dispatches::one(Dispatch::GotoLocation(
                symbol.location.to_owned(),
            )))
    }
}
