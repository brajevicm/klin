mod checks;
mod cli;
mod config;
mod contract;
mod engine;
mod facts;
mod hook;
mod modules;
mod surface;
mod syntax;
mod sys;
mod window;

use std::path::Path;
use std::process::ExitCode;
use std::sync::LazyLock;

use clap::{Parser, Subcommand};

use crate::cli::{agent, check, policy, report, setup, status, update};
use crate::sys::{error::Error, shell};

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
    Setup(setup::Args),
    /// Measure the repository against klin's quality policy, optionally only the named checks
    Check(check::Check),
    /// Read repository, setup, integration and local window state without running any check
    Status(status::Args),
    /// Explain the effective policy and where each value came from
    Policy(policy::Policy),
    /// Show what klin caught, what was resolved, and what still needs attention
    Report(report::Args),
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
        Command::Setup(args) => setup::run(args, start, out),
        Command::Check(args) => check::check(args, start, out),
        Command::Status(args) => status::run(args, start, out),
        Command::Policy(args) => policy::run(args, start, out),
        Command::Report(args) => report::run(args, start, out),
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
