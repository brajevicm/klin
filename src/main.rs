mod base;
mod build;
mod cache;
mod ceiling;
mod changed;
mod complexity;
mod config;
mod coverage;
mod doc_citations;
mod doc_size;
mod escapes;
mod files;
mod gate;
mod guard;
mod hooks;
mod host;
mod hunks;
mod init;
mod inventory;
mod journal;
mod lockfile;
mod markers;
mod radius;
mod ratchet;
mod reference;
mod sarif;
mod state;
mod stats;
mod stubs;
mod survey;
mod syntax;
mod turn;
mod update;

use std::path::Path;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "klin",
    version,
    about = "A quality ratchet for AI-driven development"
)]
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
    Escapes(markers::Args),
    /// Fail on a new placeholder marker — a stub an agent left where the work belongs
    Stubs(markers::Args),
    /// Fail on a scanner's result that sits on a line this window changed
    Sarif(sarif::Args),
    /// Run every gate the configuration names, cheapest first
    Gate(gate::Args),
    /// Survey the tree and write the configuration it can say for itself
    Init(init::Args),
    /// Refuse an agent's tool call that would edit the configuration
    Guard(guard::Args),
    /// Remove the survey cache klin keeps for this tree, or every orphaned one
    Cache(cache::Args),
    /// Move the turn stamp by its one rule, on a session start and on every prompt
    Radius(turn::Args),
    /// Move the turn stamp to the working tree, which only a person does
    Turn(turn::Moved),
    /// Report what klin caught over the last seven days, in the person's words
    Stats(stats::Args),
    /// Print the configuration reference, as Markdown, from the keys the checks declare
    Reference,
    /// Install the newest release over this binary, through the klin-update beside it
    Update,
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Guard(args) => ExitCode::from(guard::run(&args)),
        Command::Update => ExitCode::from(update::run()),
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
        Command::Stubs(args) => stubs::run(args, start, out),
        Command::Sarif(args) => sarif::run(args, start, out),
        Command::Gate(_)
        | Command::Init(_)
        | Command::Cache(_)
        | Command::Guard(_)
        | Command::Radius(_)
        | Command::Turn(_)
        | Command::Stats(_)
        | Command::Reference
        | Command::Update => {
            return None;
        }
    })
}

/// The turn stamp's two movers and the commands that only read and print.
fn tool(command: &Command, start: &Path, out: &mut String) -> Result<u8, config::Error> {
    match command {
        Command::Radius(args) => turn::run(args, start, out),
        Command::Turn(args) => turn::moved(args, start, out),
        Command::Stats(args) => stats::run(args, start, out),
        Command::Reference => reference::run(out),
        _ => runner(command, start, out),
    }
}

/// The runner, the survey and the cache. `main` takes the guard before this.
fn runner(command: &Command, start: &Path, out: &mut String) -> Result<u8, config::Error> {
    match command {
        Command::Gate(args) => gate::run(args, start, out),
        Command::Init(args) => init::run(args, start, out),
        Command::Cache(args) => cache::run(args, start, out),
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
