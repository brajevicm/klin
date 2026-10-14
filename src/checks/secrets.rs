//! `secrets`: a literal shaped like a live vendor token. A site is one file plus a redacted
//! fingerprint of the token, the vendor prefix and a hash, and never the token or its line, so
//! no output, journal record or accepted entry holds the key. Its ratcheted value is `count`,
//! the matches of that token in the file. Nothing is derived. Spec 9.1, 16.3, B.8.2.1.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::checks::markers;
use crate::config::key::{self, Key};
use crate::config::scope::{self, Scope};
use crate::contract;
use crate::contract::check::{ContentCost, Context, Counted, Lay, Line, Sink};
use crate::contract::coverage::Files;
use crate::contract::holes;
use crate::contract::project::Project;
use crate::contract::ratchet::{self, Evaluator, Finding, Remedy};
use crate::facts::files;
use crate::facts::tree::Tree;
use crate::syntax;
use crate::sys::changed::Change;
use crate::sys::error::Error;
use crate::sys::record::Values;

pub const SECTION: &str = "secrets";

/// The keys this section reads, which `klin policy --reference` prints. Spec 5.4, 5.8.
pub const KEYS: &[Key] = &[scope::IN, scope::EXCEPT];

const LABEL: &str = "secret";

/// One vendor's token: the name a report gives it, its prefix, and the rest at its usual length.
struct Vendor {
    name: &'static str,
    prefix: &'static str,
    rest: &'static str,
}

const VENDORS: &[Vendor] = &[
    Vendor {
        name: "OpenAI project key",
        prefix: "sk-proj-",
        rest: r"[A-Za-z0-9_-]{40,}",
    },
    Vendor {
        name: "Anthropic key",
        prefix: "sk-ant-",
        rest: r"[a-z]+[0-9]{2}-[A-Za-z0-9_-]{80,}",
    },
    Vendor {
        name: "Stripe live key",
        prefix: "sk_live_",
        rest: r"[A-Za-z0-9]{24,}",
    },
    Vendor {
        name: "GitHub token",
        prefix: "ghp_",
        rest: r"[A-Za-z0-9]{36}\b",
    },
    Vendor {
        name: "GitHub fine-grained token",
        prefix: "github_pat_",
        rest: r"[A-Za-z0-9_]{82}\b",
    },
    Vendor {
        name: "AWS access key",
        prefix: "AKIA",
        rest: r"[A-Z0-9]{16}\b",
    },
    Vendor {
        name: "Slack token",
        prefix: "xox",
        rest: r"[abposr]-[0-9]{8,}-[A-Za-z0-9-]{20,}",
    },
    Vendor {
        name: "Google API key",
        prefix: "AIza",
        rest: r"[A-Za-z0-9_-]{35}",
    },
    Vendor {
        name: "GitLab token",
        prefix: "glpat-",
        rest: r"[A-Za-z0-9_-]{20,}",
    },
];

/// A token's prefix and a hash of the whole token, which holds no part of the key after the
/// prefix. It is the text of a site, so no output names more of the key than this.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Fingerprint(String);

impl Fingerprint {
    fn of(vendor: &Vendor, key: &str) -> Fingerprint {
        let hash = ratchet::site_id(SECTION, vendor.name, key);
        Fingerprint(format!("{}...{hash}", vendor.prefix))
    }
}

/// The words a documented example or a test token holds, which no live key does.
const PLACEHOLDERS: &[&str] = &["test", "xxx", "example"];

/// One pattern for every vendor, so a file is scanned once, with one group per vendor in table
/// order.
fn token() -> Result<Regex, Error> {
    let vendors: Vec<String> = VENDORS
        .iter()
        .map(|vendor| format!("({}{})", regex::escape(vendor.prefix), vendor.rest))
        .collect();
    Regex::new(&format!(r"\b(?:{})", vendors.join("|"))).map_err(|why| {
        Error(format!(
            "the {SECTION} vendor table is not a regular expression: {why}"
        ))
    })
}

const EVALUATOR: Evaluator = Evaluator {
    metrics: &["count"],
    unit: "secret(s)",
    condition: "where the code holds a live credential",
    fix_advice: Remedy::Fixed(
        "Take the key out of the code and read it from the environment or a secret store. \
         Revoke it at the vendor and issue a new one, because it may already sit in history, a \
         log or a transcript. Accepting a key is a decision for a person, in the config, in a \
         reviewed commit.",
    ),
    ceiling: None,
    format_metrics: show,
    nested: None,
    rides_along: ratchet::every_row_blocks,
};

/// Every language name this section reads, with the extensions each selects. It also reads each
/// `.env` file the tree does not ignore, which is no language.
pub fn language_extensions() -> Vec<(&'static str, String)> {
    key::extensions_by_name(
        syntax::REGISTRY
            .iter()
            .map(|row| ([row.name], syntax::suffixes(row.id))),
    )
}

/// How many files the secrets gate judges in the working tree. Spec 11.6.
pub fn judges(project: &Project) -> Result<usize, Error> {
    Ok(selected(&read_scope(project)?, project.tree())?.kept.len())
}

struct Read {
    findings: Vec<Finding>,
    files: Files,
    work: ContentCost,
}

impl contract::check::Based for Read {
    fn keep_held(&mut self, was_held: impl Fn(&str) -> bool) {
        self.findings.retain(|finding| was_held(&finding.file));
    }
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    let project = at.project;
    let scope = read_scope(project)?;
    let token = token()?;
    let read = findings(&token, &scope, project.tree(), project.root(), at.changes)?;
    let before =
        contract::check::base_findings(at, Lay::Announced, (SECTION, &scope), |prior, scope| {
            findings(&token, &scope, prior.tree(), prior.root(), None)
        })?;
    let said = out.covered(&read.files.coverage(at.only));
    out.record(|records| records.work = Some(read.work + before.work));
    let sites = ratchet::scoped(&read.findings, at.only);
    let code = EVALUATOR.evaluate(
        read.findings,
        before.findings,
        ratchet::accepted(&project.config, at.gate, EVALUATOR.metrics)?,
        at,
        Line::new(
            Counted::Markers {
                sites,
                unit: EVALUATOR.unit,
                skipped: 0,
            },
            said,
        ),
        out,
    );
    holes::closed((&read.files, &before.files), &[], at.only, at, out);
    Ok(code)
}

fn read_scope(project: &Project) -> Result<Scope, Error> {
    let values = project.config.policy(SECTION, KEYS)?;
    let scope = Scope::read(&project.config, project.moves(), SECTION, &values)?;
    let applies = |file: &String| scope.inside(file) && reads(file);
    if scope.has_in() && !project.tree().files()?.iter().any(applies) {
        return Err(Error(format!(
            "{}: \"{SECTION}\" has an \"in\" scope with no applicable file",
            project.config.file.display()
        )));
    }
    Ok(scope)
}

/// Whether the check has anything to read: a source root, or a `.env` file the tree lists, which
/// no survey counts as source.
pub fn applies(project: &Project) -> bool {
    !project.found_no_source_root()
        || project
            .tree()
            .files()
            .is_ok_and(|files| files.iter().any(|file| env(file)))
}

/// Whether the check reads a file at this path: a source file of a language klin knows, or a
/// `.env` file.
fn reads(path: &str) -> bool {
    static SUFFIXES: LazyLock<Vec<&str>> = LazyLock::new(|| {
        syntax::REGISTRY
            .iter()
            .flat_map(|row| syntax::suffixes(row.id))
            .collect()
    });
    env(path) || SUFFIXES.iter().any(|end| path.ends_with(end))
}

fn env(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name == ".env" || name.starts_with(".env.")
}

fn selected(scope: &Scope, tree: &Tree) -> Result<files::Found, Error> {
    let mut found = tree.scoped(&[""], false, |path| scope.selects(path))?;
    for held in [&mut found.kept, &mut found.excluded] {
        held.retain(|file| reads(&file.to_string_lossy()));
    }
    Ok(found)
}

fn findings(
    token: &Regex,
    scope: &Scope,
    tree: &Tree,
    root: &Path,
    changes: Option<&[Change]>,
) -> Result<Read, Error> {
    let found = selected(scope, tree)?;
    let changed: Option<BTreeSet<&str>> =
        changes.map(|all| all.iter().map(|change| change.path.as_str()).collect());
    let mut seen: BTreeMap<(String, Fingerprint), Tally> = BTreeMap::new();
    let mut measured = Vec::new();
    let mut work = ContentCost::default();
    for file in &found.kept {
        let rel = files::relative(file, root);
        if changed
            .as_ref()
            .is_none_or(|changed| changed.contains(rel.as_str()))
        {
            let bytes = std::fs::read(file).map_err(|why| Error::unreadable(file, why))?;
            work.reads += 1;
            tally(token, &rel, &String::from_utf8_lossy(&bytes), &mut seen);
        }
        measured.push(rel);
    }
    let findings = seen
        .into_iter()
        .map(|((file, Fingerprint(text)), tally)| {
            let mut values = Values::new();
            values.insert(LABEL.into(), tally.vendor.name.into());
            values.insert("count".into(), tally.count.into());
            Finding {
                file,
                line: tally.line,
                text,
                values,
                body: None,
            }
        })
        .collect();
    let files = Files {
        measured,
        not_measured: Vec::new(),
        excluded: found
            .excluded
            .iter()
            .map(|file| files::relative(file, root))
            .collect(),
        unreadable: Vec::new(),
    };
    Ok(Read {
        findings,
        files,
        work,
    })
}

/// One token's site in one file: the vendor, the line of its first match, and how many times
/// the file holds it.
struct Tally {
    vendor: &'static Vendor,
    line: u64,
    count: u64,
}

/// Each live-shaped token in one file, at the site of its fingerprint. Matches come in order, so
/// one cursor counts the lines once over the whole file.
fn tally(token: &Regex, rel: &str, text: &str, seen: &mut BTreeMap<(String, Fingerprint), Tally>) {
    let (mut at_byte, mut at_line) = (0, 1);
    for found in token.captures_iter(text) {
        let Some((at, key)) = found
            .iter()
            .skip(1)
            .enumerate()
            .find_map(|(at, group)| Some((at, group?)))
        else {
            continue;
        };
        if placeholder(key.as_str()) || text[key.end()..].starts_with("...") {
            continue;
        }
        at_line += text[at_byte..key.start()].matches('\n').count() as u64;
        at_byte = key.start();
        let vendor = &VENDORS[at];
        seen.entry((rel.to_string(), Fingerprint::of(vendor, key.as_str())))
            .and_modify(|tally| tally.count += 1)
            .or_insert(Tally {
                vendor,
                line: at_line,
                count: 1,
            });
    }
}

fn placeholder(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    PLACEHOLDERS.iter().any(|word| lower.contains(word))
}

fn show(values: &Values) -> String {
    markers::show(LABEL, values)
}
