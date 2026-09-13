use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::changed::Change;
use crate::config::{self, Config, Error};
use crate::project::Project;

const BUILD: &str = config::BUILD.name;
/// The two keys one entry of the `build` list holds, which `config::BUILD` states.
pub const RUN: &str = "run";
pub const ROOT: &str = "root";

/// One command that builds part of the tree. An entry with no root covers the whole tree.
pub struct Entry {
    pub root: Option<String>,
    pub run: String,
}

pub fn entries(project: &Project) -> Result<Vec<Entry>, Error> {
    let config = &project.config;
    let shape = || {
        Error(format!(
            "{}: \"{BUILD}\" is a command, or a list of {{\"root\", \"run\"}} entries",
            config.file.display()
        ))
    };
    match project.section(BUILD) {
        Err(_) => Ok(Vec::new()),
        Ok(Value::String(run)) => Ok(vec![Entry {
            root: None,
            run: run.clone(),
        }]),
        Ok(Value::Array(items)) => items
            .iter()
            .map(|item| entry(config, item.as_object().ok_or_else(shape)?))
            .collect(),
        Ok(_) => Err(shape()),
    }
}

fn entry(config: &Config, item: &serde_json::Map<String, Value>) -> Result<Entry, Error> {
    let run = item
        .get(RUN)
        .and_then(Value::as_str)
        .ok_or_else(|| config.missing(BUILD, RUN))?;
    let root = match item.get(ROOT) {
        None => None,
        Some(Value::String(at)) => Some(at.clone()),
        Some(_) => return Err(config.malformed(BUILD, ROOT, "a directory in the tree")),
    };
    Ok(Entry {
        root,
        run: run.to_string(),
    })
}

/// The entries a run builds. Without a changed set that is every entry. With one it is the
/// entries whose root holds a changed file, and every entry when a changed file is under none,
/// because klin cannot know what that file affects.
pub fn wanted<'a>(entries: &'a [Entry], changes: Option<&[Change]>) -> Vec<&'a Entry> {
    let every = || entries.iter().collect();
    let Some(changes) = changes else {
        return every();
    };
    let mut picked = vec![false; entries.len()];
    for name in changes.iter().flat_map(named) {
        let holders: Vec<usize> = (0..entries.len())
            .filter(|at| holds(&entries[*at], name))
            .collect();
        if holders.is_empty() {
            return every();
        }
        for at in holders {
            picked[at] = true;
        }
    }
    entries
        .iter()
        .enumerate()
        .filter(|(at, _)| picked[*at])
        .map(|(_, entry)| entry)
        .collect()
}

/// Both names a change carries. A rename out of a root leaves that root a source short, so the
/// root it left is built as well as the one it landed in.
fn named(change: &Change) -> impl Iterator<Item = &str> {
    std::iter::once(change.path.as_str()).chain(change.was.as_deref())
}

fn holds(entry: &Entry, path: &str) -> bool {
    match &entry.root {
        None => true,
        Some(root) => path.starts_with(&format!("{}/", root.trim_end_matches('/'))),
    }
}

/// The output of the first entry that failed, or None when every entry built.
pub fn failure(root: &Path, wanted: &[&Entry]) -> Option<String> {
    for entry in wanted {
        let at = match &entry.root {
            Some(under) => root.join(under),
            None => root.to_path_buf(),
        };
        let done = Command::new("sh")
            .arg("-c")
            .arg(&entry.run)
            .current_dir(&at)
            .output();
        match done {
            Err(why) => return Some(format!("{}: {why}\n", entry.run)),
            Ok(done) if !done.status.success() => {
                let mut text = String::from_utf8_lossy(&done.stdout).into_owned();
                text.push_str(&String::from_utf8_lossy(&done.stderr));
                return Some(format!("$ {}\n{text}", entry.run));
            }
            Ok(_) => (),
        }
    }
    None
}
