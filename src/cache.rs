use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::Error;
use crate::state;

#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Remove the cache for this tree, or with --all the cache of every tree that is gone
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

/// The cache under one state directory, as a run holds it: a run that keeps state reads and
/// writes it, and a read-only command such as `klin policy` only reads it. Spec 11.6.
#[derive(Clone, Copy)]
pub struct Cache<'a> {
    at: &'a Path,
    keeps: bool,
}

impl<'a> Cache<'a> {
    pub fn new(at: &'a Path, keeps: bool) -> Cache<'a> {
        Cache { at, keeps }
    }

    pub fn read(self, commit: &str, key: &str) -> Option<Value> {
        read(self.at, commit, key)
    }

    pub fn write(self, commit: &str, key: &str, value: Value) {
        if self.keeps {
            write(self.at, commit, key, value);
        }
    }
}

/// What one derivation commit's survey holds, under the state directory keyed by that commit.
/// Its contents are a pure function of the commit and the binary version, so a file another
/// version wrote reads as nothing. Spec 6.6.
pub fn read(at: &Path, commit: &str, key: &str) -> Option<Value> {
    ours(&file(at, commit))?.get(key).cloned()
}

/// One key of that survey, merged into what the file already holds, so two derivations under
/// one commit do not overwrite each other. A file another version wrote is replaced rather than
/// merged into, because its other keys are that version's and not this one's. A cache klin
/// cannot write costs the next run the same derivation and nothing else.
pub fn write(at: &Path, commit: &str, key: &str, value: Value) {
    let file = file(at, commit);
    let mut fields = ours(&file).unwrap_or_default();
    fields.insert(VERSION.to_string(), env!("CARGO_PKG_VERSION").into());
    fields.insert(key.to_string(), value);
    if let Some(under) = file.parent() {
        let _ = std::fs::create_dir_all(under);
    }
    let _ = std::fs::write(&file, Value::Object(fields).to_string() + "\n");
}

/// The file's keys when this binary wrote them, and `None` for a file that is gone, unreadable
/// or another version's.
fn ours(file: &Path) -> Option<Map<String, Value>> {
    let text = std::fs::read_to_string(file).ok()?;
    let held: Value = serde_json::from_str(&text).ok()?;
    let fields = held.as_object()?;
    match fields.get(VERSION).and_then(Value::as_str) {
        Some(env!("CARGO_PKG_VERSION")) => Some(fields.clone()),
        _ => None,
    }
}

const VERSION: &str = "version";

fn file(at: &Path, commit: &str) -> PathBuf {
    at.join(state::CACHE).join(format!("{commit}.json"))
}
