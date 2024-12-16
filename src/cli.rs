use clap::{Args, Parser, Subcommand};
use shared::canonicalized_path::CanonicalizedPath;

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
    HighlightQuery {
        #[command(subcommand)]
        command: HighlightQuery,
    },
    /// Edit the file of the given path, creates a new file at the path
    /// if not exist
    Edit(EditArgs),
    /// Prints the log file path
    Log,
    /// Run Ki in the given path, treating the path as the working directory
    In(InArgs),
}
#[derive(Args)]
struct EditArgs {
    path: String,
}
#[derive(Args)]
struct InArgs {
    path: String,
}
#[derive(Subcommand)]
enum Grammar {
    Build,
    Fetch,
}

#[derive(Subcommand)]
enum HighlightQuery {
    /// Remove all donwloaded Tree-sitter highlight queries
    Clean,
    /// Prints the cache path
    CachePath,
}

pub(crate) fn cli() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::Grammar { command } => {
                match command {
                    Grammar::Build => shared::grammar::build_grammars(),
                    Grammar::Fetch => shared::grammar::fetch_grammars(),
                };
                Ok(())
            }
            Commands::HighlightQuery { command } => {
                match command {
                    HighlightQuery::Clean => shared::ts_highlight_query::clear_cache()?,
                    HighlightQuery::CachePath => {
                        println!("{}", shared::ts_highlight_query::cache_dir().display())
                    }
                };
                Ok(())
            }
            Commands::Edit(args) => {
                let path = std::path::PathBuf::from(args.path.clone());
                if !path.exists() {
                    std::fs::write(path, "")?;
                }

                let path = Some(args.path.try_into()?);

                crate::run(crate::RunConfig {
                    entry_path: path.clone(),
                    working_directory: path,
                })
            }
            Commands::Log => {
                println!(
                    "{}",
                    CanonicalizedPath::try_from(grammar::default_log_file())?.display_absolute(),
                );
                Ok(())
            }
            Commands::In(args) => crate::run(crate::RunConfig {
                working_directory: Some(args.path.try_into()?),
                ..Default::default()
            }),
        }
    } else {
        crate::run(Default::default())
    }
}
