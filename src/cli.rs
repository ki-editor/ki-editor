use crate::components::editor_keymap_printer;
use chrono::Local;
use clap::{Args, Parser, Subcommand};
use shared::canonicalized_path::CanonicalizedPath;
use std::fs::File;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

/// A combinatorial text editor.
///
/// STDIN HANDLING:
/// When input is piped through stdin (e.g., `echo "hello" | ki` or `cat file.txt | ki`),
/// the content will be automatically saved to a timestamp-based file (YYYY-MM-DD-HH-MM-SS.txt)
/// in the current working directory and opened in the editor.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<CommandPlaceholder>,

    #[command(flatten)]
    edit: EditArgs,
}

#[derive(Subcommand)]
enum CommandPlaceholder {
    #[clap(name = "@")]
    /// Run commands
    At {
        #[command(subcommand)]
        command: Commands,
    },
    /// Edit the file of the given path, creates a new file at the path
    /// if not exist
    #[command(hide = true)]
    Edit(EditArgs),
}

#[derive(Subcommand)]
enum Commands {
    /// Build and fetch tree-sitter grammar files
    Grammar {
        #[command(subcommand)]
        command: Grammar,
    },
    /// Manage cached tree-sitter highlight files
    HighlightQuery {
        #[command(subcommand)]
        command: HighlightQuery,
    },
    /// Prints the log file path
    Log,
    /// Display the keymap in various formats
    Keymap {
        #[command(subcommand)]
        command: KeymapFormat,
    },
    /// Run Ki in the given path, treating the path as the working directory
    In(InArgs),
}

#[derive(Args, Default, Clone)]
struct EditArgs {
    /// Path to file to edit. If not provided and stdin is not connected to a terminal,
    /// content will be read from stdin and saved to a timestamp-based file
    path: Option<String>,
}

#[derive(Args)]
struct InArgs {
    path: String,
}

#[derive(Subcommand)]
enum Grammar {
    /// Build existing tree-sitter grammar files
    Build,
    /// Fetch new tree-sitter grammar files
    Fetch,
}

#[derive(Subcommand)]
enum HighlightQuery {
    /// Remove all donwloaded Tree-sitter highlight queries
    Clean,
    /// Prints the cache path
    CachePath,
}

#[derive(Subcommand)]
enum KeymapFormat {
    /// Display as YAML for use with Keymap Drawer
    KeymapDrawer,
    /// Display as an ASCII table
    Table,
}

fn create_timestamp_file() -> anyhow::Result<(PathBuf, File)> {
    let timestamp = Local::now().format("%Y-%m-%d-%H-%M-%S").to_string();
    let path = PathBuf::from(format!("{}.txt", timestamp));
    let file = File::create(&path)?;
    Ok((path, file))
}

fn read_stdin() -> anyhow::Result<PathBuf> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    let (path, mut file) = create_timestamp_file()?;
    std::io::Write::write_all(&mut file, buffer.as_bytes())?;
    Ok(path)
}

fn run_edit_command(args: EditArgs) -> anyhow::Result<()> {
    match args.path {
        Some(path) => {
            let tmp_path = std::path::PathBuf::from(path.clone());
            if !tmp_path.exists() {
                std::fs::write(tmp_path, "")?;
            }

            let path: Option<CanonicalizedPath> = Some(path.try_into()?);
            let working_directory = match path.clone() {
                Some(value) if value.is_dir() => Some(value),
                Some(value) => value.parent()?,
                _ => Default::default(),
            };

            crate::run(crate::RunConfig {
                entry_path: path,
                working_directory,
            })
        }
        None => {
            // If no path is provided and stdin is not a terminal, read from stdin
            if !io::stdin().is_terminal() {
                let path = read_stdin()?;
                let canonicalized_path: Option<CanonicalizedPath> =
                    Some(path.to_string_lossy().to_string().try_into()?);

                crate::run(crate::RunConfig {
                    entry_path: canonicalized_path,
                    working_directory: None,
                })
            } else {
                crate::run(Default::default())
            }
        }
    }
}

pub(crate) fn cli() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(CommandPlaceholder::Edit(args)) => run_edit_command(args),
        Some(CommandPlaceholder::At { command }) => match command {
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
            Commands::Log => {
                println!(
                    "{}",
                    CanonicalizedPath::try_from(grammar::default_log_file())?.display_absolute(),
                );
                Ok(())
            }
            Commands::Keymap { command } => {
                match command {
                    KeymapFormat::Table => editor_keymap_printer::print_keymap_table()?,
                    KeymapFormat::KeymapDrawer => {
                        editor_keymap_printer::print_keymap_drawer_yaml()?
                    }
                }

                Ok(())
            }
            Commands::In(args) => crate::run(crate::RunConfig {
                working_directory: Some(args.path.try_into()?),
                ..Default::default()
            }),
        },
        None => run_edit_command(cli.edit),
    }
}
