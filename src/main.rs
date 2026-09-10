mod base;
mod build;
mod cache;
mod ceiling;
mod changed;
mod complexity;
mod config;
mod doc_citations;
mod doc_size;
mod escapes;
mod files;
mod gate;
mod guard;
mod hooks;
mod host;
mod init;
mod radius;
mod ratchet;
mod state;
mod survey;
mod turn;

use std::path::Path;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "klin", about = "A quality ratchet for AI-driven development")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Fail on a function over the cyclomatic or length ceiling that the base does not hold
    Complexity(complexity::Args),
    /// Fail when a document cites a file that resolves nowhere under its roots
    DocCitations(doc_citations::Args),
    /// Fail when a document has grown past its ceiling
    DocSize(doc_size::Args),
    /// Fail on a new escape site — a place where the code opts out of a check
    Escapes(escapes::Args),
    /// Run every gate the configuration names, cheapest first
    Gate(gate::Args),
    /// Survey the tree and write the configuration it can say for itself
    Init(init::Args),
    /// Refuse an agent's tool call that would edit the configuration or the hooks
    Guard(guard::Args),
    /// Remove the survey cache klin keeps for this tree, or every orphaned one
    Cache(cache::Args),
    /// Move the turn stamp by its one rule, on a session start and on every prompt
    Radius(turn::Args),
    /// Move the turn stamp to the working tree, which only a person does
    Turn(turn::Moved),
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Guard(args) => ExitCode::from(guard::run(&args)),
        command => report(|start, out| ran(&command, start, out)),
    }
}

fn ran(command: &Command, start: &Path, out: &mut String) -> Result<u8, config::Error> {
    match check(command, start, out) {
        Some(outcome) => outcome,
        None => tool(command, start, out),
    }
}

/// The checks a person runs one at a time, and `None` for a command that is not one of them.
fn check(command: &Command, start: &Path, out: &mut String) -> Option<Result<u8, config::Error>> {
    Some(match command {
        Command::Complexity(args) => complexity::run(args, start, out),
        Command::DocCitations(args) => doc_citations::run(args, start, out),
        Command::DocSize(args) => doc_size::run(args, start, out),
        Command::Escapes(args) => escapes::run(args, start, out),
        Command::Gate(_)
        | Command::Init(_)
        | Command::Cache(_)
        | Command::Guard(_)
        | Command::Radius(_)
        | Command::Turn(_) => {
            return None;
        }
    })
}

/// Everything else: the runner, the survey and the cache. `main` takes the guard before this.
fn tool(command: &Command, start: &Path, out: &mut String) -> Result<u8, config::Error> {
    match command {
        Command::Gate(args) => gate::run(args, start, out),
        Command::Init(args) => init::run(args, start, out),
        Command::Cache(args) => cache::run(args, start, out),
        Command::Radius(args) => turn::run(args, start, out),
        Command::Turn(args) => turn::moved(args, start, out),
        Command::Guard(args) => Ok(guard::run(args)),
        _ => Ok(0),
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
