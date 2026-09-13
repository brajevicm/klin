mod base;
mod build;
mod cache;
mod ceiling;
mod changed;
mod check;
mod complexity;
mod config;
mod conventions;
mod coverage;
mod dead_symbols;
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
mod project;
mod radius;
mod ratchet;
mod reachability;
mod reference;
mod sarif;
mod scope;
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

/// The three groups a command belongs to. Each is flattened into one subcommand list, so the
/// command line a person types is unchanged, and each group is matched exhaustively on its own.
/// A new command is a variant of one group and an arm beside it, and nothing compiles until its
/// dispatch is decided. ADR 0036.
#[derive(Subcommand)]
enum Command {
    #[command(flatten)]
    Check(Check),
    #[command(flatten)]
    Structural(Structural),
    #[command(flatten)]
    Runner(Runner),
    #[command(flatten)]
    Tool(Tool),
}

/// The checks a person runs one at a time, each judging its own section against the base.
#[derive(Subcommand)]
enum Check {
    /// Fail when a document cites a file that resolves nowhere under its roots
    DocCitations(doc_citations::Args),
    /// Fail when a document has grown past its ceiling
    DocSize(doc_size::Args),
    /// Fail on a new escape site — a place where the code opts out of a check
    Escapes(markers::Args),
    /// Fail on a new placeholder marker — a stub an agent left where the work belongs
    Stubs(markers::Args),
    /// Fail on a new site a project convention forbids
    Conventions(conventions::Args),
    /// Fail on a scanner's result that sits on a line this window changed
    Sarif(sarif::Args),
}

/// The checks that read source through a grammar, each judging its own section against the base.
#[derive(Subcommand)]
enum Structural {
    /// Fail on a function over the cyclomatic or length ceiling that the base does not hold
    Complexity(complexity::Args),
    /// Fail when a private declaration has no reference outside its own declaration
    DeadSymbols(dead_symbols::Args),
    /// Fail when a file of a named family is referenced by no other file in the repository
    Reachability(reachability::Args),
}

/// The runner, the survey that writes a configuration, the guard over that file, and the cache
/// the survey keeps.
#[derive(Subcommand)]
enum Runner {
    /// Run every gate the configuration names, cheapest first
    Gate(gate::Args),
    /// Survey the tree and write the configuration it can say for itself
    Init(init::Args),
    /// Refuse an agent's tool call that would edit the configuration
    Guard(guard::Args),
    /// Remove the survey cache klin keeps for this tree, or every orphaned one
    Cache(cache::Args),
}

/// The turn stamp's two movers, the two commands that only read and print, and the updater.
#[derive(Subcommand)]
enum Tool {
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

/// The guard and the updater answer before the working directory is read, because neither needs
/// it. Everything else prints through `report`.
fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Runner(Runner::Guard(args)) => ExitCode::from(guard::run(&args)),
        Command::Tool(Tool::Update) => ExitCode::from(update::run()),
        Command::Check(command) => report(|start, out| check(&command, start, out)),
        Command::Structural(command) => report(|start, out| structural(&command, start, out)),
        Command::Runner(command) => report(|start, out| runner(&command, start, out)),
        Command::Tool(command) => report(|start, out| tool(&command, start, out)),
    }
}

fn check(command: &Check, start: &Path, out: &mut String) -> Result<u8, config::Error> {
    match command {
        Check::DocCitations(args) => doc_citations::run(args, start, out),
        Check::DocSize(args) => doc_size::run(args, start, out),
        Check::Escapes(args) => escapes::run(args, start, out),
        Check::Stubs(args) => stubs::run(args, start, out),
        Check::Conventions(args) => conventions::run(args, start, out),
        Check::Sarif(args) => sarif::run(args, start, out),
    }
}

fn structural(command: &Structural, start: &Path, out: &mut String) -> Result<u8, config::Error> {
    match command {
        Structural::Complexity(args) => complexity::run(args, start, out),
        Structural::DeadSymbols(args) => dead_symbols::run(args, start, out),
        Structural::Reachability(args) => reachability::run(args, start, out),
    }
}

fn runner(command: &Runner, start: &Path, out: &mut String) -> Result<u8, config::Error> {
    match command {
        Runner::Gate(args) => gate::run(args, start, out),
        Runner::Init(args) => init::run(args, start, out),
        Runner::Guard(args) => Ok(guard::run(args)),
        Runner::Cache(args) => cache::run(args, start, out),
    }
}

fn tool(command: &Tool, start: &Path, out: &mut String) -> Result<u8, config::Error> {
    match command {
        Tool::Radius(args) => turn::run(args, start, out),
        Tool::Turn(args) => turn::moved(args, start, out),
        Tool::Stats(args) => stats::run(args, start, out),
        Tool::Reference => reference::run(out),
        Tool::Update => Ok(update::run()),
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
