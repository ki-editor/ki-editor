use clap::{Args, Parser, Subcommand};
use grammar::grammar::GrammarConfiguration;

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
    Edit(EditArgs),
    New,
}
#[derive(Args)]
struct EditArgs {
    path: String,
}
#[derive(Subcommand)]
enum Grammar {
    Build,
    Fetch,
}

pub fn cli() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::Grammar { command } => {
                match command {
                    Grammar::Build => build_grammars(),
                    Grammar::Fetch => fetch_grammars(),
                };
                Ok(())
            }
            Commands::Edit(args) => crate::run(Some(args.path.try_into()?)),
            Commands::New => todo!(),
        }
    } else {
        crate::run(None)
    }
}

fn build_grammars() {
    grammar::grammar::build_grammars(None, grammar_configs()).unwrap();
}

fn grammar_configs() -> Vec<GrammarConfiguration> {
    crate::language::LANGUAGES
        .iter()
        .flat_map(|language| language.tree_sitter_grammar_config())
        .collect()
}

fn fetch_grammars() {
    grammar::grammar::fetch_grammars(grammar_configs()).unwrap();
}
