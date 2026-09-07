mod baseline;
mod changed;
mod complexity;
mod config;
mod doc_size;
mod escapes;
mod files;
mod gate;
mod guard;

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
    /// Fail on a function over the cyclomatic or length ceiling that the baseline does not hold
    Complexity(complexity::Args),
    /// Fail when a document has grown past its ceiling
    DocSize(doc_size::Args),
    /// Fail on a new escape site — a place where the code opts out of a check
    Escapes(escapes::Args),
    /// Run every gate the configuration names, in ladder order
    Gate(gate::Args),
    /// Refuse an agent's tool call that would edit the configuration, a baseline or the hooks
    Guard,
}

fn main() -> ExitCode {
    match &Cli::parse().command {
        Command::Guard => ExitCode::from(guard::run()),
        Command::Complexity(args) => report(|start, out| complexity::run(args, start, out)),
        Command::DocSize(args) => report(|start, out| doc_size::run(args, start, out)),
        Command::Escapes(args) => report(|start, out| escapes::run(args, start, out)),
        Command::Gate(args) => report(|start, out| gate::run(args, start, out)),
    }
}

fn report(run: impl FnOnce(&Path, &mut String) -> Result<u8, config::Error>) -> ExitCode {
    let start = match std::env::current_dir() {
        Ok(directory) => directory,
        Err(why) => {
            eprintln!("FAIL: the working directory could not be read: {why}");
            return ExitCode::from(2);
        }
    };
    let mut out = String::new();
    let outcome = run(&start, &mut out);
    print!("{out}");
    match outcome {
        Ok(code) => ExitCode::from(code),
        Err(problem) => {
            eprintln!("FAIL: {problem}");
            ExitCode::from(2)
        }
    }
}
