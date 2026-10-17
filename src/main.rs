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

use std::io::IsTerminal;
use std::path::Path;
use std::process::ExitCode;
use std::sync::LazyLock;

use clap::{Parser, Subcommand};

use crate::cli::{agent, check, policy, report, setup, status, update};
use crate::sys::{error::Error, out, release, shell};

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
    /// Install the newest signed release over this binary
    Update(update::Args),
}

/// The agent ingress answers before the command line is parsed, so an argument it does not know
/// never becomes a usage error. The updater answers before the working directory is read,
/// because it does not need it. Everything else prints through `report`. Spec 10.1, 10.10.
fn main() -> ExitCode {
    LazyLock::force(&shell::STARTED);
    if agent::called() {
        return agent::run();
    }
    if std::env::args_os()
        .nth(1)
        .is_some_and(|word| word == release::CHILD)
    {
        release::child();
        return ExitCode::SUCCESS;
    }
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(refused) => return said(&refused),
    };
    match cli.command {
        Command::Update(args) => ExitCode::from(update::run(&args)),
        command => report(|start, out| public(&command, start, out)),
    }
}

/// A usage error, help or version, printed where clap would print it, in colour on a terminal
/// as clap prints it. It can echo an argument, so it goes through `out` like every other text.
/// Spec 16.3.
fn said(refused: &clap::Error) -> ExitCode {
    let rendered = refused.render();
    let styled = |terminal: bool| match terminal {
        true => rendered.ansi().to_string(),
        false => rendered.to_string(),
    };
    match refused.use_stderr() {
        true => out::eprint(&styled(std::io::stderr().is_terminal())),
        false => out::print(&styled(std::io::stdout().is_terminal())),
    }
    ExitCode::from(u8::try_from(refused.exit_code()).unwrap_or(2))
}

fn public(command: &Command, start: &Path, out: &mut String) -> Result<u8, Error> {
    match command {
        Command::Setup(args) => setup::run(args, start, out),
        Command::Check(args) => check::check(args, start, out),
        Command::Status(args) => status::run(args, start, out),
        Command::Policy(args) => policy::run(args, start, out),
        Command::Report(args) => report::run(args, start, out),
        Command::Update(args) => Ok(update::run(args)),
    }
}

fn report(run: impl FnOnce(&Path, &mut String) -> Result<u8, Error>) -> ExitCode {
    let start = match std::env::current_dir() {
        Ok(directory) => directory,
        Err(why) => {
            out::eprintln(&format!(
                "FAIL: the working directory could not be read: {why}"
            ));
            return ExitCode::from(2);
        }
    };
    let mut printed = String::new();
    let outcome = run(&start, &mut printed);
    out::print(&printed);
    match outcome {
        Ok(code) => ExitCode::from(code),
        Err(problem) => {
            out::eprintln(&format!("FAIL: {problem}"));
            ExitCode::from(2)
        }
    }
}
