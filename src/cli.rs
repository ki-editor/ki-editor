use crate::components::editor_keymap_printer;
use crate::embed;
use crate::RunConfig;
use chrono::Local;
use clap::{Args, Parser, Subcommand};
use convert_case::Casing;
use grammar::cache_dir;
use shared::absolute_path::AbsolutePath;
use std::fs::File;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;
use strum_macros::Display;

const LOGO_ASCII_ART: &str = r#"
      ██   ██   ██
      ██   ██   ██
      ██   ██   ██
      ▀██▄████▄██▀
           ██
           ██
      ▄██▀████▀██▄
      ██   ██   ██
      ██   ██   ██
      ██   ██   ██
"#;

/// A combinatorial text editor.
///
/// STDIN HANDLING:
/// When input is piped through stdin (e.g., `echo "hello" | ki` or `cat file.txt | ki`),
/// the content will be automatically saved to a timestamp-based file (YYYY-MM-DD-HH-MM-SS.txt)
/// in the current working directory and opened in the editor.
#[derive(Parser)]
#[command(
    author,
    version,
    long_about = None,
    before_help = LOGO_ASCII_ART
)]
struct Cli {
    #[command(subcommand)]
    command: Option<CommandPlaceholder>,

    #[command(flatten)]
    edit: EditArgs,
}

#[derive(Args, Default, Clone)]
struct EmbedArgs {
    /// This is crucial for Git-related features such as Git Hunks to work properly.
    working_directory: String,
}

#[derive(Subcommand)]
enum CommandPlaceholder {
    #[clap(name = "@", before_help = LOGO_ASCII_ART)]
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
    ///
    /// Log level is controlled by the `KI_LOG` environment variable.
    /// Valid values: `off`, `error`, `warn`, `info`, `debug`, `trace`.
    /// Defaults to `info`.
    ///
    /// Example: `KI_LOG=debug ki`
    Log {
        #[command(subcommand)]
        kind: Option<LogKind>,
    },
    /// Display the keymap in various formats
    Keymap {
        #[command(subcommand)]
        command: KeymapFormat,
    },
    /// Run Ki in the given path, treating the path as the working directory
    In(InArgs),

    /// Run in embedded mode.
    /// Used for running as VS Code extension for example.
    Embed(EmbedArgs),

    Version,
}

#[derive(Subcommand, Default, Display, Clone)]
pub enum LogKind {
    #[default]
    Default,
    Lsp,
}

impl LogKind {
    pub fn as_path(&self) -> anyhow::Result<PathBuf> {
        let base: PathBuf = cache_dir().join("logs");
        if !base.exists() {
            std::fs::create_dir_all(&base)?;
        }

        Ok(base.join(self.to_string().to_case(convert_case::Case::Kebab)))
    }
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
enum KeymapFormat {
    /// Display as YAML for use with Keymap Drawer
    KeymapDrawer,
    /// Display as an ASCII table
    Table,
}

fn create_timestamp_file() -> anyhow::Result<(PathBuf, File)> {
    let timestamp = Local::now().format("%Y-%m-%d-%H-%M-%S").to_string();
    let path = PathBuf::from(format!("from-stdin-{timestamp}.txt"));
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

pub(crate) enum EditAction {
    Open(AbsolutePath),
    MissingParentDirectory { parent: PathBuf },
    Launch,
}

pub(crate) fn parse_path_arg(path: String) -> anyhow::Result<EditAction> {
    let tmp_path = PathBuf::from(&path);
    if !tmp_path.exists() {
        let parent = tmp_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        if !parent.is_dir() {
            return Ok(EditAction::MissingParentDirectory {
                parent: parent.to_path_buf(),
            });
        }
        std::fs::write(&tmp_path, "")?;
    }
    Ok(EditAction::Open(path.try_into()?))
}

fn run_edit_command(args: EditArgs) -> anyhow::Result<()> {
    let action = match args.path {
        Some(path) => parse_path_arg(path)?,
        None => {
            if !io::stdin().is_terminal() {
                let path = read_stdin()?;
                EditAction::Open(path.to_string_lossy().to_string().try_into()?)
            } else {
                EditAction::Launch
            }
        }
    };

    match action {
        EditAction::Open(path) => {
            let working_directory = if path.is_dir() {
                Some(path.clone())
            } else {
                None
            };
            crate::run(RunConfig {
                entry_path: Some(path),
                working_directory,
            })
        }
        EditAction::MissingParentDirectory { parent } => {
            eprintln!("Error: directory '{}' does not exist", parent.display());
            eprintln!("Press Enter to continue...");
            let _ = io::stdin().read_line(&mut String::new());
            crate::run(RunConfig::default())
        }
        EditAction::Launch => crate::run(RunConfig::default()),
    }
}

pub fn cli() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(CommandPlaceholder::Edit(args)) => run_edit_command(args),
        Some(CommandPlaceholder::At { command }) => match command {
            Commands::Embed(args) => {
                embed::app::run_embedded_ki(args.working_directory.try_into()?)
            }
            Commands::Grammar { command } => {
                match command {
                    Grammar::Build => build_grammars(),
                    Grammar::Fetch => fetch_grammars(),
                };
                Ok(())
            }
            Commands::Log { kind } => {
                println!(
                    "{}",
                    AbsolutePath::try_from(kind.unwrap_or_default().as_path()?)?.display_absolute(),
                );
                Ok(())
            }
            Commands::Keymap { command } => {
                match command {
                    KeymapFormat::Table => editor_keymap_printer::print_keymap_table()?,
                    KeymapFormat::KeymapDrawer => {
                        editor_keymap_printer::print_keymap_drawer_yaml()?;
                    }
                }

                Ok(())
            }
            Commands::In(args) => crate::run(crate::RunConfig {
                working_directory: Some(args.path.try_into()?),
                ..Default::default()
            }),
            Commands::Version => {
                println!("{}", get_version());
                Ok(())
            }
        },
        None => run_edit_command(cli.edit),
    }
}

pub fn get_version() -> String {
    let git_hash = env!("GIT_HASH");
    let build_time = env!("BUILD_TIME");
    format!("{git_hash} (Built on {build_time})")
}

use grammar::grammar::GrammarConfiguration;

pub fn grammar_configs() -> Vec<GrammarConfiguration> {
    crate::config::AppConfig::singleton()
        .languages()
        .values()
        .flat_map(|language| language.tree_sitter_grammar_config())
        .collect()
}
pub fn build_grammars() {
    grammar::grammar::build_grammars(None, grammar_configs()).unwrap();
}

pub fn fetch_grammars() {
    grammar::grammar::fetch_grammars(grammar_configs()).unwrap();
}
