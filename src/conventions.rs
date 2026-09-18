//! The `conventions` check: the project's own rules. A convention names one thing the project
//! forbids — a literal `text`, a `code` pattern, or a `files` glob over paths — where it applies,
//! through `in` and `except`, and the `remedy` to take instead. Each convention is judged as its
//! own gate, `conventions/<name>`, so two conventions on one line are two findings and two debts.
//! A `text` or `code` site is a file and the text of the line a match starts on, and `count`
//! ratchets the matches that site holds. A `files` site is the path, at a count of 1. A site that
//! moves to another file is new, and a file git renamed keeps its sites, as spec 4.4 has it. The
//! scope and a `files` glob read the path each tree holds, so a file renamed out of `except`, or
//! into a glob, is new. The tree is walked once and each file read and parsed once, whatever the
//! number of conventions. The section is a person's policy, and the survey derives none of it.
//! Spec 8.4, ADR 0037.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::{Map, Value};

use crate::check::{Context, Sink};
use crate::config::{self, Config, Error};
use crate::coverage::Files;
use crate::project::{Project, Tree};
use crate::ratchet::{self, Evaluator, Finding, Line, Values};
use crate::reference::Key;
use crate::scope::{self, Selector};
use crate::syntax::pattern::{self, Pattern};
use crate::syntax::{self, Parsed, Unparsed};
use crate::{base, files};

mod report;

pub const SECTION: &str = "conventions";

const TEXT: Key = Key {
    name: "text",
    holds: "the literal text no line may hold, matched as written. Each convention states one of `text`, `code` and `files`",
    required: false,
    rule: None,
    default: "",
};

const CODE: Key = Key {
    name: "code",
    holds: "a code pattern no source may hold, with `$NAME` for one piece of code and `$$$ARGS` for a list. A fragment, such as a match arm or a type, is read everywhere the language holds one",
    required: false,
    rule: None,
    default: "",
};

const FILES: Key = Key {
    name: "files",
    holds: "a glob over repository-relative paths no file may sit at, where `*` stays inside one directory and `**/` crosses any number",
    required: false,
    rule: None,
    default: "",
};

const REMEDY: Key = Key {
    name: "remedy",
    holds: "the exact action to take instead, printed with every failure",
    required: true,
    rule: None,
    default: "",
};

const IN: Key = scope::IN;
const EXCEPT: Key = scope::EXCEPT;

const LANGUAGE: Key = Key {
    name: "language",
    holds: "the language a `code` pattern is written in",
    required: false,
    rule: None,
    default: "the one language the source in scope is written in",
};

pub const KEYS: &[Key] = &[TEXT, CODE, FILES, REMEDY, IN, EXCEPT, LANGUAGE];

const MATCHERS: [Key; 3] = [TEXT, CODE, FILES];

/// The keys a person writes for a convention that belong to another shape of rule, each with the
/// key a convention reads for the same intent, so a near miss names the key to use.
const INSTEAD: &[(&str, &str)] = &[
    ("exclude", EXCEPT.name),
    ("exceptions", EXCEPT.name),
    ("roots", IN.name),
    ("paths", IN.name),
    ("languages", LANGUAGE.name),
    ("pattern", CODE.name),
    ("regex", TEXT.name),
    ("glob", FILES.name),
];

const COUNT: &str = "count";
const METRICS: &[&str] = &[COUNT];
const CONVENTION: &str = "convention";
const LOGICAL: &str = "logical_gate";
const MATCHER: &str = "matcher";

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
    /// Judge only these repo-relative files, against only their sites at the base
    #[arg(long, num_args = 0.., value_name = "FILE")]
    only: Option<Vec<String>>,
    /// Summarize every convention, or explain the one named, then exit
    #[arg(long, value_name = "NAME", num_args = 0..=1)]
    report: Option<Option<String>>,
}

enum Matcher {
    Text(String),
    Code(String),
    Files(Regex),
}

struct Convention {
    name: String,
    matcher: Matcher,
    written: String,
    within: Vec<Selector>,
    except: Vec<Selector>,
    language: Option<&'static str>,
    remedy: String,
}

impl Convention {
    fn kind(&self) -> &'static str {
        match self.matcher {
            Matcher::Text(_) => TEXT.name,
            Matcher::Code(_) => CODE.name,
            Matcher::Files(_) => FILES.name,
        }
    }

    /// Whether `in` holds this path, before `except` takes anything out.
    fn selects(&self, file: &str) -> bool {
        self.within.is_empty() || scope::any_holds(&self.within, file)
    }

    fn applies(&self, file: &str) -> bool {
        self.selects(file) && !scope::any_holds(&self.except, file)
    }
}

/// A convention with its language resolved and its pattern compiled, which is what a tree is
/// measured under.
struct Rule<'a> {
    convention: &'a Convention,
    code: Option<Code>,
}

struct Code {
    language: &'static str,
    /// Whether the scope settled the language, rather than the convention pinning it.
    derived: bool,
    pattern: Pattern,
}

impl Rule<'_> {
    /// Whether this rule reads the file at all: a code rule reads its own language only.
    fn reads(&self, file: &str) -> bool {
        match &self.code {
            Some(code) => pattern::language_of(file) == Some(code.language),
            None => true,
        }
    }
}

/// Why a convention cannot run: its scope holds no language a code pattern is written in, or more
/// than one, or no place in its language reads the pattern. Each is for a person to settle, and
/// the gate and the report each say it in their own words. Spec 8.4.
enum Unresolved {
    NoLanguage,
    Languages(Vec<&'static str>),
    Unreadable {
        language: &'static str,
        unread: pattern::Unread,
    },
}

impl Unresolved {
    /// The configuration error a gate raises.
    fn said(&self, name: &str) -> String {
        match self {
            Unresolved::NoLanguage => format!(
                "convention \"{name}\" applies to no structural language — nothing in its scope is \
                 written in one of: {}\n\nAdd \"language\", or an \"in\" that holds that source.",
                pattern::languages().join(", ")
            ),
            Unresolved::Languages(found) => format!(
                "convention \"{name}\" applies to more than one structural language\n\n{}\n\nAdd \
                 \"language\": \"{}\" or narrow \"in\".",
                found
                    .iter()
                    .map(|language| format!("  {language}"))
                    .collect::<Vec<String>>()
                    .join("\n"),
                found.first().copied().unwrap_or_default()
            ),
            Unresolved::Unreadable { language, unread } => format!(
                "convention \"{name}\" has a \"code\" pattern that is not {language} code: {unread}"
            ),
        }
    }

    /// A few words for the report's summary.
    fn short(&self) -> &'static str {
        match self {
            Unresolved::NoLanguage => "No source in its scope.",
            Unresolved::Languages(_) => "More than one language in its scope.",
            Unresolved::Unreadable { .. } => "Can't read the pattern.",
        }
    }

    /// What the report's detail says, with what to do about it.
    fn told(&self) -> String {
        let called = |names: &[&str]| -> String {
            let called: Vec<&str> = names.iter().map(|name| pattern::called(name)).collect();
            joined(&called, "and")
        };
        match self {
            Unresolved::NoLanguage => format!(
                "Nothing in its scope is written in {}. Add \"language\", or an \"in\" path that \
                 holds that source.",
                joined(
                    &pattern::languages()
                        .into_iter()
                        .map(pattern::called)
                        .collect::<Vec<&str>>(),
                    "or"
                )
            ),
            Unresolved::Languages(found) => format!(
                "Its scope holds {}. Add \"language\": \"{}\", or narrow \"in\".",
                called(found),
                found.first().copied().unwrap_or_default()
            ),
            Unresolved::Unreadable { unread, .. } => format!(
                "klin can't read this pattern as {} code in any of these places: {}. Write one \
                 piece of code {} holds in one of them.",
                unread.language,
                joined(&unread.tried, "or"),
                unread.language
            ),
        }
    }
}

/// Items as a person lists them: `a`, `a and b`, `a, b, and c`.
fn joined(items: &[&str], conjunction: &str) -> String {
    match items {
        [] => String::new(),
        [only] => only.to_string(),
        [first, second] => format!("{first} {conjunction} {second}"),
        [rest @ .., last] => format!("{}, {conjunction} {last}", rest.join(", ")),
    }
}

/// One file a walk reached. `file` is the path a `text` or `code` site is keyed by, which is
/// today's, so a renamed file keeps its sites (spec 4.4). `held` is the path its own tree holds it
/// at, which the scope, the language and a `files` glob read. `path` is where it is on disk.
struct Place {
    file: String,
    held: String,
    path: PathBuf,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    if let Some(named) = &args.report {
        return report::run(named.as_deref(), args, start, out);
    }
    let project = Project::load(args.config.as_deref(), start)?;
    let at = Context {
        only: args.only.as_deref(),
        strict: args.strict,
        quiet: args.quiet,
        ..Context::by_hand(SECTION, &project)
    };
    gate(&at, &mut Sink::unrecorded(out))
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    let config = at.config();
    let conventions = conventions(config)?;
    let places = walked(config, at.project.tree())?;
    let rules = every_rule(config, &conventions, &places)?;
    let mut after = measure(&rules, &places)?;
    let mut before = at_the_base(&rules, at, out)?;
    let code = every_convention(config, &rules, (&mut after, &mut before), at, out)?;
    let code = holes_said(&holes(&conventions, &places), at, code, out);
    Ok(syntax::unread(&after.unparsed, at, code, out))
}

/// Every convention resolved, or the first one whose language or pattern a person must settle.
fn every_rule<'a>(
    config: &Config,
    conventions: &'a [Convention],
    places: &[Place],
) -> Result<Vec<Rule<'a>>, Error> {
    conventions
        .iter()
        .map(|convention| {
            resolved(convention, places).map_err(|why| {
                Error(format!(
                    "{}: {}",
                    config.file.display(),
                    why.said(&convention.name)
                ))
            })
        })
        .collect()
}

/// Each convention judged in name order, and the one `OK:` line with the coverage of the gate.
fn every_convention(
    config: &Config,
    rules: &[Rule],
    (after, before): (&mut Measured, &mut Measured),
    at: &Context,
    out: &mut Sink,
) -> Result<u8, Error> {
    let said = after.files.coverage(at.only).said(out);
    let mut code = 0;
    for rule in rules {
        code = code.max(judged(config, rule, (&mut *after, &mut *before), at, out)?);
    }
    if code == 0 && !at.quiet {
        let _ = writeln!(out.text, "OK: {} convention(s) judged{said}", rules.len());
    }
    Ok(code)
}

/// One convention judged as its own gate, against its own sites at the base and its own
/// accepted entries.
fn judged(
    config: &Config,
    rule: &Rule,
    (after, before): (&mut Measured, &mut Measured),
    at: &Context,
    out: &mut Sink,
) -> Result<u8, Error> {
    let name = &rule.convention.name;
    let gate = format!("{}/{name}", at.gate);
    let now = described(rule, &gate, after.take(name));
    let sites = ratchet::scoped(&now, at.only);
    let condition = format!("the convention {name} forbids");
    let evaluator = Evaluator {
        metrics: METRICS,
        unit: "site(s)",
        condition: &condition,
        fix_advice: &rule.convention.remedy,
        ceiling: None,
        format_metrics: show,
    };
    Ok(evaluator.evaluate(
        now,
        described(rule, &gate, before.take(name)),
        ratchet::accepted(config, &gate, METRICS)?,
        &Context { gate: &gate, ..*at },
        Line {
            state: &format!("{name}: {sites} site(s)"),
            tail: "",
        },
        out,
    ))
}

fn show(values: &Values) -> String {
    let count = values
        .get(COUNT)
        .and_then(Value::as_u64)
        .unwrap_or_default();
    match values.get(CONVENTION).and_then(Value::as_str) {
        Some(name) => format!("{name} x{count}"),
        None => format!("x{count}"),
    }
}

/// The findings with what a report and the JSON say about the rule behind them. None of these
/// values is ratcheted.
fn described(rule: &Rule, gate: &str, mut found: Vec<Finding>) -> Vec<Finding> {
    for finding in &mut found {
        let values = &mut finding.values;
        values.insert(CONVENTION.into(), rule.convention.name.clone().into());
        values.insert(LOGICAL.into(), gate.into());
        values.insert(MATCHER.into(), rule.convention.kind().into());
        if let Some(code) = &rule.code {
            values.insert(LANGUAGE.name.into(), code.language.into());
        }
        values.insert(REMEDY.name.into(), rule.convention.remedy.clone().into());
    }
    found
}

/// Every accepted entry for a convention must name one the section defines. A convention renamed
/// or removed retires its debt, and an entry left behind would otherwise hold nothing in silence
/// while the gate it names never runs. Refused before any gate runs. A section that is absent or
/// `false` runs no gate, so its entries wait for it, as an excluded gate's entries do. Spec 8.4.
pub fn no_stale_debt(file: &Path, data: &Value) -> Result<(), Error> {
    let Some(defined) = data.get(SECTION).and_then(Value::as_object) else {
        return Ok(());
    };
    let listed = data.get(config::ACCEPTED.name).and_then(Value::as_array);
    for entry in listed.into_iter().flatten() {
        let Some(gate) = entry.get("gate").and_then(Value::as_str) else {
            continue;
        };
        let Some(name) = gate
            .strip_prefix(SECTION)
            .and_then(|rest| rest.strip_prefix('/'))
        else {
            continue;
        };
        if !defined.contains_key(name) {
            return Err(Error(format!(
                "{}: the accepted entry for {gate} names no convention the \"{SECTION}\" section \
                 defines — renaming or removing a convention retires its debt, so delete the \
                 entry or restore the convention",
                file.display()
            )));
        }
    }
    Ok(())
}

fn conventions(config: &Config) -> Result<Vec<Convention>, Error> {
    let listed = config.required(SECTION)?.as_object().ok_or_else(|| {
        Error(format!(
            "{}: \"{SECTION}\" is an object of convention names, each with a \"remedy\" and one of: \
             text, code, files",
            config.file.display()
        ))
    })?;
    if listed.is_empty() {
        return Err(Error(format!(
            "{}: \"{SECTION}\" names no convention — write one, or set the section to false",
            config.file.display()
        )));
    }
    listed
        .iter()
        .map(|(name, rule)| {
            convention(name, rule).map_err(|why| {
                Error(format!(
                    "{}: convention \"{name}\" {why}",
                    config.file.display()
                ))
            })
        })
        .collect()
}

fn convention(name: &str, rule: &Value) -> Result<Convention, String> {
    if name.trim().is_empty() {
        return Err("has no name — the key is the convention's identity".into());
    }
    let fields = rule
        .as_object()
        .ok_or("must be an object with a \"remedy\" and one of: text, code, files")?;
    known(fields)?;
    let (matcher, written) = matcher(fields)?;
    let (within, except) = scope(fields)?;
    Ok(Convention {
        name: name.to_string(),
        language: language(fields, &matcher)?,
        matcher,
        written,
        within,
        except,
        remedy: remedy(fields)?,
    })
}

fn remedy(fields: &Map<String, Value>) -> Result<String, String> {
    fields
        .get(REMEDY.name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|remedy| !remedy.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "has no \"remedy\" — write the exact action to take instead".to_string())
}

fn scope(fields: &Map<String, Value>) -> Result<(Vec<Selector>, Vec<Selector>), String> {
    Ok((
        scope::selectors(fields, IN)?,
        scope::selectors(fields, EXCEPT)?,
    ))
}

/// A key a convention does not read would measure nothing, so it is refused, naming the key a
/// person most likely meant.
fn known(fields: &Map<String, Value>) -> Result<(), String> {
    let Some(unknown) = fields
        .keys()
        .find(|key| !KEYS.iter().any(|held| held.name == *key))
    else {
        return Ok(());
    };
    let candidates = || {
        KEYS.iter()
            .map(|key| (key.name, key.name))
            .chain(INSTEAD.iter().copied())
    };
    let meant = crate::config::nearest(unknown, candidates().map(|(near, _)| near))
        .and_then(|near| candidates().find(|(held, _)| *held == near));
    Err(match meant {
        Some((_, key)) => format!("has unknown field \"{unknown}\"\nDid you mean \"{key}\"?"),
        None => format!(
            "has unknown field \"{unknown}\" — a convention reads only: {}",
            KEYS.iter()
                .map(|key| key.name)
                .collect::<Vec<&str>>()
                .join(", ")
        ),
    })
}

/// The one matcher key a convention states.
fn chosen(fields: &Map<String, Value>) -> Result<&'static str, String> {
    let stated: Vec<&'static str> = MATCHERS
        .iter()
        .map(|key| key.name)
        .filter(|name| fields.contains_key(*name))
        .collect();
    let choose = "Choose exactly one of: text, code, files.";
    match stated.as_slice() {
        [one] => Ok(one),
        [] => Err(format!("defines none of: text, code, files\n{choose}")),
        [first, second, ..] => Err(format!(
            "defines both \"{first}\" and \"{second}\"\n{choose}"
        )),
    }
}

fn matcher(fields: &Map<String, Value>) -> Result<(Matcher, String), String> {
    let kind = chosen(fields)?;
    let written = fields
        .get(kind)
        .and_then(Value::as_str)
        .filter(|written| !written.trim().is_empty())
        .ok_or_else(|| format!("has a \"{kind}\" that is not a non-empty string"))?;
    let matcher = match kind {
        "text" if written.contains('\n') => {
            return Err(
                "has a \"text\" with a line break, and text is matched a line at a time".into(),
            );
        }
        "text" => Matcher::Text(written.to_string()),
        "code" => Matcher::Code(written.to_string()),
        _ => Matcher::Files(glob(written)?),
    };
    Ok((matcher, written.to_string()))
}

/// A `files` glob as the expression that matches whole repository-relative paths: `**/` crosses
/// any number of directories, `*` and `?` stay inside one, and `[...]` is a class.
fn glob(written: &str) -> Result<Regex, String> {
    if written.starts_with('/') {
        return Err(format!(
            "has a \"files\" glob \"{written}\" that is absolute — write it from the repository \
             root, such as \"**/scratch.*\""
        ));
    }
    let mut expression = String::from("^");
    let mut rest = written;
    while !rest.is_empty() {
        rest = step(rest, &mut expression).ok_or_else(|| {
            format!("has a \"files\" glob \"{written}\" with a \"[\" that does not close")
        })?;
    }
    expression.push('$');
    Regex::new(&expression)
        .map_err(|_| format!("has a \"files\" glob \"{written}\" klin cannot read"))
}

/// One token of a glob written out as an expression, and the glob left after it.
fn step<'a>(rest: &'a str, expression: &mut String) -> Option<&'a str> {
    if let Some(after) = rest.strip_prefix("**/") {
        expression.push_str("(?:.*/)?");
        return Some(after);
    }
    if let Some(after) = rest.strip_prefix("**") {
        expression.push_str(".*");
        return Some(after);
    }
    let next = rest.chars().next()?;
    let after = &rest[next.len_utf8()..];
    match next {
        '*' => expression.push_str("[^/]*"),
        '?' => expression.push_str("[^/]"),
        '[' => return class(after, expression),
        _ => expression.push_str(&regex::escape(next.encode_utf8(&mut [0; 4]))),
    }
    Some(after)
}

fn class<'a>(rest: &'a str, expression: &mut String) -> Option<&'a str> {
    let end = rest
        .char_indices()
        .skip(1)
        .find(|(_, next)| *next == ']')?
        .0;
    let (negated, members) = match rest.strip_prefix('!') {
        Some(members) => (true, &members[..end - 1]),
        None => (false, &rest[..end]),
    };
    expression.push('[');
    if negated {
        expression.push('^');
    }
    for member in members.chars() {
        if !member.is_alphanumeric() && member != '-' {
            expression.push('\\');
        }
        expression.push(member);
    }
    expression.push(']');
    Some(&rest[end + 1..])
}

fn language(
    fields: &Map<String, Value>,
    matcher: &Matcher,
) -> Result<Option<&'static str>, String> {
    let Some(stated) = fields.get(LANGUAGE.name) else {
        return Ok(None);
    };
    if !matches!(matcher, Matcher::Code(_)) {
        return Err(
            "sets \"language\" on a rule that is not \"code\" — only a code pattern is written in \
             a language"
                .into(),
        );
    }
    let named = stated.as_str().unwrap_or_default();
    let known = pattern::languages();
    match known.iter().find(|name| **name == named) {
        Some(name) => Ok(Some(name)),
        None => Err(format!(
            "names language \"{named}\", which no code pattern is written in — one of: {}",
            known.join(", ")
        )),
    }
}

/// Every file the walk every gate shares reaches under a tree, by its repository-relative path:
/// no skipped directory, no hidden directory, nothing git ignores, and no symbolic link. The
/// configuration is not one of them: it states each convention, so it holds every literal a
/// convention forbids.
fn walked(config: &Config, tree: &Tree) -> Result<Vec<Place>, Error> {
    let stated = files::relative(&config.file, config.root());
    Ok(tree
        .files()?
        .iter()
        .filter(|file| {
            let (parents, _) = file.rsplit_once('/').unwrap_or(("", file));
            !parents.split('/').any(|segment| segment.starts_with('.'))
        })
        .map(|file| Place {
            held: file.clone(),
            file: file.clone(),
            path: tree.root().join(file),
        })
        .filter(|place| place.file != stated && !place.file.split('/').any(files::skipped))
        .collect())
}

fn resolved<'a>(convention: &'a Convention, places: &[Place]) -> Result<Rule<'a>, Unresolved> {
    let Matcher::Code(written) = &convention.matcher else {
        return Ok(Rule {
            convention,
            code: None,
        });
    };
    let (language, derived) = match convention.language {
        Some(named) => (named, false),
        None => (language_in_scope(convention, places)?, true),
    };
    let pattern = Pattern::compile(written, language)
        .map_err(|unread| Unresolved::Unreadable { language, unread })?;
    Ok(Rule {
        convention,
        code: Some(Code {
            language,
            derived,
            pattern,
        }),
    })
}

/// The one language the source in a convention's scope is written in. Which grammar reads the
/// pattern plays no part: two languages, or none, is a question for a person. Spec 8.4.
fn language_in_scope(
    convention: &Convention,
    places: &[Place],
) -> Result<&'static str, Unresolved> {
    let found: BTreeSet<&'static str> = places
        .iter()
        .filter(|place| convention.applies(&place.file))
        .filter_map(|place| pattern::language_of(&place.file))
        .collect();
    let found: Vec<&'static str> = found.into_iter().collect();
    match found.as_slice() {
        [one] => Ok(one),
        [] => Err(Unresolved::NoLanguage),
        _ => Err(Unresolved::Languages(found)),
    }
}

/// The sites one tree holds: each site's first line and its count, by convention.
type Tally = BTreeMap<(String, String), (u64, u64)>;

/// One tree measured under the rules: each convention's findings by name, the files the rules
/// read, the files the grammar refused, and how many files each convention read.
#[derive(Default)]
struct Measured {
    findings: BTreeMap<String, Vec<Finding>>,
    files: Files,
    unparsed: Vec<Unparsed>,
    read: BTreeMap<String, usize>,
}

impl Measured {
    fn take(&mut self, name: &str) -> Vec<Finding> {
        self.findings.remove(name).unwrap_or_default()
    }
}

fn measure(rules: &[Rule], places: &[Place]) -> Result<Measured, Error> {
    let mut tallies: Vec<Tally> = rules.iter().map(|_| Tally::new()).collect();
    let mut measured = Measured::default();
    for rule in rules {
        measured.read.insert(rule.convention.name.clone(), 0);
    }
    for place in places {
        visit(rules, place, &mut tallies, &mut measured)?;
    }
    for (rule, tally) in rules.iter().zip(tallies) {
        let name = rule.convention.name.clone();
        measured.findings.insert(name, findings(tally));
    }
    Ok(measured)
}

/// Where one file lands: read by the rules that apply to it, taken out by an `except`, or in no
/// rule's scope at all.
fn visit(
    rules: &[Rule],
    place: &Place,
    tallies: &mut [Tally],
    measured: &mut Measured,
) -> Result<(), Error> {
    let reading = reading(rules, &place.held, &mut measured.read);
    let files = &mut measured.files;
    if !reading.is_empty() {
        let judged = one(rules, &reading, place, tallies, &mut measured.unparsed)?;
        let into = match judged {
            true => &mut files.measured,
            false => &mut files.unreadable,
        };
        into.push(place.file.clone());
    } else if rules
        .iter()
        .any(|rule| rule.convention.selects(&place.held) && rule.reads(&place.held))
    {
        files.excluded.push(place.file.clone());
    }
    Ok(())
}

/// One file read by every rule that applies to it: its bytes read once, and its grammar run once
/// for every code rule. Whether the rules judged it, which a file only code rules read and the
/// grammar refused was not.
fn one(
    rules: &[Rule],
    reading: &[usize],
    place: &Place,
    tallies: &mut [Tally],
    unparsed: &mut Vec<Unparsed>,
) -> Result<bool, Error> {
    let of = |wanted: fn(&Matcher) -> bool| -> Vec<usize> {
        reading
            .iter()
            .copied()
            .filter(|at| wanted(&rules[*at].convention.matcher))
            .collect()
    };
    let texts = of(|matcher| matches!(matcher, Matcher::Text(_)));
    let codes = of(|matcher| matches!(matcher, Matcher::Code(_)));
    globbed(rules, reading, &place.held, tallies);
    if texts.is_empty() && codes.is_empty() {
        return Ok(true);
    }
    let path = &place.path;
    let bytes = std::fs::read(path).map_err(|why| Error::unreadable(path, why))?;
    let source = String::from_utf8_lossy(&bytes);
    if !bytes.contains(&0) {
        lines(rules, &texts, &place.file, &source, tallies);
    }
    let parsed = codes.is_empty() || coded(rules, &codes, (place, &source), tallies, unparsed)?;
    Ok(parsed || codes.len() < reading.len())
}

/// Every `files` rule that applies to the path and whose glob matches it, which reads nothing of
/// the file.
fn globbed(rules: &[Rule], reading: &[usize], file: &str, tallies: &mut [Tally]) {
    for at in reading {
        if let Matcher::Files(glob) = &rules[*at].convention.matcher
            && glob.is_match(file)
        {
            tally(&mut tallies[*at], file, 0, file, 1);
        }
    }
}

/// Every literal match on every line, which a text rule counts whatever the line is: a comment
/// and a string are text too.
fn lines(rules: &[Rule], texts: &[usize], file: &str, source: &str, tallies: &mut [Tally]) {
    for (row, line) in source.lines().enumerate() {
        for at in texts {
            let Matcher::Text(literal) = &rules[*at].convention.matcher else {
                continue;
            };
            let count = line.matches(literal.as_str()).count() as u64;
            if count > 0 {
                tally(&mut tallies[*at], file, row as u64 + 1, line.trim(), count);
            }
        }
    }
}

/// Every code rule's matches in one parse of the file, and whether the grammar read it.
fn coded(
    rules: &[Rule],
    codes: &[usize],
    (place, source): (&Place, &str),
    tallies: &mut [Tally],
    unparsed: &mut Vec<Unparsed>,
) -> Result<bool, Error> {
    let file = place.file.as_str();
    let parsed = match syntax::parse(&place.held, source)? {
        Some(Parsed::Read(parsed)) => parsed,
        Some(Parsed::Rejected(refused)) => {
            unparsed.push(refused);
            return Ok(false);
        }
        None => return Ok(true),
    };
    let patterns: Vec<&Pattern> = codes
        .iter()
        .filter_map(|at| rules[*at].code.as_ref().map(|code| &code.pattern))
        .collect();
    let lines = parsed.lines();
    for (at, rows) in codes.iter().zip(pattern::rows(&parsed, &patterns)) {
        for row in rows {
            let text = syntax::line_at(&lines, row);
            tally(&mut tallies[*at], file, row as u64 + 1, &text, 1);
        }
    }
    Ok(true)
}

fn tally(held: &mut Tally, file: &str, line: u64, text: &str, count: u64) {
    held.entry((file.to_string(), text.to_string()))
        .and_modify(|site| site.1 += count)
        .or_insert((line, count));
}

fn findings(tally: Tally) -> Vec<Finding> {
    let mut out: Vec<Finding> = tally
        .into_iter()
        .map(|((file, text), (line, count))| {
            let mut values = Values::new();
            values.insert(COUNT.into(), count.into());
            Finding {
                file,
                line,
                text,
                values,
                body: None,
            }
        })
        .collect();
    out.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    out
}

/// The base tree measured under the same rules. The base's copy of a renamed file is laid out at
/// today's path, so its sites keep their key, and its `held` path is the one the base commit holds,
/// so the scope and a `files` glob read what the base held. Spec 8.4.
fn at_the_base(rules: &[Rule], at: &Context, out: &mut Sink) -> Result<Measured, Error> {
    let project = at.project;
    let owned;
    let (prior, commit) = match (at.prior, at.base) {
        (Some(prior), Some(commit)) => (prior, commit.to_string()),
        _ => {
            let window = base::announced(project.root(), at, out)?;
            owned = base::materialize(project, &window.before, None)?;
            (&owned, window.before)
        }
    };
    let was: BTreeMap<String, String> = project
        .changes(&commit)?
        .iter()
        .filter_map(|change| {
            let was = change.was.as_ref().filter(|was| **was != change.path)?;
            Some((change.path.clone(), was.clone()))
        })
        .collect();
    let places: Vec<Place> = walked(&project.config, prior.tree())?
        .into_iter()
        .map(|place| Place {
            held: was
                .get(&place.file)
                .cloned()
                .unwrap_or_else(|| place.file.clone()),
            ..place
        })
        .collect();
    measure(rules, &places)
}

/// An `in` or `except` path the working tree holds nothing at, which is most often a path written
/// wrong.
struct Hole<'a> {
    convention: &'a str,
    key: &'static str,
    path: &'a str,
}

impl Hole<'_> {
    fn said(&self) -> String {
        format!(
            "has an \"{}\" path \"{}\" that names nothing in the tree",
            self.key, self.path
        )
    }

    fn named(&self) -> String {
        format!("convention \"{}\" {}", self.convention, self.said())
    }
}

fn holes<'a>(conventions: &'a [Convention], places: &[Place]) -> Vec<Hole<'a>> {
    let mut out = Vec::new();
    for convention in conventions {
        for (key, listed) in [(IN, &convention.within), (EXCEPT, &convention.except)] {
            let empty = listed
                .iter()
                .filter(|at| !places.iter().any(|place| at.holds(&place.file)));
            out.extend(empty.map(|path| Hole {
                convention: &convention.name,
                key: key.name,
                path: path.as_str(),
            }));
        }
    }
    out
}

/// What a gate does about its holes. An `except` path that names nothing takes nothing out, so it
/// is a NOTE. An `in` path that names nothing leaves its convention measuring nothing there, so it
/// is exit 2, and a NOTE in the hook, where the agent cannot edit the configuration. Spec 8.4.
fn holes_said(holes: &[Hole], at: &Context, code: u8, out: &mut Sink) -> u8 {
    let (refused, noted): (Vec<&Hole>, Vec<&Hole>) = holes
        .iter()
        .partition(|hole| hole.key == IN.name && !at.hook());
    let noted: Vec<(String, String)> = noted
        .iter()
        .map(|hole| (String::new(), hole.named()))
        .collect();
    ratchet::noted(&noted, out);
    for hole in &refused {
        let _ = writeln!(
            out.text,
            "FAIL: {} — correct the path, or take it out of \"in\".",
            hole.named()
        );
    }
    match refused.is_empty() {
        true => code,
        false => 2,
    }
}

/// The rules that read this file, each of them counting it once.
fn reading(rules: &[Rule], file: &str, read: &mut BTreeMap<String, usize>) -> Vec<usize> {
    let reading: Vec<usize> = (0..rules.len())
        .filter(|at| rules[*at].convention.applies(file) && rules[*at].reads(file))
        .collect();
    for at in &reading {
        *read.entry(rules[*at].convention.name.clone()).or_default() += 1;
    }
    reading
}
