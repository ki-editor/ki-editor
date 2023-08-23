use clap::{Parser, Subcommand};
use grammar::grammar::GrammarConfiguration;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Grammar {
        #[command(subcommand)]
        command: Grammar,
    },
}

#[derive(Subcommand)]
enum Grammar {
    Build,
    Fetch,
}

pub fn cli() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Grammar { command }) => match command {
            Grammar::Build => build_grammars(),
            Grammar::Fetch => fetch_grammars(),
        },
        None => crate::run().unwrap(),
    }
}

fn build_grammars() {
    grammar::grammar::build_grammars(None, grammar_configs()).unwrap();
}

fn grammar_configs() -> Vec<GrammarConfiguration> {
    crate::language::languages()
        .into_iter()
        .flat_map(|language| language.tree_sitter_grammar_config())
        .collect()
}

fn fetch_grammars() {
    grammar::grammar::fetch_grammars(grammar_configs()).unwrap();
}
