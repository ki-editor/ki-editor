#[derive(Debug, PartialEq, Eq)]
pub(crate) struct GetHighlightQueryResult {
    pub(crate) query: String,
}

/// Get highlight query from nvim-treesitter, possibly by using parent languages
pub(crate) fn get_highlight_query(language_id: &str) -> Option<GetHighlightQueryResult> {
    let current =
        nvim_treesitter_highlight_queries::get_nvim_treesitter_highlight_query_by_id(language_id)?;
    let parent = get_highlight_query_parents(&current)
        .into_iter()
        .map(|parent| get_highlight_query(&parent).map(|result| result.query))
        .collect::<Option<Vec<_>>>()?
        .join("\n\n");

    let result = format!("{parent}\n\n{current}");
    Some(GetHighlightQueryResult { query: result })
}

/// This function extracts the parent of a Tree-sitter highlight query parents,
/// based on the format defined by `nvim-treesitter`.
///
/// Reference:
///   - https://github.com/nvim-treesitter/nvim-treesitter/blob/8f5513a1f2ec6ee5b378c2e32e53fc3c2a8f1e13/CONTRIBUTING.md#inheriting-languages
fn get_highlight_query_parents(content: &str) -> Vec<String> {
    regex::Regex::new(r"inherits:\s*([\w,]+)")
        .unwrap()
        .captures(content)
        .and_then(|capture| capture.get(1))
        .map(|content| {
            content
                .as_str()
                .split(',')
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod test_language {
    use super::*;
    #[test]
    fn test_get_highlight_query() {
        assert!(get_highlight_query("tsx")
            .unwrap()
            .query
            .contains("\"require\" @keyword.import"));
        assert!(get_highlight_query("Not a Language").is_none());
    }

    #[test]
    fn test_get_highlight_query_parents() {
        assert_eq!(
            get_highlight_query_parents("; inherits: ecma,jsx"),
            vec!["ecma", "jsx"],
        );
        assert_eq!(
            get_highlight_query_parents("; inherits: html"),
            vec!["html"],
        )
    }
}
