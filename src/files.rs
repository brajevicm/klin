use std::collections::HashSet;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};

use crate::changed::git;
use crate::config::{Config, Error};
use crate::project::Tree;
use crate::ratchet::Values;
use crate::reference::{Key, SKIP_DIRS};
use crate::scope;

const DEFAULT_SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "vendor",
    "build",
    ".build",
    "dist",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    "DerivedData",
    "Pods",
    "coverage",
    ".next",
    "out",
    "fixtures",
];

/// Whether a directory of this name is one every gate skips, without building the list.
pub fn skipped(name: &str) -> bool {
    DEFAULT_SKIP_DIRS.contains(&name)
}

/// The directories every gate skips, for a survey that has no section to read.
pub fn default_skip_dirs() -> Vec<String> {
    DEFAULT_SKIP_DIRS
        .iter()
        .map(|dir| dir.to_string())
        .collect()
}

pub fn roots(
    config: &Config,
    section_name: &str,
    section: &Values,
    key: Key,
) -> Result<Option<Vec<PathBuf>>, Error> {
    let Some(listed) = section.get(key.name) else {
        return Ok(None);
    };
    let malformed = || config.malformed(section_name, key.name, "a list of paths");
    let Some(listed) = listed.as_array() else {
        return Err(malformed());
    };
    listed
        .iter()
        .map(|root| {
            root.as_str()
                .map(|name| config.path(name))
                .ok_or_else(malformed)
        })
        .collect::<Result<Vec<PathBuf>, Error>>()
        .map(Some)
}

pub fn strings(
    config: &Config,
    section_name: &str,
    section: &Values,
    key: Key,
) -> Result<Vec<String>, Error> {
    let Some(listed) = section.get(key.name) else {
        return Ok(Vec::new());
    };
    let malformed = || config.malformed(section_name, key.name, "a list of strings");
    listed
        .as_array()
        .ok_or_else(malformed)?
        .iter()
        .map(|item| item.as_str().map(str::to_string).ok_or_else(malformed))
        .collect()
}

pub fn skip_dirs(
    config: &Config,
    section_name: &str,
    section: &Values,
) -> Result<Vec<String>, Error> {
    let mut dirs: Vec<String> = DEFAULT_SKIP_DIRS
        .iter()
        .map(|dir| dir.to_string())
        .collect();
    dirs.extend(strings(config, section_name, section, SKIP_DIRS)?);
    Ok(dirs)
}

pub struct Wanted<'a> {
    pub extensions: &'a [&'a str],
    pub skip_dirs: &'a [String],
    pub exclude: &'a [String],
    pub exclude_except: &'a [PathBuf],
    pub skip_hidden: bool,
}

impl Wanted<'_> {
    fn matches(&self, name: &str) -> bool {
        self.extensions
            .iter()
            .any(|extension| name.ends_with(extension))
    }

    fn descends(&self, name: &str) -> bool {
        !(self.skip_hidden && name.starts_with('.'))
            && !self.skip_dirs.iter().any(|skipped| skipped == name)
    }

    fn excluded(&self, path: &Path, name: &str) -> bool {
        if self.exclude_except.iter().any(|kept| kept == path) {
            return false;
        }
        let whole = path.to_string_lossy();
        self.exclude.iter().any(|glob| {
            glob_matches(glob.as_bytes(), name.as_bytes())
                || glob_matches(glob.as_bytes(), whole.as_bytes())
        })
    }
}

/// What a walk reached: the files the check measures, and the ones an exclusion dropped, which
/// its coverage counts. Spec 8.6.
#[derive(Default)]
pub struct Found {
    pub kept: Vec<PathBuf>,
    pub excluded: Vec<PathBuf>,
}

pub fn under(tree: &Tree, roots: &[PathBuf], wanted: &Wanted) -> Result<Vec<PathBuf>, Error> {
    Ok(found(tree, roots, wanted)?.kept)
}

/// The files under each root that the check wants, read off the tree's one file list, so
/// however many roots a section names the tree is walked once and git is asked once what it
/// ignores. A root the list did not reach — outside the tree, under a directory every walk
/// skips, or behind a symbolic link — is walked on its own, as every root once was. ADR 0038.
pub fn found(tree: &Tree, roots: &[PathBuf], wanted: &Wanted) -> Result<Found, Error> {
    let mut found = Found::default();
    for root in roots {
        match tree.covers(root) {
            Some(directory) => select(tree, &directory, wanted, &mut found)?,
            None => walk(root, wanted, &ignored(root), &mut found)?,
        }
    }
    for files in [&mut found.kept, &mut found.excluded] {
        files.sort();
        files.dedup();
    }
    Ok(found)
}

/// One root's files out of the tree's list, under the same rules the walk applies below a
/// root: a hidden or skipped directory below it is not descended, and the file's name and its
/// path decide the rest. Spec 5.6, ADR 0038.
fn select(tree: &Tree, directory: &str, wanted: &Wanted, into: &mut Found) -> Result<(), Error> {
    for file in tree.files()? {
        if !scope::under_or_at(file, directory) {
            continue;
        }
        let below = match directory {
            scope::ROOT => file.as_str(),
            _ => &file[directory.len() + 1..],
        };
        let (parents, name) = below.rsplit_once('/').unwrap_or(("", below));
        if !parents.is_empty() && !parents.split('/').all(|segment| wanted.descends(segment)) {
            continue;
        }
        keep(tree.root().join(file), name, wanted, into);
    }
    Ok(())
}

/// Every file under the root, by its relative path, sorted: the default skip set pruned, every
/// path git ignores pruned, and no symbolic link. Hidden directories are walked, and a caller
/// that skips them filters them out. Read once per tree, by `project::Tree`. Spec 4.3.
pub fn listing(root: &Path) -> Result<Vec<String>, Error> {
    let skip_dirs = default_skip_dirs();
    let wanted = Wanted {
        extensions: &[""],
        skip_dirs: &skip_dirs,
        exclude: &[],
        exclude_except: &[],
        skip_hidden: false,
    };
    let mut found = Found::default();
    walk(root, &wanted, &ignored(root), &mut found)?;
    let mut files: Vec<String> = found.kept.iter().map(|path| relative(path, root)).collect();
    files.sort();
    Ok(files)
}

/// What git ignores under a root. A gate judges the tree git describes, so a generated file
/// beside it is not measured: the base commit holds no copy of it to ratchet against.
fn ignored(root: &Path) -> HashSet<PathBuf> {
    let listed = git(
        root,
        &[
            "ls-files",
            "--others",
            "--ignored",
            "--exclude-standard",
            "--directory",
        ],
    );
    listed
        .unwrap_or_default()
        .lines()
        .filter(|name| !name.is_empty())
        .map(|name| root.join(name.trim_end_matches('/')))
        .collect()
}

fn walk(
    root: &Path,
    wanted: &Wanted,
    ignored: &HashSet<PathBuf>,
    into: &mut Found,
) -> Result<(), Error> {
    let listing = std::fs::read_dir(root).map_err(|why| Error::unreadable(root, why))?;
    for entry in listing {
        let entry = entry.map_err(|why| Error::unreadable(root, why))?;
        visit(&entry, wanted, ignored, into)?;
    }
    Ok(())
}

fn visit(
    entry: &DirEntry,
    wanted: &Wanted,
    ignored: &HashSet<PathBuf>,
    into: &mut Found,
) -> Result<(), Error> {
    let path = entry.path();
    if entry
        .file_type()
        .map_err(|why| Error::unreadable(&path, why))?
        .is_symlink()
        || ignored.contains(&path)
    {
        return Ok(());
    }
    let name = entry.file_name().to_string_lossy().to_string();
    if path.is_dir() {
        if wanted.descends(&name) {
            walk(&path, wanted, ignored, into)?;
        }
    } else {
        keep(path, &name, wanted, into);
    }
    Ok(())
}

/// Where one file the walk reached lands: the set the check measures, the set an exclusion
/// dropped, or neither, because no discovery rule of this check names it.
fn keep(path: PathBuf, name: &str, wanted: &Wanted, into: &mut Found) {
    if !wanted.matches(name) {
        return;
    }
    match wanted.excluded(&path, name) {
        true => into.excluded.push(path),
        false => into.kept.push(path),
    }
}

pub fn glob_matches(glob: &[u8], text: &[u8]) -> bool {
    match glob.first() {
        None => text.is_empty(),
        Some(b'*') => star_matches(glob, text),
        Some(b'[') => class_matches(glob, text),
        Some(b'?') => !text.is_empty() && glob_matches(&glob[1..], &text[1..]),
        Some(first) => text.first() == Some(first) && glob_matches(&glob[1..], &text[1..]),
    }
}

fn star_matches(glob: &[u8], text: &[u8]) -> bool {
    glob_matches(&glob[1..], text) || (!text.is_empty() && glob_matches(glob, &text[1..]))
}

fn class_matches(glob: &[u8], text: &[u8]) -> bool {
    let Some(end) = class_end(glob) else {
        return text.first() == Some(&b'[') && glob_matches(&glob[1..], &text[1..]);
    };
    let (negated, set) = match glob[1] {
        b'!' => (true, &glob[2..end]),
        _ => (false, &glob[1..end]),
    };
    text.first()
        .is_some_and(|byte| in_class(set, *byte) != negated)
        && glob_matches(&glob[end + 1..], &text[1..])
}

fn class_end(glob: &[u8]) -> Option<usize> {
    let opens = match glob.get(1) {
        Some(b'!') => 2,
        _ => 1,
    };
    glob[opens..]
        .iter()
        .position(|byte| *byte == b']')
        .map(|at| at + opens)
        .filter(|end| *end > opens)
}

fn in_class(set: &[u8], byte: u8) -> bool {
    let mut at = 0;
    while at < set.len() {
        let ranged = at + 2 < set.len() && set[at + 1] == b'-';
        if ranged && (set[at]..=set[at + 2]).contains(&byte) {
            return true;
        }
        if !ranged && set[at] == byte {
            return true;
        }
        at += if ranged { 3 } else { 1 };
    }
    false
}

pub fn relative(path: &Path, repo_root: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}
