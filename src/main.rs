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
mod error;
mod escapes;
mod files;
mod gate;
mod git;
mod guard;
mod handoff;
mod hooks;
mod host;
mod hunks;
mod init;
mod inventory;
mod journal;
mod key;
mod layering;
mod lockfile;
mod markers;
mod measurement;
mod modules;
mod project;
mod public_api;
mod radius;
mod ratchet;
mod reachability;
mod record;
mod reference;
mod sarif;
mod scope;
mod shell;
mod stamp;
mod state;
mod stats;
mod stubs;
mod surface;
mod survey;
mod syntax;
mod tree;
mod turn;
mod update;
mod write;

use std::path::Path;
use std::process::ExitCode;
use std::sync::LazyLock;

use clap::{Parser, Subcommand};

use crate::check::catalogue;
use crate::error::Error;

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

/// The public commands of spec 11.1, less `status`, which #498 adds, and the hidden entry points
/// the host hooks and a person still reach until the agent ingress replaces them.
#[derive(Subcommand)]
enum Command {
    #[command(flatten)]
    Public(Public),
    #[command(flatten)]
    Hidden(Hidden),
}

#[derive(Subcommand)]
enum Public {
    /// Set up or repair klin integration for this repository, or for one person's host files
    Setup(hooks::Args),
    /// Measure the repository against klin's quality policy, optionally only the named checks
    Check(gate::Check),
    /// Explain the effective policy and where each value came from
    Policy(gate::Policy),
    /// Show what klin caught, what was resolved, and what still needs attention
    Report(stats::Args),
    /// Install the newest release over this binary, through the klin-update beside it
    Update,
}

#[derive(Subcommand)]
enum Hidden {
    #[command(hide = true)]
    Guard(guard::Args),
    #[command(hide = true)]
    Radius(turn::Args),
    #[command(hide = true)]
    Turn(turn::Moved),
    #[command(hide = true)]
    Cache(cache::Args),
}

/// The guard and the updater answer before the working directory is read, because neither needs
/// it. The Stop hook's `klin gate --hook` line is read before the public commands, so a plain
/// `klin gate` stays an unknown command. Everything else prints through `report`.
fn main() -> ExitCode {
    LazyLock::force(&shell::STARTED);
    if let Some(hook) = gate::Hook::called() {
        return report(|start, out| gate::hooked(&hook, start, out));
    }
    match Cli::parse().command {
        Command::Hidden(Hidden::Guard(args)) => ExitCode::from(guard::run(&args)),
        Command::Public(Public::Update) => ExitCode::from(update::run()),
        Command::Public(command) => report(|start, out| public(&command, start, out)),
        Command::Hidden(command) => report(|start, out| hidden(&command, start, out)),
    }
}

fn public(command: &Public, start: &Path, out: &mut String) -> Result<u8, Error> {
    match command {
        Public::Setup(args) => hooks::run(args, start, out),
        Public::Check(args) => gate::check(args, start, out),
        Public::Policy(args) => gate::policy(args, start, out),
        Public::Report(args) => stats::run(args, start, out),
        Public::Update => Ok(update::run()),
    }
}

fn hidden(command: &Hidden, start: &Path, out: &mut String) -> Result<u8, Error> {
    match command {
        Hidden::Guard(args) => Ok(guard::run(args)),
        Hidden::Radius(args) => turn::run(args, &catalogue::sections(), start, out),
        Hidden::Turn(args) => turn::moved(args, start, out),
        Hidden::Cache(args) => cache::run(args, start, out),
    }
}

fn report(run: impl FnOnce(&Path, &mut String) -> Result<u8, Error>) -> ExitCode {
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
