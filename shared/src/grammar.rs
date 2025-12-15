use grammar::grammar::GrammarConfiguration;

use crate::config::AppConfig;

pub(crate) fn grammar_configs() -> Vec<GrammarConfiguration> {
    AppConfig::singleton()
        .languages()
        .iter()
        .flat_map(|(_, language)| language.tree_sitter_grammar_config())
        .collect()
}
pub fn build_grammars() {
    grammar::grammar::build_grammars(None, grammar_configs()).unwrap();
}

pub fn fetch_grammars() {
    grammar::grammar::fetch_grammars(grammar_configs()).unwrap();
}
