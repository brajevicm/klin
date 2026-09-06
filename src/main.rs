mod config;
mod doc_size;
mod escapes;
mod guard;
mod ratchet;

use std::path::Path;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "detent", about = "A quality ratchet for AI-driven development")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Fail when a document has grown past its ceiling
    DocSize(doc_size::Args),
    /// Fail on a new escape site — a place where the code opts out of a check
    Escapes(escapes::Args),
    /// Refuse an agent's tool call that would edit the configuration, a baseline or the hooks
    Guard,
}

fn main() -> ExitCode {
    match &Cli::parse().command {
        Command::Guard => ExitCode::from(guard::run()),
        Command::DocSize(args) => gate(|start| doc_size::run(args, start)),
        Command::Escapes(args) => gate(|start| escapes::run(args, start)),
    }
}

fn gate(run: impl FnOnce(&Path) -> Result<u8, config::Error>) -> ExitCode {
    let start = match std::env::current_dir() {
        Ok(directory) => directory,
        Err(why) => {
            eprintln!("FAIL: the working directory could not be read: {why}");
            return ExitCode::from(2);
        }
    };
    match run(&start) {
        Ok(code) => ExitCode::from(code),
        Err(problem) => {
            eprintln!("FAIL: {problem}");
            ExitCode::from(2)
        }
    }
}
