//! `klin conventions --report`: what the conventions forbid and what they match now, for a person.
//! The summary gives each convention one line, with the one thing to act on first, and
//! `--report <name>` explains one convention. It reads the configuration and writes nothing.
//! Spec 8.4.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

use crate::check::{Context, Sink};
use crate::config::{Config, Error};
use crate::ratchet::{self, Finding};
use crate::syntax::{Unparsed, pattern};

use super::{
    Args, Code, Convention, Hole, IN, METRICS, Matcher, Measured, Place, Rule, SECTION, Unresolved,
    at_the_base, conventions, holes, joined, measure, resolved, walked,
};

/// Each site now, with what the ratchet makes of it, or `""` when no base resolved.
type Sites = Vec<(Finding, &'static str)>;

/// Every convention's sites by name, or why no base resolved to judge them against.
type Compared = Result<BTreeMap<String, Sites>, Error>;

/// The outcomes a person acts on, and the words the summary counts them in.
const FRESH: &[(&str, &str)] = &[("new", "new"), ("worsened", "worse")];

/// The outcomes the base or the accepted list carries.
const KEPT: &[(&str, &str)] = &[("held", "held"), ("accepted", "accepted")];

/// What the report says of one convention beyond what it states.
struct Explained<'a> {
    convention: &'a Convention,
    rule: Option<&'a Rule<'a>>,
    problem: Option<&'a Unresolved>,
    /// The `in` and `except` paths that match nothing in the tree.
    empty: Vec<&'a Hole<'a>>,
    /// The files in scope that klin could not parse.
    unparsed: Vec<String>,
    read: usize,
    sites: Sites,
}

pub(super) fn run(
    named: Option<&str>,
    args: &Args,
    start: &Path,
    out: &mut String,
) -> Result<u8, Error> {
    let config = Config::load(args.config.as_deref(), start)?;
    let conventions = conventions(&config)?;
    let places = walked(&config, config.root())?;
    let (rules, problems) = partitioned(&conventions, &places);
    let mut after = measure(&rules, &places)?;
    let at = Context::by_hand(SECTION, start, args.config.as_deref());
    let mut window = String::new();
    let compared = statuses(
        &config,
        &rules,
        &after,
        &at,
        &mut Sink::unrecorded(&mut window),
    );
    let holes = holes(&conventions, &places);
    let explained: Vec<Explained> = conventions
        .iter()
        .map(|convention| {
            explanation(
                convention,
                (&rules, &problems),
                &holes,
                &compared,
                &mut after,
            )
        })
        .collect();
    match named {
        None => Ok(summary(&explained, &compared, out)),
        Some(name) => detail(&config, name, &explained, (&window, &compared), out),
    }
}

/// The conventions that resolved, and why each of the rest did not.
fn partitioned<'a>(
    conventions: &'a [Convention],
    places: &[Place],
) -> (Vec<Rule<'a>>, BTreeMap<&'a str, Unresolved>) {
    let mut rules = Vec::new();
    let mut problems = BTreeMap::new();
    for convention in conventions {
        match resolved(convention, places) {
            Ok(rule) => rules.push(rule),
            Err(why) => {
                problems.insert(convention.name.as_str(), why);
            }
        }
    }
    (rules, problems)
}

fn statuses(
    config: &Config,
    rules: &[Rule],
    after: &Measured,
    at: &Context,
    out: &mut Sink,
) -> Compared {
    let mut before = at_the_base(config, rules, at, out)?;
    let mut judged = BTreeMap::new();
    for rule in rules {
        let name = &rule.convention.name;
        let gate = format!("{SECTION}/{name}");
        let now = after.findings.get(name).cloned().unwrap_or_default();
        let sites = ratchet::outcomes(
            now,
            before.take(name),
            ratchet::accepted(config, &gate, METRICS)?,
            METRICS,
        );
        judged.insert(name.clone(), sites);
    }
    Ok(judged)
}

fn explanation<'a>(
    convention: &'a Convention,
    (rules, problems): (&'a [Rule<'a>], &'a BTreeMap<&str, Unresolved>),
    holes: &'a [Hole<'a>],
    compared: &Compared,
    after: &mut Measured,
) -> Explained<'a> {
    let rule = rules
        .iter()
        .find(|rule| std::ptr::eq(rule.convention, convention));
    Explained {
        convention,
        rule,
        problem: problems.get(convention.name.as_str()),
        empty: holes
            .iter()
            .filter(|hole| hole.convention == convention.name)
            .collect(),
        unparsed: unparsed_in(convention, rule, &after.unparsed),
        read: after
            .read
            .get(&convention.name)
            .copied()
            .unwrap_or_default(),
        sites: sites_of(compared, after, &convention.name),
    }
}

/// The files in a code convention's scope that klin could not parse.
fn unparsed_in(convention: &Convention, rule: Option<&Rule>, unparsed: &[Unparsed]) -> Vec<String> {
    let parses = rule.filter(|rule| rule.code.is_some());
    unparsed
        .iter()
        .filter(|file| parses.is_some_and(|rule| rule.reads(&file.file)))
        .filter(|file| convention.applies(&file.file))
        .map(|file| file.file.clone())
        .collect()
}

/// One convention's sites now, with each one's outcome when a base resolved and none when it
/// did not.
fn sites_of(compared: &Compared, after: &mut Measured, name: &str) -> Sites {
    match compared {
        Ok(judged) => judged.get(name).cloned().unwrap_or_default(),
        Err(_) => after
            .take(name)
            .into_iter()
            .map(|finding| (finding, ""))
            .collect(),
    }
}

/// One line of counts, one row per convention, and where the detail is.
fn summary(explained: &[Explained], compared: &Compared, out: &mut String) -> u8 {
    let _ = writeln!(out, "{}", headline(explained));
    if let Err(why) = compared {
        let _ = writeln!(
            out,
            "Can't compare with the base, so no site has an outcome: {why}"
        );
    }
    let rows: Vec<(&str, String, String)> = explained
        .iter()
        .map(|one| {
            let (status, place) = status(one);
            (one.convention.name.as_str(), status, place)
        })
        .collect();
    let named = widest(rows.iter().map(|row| row.0)) + 4;
    let said = widest(rows.iter().map(|row| row.1.as_str())) + 4;
    let _ = writeln!(out);
    for (name, status, place) in &rows {
        let row = format!("{name:<named$}{status:<said$}{place}");
        let _ = writeln!(out, "{}", row.trim_end());
    }
    let _ = writeln!(out, "\nFor details, run: klin conventions --report <name>");
    exit(explained)
}

fn widest<'a>(texts: impl Iterator<Item = &'a str>) -> usize {
    texts
        .map(|text| text.chars().count())
        .max()
        .unwrap_or_default()
}

fn exit(explained: &[Explained]) -> u8 {
    match explained.iter().any(|one| one.problem.is_some()) {
        true => 2,
        false => 0,
    }
}

/// How many conventions, and what needs a person: new sites, sites that got worse, and
/// conventions that cannot run.
fn headline(explained: &[Explained]) -> String {
    let outcome = |wanted: &str| {
        explained
            .iter()
            .flat_map(|one| &one.sites)
            .filter(|site| site.1 == wanted)
            .count()
    };
    let (new, worse) = (outcome("new"), outcome("worsened"));
    let stuck = explained.iter().filter(|one| one.problem.is_some()).count();
    let parts: Vec<String> = [
        (new > 0).then(|| counted(new, "new site", "new sites")),
        (worse > 0).then(|| format!("{} got worse", counted(worse, "site", "sites"))),
        (stuck > 0).then(|| format!("{stuck} can't run")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let conventions = counted(explained.len(), "convention", "conventions");
    match parts.is_empty() {
        true => format!("{conventions}, nothing new"),
        false => format!("{conventions}: {}", parts.join(", ")),
    }
}

fn counted(count: usize, one: &str, many: &str) -> String {
    match count {
        1 => format!("1 {one}"),
        _ => format!("{count} {many}"),
    }
}

/// The one thing to act on first for a convention, and where it is when it has a place.
fn status(one: &Explained) -> (String, String) {
    if let Some(problem) = one.problem {
        return (problem.short().to_string(), String::new());
    }
    let fresh = tallied(&one.sites, FRESH);
    if !fresh.is_empty() {
        return (fresh, located(&one.sites));
    }
    if one.empty.iter().any(|hole| hole.key == IN.name) {
        return (
            "Its \"in\" path matches nothing.".to_string(),
            String::new(),
        );
    }
    if !one.unparsed.is_empty() {
        let files = counted(one.unparsed.len(), "file", "files");
        return (format!("Can't parse {files}."), String::new());
    }
    let kept = tallied(&one.sites, KEPT);
    match (kept.is_empty(), one.sites.len()) {
        (false, _) => (kept, String::new()),
        (true, 0) => ("Clear".to_string(), String::new()),
        (true, count) => (counted(count, "site", "sites"), located(&one.sites)),
    }
}

/// Each outcome these sites hold, counted: `1 new, 2 worse`.
fn tallied(sites: &Sites, outcomes: &[(&str, &str)]) -> String {
    outcomes
        .iter()
        .filter_map(|(outcome, said)| {
            let count = sites.iter().filter(|site| site.1 == *outcome).count();
            (count > 0).then(|| format!("{count} {said}"))
        })
        .collect::<Vec<String>>()
        .join(", ")
}

/// Where the first site to act on is, and how many more there are.
fn located(sites: &Sites) -> String {
    let fresh: Vec<&Finding> = sites
        .iter()
        .filter(|site| FRESH.iter().any(|(outcome, _)| *outcome == site.1))
        .map(|site| &site.0)
        .collect();
    let listed = match fresh.is_empty() {
        true => sites.iter().map(|site| &site.0).collect(),
        false => fresh,
    };
    match listed.as_slice() {
        [] => String::new(),
        [only] => location(only),
        [first, rest @ ..] => format!("{} and {} more", location(first), rest.len()),
    }
}

fn location(finding: &Finding) -> String {
    format!("{}:{}", finding.file, finding.line)
}

/// One convention in full: what it forbids and where, how klin reads it, what it cannot see, its
/// sites, and the fix.
fn detail(
    config: &Config,
    name: &str,
    explained: &[Explained],
    (window, compared): (&str, &Compared),
    out: &mut String,
) -> Result<u8, Error> {
    let Some(one) = explained.iter().find(|one| one.convention.name == name) else {
        return Err(unknown(config, name, explained));
    };
    let _ = match compared {
        Ok(_) => write!(out, "{window}"),
        Err(why) => writeln!(
            out,
            "Can't compare with the base, so no site has an outcome: {why}"
        ),
    };
    let _ = writeln!(out, "\n{name}\n");
    for line in sentences(config, one) {
        let _ = writeln!(out, "{line}");
    }
    if one.problem.is_none() {
        let _ = writeln!(out);
        table(&one.sites, out);
    }
    let _ = writeln!(out, "\nFix: {}", one.convention.remedy);
    Ok(exit(std::slice::from_ref(one)))
}

fn unknown(config: &Config, name: &str, explained: &[Explained]) -> Error {
    let names: Vec<&str> = explained
        .iter()
        .map(|one| one.convention.name.as_str())
        .collect();
    Error(format!(
        "{}: no convention is named \"{name}\" — the conventions are {}",
        config.file.display(),
        joined(&names, "and")
    ))
}

/// The sentences above a convention's sites.
fn sentences(config: &Config, one: &Explained) -> Vec<String> {
    let mut out = vec![forbids(one)];
    if let Some(code) = one.rule.and_then(|rule| rule.code.as_ref()) {
        out.push(read_as(config, code));
    }
    out.extend(one.empty.iter().map(|hole| {
        format!(
            "The \"{}\" path {} matches nothing in the tree.",
            hole.key, hole.path
        )
    }));
    out.extend(
        one.unparsed
            .iter()
            .map(|file| format!("klin can't parse {file}, so nothing in it was measured.")),
    );
    out.extend(one.problem.map(Unresolved::told));
    out
}

fn forbids(one: &Explained) -> String {
    let convention = one.convention;
    let written = &convention.written;
    let what = match convention.matcher {
        Matcher::Text(_) => format!("the text \"{written}\""),
        Matcher::Code(_) => written.clone(),
        Matcher::Files(_) => format!("files matching {written}"),
    };
    let files = match one.problem {
        Some(_) => String::new(),
        None => format!(" ({})", counted(one.read, "file", "files")),
    };
    format!("Forbids {what} in {}{files}.", scope(convention))
}

fn scope(convention: &Convention) -> String {
    let names = |paths: &[String]| {
        let names: Vec<&str> = paths.iter().map(String::as_str).collect();
        joined(&names, "and")
    };
    let within = match convention.within.is_empty() {
        true => "the repository".to_string(),
        false => names(&convention.within),
    };
    match convention.except.is_empty() {
        true => within,
        false => format!("{within} except {}", names(&convention.except)),
    }
}

/// What a code pattern reads as, in which language, and how the language was settled.
fn read_as(config: &Config, code: &Code) -> String {
    let settled = match code.derived {
        true => "derived from the files in scope".to_string(),
        false => format!(
            "pinned in {}",
            config.file.file_name().map_or_else(
                || config.file.display().to_string(),
                |name| { name.to_string_lossy().to_string() }
            )
        ),
    };
    format!(
        "Reads as {} in {}. The language is {settled}.",
        joined(&code.pattern.readings(), "or"),
        pattern::called(code.language)
    )
}

/// Each site with its place, its line and its outcome, in aligned columns.
fn table(sites: &Sites, out: &mut String) {
    if sites.is_empty() {
        let _ = writeln!(out, "No matches.");
        return;
    }
    let rows: Vec<(String, String, &str)> = sites
        .iter()
        .map(|(finding, outcome)| {
            let line: String = finding.text.chars().take(60).collect();
            (location(finding), line, shown(outcome))
        })
        .collect();
    let place = widest(rows.iter().map(|row| row.0.as_str()));
    let text = widest(rows.iter().map(|row| row.1.as_str()));
    for (at, line, outcome) in &rows {
        let row = format!("  {at:<place$}   {line:<text$}   {outcome}");
        let _ = writeln!(out, "{}", row.trim_end());
    }
}

fn shown(outcome: &str) -> &'static str {
    match outcome {
        "new" => "New",
        "worsened" => "Worse",
        "held" => "Held",
        "accepted" => "Accepted",
        _ => "",
    }
}
