mod agent;
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
mod status;
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

/// The public commands of spec 11.1.
#[derive(Subcommand)]
enum Command {
    /// Set up or repair klin integration for this repository, or for one person's host files
    Setup(hooks::Args),
    /// Measure the repository against klin's quality policy, optionally only the named checks
    Check(gate::Check),
    /// Read repository, setup, integration and local window state without running any check
    Status(status::Args),
    /// Explain the effective policy and where each value came from
    Policy(gate::Policy),
    /// Show what klin caught, what was resolved, and what still needs attention
    Report(stats::Args),
    /// Install the newest release over this binary, through the klin-update beside it
    Update,
}

/// The agent ingress answers before the command line is parsed, so an argument it does not know
/// never becomes a usage error. The updater answers before the working directory is read,
/// because it does not need it. Everything else prints through `report`. Spec 10.1, 10.10.
fn main() -> ExitCode {
    LazyLock::force(&shell::STARTED);
    if agent::called() {
        return agent::run();
    }
    match Cli::parse().command {
        Command::Update => ExitCode::from(update::run()),
        command => report(|start, out| public(&command, start, out)),
    }
}

fn public(command: &Command, start: &Path, out: &mut String) -> Result<u8, Error> {
    match command {
        Command::Setup(args) => hooks::run(args, start, out),
        Command::Check(args) => gate::check(args, start, out),
        Command::Status(args) => status::run(args, start, out),
        Command::Policy(args) => gate::policy(args, start, out),
        Command::Report(args) => stats::run(args, start, out),
        Command::Update => Ok(update::run()),
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
