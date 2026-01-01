use std::{collections::HashMap, sync::LazyLock};

const COMPILED_QUERY_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/compiled_highlight_queries.bin"));

pub fn get_nvim_treesitter_highlight_query_by_id(language_id: &str) -> Option<String> {
    let highlight_queries = &*HIGHLIGHT_QUERIES;
    highlight_queries.get(language_id).and_then(Option::clone)
}

// For testing that this crate has the same set of languages that the rest of ki does.
// Can't be marked cfg(test) because this crate is a dependency not built in the
// test profile when the outside unit test is run.
pub fn all() -> HashMap<String, Option<String>> {
    HIGHLIGHT_QUERIES.clone()
}

// Why is the value type an Option<String>?
// Because we want to differentiate between "this language has no highlight query" and "this language doesn't exist"
// We differentiate so we can ensure the build.rs script doesn't go out of sync with our list of builtin languages we maintain elsewhere.
static HIGHLIGHT_QUERIES: LazyLock<HashMap<String, Option<String>>> = LazyLock::new(|| {
    let decompressed_queries = miniz_oxide::inflate::decompress_to_vec(COMPILED_QUERY_BYTES)
        .expect("Compiled queries can't be decompressed?");

    String::from_utf8(decompressed_queries)
        .expect("Decompressed queries aren't a valid string?")
        .split('\0')
        .map(|entry| {
            entry
                .split_once('=')
                .expect("Malformed entry in compiled highlight queries")
        })
        .map(|(lang, query)| {
            (
                lang.to_string(),
                match query {
                    "" => None,
                    _ => Some(query.to_string()),
                },
            )
        })
        .collect()
});

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_nvim_treesitter_highlight_queries_can_be_loaded() {
        assert!(!HIGHLIGHT_QUERIES.is_empty());
    }
}
