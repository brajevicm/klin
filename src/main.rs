mod config;
mod doc_size;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "detent", about = "A quality ratchet for AI-driven development")]
struct Cli {
    #[command(subcommand)]
    gate: Gate,
}

#[derive(Subcommand)]
enum Gate {
    /// Fail when a document has grown past its ceiling
    DocSize(doc_size::Args),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let start = match std::env::current_dir() {
        Ok(directory) => directory,
        Err(why) => {
            eprintln!("FAIL: the working directory could not be read: {why}");
            return ExitCode::from(2);
        }
    };
    let outcome = match &cli.gate {
        Gate::DocSize(args) => doc_size::run(args, &start),
    };
    match outcome {
        Ok(code) => ExitCode::from(code),
        Err(problem) => {
            eprintln!("FAIL: {problem}");
            ExitCode::from(2)
        }
    }
}
