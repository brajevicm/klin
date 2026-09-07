use std::fs::DirEntry;
use std::path::{Path, PathBuf};

use crate::baseline::Values;
use crate::config::{Config, Error};

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

pub fn roots(
    config: &Config,
    section_name: &str,
    section: &Values,
    key: &str,
) -> Result<Option<Vec<PathBuf>>, Error> {
    let Some(listed) = section.get(key) else {
        return Ok(None);
    };
    let malformed = || config.malformed(section_name, key, "a list of paths");
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
    key: &str,
) -> Result<Vec<String>, Error> {
    let Some(listed) = section.get(key) else {
        return Ok(Vec::new());
    };
    let malformed = || config.malformed(section_name, key, "a list of strings");
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
    dirs.extend(strings(config, section_name, section, "skip_dirs")?);
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
    fn keeps(&self, path: &Path, name: &str) -> bool {
        self.extensions
            .iter()
            .any(|extension| name.ends_with(extension))
            && !self.excluded(path, name)
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

pub fn under(roots: &[PathBuf], wanted: &Wanted) -> Result<Vec<PathBuf>, Error> {
    let mut files = Vec::new();
    for root in roots {
        walk(root, wanted, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn walk(root: &Path, wanted: &Wanted, into: &mut Vec<PathBuf>) -> Result<(), Error> {
    let listing = std::fs::read_dir(root).map_err(|why| Error::unreadable(root, why))?;
    for entry in listing {
        let entry = entry.map_err(|why| Error::unreadable(root, why))?;
        visit(&entry, wanted, into)?;
    }
    Ok(())
}

fn visit(entry: &DirEntry, wanted: &Wanted, into: &mut Vec<PathBuf>) -> Result<(), Error> {
    let path = entry.path();
    if entry
        .file_type()
        .map_err(|why| Error::unreadable(&path, why))?
        .is_symlink()
    {
        return Ok(());
    }
    let name = entry.file_name().to_string_lossy().to_string();
    if path.is_dir() {
        if wanted.descends(&name) {
            walk(&path, wanted, into)?;
        }
    } else if wanted.keeps(&path, &name) {
        into.push(path);
    }
    Ok(())
}

fn glob_matches(glob: &[u8], text: &[u8]) -> bool {
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
