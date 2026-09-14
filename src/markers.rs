use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::Value;

use crate::base;
use crate::changed::Change;
use crate::check::{ContentCost, Context, Sink};
use crate::config::{Config, Error};
use crate::coverage::{self, Files};
use crate::files;
use crate::project::{Project, Tree};
use crate::ratchet::{self, Evaluator, Finding, Values};
use crate::reference::{self, Key};
use crate::scope::Scope;
use crate::syntax;

/// One row of a table: the name the report prints, the pattern to look for, and the remedy for
/// a site it matches. A row that names no remedy carries the empty string.
pub type Row = (&'static str, &'static str, &'static str);

/// One language's table: the names a config may call it by, the files it reads, and its rows.
pub struct Language {
    pub names: &'static [&'static str],
    pub suffixes: &'static [&'static str],
    pub patterns: &'static [Row],
}

/// One line-pattern check. `escapes` and `stubs` are the same engine over two tables, and
/// differ only in what this holds. Spec 8.2.
pub struct Kind {
    pub section: &'static str,
    pub languages: &'static [Language],
    /// The configuration keys this section reads, which `klin reference` prints. The two kinds
    /// share most of them and differ in the rule that derives `languages`. Spec 5.8.
    pub keys: &'static [Key],
    /// The values key a matched row's name is recorded under.
    pub label: &'static str,
    /// Whether a site inside an inline Rust test module is one the config may skip. A stub in a
    /// test is the case the spec names, so `stubs` never skips one.
    pub skips_tests: bool,
    /// Whether a match that lies inside a string literal is thrown away.
    pub skips_literals: bool,
    /// Whether the function walk judges body shapes too, which only a parser can see. #114.
    pub reads_shapes: bool,
    pub evaluator: Evaluator<'static>,
}

/// The key only a kind that may skip them reads. A kind whose check judges an inline test module
/// either way refuses it, so nothing turns it on and measures the same set in silence.
pub const SKIP_RUST_TESTS: Key = Key {
    name: "skip_rust_tests",
    holds: "whether an inline Rust test module is left out",
    required: false,
    rule: None,
    default: "`true`",
};

/// Every language name a kind's table holds, with the extensions that name selects.
pub fn language_extensions(kind: &Kind) -> Vec<(&'static str, String)> {
    reference::extensions_by_name(
        kind.languages
            .iter()
            .map(|language| (language.names, language.suffixes)),
    )
}

#[derive(clap::Args)]
pub struct Args {
    /// The klin.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Print nothing on success
    #[arg(long)]
    quiet: bool,
    /// Fail when an accepted entry matches nothing — what CI runs
    #[arg(long)]
    strict: bool,
    /// Print the built-in pattern sets and exit
    #[arg(long)]
    list_languages: bool,
    /// Judge only these repo-relative files, against only their sites at the base
    #[arg(long, num_args = 0.., value_name = "FILE")]
    only: Option<Vec<String>>,
}

#[derive(Clone)]
struct Pattern {
    name: String,
    regex: Regex,
    remedy: String,
}

#[derive(Clone)]
struct Set {
    suffixes: Vec<String>,
    patterns: Vec<Pattern>,
    /// Whether the function walk judges the body shapes of the files this set reads. A set the
    /// project's own patterns make is not a language, so it names no shapes. #114.
    shapes: bool,
}

struct Spec {
    search: Search,
}

#[derive(Clone)]
struct Search {
    sets: Vec<Set>,
    scope: Scope,
    skip_rust_tests: bool,
}

/// What one file says about where a match does not count: the inline test modules, and the
/// quoted spans.
#[derive(Default, Clone)]
struct Skipped {
    tests: Vec<(u64, u64)>,
    literals: Vec<(usize, usize)>,
}

/// One tree read: the sites, how many an inline test module took out of the count, and the
/// files the walk reached, which is what the gate's coverage counts.
struct Read {
    findings: Vec<Finding>,
    skipped: u64,
    files: Files,
    work: ContentCost,
}

struct Tally {
    line: u64,
    name: String,
    remedy: String,
    count: u64,
}

pub fn run(kind: &Kind, args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    if args.list_languages {
        list_languages(kind, out);
        return Ok(0);
    }
    let project = Project::load(args.config.as_deref(), start)?;
    gate(
        kind,
        &context(kind, args, &project),
        &mut Sink::unrecorded(out),
    )
}

/// A matched row's name, its count and its remedy, as one report column.
pub fn show(label: &str, values: &Values) -> String {
    let name = values.get(label).and_then(Value::as_str).unwrap_or("?");
    let named = match values.get("count").and_then(Value::as_u64) {
        Some(count) if count > 1 => format!("{name} x{count}"),
        _ => name.to_string(),
    };
    match values.get("remedy").and_then(Value::as_str) {
        Some(remedy) if !remedy.is_empty() => format!("{named} — {remedy}"),
        _ => named,
    }
}

pub fn gate(kind: &Kind, at: &Context, out: &mut Sink) -> Result<u8, Error> {
    let project = at.project;
    let spec = spec(kind, project)?;
    let read = findings(
        kind,
        &spec.search,
        project.tree(),
        project.root(),
        at.changes.filter(|_| !at.strict),
    )?;
    let sites = ratchet::scoped(&read.findings, at.only);
    let aside = match read.skipped {
        0 => String::new(),
        count => format!(" ({count} in inline Rust tests skipped)"),
    };
    let unit = kind.evaluator.unit;
    let said = read.files.coverage(at.only).said(out);
    let (prior, before, before_work) = at_the_base(kind, &spec, at, out)?;
    out.record(|records| records.work = Some(read.work + before_work));
    let lost = read.files.lost(&before, project, at.only);
    let code = kind.evaluator.evaluate(
        read.findings,
        prior,
        ratchet::accepted(&project.config, at.gate, kind.evaluator.metrics)?,
        at,
        &format!("OK: {sites} {unit} in the tree, all held at the base{aside}{said}"),
        out,
    );
    Ok(coverage::lost_said(&lost, at, code, out))
}

fn at_the_base(
    kind: &Kind,
    spec: &Spec,
    at: &Context,
    out: &mut Sink,
) -> Result<(Vec<Finding>, Files, ContentCost), Error> {
    let owned;
    let prior = match at.prior {
        Some(prior) => prior,
        None => {
            owned = base::own(at, out)?;
            &owned
        }
    };
    let project = at.project;
    let search = Search {
        scope: Scope::at_base(
            &project.config,
            kind.section,
            prior.root(),
            &spec.search.scope,
        ),
        ..spec.search.clone()
    };
    let before = findings(kind, &search, prior.tree(), prior.root(), None)?;
    let mut held = before.findings;
    held.retain(|finding| project.was_held(&finding.file));
    Ok((held, before.files, before.work))
}

fn context<'a>(kind: &'a Kind, args: &'a Args, project: &'a Project) -> Context<'a> {
    Context {
        only: args.only.as_deref(),
        strict: args.strict,
        quiet: args.quiet,
        ..Context::by_hand(kind.section, project)
    }
}

fn spec(kind: &Kind, project: &Project) -> Result<Spec, Error> {
    let values = project.config.policy(kind.section, kind.keys)?;
    let search = search(kind, &project.config, &values)?;
    if search.scope.has_in() && !applicable(kind, project.tree(), &search.scope)? {
        return Err(Error(format!(
            "{}: \"{}\" has an \"in\" scope with no applicable file",
            project.config.file.display(),
            kind.section
        )));
    }
    Ok(Spec { search })
}

fn list_languages(kind: &Kind, out: &mut String) {
    for language in kind.languages {
        let mut names: Vec<&str> = language.patterns.iter().map(|(name, _, _)| *name).collect();
        names.sort_unstable();
        let _ = writeln!(
            out,
            "{:<22} {}",
            language.names.join(", "),
            names.join(", ")
        );
    }
}

fn search(kind: &Kind, config: &Config, section: &Values) -> Result<Search, Error> {
    Ok(Search {
        sets: language_sets(kind, config)?,
        scope: Scope::read(config, kind.section, section)?,
        skip_rust_tests: skips_tests(kind, config, section)?,
    })
}

fn language_sets(kind: &Kind, config: &Config) -> Result<Vec<Set>, Error> {
    kind.languages
        .iter()
        .map(|set| {
            Ok(Set {
                shapes: kind.reads_shapes,
                suffixes: set.suffixes.iter().map(|s| s.to_string()).collect(),
                patterns: compiled(
                    kind,
                    config,
                    set.patterns.iter().map(|(name, regex, remedy)| {
                        (name.to_string(), regex.to_string(), remedy.to_string())
                    }),
                )?,
            })
        })
        .collect()
}

fn compiled(
    kind: &Kind,
    config: &Config,
    patterns: impl Iterator<Item = (String, String, String)>,
) -> Result<Vec<Pattern>, Error> {
    patterns
        .map(|(name, regex, remedy)| {
            Regex::new(&format!("(?m){regex}"))
                .map(|compiled| Pattern {
                    name: name.clone(),
                    regex: compiled,
                    remedy,
                })
                .map_err(|why| {
                    Error(format!(
                        "{}: \"{}\" pattern \"{name}\" is not a regular expression: {why}",
                        config.file.display(),
                        kind.section
                    ))
                })
        })
        .collect()
}

/// Whether this run leaves the inline Rust test modules out. A section whose check judges them
/// either way is refused the key, so nothing turns it on and measures the same set in silence.
fn skips_tests(kind: &Kind, config: &Config, section: &Values) -> Result<bool, Error> {
    let key = SKIP_RUST_TESTS.name;
    match section.get(key) {
        None => Ok(true),
        Some(_) if !kind.skips_tests => Err(Error(format!(
            "{}: \"{}\" names a \"{key}\", which it does not read — a stub inside an inline test \
             module is a stub, so this check judges one. Delete the key.",
            config.file.display(),
            kind.section
        ))),
        Some(value) => value
            .as_bool()
            .ok_or_else(|| config.malformed(kind.section, key, "true or false")),
    }
}

fn findings(
    kind: &Kind,
    search: &Search,
    tree: &Tree,
    repo_root: &Path,
    changes: Option<&[Change]>,
) -> Result<Read, Error> {
    let mut seen: BTreeMap<(String, String), Tally> = BTreeMap::new();
    let mut cache: BTreeMap<String, Skipped> = BTreeMap::new();
    let mut measured: BTreeSet<String> = BTreeSet::new();
    let mut excluded: BTreeSet<String> = BTreeSet::new();
    let mut shaped: BTreeSet<String> = BTreeSet::new();
    let mut skipped = 0;
    let mut work = ContentCost::default();
    let changed: Option<BTreeSet<&str>> =
        changes.map(|changes| changes.iter().map(|change| change.path.as_str()).collect());
    let suffixes: Vec<&str> = search
        .sets
        .iter()
        .flat_map(|set| set.suffixes.iter().map(String::as_str))
        .collect();
    let skip_dirs = files::default_skip_dirs();
    let wanted = files::Wanted {
        extensions: &suffixes,
        skip_dirs: &skip_dirs,
        exclude: &[],
        exclude_except: &[],
        skip_hidden: false,
    };
    for file in files::found(tree, &[repo_root.to_path_buf()], &wanted)?.kept {
        let rel = files::relative(&file, repo_root);
        if !search.scope.selects(&rel) {
            excluded.insert(rel);
            continue;
        }
        measured.insert(rel.clone());
        if changed
            .as_ref()
            .is_some_and(|changed| !changed.contains(rel.as_str()))
        {
            continue;
        }
        let bytes = std::fs::read(&file).map_err(|why| Error::unreadable(&file, why))?;
        work.reads += 1;
        let text = String::from_utf8_lossy(&bytes).to_string();
        if kind.skips_tests && search.skip_rust_tests && rel.ends_with(".rs") {
            work.parses += 1;
        }
        let past = cached(kind, search, &rel, &text, &mut cache);
        for set in search
            .sets
            .iter()
            .filter(|set| set.suffixes.iter().any(|end| rel.ends_with(end)))
        {
            skipped += tally(set, &rel, &text, &past, &mut seen);
            if set.shapes && shaped.insert(rel.clone()) {
                work.parses += 1;
                shapes(&rel, &text, &mut seen);
            }
        }
    }
    Ok(Read {
        findings: collected(kind, seen),
        skipped,
        files: Files {
            measured: measured.into_iter().collect(),
            not_measured: Vec::new(),
            excluded: excluded.into_iter().collect(),
            unreadable: Vec::new(),
        },
        work,
    })
}

fn applicable(kind: &Kind, tree: &Tree, scope: &Scope) -> Result<bool, Error> {
    Ok(tree.files()?.iter().any(|file| {
        scope.inside(file)
            && kind
                .languages
                .iter()
                .any(|language| language.suffixes.iter().any(|end| file.ends_with(end)))
    }))
}

/// The body shapes of one file's functions, which only a parser sees, recorded as sites. #114.
fn shapes(rel: &str, text: &str, seen: &mut BTreeMap<(String, String), Tally>) {
    for stub in syntax::convention::stubs(rel, text) {
        record(seen, rel, stub.line, &stub.text, stub.name, stub.remedy);
    }
}

fn cached(
    kind: &Kind,
    search: &Search,
    rel: &str,
    text: &str,
    cache: &mut BTreeMap<String, Skipped>,
) -> Skipped {
    cache
        .entry(rel.to_string())
        .or_insert_with(|| Skipped {
            tests: match kind.skips_tests && search.skip_rust_tests && rel.ends_with(".rs") {
                true => syntax::convention::test_module_ranges(rel, text),
                false => Vec::new(),
            },
            literals: match kind.skips_literals {
                true => literals(text),
                false => Vec::new(),
            },
        })
        .clone()
}

fn tally(
    set: &Set,
    rel: &str,
    text: &str,
    past: &Skipped,
    seen: &mut BTreeMap<(String, String), Tally>,
) -> u64 {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut skipped = 0;
    for pattern in &set.patterns {
        for found in pattern.regex.find_iter(text) {
            if quoted(past, found.range()) {
                continue;
            }
            let line = text[..found.start()].matches('\n').count() as u64 + 1;
            if past
                .tests
                .iter()
                .any(|(from, to)| *from <= line && line <= *to)
            {
                skipped += 1;
                continue;
            }
            let body = lines.get(line as usize - 1).unwrap_or(&"").trim();
            record(seen, rel, line, body, &pattern.name, &pattern.remedy);
        }
    }
    skipped
}

/// Whether one quoted span holds the whole match. A match that only starts in one, such as the
/// `//` of a URL in a string ahead of a real comment marker, is a site.
fn quoted(past: &Skipped, at: std::ops::Range<usize>) -> bool {
    past.literals
        .iter()
        .any(|(from, to)| *from <= at.start && at.end <= *to)
}

fn collected(kind: &Kind, seen: BTreeMap<(String, String), Tally>) -> Vec<Finding> {
    let mut out: Vec<Finding> = seen
        .into_iter()
        .map(|((file, text), tally)| {
            let mut values = Values::new();
            values.insert(kind.label.into(), tally.name.into());
            values.insert("count".into(), tally.count.into());
            if !tally.remedy.is_empty() {
                values.insert("remedy".into(), tally.remedy.into());
            }
            Finding {
                file,
                line: tally.line,
                text,
                values,
                body: None,
            }
        })
        .collect();
    out.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    out
}

fn record(
    seen: &mut BTreeMap<(String, String), Tally>,
    file: &str,
    line: u64,
    text: &str,
    name: &str,
    remedy: &str,
) {
    seen.entry((file.to_string(), text.to_string()))
        .and_modify(|tally| tally.count += 1)
        .or_insert(Tally {
            line,
            name: name.to_string(),
            remedy: remedy.to_string(),
            count: 1,
        });
}

/// The quoted spans of this text, so a match that starts inside one is not a site. A quote that
/// does not close on its own line opens nothing, which keeps an apostrophe in a comment from
/// swallowing the rest of the line.
///
/// ponytail: one scan, no grammar. A quote that opens on one line and closes on another, such
/// as a Python docstring, holds no span. Parse per language if a tree needs it.
fn literals(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        spans(line, at, &mut out);
        at += line.len();
    }
    out
}

fn spans(line: &str, offset: usize, out: &mut Vec<(usize, usize)>) {
    let bytes = line.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        let quote = bytes[at];
        if !matches!(quote, b'"' | b'\'' | b'`') {
            at += 1;
            continue;
        }
        match closing(bytes, at + 1, quote) {
            Some(end) => {
                out.push((offset + at, offset + end));
                at = end + 1;
            }
            None => at += 1,
        }
    }
}

fn closing(bytes: &[u8], from: usize, quote: u8) -> Option<usize> {
    let mut at = from;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => at += 2,
            b'\n' => return None,
            byte if byte == quote => return Some(at),
            _ => at += 1,
        }
    }
    None
}
