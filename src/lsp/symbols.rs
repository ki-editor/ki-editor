use crate::{
    app::{Dispatch, Dispatches},
    buffer::Buffer,
    components::dropdown::DropdownItem,
    quickfix_list::Location,
};
use lsp_types::{DocumentSymbolResponse, SymbolKind};
use shared::{canonicalized_path::CanonicalizedPath, icons::get_icon_config};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbols {
    pub symbols: Vec<Symbol>,
}

/// This limit is defined so that we don't send too much symbols to the main event loop,
/// which can cause lagginess.
const WORKSPACE_SYMBOLS_LIMIT: usize = 100;

impl Symbols {
    fn collect_document_symbols(
        document_symbol: &lsp_types::DocumentSymbol,
        parent_name: Option<String>,
        path: &CanonicalizedPath,
    ) -> Result<Vec<Symbol>, anyhow::Error> {
        let root_symbol = Symbol::try_from_document_symbol(
            document_symbol.clone(),
            parent_name.clone(),
            path.clone(),
        )?;

        let symbols = document_symbol
            .children
            .iter()
            .flatten()
            .flat_map(|child| {
                let parent_name = format!(
                    "{}{}",
                    parent_name
                        .as_ref()
                        .map(|name| format!("{name} ▶ ",))
                        .unwrap_or_default(),
                    document_symbol.name.clone()
                );
                Self::collect_document_symbols(child, Some(parent_name), path).unwrap_or_default()
            })
            .chain(std::iter::once(root_symbol))
            .collect();

        Ok(symbols)
    }

    pub fn try_from_document_symbol_response(
        value: DocumentSymbolResponse,
        path: CanonicalizedPath,
    ) -> anyhow::Result<Self> {
        let symbols = match value {
            DocumentSymbolResponse::Flat(flat_symbols) => flat_symbols
                .into_iter()
                .map(|symbol| symbol.try_into())
                .collect::<Result<Vec<_>, _>>()?,
            DocumentSymbolResponse::Nested(nested_symbols) => nested_symbols
                .into_iter()
                .map(|symbol| Self::collect_document_symbols(&symbol, None, &path))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect(),
        };

        Ok(Self { symbols })
    }

    pub fn try_from_workspace_symbol_response(
        workspace_symbol_response: lsp_types::WorkspaceSymbolResponse,
        working_directory: &CanonicalizedPath,
    ) -> anyhow::Result<Self> {
        match workspace_symbol_response {
            lsp_types::WorkspaceSymbolResponse::Flat(symbol_informations) => Ok(Self {
                symbols: symbol_informations
                    .into_iter()
                    .filter_map(|symbol_information| {
                        let location: Location = symbol_information.location.try_into().ok()?;
                        Some(Symbol {
                            name: symbol_information.name,
                            kind: symbol_information.kind,
                            container_name: Some(
                                symbol_information
                                    .container_name
                                    .map(|container_name| {
                                        format!(
                                            "{} [{}]",
                                            location.path.try_display_relative(),
                                            container_name
                                        )
                                    })
                                    .unwrap_or_else(|| {
                                        location.path.try_display_relative_to(working_directory)
                                    }),
                            ),
                            location,
                        })
                    })
                    .take(WORKSPACE_SYMBOLS_LIMIT)
                    .collect(),
            }),
            lsp_types::WorkspaceSymbolResponse::Nested(workspace_symbols) => Ok(Self {
                symbols: workspace_symbols
                    .into_iter()
                    .filter_map(|workspace_symbol| {
                        Some(Symbol {
                            name: workspace_symbol.name,
                            kind: workspace_symbol.kind,
                            location: match workspace_symbol.location {
                                lsp_types::OneOf::Left(location) => location.try_into().ok(),
                                lsp_types::OneOf::Right(workspace_location) => {
                                    log::info!(
                                        "[Symbols] Workspace location is not handled: {} ",
                                        workspace_location.uri
                                    );
                                    None
                                }
                            }?,
                            container_name: workspace_symbol.container_name,
                        })
                    })
                    .take(WORKSPACE_SYMBOLS_LIMIT)
                    .collect(),
            }),
        }
    }
}

impl TryFrom<lsp_types::SymbolInformation> for Symbol {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::SymbolInformation) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            kind: value.kind,
            location: value.location.try_into()?,
            container_name: value.container_name,
        })
    }
}

impl Symbol {
    fn try_from_document_symbol(
        value: lsp_types::DocumentSymbol,
        container_name: Option<String>,
        path: CanonicalizedPath,
    ) -> anyhow::Result<Self> {
        let buffer = Buffer::from_path(&path, false)?;

        let start_position = value.range.start.into();
        let end_position = value.range.end.into();
        let range = buffer.position_range_to_char_index_range(&(start_position..end_position))?;
        Ok(Self {
            name: value.name,
            kind: value.kind,
            location: Location { path, range },
            container_name,
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
    pub fn display(&self) -> String {
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
            .set_rank(Some(Box::new([symbol.location.range.start.0])))
            .set_dispatches(dispatches)
    }
}
