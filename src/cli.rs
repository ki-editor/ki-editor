use clap::{Args, Parser, Subcommand};
use shared::canonicalized_path::CanonicalizedPath;

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
        None => crate::run(Default::default()),
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
                    KeymapFormat::Table => write_keymap_table()?,
                    KeymapFormat::KeymapDrawer => write_keymap_drawer()?,
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

use crate::components::{
    editor::{Editor, Mode},
    editor_keymap::{
        shifted, Meaning, KEYMAP_CONTROL, KEYMAP_NORMAL, KEYMAP_NORMAL_SHIFTED, QWERTY,
    },
    keymap_legend::Keymaps,
};
use crate::context::Context;
use comfy_table::{Cell, CellAlignment, ColumnConstraint::Absolute, Table, Width::Fixed};
use crossterm::event::KeyCode;
use event::{KeyEvent, KeyModifiers};

fn write_keymap_table() -> anyhow::Result<()> {
    let context = Context::default();
    let mut editor = Editor::from_text(None, "");
    let normal_keymaps: Vec<Keymaps> = editor.normal_mode_keymap_legend_config(&context).into();

    print_single_keymap_table("Normal", KeyModifiers::None, &normal_keymaps);
    print_single_keymap_table("Normal Shift", KeyModifiers::Shift, &normal_keymaps);
    print_single_keymap_table("Normal Control", KeyModifiers::Ctrl, &normal_keymaps);
    print_single_keymap_table("Normal Alternate", KeyModifiers::Alt, &normal_keymaps);

    editor.mode = Mode::MultiCursor;
    let multicursor_keymaps: Vec<Keymaps> =
        editor.normal_mode_keymap_legend_config(&context).into();
    print_single_keymap_table("Multi-Cursor", KeyModifiers::None, &multicursor_keymaps);
    print_single_keymap_table(
        "Multi-Cursor Shift",
        KeyModifiers::Shift,
        &multicursor_keymaps,
    );

    let mut vmode_keymaps = normal_keymaps.clone();
    vmode_keymaps.insert(0, editor.visual_mode_initialized_keymaps());

    print_single_keymap_table("V-mode", KeyModifiers::None, &vmode_keymaps);

    print_single_keymap_table(
        "Insert",
        KeyModifiers::Alt,
        &editor.insert_mode_keymap_legend_config().into(),
    );

    Ok(())
}

fn print_single_keymap_table(name: &str, modifiers: KeyModifiers, keymaps: &Vec<Keymaps>) {
    let rows: Vec<Vec<Option<String>>> = QWERTY
        .iter()
        .map(|row| {
            row.iter()
                .map(|key| {
                    let ke = match modifiers {
                        KeyModifiers::Shift => KeyEvent {
                            code: KeyCode::Char(shifted(key).chars().next().unwrap()),
                            modifiers: KeyModifiers::Shift,
                        },
                        _ => KeyEvent {
                            code: KeyCode::Char(key.chars().next().unwrap()),
                            modifiers: modifiers.clone(),
                        },
                    };

                    match keymaps.iter().find_map(|p| p.get(&ke)) {
                        Some(keymap) => keymap.short_description.clone(),
                        None => None,
                    }
                })
                .collect()
        })
        .collect();

    if rows.iter().any(|v| v.iter().any(Option::is_some)) {
        println!("{}:", name);

        let mut table = Table::new();
        let table_rows = rows.iter().map(|row| {
            row.into_iter().map(|value| {
                let display = match value {
                    Some(value) => value.to_string(),
                    None => "".to_string(),
                };

                Cell::new(display).set_alignment(CellAlignment::Center)
            })
        });

        table
            .add_rows(table_rows)
            .set_constraints(vec![
                Absolute(Fixed(8)),
                Absolute(Fixed(8)),
                Absolute(Fixed(8)),
                Absolute(Fixed(8)),
                Absolute(Fixed(8)),
                Absolute(Fixed(8)),
                Absolute(Fixed(8)),
                Absolute(Fixed(8)),
                Absolute(Fixed(8)),
                Absolute(Fixed(8)),
            ])
            .load_preset(comfy_table::presets::UTF8_FULL)
            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);

        println!("{}", table);
        println!("");
    }
}

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
        Meaning::BuffN => "buff →",
        Meaning::BuffP => "buff ←",
        Meaning::CSrch => "cfg search",
        Meaning::CharN => "→",
        Meaning::CharP => "←",
        Meaning::Char_ => "char",
        Meaning::ChngX => "change cut",
        Meaning::Chng_ => "change",
        Meaning::Copy_ => "copy",
        Meaning::CrsrN => "curs →",
        Meaning::CrsrP => "curs ←",
        Meaning::DTknN => "del token →",
        Meaning::DTknP => "del token ←",
        Meaning::DWrdN => "del word →",
        Meaning::DWrdP => "del word ←",
        Meaning::DeDnt => "dedent",
        Meaning::DeltN => "del →",
        Meaning::DeltP => "del ←",
        Meaning::Down_ => "↓",
        Meaning::Exchg => "exchng",
        Meaning::FileN => "file →",
        Meaning::FileP => "file ←",
        Meaning::FindN => "find →",
        Meaning::FindP => "find ←",
        Meaning::First => "first",
        Meaning::GBack => "⇤",
        Meaning::GForw => "⇥",
        Meaning::Globl => "glb find",
        Meaning::Indnt => "indent",
        Meaning::InstN => "insert →",
        Meaning::InstP => "insert ←",
        Meaning::Join_ => "join",
        Meaning::Jump_ => "jump",
        Meaning::KilLN => "kill line →",
        Meaning::KilLP => "kill line ←",
        Meaning::Last_ => "last",
        Meaning::Left_ => "←",
        Meaning::LineF => "line full",
        Meaning::LineN => "end",
        Meaning::LineP => "home",
        Meaning::Line_ => "line",
        Meaning::LstNc => "last non contig",
        Meaning::Mark_ => "mark",
        Meaning::MultC => "multi cursor",
        Meaning::Next_ => "⇥",
        Meaning::OpenN => "open →",
        Meaning::OpenP => "open ←",
        Meaning::PRplc => "rplace pat",
        Meaning::Prev_ => "⇤",
        Meaning::PsteN => "paste →",
        Meaning::PsteP => "paste ←",
        Meaning::Raise => "raise",
        Meaning::Redo_ => "redo",
        Meaning::Right => "→",
        Meaning::RplcN => "rplace →",
        Meaning::RplcP => "rplace ←",
        Meaning::RplcX => "rplace cut",
        Meaning::Rplc_ => "rplace",
        Meaning::SView => "switch view",
        Meaning::ScrlD => "scroll ↓",
        Meaning::ScrlU => "scroll ↑",
        Meaning::SrchC => "search cur",
        Meaning::SrchN => "srch →",
        Meaning::SrchP => "srch ←",
        Meaning::StyxF => "fine syntax",
        Meaning::Sytx_ => "syntax",
        Meaning::ToIdx => "to index",
        Meaning::Token => "token",
        Meaning::Trsfm => "trnsfrm",
        Meaning::UPstE => "paste →",
        Meaning::Undo_ => "undo",
        Meaning::Up___ => "↑",
        Meaning::VMode => "visual mode",
        Meaning::WClse => "close window",
        Meaning::WSwth => "switch window",
        Meaning::WordN => "word →",
        Meaning::WordP => "word ←",
        Meaning::Word_ => "word",
        Meaning::XAchr => "xchng anchor",
        Meaning::_____ => "",
    }
}
