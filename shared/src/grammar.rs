use grammar::grammar::GrammarConfiguration;

pub fn grammar_configs() -> Vec<GrammarConfiguration> {
    crate::languages::LANGUAGES
        .iter()
        .flat_map(|language| language.tree_sitter_grammar_config())
        .collect()
}
pub fn build_grammars() {
    grammar::grammar::build_grammars(None, grammar_configs()).unwrap();
}

pub fn fetch_grammars() {
    grammar::grammar::fetch_grammars(grammar_configs()).unwrap();
}
