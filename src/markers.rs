use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::Value;
use tree_sitter::{Node, Parser};

use crate::base;
use crate::config::{Config, Error, Flags};
use crate::coverage::{self, Files};
use crate::files;
use crate::ratchet::{self, Evaluator, Finding, Values};

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
    /// The values key a matched row's name is recorded under.
    pub label: &'static str,
    /// Whether a site inside an inline Rust test module is one the config may skip. A stub in a
    /// test is the case the spec names, so `stubs` never skips one.
    pub skips_tests: bool,
    /// Whether a match that lies inside a string literal is thrown away.
    pub skips_literals: bool,
    pub evaluator: Evaluator<'static>,
}

const EVERY_FILE: &str = "";
const PRELUDE: &[&str] = &[
    "attribute_item",
    "line_comment",
    "block_comment",
    "doc_comment",
];

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
}

struct Spec {
    search: Search,
    roots: Vec<PathBuf>,
}

#[derive(Clone)]
struct Search {
    sets: Vec<Set>,
    skip_dirs: Vec<String>,
    exclude: Vec<String>,
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
    evaluate(kind, &flags(kind, args), start, out)
}

pub fn gate(kind: &Kind, flags: &Flags, start: &Path, out: &mut String) -> Result<u8, Error> {
    evaluate(kind, flags, start, out)
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

/// Every suffix a built-in pattern set reads, so a survey can find a tree's sources.
pub fn suffixes(kind: &'static Kind) -> impl Iterator<Item = &'static str> {
    kind.languages
        .iter()
        .flat_map(|language| language.suffixes.iter().copied())
}

pub fn holds(kind: &'static Kind, file: &str) -> Option<&'static Language> {
    kind.languages.iter().find(|language| {
        language
            .suffixes
            .iter()
            .any(|suffix| file.ends_with(suffix))
    })
}

fn evaluate(kind: &Kind, flags: &Flags, start: &Path, out: &mut String) -> Result<u8, Error> {
    let config = Config::open(flags, start)?;
    let spec = spec(kind, &config)?;
    config.say(flags, kind.section, out);
    let read = findings(kind, &spec.search, &spec.roots, config.root())?;
    let sites = ratchet::scoped(&read.findings, flags.only.as_deref());
    let aside = match read.skipped {
        0 => String::new(),
        count => format!(" ({count} in inline Rust tests skipped)"),
    };
    let unit = kind.evaluator.unit;
    let said = read.files.coverage(flags.only.as_deref()).said(flags);
    let (prior, before) = at_the_base(kind, &config, &spec, flags, out)?;
    let lost = read.files.lost(&before, &config, flags.only.as_deref());
    let code = kind.evaluator.evaluate(
        read.findings,
        prior,
        ratchet::accepted(&config, &flags.gate, kind.evaluator.metrics)?,
        flags,
        &format!("OK: {sites} {unit} in the tree, all held at the base{aside}{said}"),
        out,
    );
    Ok(coverage::lost_said(&lost, flags, code, out))
}

fn at_the_base(
    kind: &Kind,
    config: &Config,
    spec: &Spec,
    flags: &Flags,
    out: &mut String,
) -> Result<(Vec<Finding>, Files), Error> {
    let owned;
    let prior = match flags.prior.as_deref() {
        Some(dir) => dir,
        None => {
            owned = base::own(config, flags, out)?;
            owned.root()
        }
    };
    let search = Search {
        exclude: files::base_exclusions(config, kind.section, prior, &spec.search.exclude),
        ..spec.search.clone()
    };
    let before = findings(
        kind,
        &search,
        &base::roots(&spec.roots, config, prior)?,
        prior,
    )?;
    let mut held = before.findings;
    held.retain(|finding| config.was_held(&finding.file));
    Ok((held, before.files))
}

fn flags(kind: &Kind, args: &Args) -> Flags {
    Flags {
        config: args.config.clone(),
        gate: kind.section.to_string(),
        prior: None,
        base: None,
        quiet: args.quiet,
        context: !args.quiet,
        strict: args.strict,
        hook: false,
        only: args.only.clone(),
        records: None,
        with: None,
    }
}

fn spec(kind: &Kind, config: &Config) -> Result<Spec, Error> {
    let section = ratchet::section(config, kind.section)?;
    let values = &section.values;
    Ok(Spec {
        search: search(kind, section.config, values)?,
        roots: files::roots(section.config, section.name, values, "roots")?
            .unwrap_or_else(|| vec![section.config.root().to_path_buf()]),
    })
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
        sets: sets(kind, config, section)?,
        skip_dirs: files::skip_dirs(config, kind.section, section)?,
        exclude: files::strings(config, kind.section, section, "exclude")?,
        skip_rust_tests: skips_tests(kind, config, section)?,
    })
}

fn sets(kind: &Kind, config: &Config, section: &Values) -> Result<Vec<Set>, Error> {
    let named = files::strings(config, kind.section, section, "languages")?;
    let mut sets = language_sets(kind, config, &named)?;
    let project = project_patterns(kind, config, section)?;
    if named.is_empty() && project.is_empty() {
        return Err(Error(format!(
            "{}: \"{}\" names no \"languages\" and no \"patterns\" — nothing to look for",
            config.file.display(),
            kind.section
        )));
    }
    if !project.is_empty() {
        sets.push(project_set(kind, config, &sets, project)?);
    }
    Ok(sets)
}

/// One set per language, however many names the config gives it. Two names for one set, such as
/// javascript and typescript, would otherwise read every file twice and double every count.
fn language_sets(kind: &Kind, config: &Config, named: &[String]) -> Result<Vec<Set>, Error> {
    let mut wanted: Vec<&'static Language> = Vec::new();
    for name in named {
        let set = language(kind, name).ok_or_else(|| unknown_language(kind, config, name))?;
        if !wanted.iter().any(|held| std::ptr::eq(*held, set)) {
            wanted.push(set);
        }
    }
    wanted
        .into_iter()
        .map(|set| {
            Ok(Set {
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

fn language(kind: &Kind, name: &str) -> Option<&'static Language> {
    kind.languages
        .iter()
        .find(|language| language.names.contains(&name))
}

fn project_set(
    kind: &Kind,
    config: &Config,
    sets: &[Set],
    project: Vec<(String, String, String)>,
) -> Result<Set, Error> {
    let mut suffixes: Vec<String> = sets.iter().flat_map(|set| set.suffixes.clone()).collect();
    suffixes.sort();
    suffixes.dedup();
    if suffixes.is_empty() {
        suffixes.push(EVERY_FILE.to_string());
    }
    Ok(Set {
        suffixes,
        patterns: compiled(kind, config, project.into_iter())?,
    })
}

fn unknown_language(kind: &Kind, config: &Config, name: &str) -> Error {
    let known: Vec<&str> = kind
        .languages
        .iter()
        .flat_map(|language| language.names.iter().copied())
        .collect();
    Error(format!(
        "{}: \"{}\" names no built-in patterns for \"{name}\" — one of: {}",
        config.file.display(),
        kind.section,
        known.join(", ")
    ))
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
    let key = "skip_rust_tests";
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

fn project_patterns(
    kind: &Kind,
    config: &Config,
    section: &Values,
) -> Result<Vec<(String, String, String)>, Error> {
    let Some(listed) = section.get("patterns") else {
        return Ok(Vec::new());
    };
    let malformed = || {
        config.malformed(
            kind.section,
            "patterns",
            "an object of name to regex, or to a {\"match\", \"remedy\"} pair",
        )
    };
    listed
        .as_object()
        .ok_or_else(malformed)?
        .iter()
        .map(|(name, stated)| match stated {
            Value::String(regex) => Ok((name.clone(), regex.clone(), String::new())),
            Value::Object(pair) => Ok((
                name.clone(),
                pair.get("match")
                    .and_then(Value::as_str)
                    .ok_or_else(malformed)?
                    .to_string(),
                pair.get("remedy")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            )),
            _ => Err(malformed()),
        })
        .collect()
}

fn findings(
    kind: &Kind,
    search: &Search,
    roots: &[PathBuf],
    repo_root: &Path,
) -> Result<Read, Error> {
    let mut seen: BTreeMap<(String, String), Tally> = BTreeMap::new();
    let mut cache: BTreeMap<String, Skipped> = BTreeMap::new();
    let mut measured: BTreeSet<String> = BTreeSet::new();
    let mut excluded: BTreeSet<String> = BTreeSet::new();
    let mut skipped = 0;
    for set in &search.sets {
        let suffixes: Vec<&str> = set.suffixes.iter().map(String::as_str).collect();
        let wanted = files::Wanted {
            extensions: &suffixes,
            skip_dirs: &search.skip_dirs,
            exclude: &search.exclude,
            exclude_except: &[],
            skip_hidden: false,
        };
        let found = files::found(roots, &wanted)?;
        excluded.extend(
            found
                .excluded
                .iter()
                .map(|file| files::relative(file, repo_root)),
        );
        for file in found.kept {
            let bytes = std::fs::read(&file).map_err(|why| Error::unreadable(&file, why))?;
            let text = String::from_utf8_lossy(&bytes).to_string();
            let rel = files::relative(&file, repo_root);
            let past = cached(kind, search, &rel, &text, &mut cache);
            skipped += tally(set, &rel, &text, &past, &mut seen);
            measured.insert(rel);
        }
    }
    Ok(Read {
        findings: collected(kind, seen),
        skipped,
        files: Files {
            measured: measured.into_iter().collect(),
            excluded: excluded.into_iter().collect(),
            unreadable: Vec::new(),
        },
    })
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
                true => rust_test_ranges(text),
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
            record(seen, rel, line, body, pattern);
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
    pattern: &Pattern,
) {
    seen.entry((file.to_string(), text.to_string()))
        .and_modify(|tally| tally.count += 1)
        .or_insert(Tally {
            line,
            name: pattern.name.clone(),
            remedy: pattern.remedy.clone(),
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

fn rust_test_ranges(source: &str) -> Vec<(u64, u64)> {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .is_err()
    {
        return Vec::new();
    }
    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    test_ranges(tree.root_node(), source.as_bytes(), &mut out);
    out
}

fn test_ranges(node: Node, source: &[u8], out: &mut Vec<(u64, u64)>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "attribute_item" && is_cfg_test(child, source) {
            out.push((
                child.start_position().row as u64 + 1,
                item_after(child).end_position().row as u64 + 1,
            ));
            continue;
        }
        test_ranges(child, source, out);
    }
}

fn item_after(attribute: Node) -> Node {
    let mut node = attribute;
    while let Some(next) = node.next_named_sibling() {
        node = next;
        if !PRELUDE.contains(&node.kind()) {
            break;
        }
    }
    node
}

fn is_cfg_test(node: Node, source: &[u8]) -> bool {
    node.utf8_text(source)
        .is_ok_and(|text| text.split_whitespace().collect::<String>() == "#[cfg(test)]")
}
