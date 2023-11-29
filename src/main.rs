use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use gitsync::Repository;

#[derive(Debug, Parser)]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// does testing things
    Init(InitArgs),
}

#[derive(Debug, Args)]
struct InitArgs {
    path: PathBuf,
}

fn main() {
    let args = Arguments::parse();

    match args.command {
        Command::Init(args) => init(args),
    }
}

fn init(args: InitArgs) {
    Repository::create_at(args.path).unwrap();
}
