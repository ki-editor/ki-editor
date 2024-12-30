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
    println!("  layout_name: LAYOUT_split_3x5_3");
    println!("layers:");

    print_keymap("Normal", KEYMAP_NORMAL)?;
    print_keymap("Shifted", KEYMAP_NORMAL_SHIFTED)?;
    print_keymap("Control", KEYMAP_CONTROL)?;

    println!("draw_config:");
    println!("  footer_text: Keymap for the <a href=\"https://ki-editor.github.io/ki-editor/\">Ki editor</a>");

    Ok(())
}

fn print_keymap(name: &str, keymap: [[Meaning; 10]; 3]) -> anyhow::Result<()> {
    println!("  {}:", name);
    for row in keymap.iter() {
        let row_strings: Vec<&str> = row
            .iter()
            .map(|meaning| meaning_to_string(meaning))
            .collect();

        println!("    - [\"{}\"]", row_strings.join("\", \""));
    }

    println!("    - [\"\", \"\", \"\", \"\", \"\", \"\"]");

    Ok(())
}

fn meaning_to_string(meaning: &Meaning) -> &'static str {
    match meaning {
        Meaning::Break => "break",
        Meaning::BuffN => "buff >",
        Meaning::BuffP => "buff <",
        Meaning::CSrch => "cfg search",
        Meaning::CharN => ">",
        Meaning::CharP => "<",
        Meaning::Char_ => "char",
        Meaning::ChngX => "change cut",
        Meaning::Chng_ => "change",
        Meaning::Copy_ => "copy",
        Meaning::CrsrN => "curs >",
        Meaning::CrsrP => "curs <",
        Meaning::DTknN => "del token >",
        Meaning::DTknP => "del token <",
        Meaning::DWrdN => "del word >",
        Meaning::DWrdP => "del word <",
        Meaning::DeDnt => "dedent",
        Meaning::DeltN => "del >",
        Meaning::DeltP => "del <",
        Meaning::Down_ => "v",
        Meaning::Exchg => "exchng",
        Meaning::FileN => "file >",
        Meaning::FileP => "file <",
        Meaning::FindN => "find >",
        Meaning::FindP => "find <",
        Meaning::First => "first",
        Meaning::GBack => "<|",
        Meaning::GForw => "|>",
        Meaning::Globl => "glb find",
        Meaning::Indnt => "indent",
        Meaning::InstN => "insert >",
        Meaning::InstP => "insert <",
        Meaning::Join_ => "join",
        Meaning::Jump_ => "jump",
        Meaning::KilLN => "kill line >",
        Meaning::KilLP => "kill line <",
        Meaning::Last_ => "last",
        Meaning::Left_ => "<",
        Meaning::LineF => "line full",
        Meaning::LineN => "end",
        Meaning::LineP => "home",
        Meaning::Line_ => "line",
        Meaning::LstNc => "last non contig",
        Meaning::Mark_ => "mark",
        Meaning::MultC => "multi cursor",
        Meaning::Next_ => "next",
        Meaning::OpenN => "open >",
        Meaning::OpenP => "open <",
        Meaning::PRplc => "rplace pat",
        Meaning::Prev_ => "prev",
        Meaning::PsteN => "paste >",
        Meaning::PsteP => "paste <",
        Meaning::Raise => "raise",
        Meaning::Redo_ => "redo",
        Meaning::Right => ">",
        Meaning::RplcN => "rplace >",
        Meaning::RplcP => "rplace prev",
        Meaning::RplcX => "rplace xut",
        Meaning::Rplc_ => "rplace",
        Meaning::SView => "switch view",
        Meaning::ScrlD => "scroll v",
        Meaning::ScrlU => "scroll ^",
        Meaning::SrchC => "search cur",
        Meaning::SrchN => "srch >",
        Meaning::SrchP => "srch <",
        Meaning::StyxF => "fine syntax",
        Meaning::Sytx_ => "syntax",
        Meaning::ToIdx => "to idx",
        Meaning::Token => "token",
        Meaning::Trsfm => "trnsfrm",
        Meaning::UPstE => "paste >",
        Meaning::Undo_ => "undo",
        Meaning::Up___ => "^",
        Meaning::VMode => "visual mode",
        Meaning::WClse => "close window",
        Meaning::WSwth => "switch window",
        Meaning::WordN => "word >",
        Meaning::WordP => "word <",
        Meaning::Word_ => "word",
        Meaning::XAchr => "xchng anchor",
        Meaning::_____ => "",
        _ => "?",
    }
}
