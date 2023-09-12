use clap::{Args, Parser, Subcommand};

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
                    Grammar::Build => shared::grammar::build_grammars(),
                    Grammar::Fetch => shared::grammar::fetch_grammars(),
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
