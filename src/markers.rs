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
use crate::project::{Project, Tests, Tree};
use crate::ratchet::{self, Evaluator, Finding, Line, Values};
use crate::reference::{self, Key};
use crate::scope::Scope;
use crate::syntax;

/// One row of a table: the name the report prints, the pattern to look for, and the remedy for
/// a site it matches. A row that names no remedy carries the empty string.
pub type Row = (&'static str, &'static str, &'static str);

/// One language's table: the names a config may call it by, the files it reads, its rows, and
/// the test idioms of its test code, which `skip_test_idioms` leaves out.
pub struct Language {
    pub names: &'static [&'static str],
    pub suffixes: &'static [&'static str],
    pub patterns: &'static [Row],
    pub test_idioms: Option<TestIdioms>,
}

/// The rows a test in one language writes on purpose, and where that language's test code is.
/// Every other row is judged in a test like anywhere else. #279, ADR 0049.
#[derive(Clone, Copy)]
pub struct TestIdioms {
    pub rows: &'static [&'static str],
    pub code: TestCode,
}

/// Where one language's test code is, for the test idioms left out there. Spec 8.2.
#[derive(Clone, Copy)]
pub enum TestCode {
    /// Rust test code: an inline `#[cfg(test)]` module, or a file under a test root.
    InlineModulesAndRoots,
    /// A test file of spec 5.4: one under a test root, or one a test directory segment or a
    /// test affix marks.
    Files,
}

impl TestCode {
    fn holds_file(self, tests: &Tests, rel: &str) -> bool {
        match self {
            TestCode::InlineModulesAndRoots => tests.root_holds(rel),
            TestCode::Files => tests.file_holds(rel),
        }
    }

    /// The line ranges of the test code inside one file, which only a parser sees, and none
    /// where the rule reads no text and parses nothing.
    fn ranges(self, rel: &str, text: &str) -> Option<Vec<(u64, u64)>> {
        match self {
            TestCode::InlineModulesAndRoots => {
                Some(syntax::convention::test_module_ranges(rel, text))
            }
            TestCode::Files => None,
        }
    }
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
    /// Whether a match that lies inside a string literal is thrown away.
    pub skips_literals: bool,
    /// Whether the function walk judges body shapes too, which only a parser can see. #114.
    pub reads_shapes: bool,
    /// Whether a Rust `cfg_attr` a pattern finds is a site only where it skips its test on every
    /// target, which only a parser can see. Spec 8.2.
    pub reads_cfg_attr: bool,
    /// The rows whose matches in one file are one site, keyed by the row and not by a line,
    /// because a comment is not a declaration. Spec 8.2.1, ADR 0064.
    pub counted: &'static [&'static str],
    pub evaluator: Evaluator<'static>,
}

/// The key only a kind whose languages name test idioms reads. A stub in a test is the case the
/// spec names, so `stubs` names none and does not read the key.
pub const SKIP_TEST_IDIOMS: Key = Key {
    name: "skip_test_idioms",
    holds: "whether the test idioms inside test code are left out: `unwrap` and `expect` in \
            Rust tests, `@ts-expect-error` in TypeScript and JavaScript test files",
    required: false,
    rule: None,
    default: "`true`",
    shape: crate::reference::Shape::Boolean,
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
    /// Whether `skip_test_idioms` leaves a match of this row out inside test code.
    test_idiom: bool,
    /// Whether this row's matches in one file are one site, keyed by the row.
    counted: bool,
}

impl Pattern {
    /// The text a match on this line is keyed by: the row for a counted row, which marks the
    /// line it matched, and the line's own text for any other.
    fn key<'a>(&'a self, rel: &str, line: u64, body: &'a str, marks: &mut Marks) -> &'a str {
        if !self.counted {
            return body;
        }
        marks
            .entry((rel.to_string(), self.name.clone()))
            .or_default()
            .push((line, body.to_string()));
        &self.name
    }
}

#[derive(Clone)]
struct Set {
    suffixes: Vec<String>,
    patterns: Vec<Pattern>,
    /// Whether the function walk judges the body shapes of the files this set reads. A set the
    /// project's own patterns make is not a language, so it names no shapes. #114.
    shapes: bool,
    cfg_attr: bool,
    test_code: Option<TestCode>,
}

impl Set {
    fn reads(&self, rel: &str) -> bool {
        self.suffixes.iter().any(|end| rel.ends_with(end))
    }

    /// Where this run leaves this set's test idioms out, and nowhere when it judges them.
    fn skipped_tests(&self, search: &Search) -> Option<TestCode> {
        self.test_code.filter(|_| search.skip_test_idioms)
    }

    /// Whether a match stands as a site. A Rust `cfg_attr` stands only where the grammar read
    /// it as skipping its test on every target. Spec 8.2.
    fn stands(&self, past: &Skipped, found: &regex::Match) -> bool {
        !self.cfg_attr
            || !found.as_str().ends_with("cfg_attr")
            || past.everywhere.contains(&found.start())
    }
}

struct Spec {
    search: Search,
}

#[derive(Clone)]
struct Search {
    sets: Vec<Set>,
    scope: Scope,
    skip_test_idioms: bool,
}

/// What one file says about where a test idiom does not count: the whole file when it is test
/// code, the test code inside it, and where a quoted span hides any match. It also
/// holds the byte each Rust `cfg_attr` that skips its test on every target starts at.
struct Skipped {
    test_file: bool,
    /// Whether finding the test code inside the file took a parse.
    parsed: bool,
    tests: Vec<(u64, u64)>,
    literals: Vec<(usize, usize)>,
    everywhere: Vec<usize>,
}

/// The line and trimmed text of each match a counted site holds, by the site's file and text.
type Marks = BTreeMap<(String, String), Vec<(u64, String)>>;

/// One tree read: the sites, how many test idioms test code took out of the count, and the
/// files the walk reached, which is what the gate's coverage counts.
struct Read {
    findings: Vec<Finding>,
    marks: Marks,
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

/// One tree walk: the tree's tests, and what the walk accumulates across its files. The
/// sites, the files whose shapes were read, the tally of idioms left out, and the reads and
/// parses the walk cost.
struct Walk {
    tests: Option<Tests>,
    seen: BTreeMap<(String, String), Tally>,
    marks: Marks,
    shaped: BTreeSet<String>,
    skipped: u64,
    work: ContentCost,
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
    let mut named = match values.get("count").and_then(Value::as_u64) {
        Some(count) if count > 1 => format!("{name} x{count}"),
        _ => name.to_string(),
    };
    if let Some(lines) = values.get("lines").and_then(Value::as_str) {
        let plural = if lines.contains(',') { "s" } else { "" };
        let _ = write!(named, ", new on line{plural} {lines}");
    }
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
        count => format!(" ({count} in tests skipped)"),
    };
    let unit = kind.evaluator.unit;
    let said = read.files.coverage(at.only).said(out);
    let before = at_the_base(kind, &spec, at, out)?;
    out.record(|records| records.work = Some(read.work + before.work));
    let lost = read.files.lost(&before.files, project, at.only);
    let code = kind.evaluator.evaluate(
        named(read.findings, &read.marks, &before.marks),
        before.findings,
        ratchet::accepted(&project.config, at.gate, kind.evaluator.metrics)?,
        at,
        Line {
            state: &format!("{sites} {unit} in the tree"),
            tail: &format!("{aside}{said}"),
        },
        out,
    );
    Ok(coverage::lost_said(&lost, at, code, out))
}

fn at_the_base(kind: &Kind, spec: &Spec, at: &Context, out: &mut Sink) -> Result<Read, Error> {
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
    let mut before = findings(kind, &search, prior.tree(), prior.root(), None)?;
    before
        .findings
        .retain(|finding| project.was_held(&finding.file));
    before.marks.retain(|(file, _), _| project.was_held(file));
    Ok(before)
}

/// Each counted site's lines whose text the base file lacks, one base line taken per match, so
/// a failure names them. The site moves to the first, and its values list them all. Spec 8.2.1.
fn named(mut findings: Vec<Finding>, now: &Marks, before: &Marks) -> Vec<Finding> {
    for finding in &mut findings {
        let key = (finding.file.clone(), finding.text.clone());
        let Some(marks) = now.get(&key) else {
            continue;
        };
        let mut held: Vec<&str> = before.get(&key).map_or_else(Vec::new, |was| {
            was.iter().map(|(_, text)| text.as_str()).collect()
        });
        let fresh: Vec<u64> = marks
            .iter()
            .filter(|(_, text)| match held.iter().position(|was| was == text) {
                Some(at) => {
                    held.swap_remove(at);
                    false
                }
                None => true,
            })
            .map(|(line, _)| *line)
            .collect();
        if let Some(first) = fresh.first() {
            finding.line = *first;
            let lines: Vec<String> = fresh.iter().map(u64::to_string).collect();
            finding
                .values
                .insert("lines".into(), lines.join(", ").into());
        }
    }
    findings.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    findings
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
        skip_test_idioms: skips_test_idioms(config, kind.section, section)?,
    })
}

fn language_sets(kind: &Kind, config: &Config) -> Result<Vec<Set>, Error> {
    debug_assert_eq!(
        kind.languages
            .iter()
            .any(|language| language.test_idioms.is_some()),
        kind.keys
            .iter()
            .any(|key| key.name == SKIP_TEST_IDIOMS.name),
        "{}: a language names test idioms exactly when the section reads the key",
        kind.section,
    );
    kind.languages
        .iter()
        .map(|set| {
            let idioms = set.test_idioms.map_or(&[][..], |idioms| idioms.rows);
            Ok(Set {
                shapes: kind.reads_shapes,
                cfg_attr: kind.reads_cfg_attr,
                test_code: set.test_idioms.map(|idioms| idioms.code),
                suffixes: set.suffixes.iter().map(|s| s.to_string()).collect(),
                patterns: compiled(
                    kind,
                    config,
                    idioms,
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
    idioms: &[&str],
    patterns: impl Iterator<Item = (String, String, String)>,
) -> Result<Vec<Pattern>, Error> {
    patterns
        .map(|(name, regex, remedy)| {
            Regex::new(&format!("(?m){regex}"))
                .map(|compiled| Pattern {
                    test_idiom: idioms.contains(&name.as_str()),
                    counted: kind.counted.contains(&name.as_str()),
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

/// Whether this run leaves the test idioms of test code out. A section whose languages name no
/// test idiom does not read the key, so the policy reader refuses it there.
fn skips_test_idioms(config: &Config, name: &str, section: &Values) -> Result<bool, Error> {
    let key = SKIP_TEST_IDIOMS.name;
    section.get(key).map_or(Ok(true), |value| {
        value
            .as_bool()
            .ok_or_else(|| config.malformed(name, key, "true or false"))
    })
}

fn findings(
    kind: &Kind,
    search: &Search,
    tree: &Tree,
    repo_root: &Path,
    changes: Option<&[Change]>,
) -> Result<Read, Error> {
    let mut measured: BTreeSet<String> = BTreeSet::new();
    let mut excluded: BTreeSet<String> = BTreeSet::new();
    let skips = search
        .sets
        .iter()
        .any(|set| set.skipped_tests(search).is_some());
    let mut walk = Walk::over(skips.then(|| tree.tests()));
    let changed: Option<BTreeSet<&str>> =
        changes.map(|changes| changes.iter().map(|change| change.path.as_str()).collect());
    let suffixes: Vec<&str> = search
        .sets
        .iter()
        .flat_map(|set| set.suffixes.iter().map(String::as_str))
        .collect();
    let wanted = files::Wanted {
        extensions: &suffixes,
        skip_dirs: &files::default_skip_dirs(),
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
        walk.read(kind, search, &file, &rel)?;
    }
    Ok(Read {
        findings: collected(kind, walk.seen),
        marks: walk.marks,
        skipped: walk.skipped,
        files: covered(measured, excluded),
        work: walk.work,
    })
}

fn covered(measured: BTreeSet<String>, excluded: BTreeSet<String>) -> Files {
    Files {
        measured: measured.into_iter().collect(),
        not_measured: Vec::new(),
        excluded: excluded.into_iter().collect(),
        unreadable: Vec::new(),
    }
}

impl Walk {
    fn over(tests: Option<Tests>) -> Walk {
        Walk {
            tests,
            seen: BTreeMap::new(),
            marks: Marks::new(),
            shaped: BTreeSet::new(),
            skipped: 0,
            work: ContentCost::default(),
        }
    }

    fn read(
        &mut self,
        kind: &Kind,
        search: &Search,
        file: &std::path::Path,
        rel: &str,
    ) -> Result<(), Error> {
        let bytes = std::fs::read(file).map_err(|why| Error::unreadable(file, why))?;
        self.work.reads += 1;
        let text = String::from_utf8_lossy(&bytes).to_string();
        for set in search.sets.iter().filter(|set| set.reads(rel)) {
            let tests = set.skipped_tests(search).zip(self.tests.as_ref());
            let past = skipped(kind, tests, rel, &text);
            if past.parsed {
                self.work.parses += 1;
            }
            self.skipped += tally(set, rel, &text, &past, &mut self.seen, &mut self.marks);
            if set.shapes && self.shaped.insert(rel.to_string()) {
                self.work.parses += 1;
                shapes(rel, &text, &mut self.seen);
            }
        }
        Ok(())
    }
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

fn skipped(kind: &Kind, tests: Option<(TestCode, &Tests)>, rel: &str, text: &str) -> Skipped {
    let ranges = tests.and_then(|(code, _)| code.ranges(rel, text));
    Skipped {
        test_file: tests.is_some_and(|(code, held)| code.holds_file(held, rel)),
        parsed: ranges.is_some(),
        tests: ranges.unwrap_or_default(),
        literals: match kind.skips_literals {
            true => literals(text),
            false => Vec::new(),
        },
        everywhere: match kind.reads_cfg_attr {
            true => syntax::convention::skipped_everywhere(rel, text),
            false => Vec::new(),
        },
    }
}

fn tally(
    set: &Set,
    rel: &str,
    text: &str,
    past: &Skipped,
    seen: &mut BTreeMap<(String, String), Tally>,
    marks: &mut Marks,
) -> u64 {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut skipped = 0;
    for pattern in &set.patterns {
        let stands = pattern
            .regex
            .find_iter(text)
            .filter(|found| set.stands(past, found));
        for found in stands {
            if quoted(past, found.range()) {
                continue;
            }
            let line = text[..found.start()].matches('\n').count() as u64 + 1;
            if pattern.test_idiom
                && (past.test_file
                    || past
                        .tests
                        .iter()
                        .any(|(from, to)| *from <= line && line <= *to))
            {
                skipped += 1;
                continue;
            }
            let body = lines.get(line as usize - 1).unwrap_or(&"").trim();
            let key = pattern.key(rel, line, body, marks);
            record(seen, rel, line, key, &pattern.name, &pattern.remedy);
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
