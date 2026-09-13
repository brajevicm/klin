use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::base;
use crate::changed::git;
use crate::check::{Context, DELETED, Sink};
use crate::config::{Config, Error};
use crate::coverage::{self, Coverage};
use crate::files;
use crate::project::{Project, Tree};
use crate::ratchet::{self, Evaluator, Finding, Values};
use crate::reference::Key;
use crate::scope::ROOT;
use crate::survey::{TEST_DIRS, TEST_PREFIXES, TEST_SUFFIXES};
use crate::syntax::convention::{self, Test};
use crate::syntax::{self, Unparsed};
use crate::turn;

pub const SECTION: &str = "inventory";

/// The keys this section reads, which `klin reference` prints. Spec 5.4, 5.8.
pub const KEYS: &[Key] = &[NAME, PATH, PATTERN];

pub const NAME: Key = Key {
    name: "name",
    holds: "what a finding calls this entry",
    required: true,
    rule: Some("the test root's path"),
    default: "",
};

pub const PATH: Key = Key {
    name: "path",
    holds: "the directory whose test sites this entry holds",
    required: true,
    rule: Some("one entry per test root the survey found"),
    default: "",
};

const PATTERN: Key = Key {
    name: "pattern",
    holds: "a glob on the basename that limits the entry",
    required: false,
    rule: None,
    default: "every file under `path`",
};
const LABEL: &str = "test file";
const MISSING: &str = "missing";
/// The question the hook's block puts to the agent. Removing a test is ordinary work, and
/// deleting a failing one is the cheapest route to green, so klin asks once and does not judge
/// the answer. Spec 8.2.
const REMEDY: &str = "If a test failed because the code is wrong, restore the test and fix the \
    code. If the removal is intended, say why in your reply and stop again.";

/// The one affix table of spec 8.2, which the survey reads to find a test root and this gate
/// reads to name a test file's subject. Printed with the NOTE.
const RULE: &str = "the affix table: a test_ or spec_ prefix, a _test, _spec, .test or .spec \
    suffix, a Test or Tests suffix on the basename, and a tests/, test/, spec/ or __tests__/ \
    directory segment";

struct Entry {
    path: String,
    pattern: Option<String>,
}

/// One test file the base holds: whether the working tree still has it, and the subject that
/// went with it, if the same window deleted one.
struct Site {
    path: String,
    gone: bool,
    subject: Option<String>,
}

/// One test function the base holds: where it was declared, whether the working tree still
/// holds it by site or by body, and whether the file that held it went in the same window.
/// The second identity of spec 8.2.
struct Function {
    site: Test,
    gone: bool,
    file_went: bool,
}

impl Function {
    /// Whether this is the NOTE of spec 8.2: the function went and so did its one subject, the
    /// file that held it, which #45 judges instead.
    fn orphaned(&self) -> bool {
        self.gone && self.file_went
    }
}

/// What the function level of this gate measured: every test function the base holds, and the
/// files in the working tree no grammar read. ADR 0003.
struct Measured {
    functions: Vec<Function>,
    unparsed: Vec<Unparsed>,
}

/// One tree walked for test functions: the sites it holds, and the files no grammar read.
struct Walk {
    tests: Vec<Test>,
    unparsed: Vec<Unparsed>,
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    let project = at.project;
    let config = &project.config;
    at.say(SECTION, out);
    let entries = entries(project)?;
    let commit = base::commit(config.root(), at, out)?;
    let listed = at_the_base(config.root(), &commit)?;
    let sites = sites(&entries, &listed, config.root());
    let (judged, mut paired): (Vec<Site>, Vec<Site>) =
        sites.into_iter().partition(|site| site.subject.is_none());
    let measured = tests(&entries, at, &commit)?;
    let (mut orphans, functions): (Vec<Function>, Vec<Function>) =
        measured.functions.into_iter().partition(Function::orphaned);
    if let Some(only) = at.only {
        paired.retain(|site| only.contains(&site.path));
        orphans.retain(|function| only.contains(&function.site.file));
    }
    let now: Vec<Finding> = judged
        .iter()
        .map(|site| finding(&site.path, LABEL, 0, site.gone))
        .chain(functions.iter().map(|function| {
            finding(
                &function.site.file,
                &function.site.text,
                function.site.line,
                function.gone,
            )
        }))
        .collect();
    let (before, went) = let_through(&now, at, config.root());
    let held = ratchet::scoped(&now, at.only);
    let accepted = ratchet::accepted(config, at.gate, evaluator().metrics)?;
    let said = covered(&judged, &paired, &measured.unparsed, at).said(out);
    let ok = standing(held, went.len(), &said);
    let code = evaluator().evaluate(now, before, accepted, at, &ok, out);
    deleted(&went, out);
    noted(&paired, out);
    orphaned(&orphans, out);
    Ok(syntax::unread(&measured.unparsed, at, code, out))
}

/// The OK line: how many test sites the base holds, and how many of them the run let go.
fn standing(held: usize, gone: usize, said: &str) -> String {
    match gone {
        0 => format!("OK: {held} test site(s) the base holds, all still there{said}"),
        gone => format!("OK: {held} test site(s) the base holds, {gone} of them gone{said}"),
    }
}

/// Every test function the base holds under the entries, with what the working tree says about
/// it. A match is by site first and then by body hash across files, so a test renamed or moved
/// with its body unchanged is held and only a test that was edited as it moved reads as gone.
/// Spec 4.4, 8.2, 16.4.
fn tests(entries: &[Entry], at: &Context, commit: &str) -> Result<Measured, Error> {
    let config = &at.project.config;
    let owned;
    let prior = match at.prior {
        Some(prior) => prior,
        None => {
            owned = base::materialize(at.project, commit, None)?;
            &owned
        }
    };
    let after = walked(entries, at.project.tree())?;
    let before = walked(entries, prior.tree())?.tests;
    let found = still_there(&before, &after.tests);
    let refused: BTreeSet<&str> = after
        .unparsed
        .iter()
        .map(|file| file.file.as_str())
        .collect();
    Ok(Measured {
        functions: before
            .iter()
            .enumerate()
            .filter(|(_, site)| !refused.contains(site.file.as_str()))
            .map(|(at, site)| Function {
                gone: !found[at],
                file_went: !config.root().join(&site.file).is_file(),
                site: site.clone(),
            })
            .collect(),
        unparsed: after.unparsed,
    })
}

/// Which of the base's test functions the working tree still holds: by site first, then by
/// body hash across files, and one to one on both passes. Spec 4.4, 16.4.
fn still_there(before: &[Test], after: &[Test]) -> Vec<bool> {
    let mut taken = vec![false; after.len()];
    let mut found = vec![false; before.len()];
    for (at, site) in before.iter().enumerate() {
        found[at] = claimed(after, &mut taken, |other| {
            other.file == site.file && other.text == site.text
        });
    }
    for (at, site) in before.iter().enumerate() {
        if !found[at] {
            found[at] = claimed(after, &mut taken, |other| other.body == site.body);
        }
    }
    found
}

/// Whether the working tree holds a test function this rule names that no earlier site has
/// already claimed. Pairing is one to one, so two tests that share a site and lose one of the
/// pair leave that one gone. Spec 4.4.
fn claimed(after: &[Test], taken: &mut [bool], matches: impl Fn(&Test) -> bool) -> bool {
    let found = after
        .iter()
        .enumerate()
        .find(|(at, other)| !taken[*at] && matches(other));
    match found {
        Some((at, _)) => {
            taken[at] = true;
            true
        }
        None => false,
    }
}

/// Every test function one tree holds under the entries, by the convention table of 8.2. The
/// walk is the one the complexity gate does, over the files an entry holds and no others, so
/// the pattern that limits an entry limits this identity too.
fn walked(entries: &[Entry], tree: &Tree) -> Result<Walk, Error> {
    let root = tree.root();
    let extensions = syntax::extensions(&[]);
    let skip_dirs = files::default_skip_dirs();
    let wanted = files::Wanted {
        extensions: &extensions,
        skip_dirs: &skip_dirs,
        exclude: &[],
        exclude_except: &[],
        skip_hidden: true,
    };
    let mut walk = Walk {
        tests: Vec::new(),
        unparsed: Vec::new(),
    };
    for path in reachable(entries, tree, &wanted)? {
        let file = files::relative(&path, root);
        if !entries.iter().any(|entry| entry.holds(&file)) {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|why| Error::unreadable(&path, why))?;
        let source = String::from_utf8_lossy(&bytes);
        walk.tests
            .extend(convention::tests(&file, &source, &mut walk.unparsed));
    }
    walk.tests
        .sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok(walk)
}

/// Every file under the entries this tree holds. An entry that names a directory is walked,
/// and one that names a single file is that file, so both shapes of `path` reach this identity.
fn reachable(
    entries: &[Entry],
    tree: &Tree,
    wanted: &files::Wanted,
) -> Result<Vec<PathBuf>, Error> {
    let mut directories = Vec::new();
    let mut singles = Vec::new();
    for entry in entries {
        let at = under(tree.root(), &entry.path);
        match at.is_dir() {
            true => directories.push(at),
            false => singles.push(at),
        }
    }
    let mut found = files::under(tree, &directories, wanted)?;
    found.extend(singles.into_iter().filter(|path| path.is_file()));
    found.sort();
    found.dedup();
    Ok(found)
}

fn under(root: &Path, path: &str) -> PathBuf {
    match path {
        ROOT => root.to_path_buf(),
        named => root.join(named),
    }
}

/// What this gate discovered: every test file the base holds under its entries, which is also
/// the file scope the function identity reads. A deleted test whose subject went with it is
/// found and not measured, because it is a NOTE and not a site the gate judges. A file no
/// grammar read is unreadable and not measured, because no function in it was seen. Spec 8.6.
fn covered(judged: &[Site], paired: &[Site], unparsed: &[Unparsed], at: &Context) -> Coverage {
    let only = at.only;
    let paths =
        |sites: &[Site]| -> Vec<String> { sites.iter().map(|site| site.path.clone()).collect() };
    let refused: Vec<String> = unparsed.iter().map(|file| file.file.clone()).collect();
    let mut read = paths(judged);
    read.retain(|file| !refused.contains(file));
    let measured = coverage::scoped(&read, only);
    let unreadable = coverage::scoped(&refused, only);
    Coverage {
        found: measured + unreadable + coverage::scoped(&paths(paired), only),
        measured,
        not_measured: 0,
        excluded: 0,
        unreadable,
    }
}

fn evaluator() -> Evaluator<'static> {
    Evaluator {
        metrics: &[MISSING],
        unit: "test site(s)",
        condition: "where the base holds a test site the working tree no longer has",
        fix_advice: REMEDY,
        ceiling: None,
        format_metrics: show,
    }
}

fn show(values: &Values) -> String {
    format!(
        "{MISSING} {}",
        values.get(MISSING).and_then(Value::as_u64).unwrap_or(0)
    )
}

fn finding(path: &str, text: &str, line: u64, gone: bool) -> Finding {
    let mut values = Values::new();
    values.insert(MISSING.into(), u64::from(gone).into());
    Finding {
        file: path.to_string(),
        line,
        text: text.to_string(),
        values,
        body: None,
    }
}

fn gone(site: &Finding) -> bool {
    site.values.get(MISSING).and_then(Value::as_u64) == Some(1)
}

/// The base's entry for every finding, and the deleted tests this run lets through: outside the
/// hook every one, and in the hook the ones a stop's block already asked the agent about. The
/// base entry of a deletion let through carries `missing: 1`, so the one judge holds it and an
/// accepted entry naming it still takes the match. Spec 8.2, 16.4.
fn let_through(now: &[Finding], at: &Context, root: &Path) -> (Vec<Finding>, Vec<Finding>) {
    let asked = match at.hook() {
        true => turn::asked(root),
        false => Vec::new(),
    };
    let through = |site: &Finding| {
        gone(site) && (!at.hook() || asked.contains(&ratchet::identity(at.gate, site)))
    };
    let before = now
        .iter()
        .map(|site| finding(&site.file, &site.text, site.line, through(site)))
        .collect();
    let went = now
        .iter()
        .filter(|site| through(site) && in_scope(at, &site.file))
        .map(|site| finding(&site.file, &site.text, site.line, true))
        .collect();
    (before, went)
}

fn in_scope(at: &Context, file: &str) -> bool {
    at.only
        .is_none_or(|only| only.iter().any(|named| named == file))
}

/// The deleted tests this run lets through, which are a NOTE and not a finding. Removing a test
/// is ordinary work, and klin cannot tell why a test went, so outside the hook a deletion is
/// left to the reviewer who reads the diff. Spec 8.2.
fn deleted(went: &[Finding], out: &mut Sink) {
    if went.is_empty() {
        return;
    }
    let _ = writeln!(
        out.text,
        "NOTE: {} test site(s) the base holds went in this window:",
        went.len()
    );
    for site in went {
        let _ = writeln!(out.text, "  {}:{}  {}", site.file, site.line, site.text);
    }
    out.record(|records| {
        for site in went {
            let mut record = Map::new();
            record.insert("outcome".into(), DELETED.into());
            record.insert("file".into(), site.file.clone().into());
            record.insert("line".into(), site.line.into());
            record.insert(
                "text".into(),
                format!(
                    "the test site {} in {} went in this window",
                    site.text, site.file
                )
                .into(),
            );
            records.notes.push(Value::Object(record));
        }
    });
}

/// A deleted test function whose file went in the same window, which is a NOTE and not a
/// finding. #45 judges the file, and for a function the file is the whole subject. Spec 8.2.
fn orphaned(orphans: &[Function], out: &mut Sink) {
    if orphans.is_empty() {
        return;
    }
    let _ = writeln!(
        out.text,
        "NOTE: {} deleted test function(s) whose file went in the same window:",
        orphans.len()
    );
    for function in orphans {
        let site = &function.site;
        let _ = writeln!(
            out.text,
            "  {}:{}  {}  its file went too",
            site.file, site.line, site.text
        );
        out.record(|records| {
            let mut record = Map::new();
            record.insert("outcome".into(), "note".into());
            record.insert("file".into(), site.file.clone().into());
            record.insert("line".into(), site.line.into());
            record.insert(
                "text".into(),
                format!(
                    "the test function {} went with the file {} that held it",
                    site.text, site.file
                )
                .into(),
            );
            records.notes.push(Value::Object(record));
        });
    }
}

/// A deleted test whose subject went in the same window, which is a NOTE and not a finding.
/// Spec 8.2, 16.4.
fn noted(paired: &[Site], out: &mut Sink) {
    if paired.is_empty() {
        return;
    }
    let _ = writeln!(
        out.text,
        "NOTE: {} deleted test file(s) whose subject went in the same window:",
        paired.len()
    );
    for site in paired {
        let subject = site.subject.as_deref().unwrap_or("");
        let _ = writeln!(out.text, "  {}  its subject {subject} went too", site.path);
    }
    let _ = writeln!(out.text, "  each subject matched by {RULE}");
    out.record(|records| {
        for site in paired {
            let mut record = Map::new();
            record.insert("outcome".into(), "note".into());
            record.insert("file".into(), site.path.clone().into());
            record.insert(
                "text".into(),
                format!(
                    "the test file {} went with its subject {}, matched by {RULE}",
                    site.path,
                    site.subject.as_deref().unwrap_or("")
                )
                .into(),
            );
            records.notes.push(Value::Object(record));
        }
    });
}

/// Every test file the base holds under the entries, each with what the working tree says
/// about it. This is the one measure of `after` that reads `before`. Spec 16.4.
fn sites(entries: &[Entry], listed: &BTreeSet<String>, root: &Path) -> Vec<Site> {
    let mut wanted: Vec<&String> = listed
        .iter()
        .filter(|path| entries.iter().any(|entry| entry.holds(path)))
        .collect();
    wanted.sort();
    wanted
        .into_iter()
        .map(|path| {
            let gone = !root.join(path).is_file();
            Site {
                path: path.clone(),
                gone,
                subject: gone.then(|| subject(path, listed, root)).flatten(),
            }
        })
        .collect()
}

impl Entry {
    fn holds(&self, path: &str) -> bool {
        if !under_or_at(path, &self.path) {
            return false;
        }
        let name = path.rsplit_once('/').map_or(path, |(_, name)| name);
        match &self.pattern {
            Some(glob) => files::glob_matches(glob.as_bytes(), name.as_bytes()),
            None => true,
        }
    }
}

fn under_or_at(path: &str, root: &str) -> bool {
    root == ROOT || path == root || path.starts_with(&format!("{root}/"))
}

/// The file the base holds at the test's path with the affixes stripped, gone from the working
/// tree too. `None` when no candidate resolves, and then the test is judged normally.
fn subject(path: &str, listed: &BTreeSet<String>, root: &Path) -> Option<String> {
    candidates(path)
        .into_iter()
        .find(|candidate| listed.contains(candidate) && !root.join(candidate).is_file())
}

/// Every path the affix table names as a subject: the directory with one test segment removed,
/// the basename with one affix removed, and both together.
fn candidates(path: &str) -> Vec<String> {
    let (directory, name) = match path.rsplit_once('/') {
        Some((directory, name)) => (directory.to_string(), name.to_string()),
        None => (String::new(), path.to_string()),
    };
    let mut directories = vec![directory.clone()];
    directories.extend(without_a_test_dir(&directory));
    let mut names = vec![name.clone()];
    names.extend(without_an_affix(&name));
    let mut out = Vec::new();
    for directory in &directories {
        for name in &names {
            let candidate = match directory.is_empty() {
                true => name.clone(),
                false => format!("{directory}/{name}"),
            };
            if candidate != path {
                out.push(candidate);
            }
        }
    }
    out
}

fn without_a_test_dir(directory: &str) -> Vec<String> {
    let segments: Vec<&str> = match directory.is_empty() {
        true => Vec::new(),
        false => directory.split('/').collect(),
    };
    segments
        .iter()
        .enumerate()
        .filter(|(_, segment)| TEST_DIRS.contains(*segment))
        .map(|(at, _)| {
            let mut kept = segments.clone();
            kept.remove(at);
            kept.join("/")
        })
        .collect()
}

fn without_an_affix(name: &str) -> Vec<String> {
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) => (stem, format!(".{extension}")),
        None => (name, String::new()),
    };
    let prefixes = TEST_PREFIXES
        .iter()
        .filter_map(|prefix| stem.strip_prefix(prefix));
    let suffixes = TEST_SUFFIXES
        .iter()
        .filter_map(|suffix| stem.strip_suffix(suffix));
    prefixes
        .chain(suffixes)
        .filter(|stripped| !stripped.is_empty())
        .map(|stripped| format!("{stripped}{extension}"))
        .collect()
}

/// The base commit the runner chose, or the one this gate chooses for itself.
/// The base tree's file list, read out of git so the file level needs no base worktree. A
/// listing git refuses is an error: an empty base holds no test file and reports green.
fn at_the_base(root: &Path, commit: &str) -> Result<BTreeSet<String>, Error> {
    let listed =
        git(root, &["ls-tree", "-r", "--name-only", commit, "--", "."]).ok_or_else(|| {
            Error(format!(
                "the base commit {} could not be listed under {} — fetch history, or give CI \
                 the full clone",
                &commit[..7.min(commit.len())],
                root.display()
            ))
        })?;
    Ok(listed
        .lines()
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect())
}

fn entries(project: &Project) -> Result<Vec<Entry>, Error> {
    let config = &project.config;
    let Some(listed) = project.section(SECTION)?.as_array() else {
        return Err(Error(format!(
            "{}: \"{SECTION}\" must be a list of {{\"name\", \"path\"}} entries",
            config.file.display()
        )));
    };
    listed.iter().map(|item| entry(config, item)).collect()
}

fn entry(config: &Config, item: &Value) -> Result<Entry, Error> {
    let values = item
        .as_object()
        .ok_or_else(|| config.malformed(SECTION, PATH.name, "an object"))?;
    for key in [NAME, PATH] {
        if !values.get(key.name).is_some_and(Value::is_string) {
            return Err(config.missing(SECTION, key.name));
        }
    }
    let pattern = match values.get(PATTERN.name) {
        None => None,
        Some(Value::String(glob)) => Some(glob.clone()),
        Some(_) => return Err(config.malformed(SECTION, PATTERN.name, "a glob on the basename")),
    };
    Ok(Entry {
        path: values[PATH.name].as_str().unwrap_or_default().to_string(),
        pattern,
    })
}
