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
    /// Display the keymap in various formats
    Keymap,
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

                let path: Option<CanonicalizedPath> = Some(args.path.try_into()?);
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
            Commands::Log => {
                println!(
                    "{}",
                    CanonicalizedPath::try_from(grammar::default_log_file())?.display_absolute(),
                );
                Ok(())
            }
            Commands::Keymap => {
                write_keymap_drawer()?;

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

use crate::components::editor_keymap::{
    Meaning, KEYMAP_CONTROL, KEYMAP_NORMAL, KEYMAP_NORMAL_SHIFTED,
};

fn write_keymap_drawer() -> anyhow::Result<()> {
    println!("layout:");
    println!("  qmk_keyboard: corne_rotated");
    println!("  layout_name: LAYOUT_split_3x6_3");
    println!("layers:");

    print_keymap("Normal", KEYMAP_NORMAL)?;
    print_keymap("Shifted", KEYMAP_NORMAL_SHIFTED)?;
    print_keymap("Control", KEYMAP_CONTROL)?;

    Ok(())
}

fn print_keymap(name: &str, keymap: [[Meaning; 10]; 3]) -> anyhow::Result<()> {
    println!("  {}:", name);
    for row in keymap.iter() {
        let row_strings: Vec<&str> = row
            .iter()
            .map(|meaning| meaning_to_string(meaning))
            .collect();

        println!("    - [\"\", \"{}\", \"\"]", row_strings.join("\", \""));
    }

    println!("    - [\"\", \"\", \"\", \"\", \"\", \"\"]");

    Ok(())
}
fn meaning_to_string(meaning: &Meaning) -> &'static str {
    match meaning {
        Meaning::ToIdx => "to idx",
        Meaning::Indnt => "indent",
        Meaning::UPstE => "paste >",
        Meaning::DeDnt => "dedent",
        Meaning::OpenP => "open <",
        Meaning::OpenN => "open >",
        Meaning::Join_ => "join",
        Meaning::SView => "switch view",
        Meaning::LineF => "line full",
        Meaning::BuffN => "buff >",
        Meaning::BuffP => "buff <",
        Meaning::StyxF => "fine syntax",
        Meaning::Raise => "raise",
        Meaning::FileP => "file <",
        Meaning::FileN => "file >",
        Meaning::FindP => "find <",
        Meaning::GBack => "<|",
        Meaning::GForw => "|>",
        Meaning::WClse => "close window",
        Meaning::WSwth => "switch window",
        Meaning::FindN => "find >",
        Meaning::Break => "break",
        Meaning::CrsrP => "curs <",
        Meaning::CrsrN => "curs >",
        Meaning::XAchr => "xchng anchor",
        Meaning::Undo_ => "undo",
        Meaning::PRplc => "rplace pat",
        Meaning::RplcP => "rplace prev",
        Meaning::RplcN => "rplace >",
        Meaning::ScrlU => "scroll ^",
        Meaning::ScrlD => "scroll v",
        Meaning::LstNc => "last non contig",
        Meaning::Redo_ => "redo",
        Meaning::Exchg => "exchng",
        Meaning::Copy_ => "copy",
        Meaning::PsteN => "paste >",
        Meaning::PsteP => "paste <",
        Meaning::Rplc_ => "rplace",
        Meaning::RplcX => "rplace xut",
        Meaning::Left_ => "<",
        Meaning::Right => ">",
        Meaning::DeltP => "del <",
        Meaning::DeltN => "del >",
        Meaning::SrchN => "srch >",
        Meaning::SrchP => "srch <",
        Meaning::Prev_ => "prev",
        Meaning::Next_ => "next",
        Meaning::Down_ => "v",
        Meaning::Globl => "glb find",
        Meaning::First => "first",
        Meaning::Jump_ => "jump",
        Meaning::Last_ => "last",
        Meaning::Trsfm => "trnsfrm",
        Meaning::Mark_ => "mark",
        Meaning::InstP => "keep match",
        Meaning::InstN => "remove match",
        Meaning::Up___ => "^",
        Meaning::Word_ => "word",
        Meaning::VMode => "v-mode",
        Meaning::Chng_ => "change",
        Meaning::ChngX => "change cut",
        Meaning::MultC => "multi cursor",
        Meaning::SrchC => "search cur",
        Meaning::Char_ => "char",
        Meaning::Line_ => "line",
        Meaning::Token => "token",
        Meaning::Sytx_ => "syntax",
        Meaning::CSrch => "cfg search",
        Meaning::DTknP => "del token <",
        Meaning::DWrdP => "del word <",
        Meaning::DWrdN => "del word >",
        Meaning::DTknN => "del token >",
        Meaning::WordP => "word <",
        Meaning::CharP => "<",
        Meaning::CharN => ">",
        Meaning::WordN => "word >",
        Meaning::KilLN => "kill line >",
        Meaning::LineN => "end",
        Meaning::LineP => "home",
        Meaning::KilLP => "kill line <",
        Meaning::_____ => "",
        _ => "?",
    }
}
