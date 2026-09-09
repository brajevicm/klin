use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::config::Error;
use crate::state;

#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Remove the survey cache for this tree, or with --all the cache of every tree that is gone
    Clean {
        /// Remove the cache under every KLIN_STATE_DIR entry whose repository no longer exists
        #[arg(long)]
        all: bool,
    },
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let Command::Clean { all } = args.command;
    match all {
        true => Ok(orphans(out)),
        false => Ok(mine(start, out)),
    }
}

fn mine(start: &Path, out: &mut String) -> u8 {
    let Some(at) = state::dir(start).map(|at| at.join(state::CACHE)) else {
        let _ = writeln!(
            out,
            "klin: no git repository here, so there is no cache to remove."
        );
        return 0;
    };
    let _ = match std::fs::remove_dir_all(&at) {
        Ok(()) => writeln!(out, "klin: removed {}.", at.display()),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            writeln!(out, "klin: no cache at {}.", at.display())
        }
        Err(why) => {
            return unremovable(&at, &why, out);
        }
    };
    0
}

fn unremovable(at: &Path, why: &std::io::Error, out: &mut String) -> u8 {
    let _ = writeln!(out, "klin: {} could not be removed: {why}", at.display());
    2
}

fn orphans(out: &mut String) -> u8 {
    let Some(under) = std::env::var_os(state::OVERRIDE).map(PathBuf::from) else {
        let _ = writeln!(
            out,
            "klin: {} is not set, so every tree keeps its state in its own git \
             directory and it goes when the repository goes.",
            state::OVERRIDE
        );
        return 0;
    };
    let Ok(entries) = std::fs::read_dir(&under) else {
        let _ = writeln!(out, "klin: nothing under {}.", under.display());
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten().filter(|entry| gone(&entry.path())) {
        if std::fs::remove_dir_all(entry.path().join(state::CACHE)).is_ok() {
            removed += 1;
        }
    }
    let _ = writeln!(
        out,
        "klin: removed {removed} cache{} under {} whose repository is gone. The stamps stay: a \
         tree klin cannot see from here may still be there.",
        match removed == 1 {
            true => "",
            false => "s",
        },
        under.display()
    );
    0
}

/// An entry outlived its repository when the tree its `repository` file names is not there. An
/// entry that names no tree is left alone, because klin cannot say whose it is.
fn gone(entry: &Path) -> bool {
    let Ok(named) = std::fs::read_to_string(entry.join(state::REPOSITORY)) else {
        return false;
    };
    !Path::new(named.trim()).is_dir()
}
