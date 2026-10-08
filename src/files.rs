use std::collections::HashSet;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::error::Error;
use crate::git::Repo;
use crate::key::{Key, SKIP_DIRS};
use crate::record::Values;
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

pub fn under<'a>(
    tree_root: &Path,
    files: impl Fn() -> Result<&'a [String], Error>,
    roots: &[PathBuf],
    wanted: &Wanted,
) -> Result<Vec<PathBuf>, Error> {
    Ok(found(tree_root, files, roots, wanted)?.kept)
}

/// The files under each root that the check wants, read off the tree's one file list, so
/// however many roots a section names the tree is walked once and git is asked once what it
/// ignores. The list is asked for only when a root is one it covers. A root the list did not
/// reach — outside the tree, under a directory every walk skips, or behind a symbolic link —
/// is walked on its own, as every root once was. ADR 0038.
pub fn found<'a>(
    tree_root: &Path,
    files: impl Fn() -> Result<&'a [String], Error>,
    roots: &[PathBuf],
    wanted: &Wanted,
) -> Result<Found, Error> {
    let mut found = Found::default();
    for root in roots {
        match covers(tree_root, root) {
            Some(directory) => select(tree_root, files()?, &directory, wanted, &mut found),
            None => walk(root, wanted, &ignored(root), &mut found)?,
        }
    }
    for files in [&mut found.kept, &mut found.excluded] {
        files.sort();
        files.dedup();
    }
    Ok(found)
}

/// The file list's name for a directory the tree at this root holds, and `None` for one the
/// walk did not reach: outside the root, not a directory, under a directory every walk skips,
/// or behind a symbolic link, which the walk does not follow and a root named through one
/// still reads.
fn covers(tree_root: &Path, directory: &Path) -> Option<String> {
    if !directory.is_dir() {
        return None;
    }
    let inside = directory.strip_prefix(tree_root).ok()?;
    let named = inside.to_str()?;
    if named.split('/').any(skipped) || linked(tree_root, inside) {
        return None;
    }
    Some(match named.is_empty() {
        true => scope::ROOT.to_string(),
        false => named.to_string(),
    })
}

/// Whether any directory between the root and this one is a symbolic link.
fn linked(tree_root: &Path, inside: &Path) -> bool {
    let mut at = tree_root.to_path_buf();
    inside.components().any(|part| {
        at.push(part);
        at.symlink_metadata()
            .is_ok_and(|held| held.file_type().is_symlink())
    })
}

/// One root's files out of the tree's list, under the same rules the walk applies below a
/// root: a hidden or skipped directory below it is not descended, and the file's name and its
/// path decide the rest. Spec 5.6, ADR 0038.
fn select(tree_root: &Path, files: &[String], directory: &str, wanted: &Wanted, into: &mut Found) {
    for file in files {
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
        keep(tree_root.join(file), name, wanted, into);
    }
}

/// What one tree's file list cost: asking git what it ignores, and walking the directories.
/// Spec 11.2.
#[derive(Default, Clone, Copy)]
pub struct Listing {
    pub ignored: Duration,
    pub walk: Duration,
}

/// Every file under the root, by its relative path, sorted: the default skip set pruned, every
/// path git ignores pruned, and no symbolic link. Hidden directories are walked, and a caller
/// that skips them filters them out. Read once per tree, by `tree::Tree`, with what the two
/// parts of the read took. Spec 4.3.
pub fn listing(root: &Path) -> Result<(Vec<String>, Listing), Error> {
    let skip_dirs = default_skip_dirs();
    let wanted = Wanted {
        extensions: &[""],
        skip_dirs: &skip_dirs,
        exclude: &[],
        exclude_except: &[],
        skip_hidden: false,
    };
    let mut found = Found::default();
    let started = Instant::now();
    let ignored = ignored(root);
    let mut cost = Listing {
        ignored: started.elapsed(),
        walk: Duration::ZERO,
    };
    let started = Instant::now();
    walk(root, &wanted, &ignored, &mut found)?;
    let mut files: Vec<String> = found.kept.iter().map(|path| relative(path, root)).collect();
    files.sort();
    cost.walk = started.elapsed();
    Ok((files, cost))
}

/// What git ignores under a root. A gate judges the tree git describes, so a generated file
/// beside it is not measured: the base commit holds no copy of it to ratchet against.
fn ignored(root: &Path) -> HashSet<PathBuf> {
    Repo::at(root)
        .ignored_paths()
        .unwrap_or_default()
        .into_iter()
        .map(|name| root.join(name))
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

/// What the in-tree `.gitattributes` files say of one path's form: `binary` or `-diff`, a
/// `filter`, or a `working-tree-encoding` other than UTF-8. `-text` alone says nothing, because
/// it only turns off end-of-line conversion. Spec 7.2.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Form {
    pub binary: bool,
    pub filter: bool,
    pub encoding: bool,
}

impl Form {
    pub fn any(self) -> bool {
        self.binary || self.filter || self.encoding
    }
}

/// One attribute's state as the last line that names it left it.
#[derive(Clone, PartialEq, Eq)]
enum State {
    Set,
    Unset,
    Unspecified,
    Value(String),
}

/// The `.gitattributes` files that can name a path: the one at the tree root and one in each
/// directory above the path, shallowest first, so a deeper file wins.
pub fn attribute_files(path: &str) -> Vec<String> {
    let mut out = vec![".gitattributes".to_string()];
    let mut at = String::new();
    for directory in path
        .split('/')
        .rev()
        .skip(1)
        .collect::<Vec<&str>>()
        .into_iter()
        .rev()
    {
        at.push_str(directory);
        at.push('/');
        out.push(format!("{at}.gitattributes"));
    }
    out
}

/// The form these `.gitattributes` texts give one path. Each text is keyed by the file that
/// holds it, shallowest first. klin reads only in-tree files, never `.git/info/attributes` or
/// `core.attributesFile`, so two machines agree. Spec 7.2.
pub fn form(path: &str, texts: &[(String, String)]) -> Form {
    let mut states: [Option<State>; 3] = [None, None, None];
    for (file, text) in texts {
        let directory = file.trim_end_matches(".gitattributes");
        let Some(below) = path.strip_prefix(directory) else {
            continue;
        };
        for line in text.lines() {
            apply(line, below, &mut states);
        }
    }
    let [diff, filter, encoding] = states;
    Form {
        binary: diff == Some(State::Unset),
        filter: matches!(filter, Some(State::Set | State::Value(_))),
        encoding: matches!(encoding, Some(State::Value(name)) if !name.eq_ignore_ascii_case("utf-8") && !name.eq_ignore_ascii_case("utf8")),
    }
}

/// One `.gitattributes` line applied to a path below its directory.
fn apply(line: &str, below: &str, states: &mut [Option<State>; 3]) {
    let Some((pattern, attributes)) = pattern_of(line.trim_start()) else {
        return;
    };
    if pattern.starts_with('#') || pattern.starts_with('!') || !attribute_matches(&pattern, below) {
        return;
    }
    for (at, state) in attributes.split_whitespace().filter_map(attribute) {
        states[at] = Some(state);
    }
}

/// A line's pattern and the attributes after it. A pattern in double quotes may hold spaces and
/// the escapes `\"`, `\\`, `\t` and `\n`, as git reads it.
fn pattern_of(line: &str) -> Option<(String, &str)> {
    let Some(quoted) = line.strip_prefix('"') else {
        let (pattern, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        return (!pattern.is_empty()).then(|| (pattern.to_string(), rest));
    };
    let mut pattern = String::new();
    let mut chars = quoted.char_indices();
    while let Some((at, unit)) = chars.next() {
        match unit {
            '"' => return Some((pattern, &quoted[at + 1..])),
            '\\' => pattern.push(unescaped(chars.next()?.1)),
            _ => pattern.push(unit),
        }
    }
    None
}

fn unescaped(unit: char) -> char {
    match unit {
        't' => '\t',
        'n' => '\n',
        other => other,
    }
}

/// Which of the three attributes klin reads one word sets, and to what. The `binary` macro
/// unsets `diff`, which is the one part of it klin reads.
fn attribute(word: &str) -> Option<(usize, State)> {
    match named(word) {
        ("binary", State::Set) => Some((0, State::Unset)),
        ("diff", state) => Some((0, state)),
        ("filter", state) => Some((1, state)),
        ("working-tree-encoding", state) => Some((2, state)),
        _ => None,
    }
}

/// The attribute one word names and the state it gives it: `name=value`, `-name`, `!name` or
/// `name`.
fn named(word: &str) -> (&str, State) {
    if let Some((name, value)) = word.split_once('=') {
        return (name, State::Value(value.to_string()));
    }
    match word.as_bytes().first() {
        Some(b'-') => (&word[1..], State::Unset),
        Some(b'!') => (&word[1..], State::Unspecified),
        _ => (word, State::Set),
    }
}

/// Whether a `.gitattributes` pattern names a path below its directory: by the basename for a
/// pattern with no slash, and by the whole path below the directory otherwise. A pattern that
/// ends in a slash names a directory, which names no file.
fn attribute_matches(pattern: &str, below: &str) -> bool {
    if pattern.ends_with('/') {
        return false;
    }
    let anchored = pattern.trim_start_matches('/');
    match pattern.contains('/') {
        true => wildmatch(anchored.as_bytes(), below.as_bytes()),
        false => {
            let name = below.rsplit('/').next().unwrap_or(below);
            wildmatch(pattern.as_bytes(), name.as_bytes())
        }
    }
}

/// git's wildmatch for a path: `*` and `?` stay inside one path segment, `**` between slashes
/// spans any number of directories, `[...]` is a class, and a backslash quotes the next byte.
/// Each position pair is decided once, so a pattern an agent writes cannot make the match take
/// exponential time.
fn wildmatch(pattern: &[u8], path: &[u8]) -> bool {
    Wild {
        pattern,
        path,
        decided: vec![None; (pattern.len() + 1) * (path.len() + 1)],
    }
    .at(0, 0)
}

struct Wild<'a> {
    pattern: &'a [u8],
    path: &'a [u8],
    decided: Vec<Option<bool>>,
}

impl Wild<'_> {
    /// Whether the pattern from `p` matches the path from `t`.
    fn at(&mut self, p: usize, t: usize) -> bool {
        let slot = p * (self.path.len() + 1) + t;
        if let Some(known) = self.decided[slot] {
            return known;
        }
        let matched = match &self.pattern[p..] {
            [] => t == self.path.len(),
            [b'*', b'*', ..] => self.any_depth(p + 2, t),
            [b'*', ..] => self.within_segment(p + 1, t),
            [b'[', ..] => self.classed(p, t),
            _ => self.one_byte(p, t),
        };
        self.decided[slot] = Some(matched);
        matched
    }

    /// `**`: before a slash it matches no directory or any run of whole directories, and
    /// anywhere else it matches the rest of the path whatever it holds.
    fn any_depth(&mut self, p: usize, t: usize) -> bool {
        if self.pattern.get(p) == Some(&b'/') {
            return self.at(p + 1, t)
                || (t..self.path.len()).any(|at| self.path[at] == b'/' && self.at(p + 1, at + 1));
        }
        (t..=self.path.len()).any(|at| self.at(p, at))
    }

    /// `*`: any run of bytes that holds no slash.
    fn within_segment(&mut self, p: usize, t: usize) -> bool {
        let segment = self.path[t..]
            .iter()
            .position(|byte| *byte == b'/')
            .map_or(self.path.len(), |at| t + at);
        (t..=segment).any(|at| self.at(p, at))
    }

    /// `[...]`: one byte, never a slash, in or, after `!` or `^`, out of the class. A `[` with
    /// no closing `]` is a literal `[`.
    fn classed(&mut self, p: usize, t: usize) -> bool {
        let negated = matches!(self.pattern.get(p + 1), Some(b'!' | b'^'));
        let opens = p + 1 + usize::from(negated);
        let end = self
            .pattern
            .get(opens + 1..)
            .and_then(|rest| rest.iter().position(|byte| *byte == b']'))
            .map(|at| at + opens + 1);
        let Some(end) = end else {
            return self.path.get(t) == Some(&b'[') && self.at(p + 1, t + 1);
        };
        let set = &self.pattern[opens..end];
        self.path
            .get(t)
            .is_some_and(|byte| *byte != b'/' && in_class(set, *byte) != negated)
            && self.at(end + 1, t + 1)
    }

    /// `?`, a byte a backslash quotes, or a literal byte: one byte of the path, and `?` never a
    /// slash.
    fn one_byte(&mut self, p: usize, t: usize) -> bool {
        let (wanted, next) = match &self.pattern[p..] {
            [b'\\', quoted, ..] => (Some(*quoted), p + 2),
            [b'?', ..] => (None, p + 1),
            [literal, ..] => (Some(*literal), p + 1),
            [] => return t == self.path.len(),
        };
        let Some(byte) = self.path.get(t) else {
            return false;
        };
        wanted.map_or(*byte != b'/', |wanted| wanted == *byte) && self.at(next, t + 1)
    }
}

/// The form the working tree's `.gitattributes` files give one path.
pub fn form_in(root: &Path, path: &str) -> Form {
    let texts: Vec<(String, String)> = attribute_files(path)
        .into_iter()
        .filter_map(|file| {
            Some((
                file.clone(),
                std::fs::read_to_string(root.join(&file)).ok()?,
            ))
        })
        .collect();
    form(path, &texts)
}

/// The form a commit's `.gitattributes` files give one path, read out of git.
pub fn form_at(root: &Path, commit: &str, path: &str) -> Form {
    let files = attribute_files(path);
    let named: Vec<&str> = files.iter().map(String::as_str).collect();
    let mut texts = Vec::new();
    crate::changed::blobs(root, commit, &named, |file, bytes| {
        if let Some(bytes) = bytes {
            texts.push((
                file.to_string(),
                String::from_utf8_lossy(bytes).into_owned(),
            ));
        }
    });
    form(path, &texts)
}
