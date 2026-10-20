//! `klin status`: the configuration, each host integration, the state directory and the local
//! window, read without running a check and without writing anything. Spec 11.4, 11.7.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::cli::setup::{self, Integration};
use crate::config::file::{self, Config, Discovered};
use crate::engine::catalogue;
use crate::sys::error::Error;
use crate::sys::release::{self, Expected};
use crate::window::stamp::{Here, Verdict};
use crate::{hook::journal, hook::turn, sys::state, window::stamp};

const NO_REPOSITORY: &str = "klin status reads a repository, and this is no git repository.";

#[derive(clap::Args)]
pub struct Args {
    /// Print one JSON object instead of the text
    #[arg(long)]
    json: bool,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let found = Discovered::from(start);
    let Some(root) = found.root.as_deref() else {
        return Err(Error(NO_REPOSITORY.to_string()));
    };
    let expected = file::expected(start);
    release::refresh(expected.as_ref());
    let latest = release::known();
    let document = document(&found, root, start, latest.as_ref(), expected.as_ref());
    match args.json {
        true => {
            let _ = writeln!(out, "{document}");
        }
        false => {
            let newer = latest.filter(release::Version::is_newer);
            let unmet = expected.filter(|expected| !expected.met());
            text(
                &document,
                &found.said(),
                newer.as_ref(),
                unmet.as_ref(),
                out,
            );
        }
    }
    Ok(0)
}

fn document(
    found: &Discovered,
    root: &Path,
    start: &Path,
    latest: Option<&release::Version>,
    expected: Option<&Expected>,
) -> Value {
    let error = found
        .config
        .as_ref()
        .and_then(|_| Config::load(None, start, &catalogue::sections()).err());
    let at = state::dir(root);
    let (lines, _) = journal::read(root);
    json!({
        "schema_version": 1,
        "command": "status",
        "klin": {"version": env!("CARGO_PKG_VERSION"), "latest_known": latest.map(ToString::to_string),
            "update_available": latest.is_some_and(release::Version::is_newer),
            "auto_update": release::auto_update(),
            "updated_from": release::updated_from().map(|from| from.to_string()),
            "expected": expected.map(ToString::to_string),
            "expected_met": expected.is_none_or(Expected::met),
        },
        "config": {
            "path": found.config.as_ref().map(|file| file.display().to_string()),
            "present": found.config.is_some(),
            "valid": error.is_none(),
            "error": error.map(|error| error.to_string()),
            "ignored": found.ignored_paths(),
        },
        "integrations": setup::integrations(root).iter().map(integration).collect::<Vec<Value>>(),
        "state_dir": at.as_ref().map(|at| at.display().to_string()),
        "cache_dir": at.as_ref().map(|at| at.join(state::CACHE).display().to_string()),
        "window": at.as_deref().and_then(|at| window(root, at, &lines)),
        "last_stop": last_stop(&lines),
    })
}

fn integration(one: &Integration) -> Value {
    json!({
        "host": one.host,
        "scope": one.scope.as_str(),
        "route": one.route.as_str(),
        "state": one.state.as_str(),
        "detail": one.detail,
    })
}

/// The local window the stamp holds: its verdict, its age, why it is red, unjudged or aborted,
/// the default-branch ref the advisory rules read, the last advisory Stop, and the notices of
/// the window that only the journal holds. It never claims that the working tree passes.
/// Spec 6.6, 10.7, 11.4.
fn window(root: &Path, at: &Path, lines: &[Value]) -> Option<Value> {
    let held = stamp::read(at)?;
    let here = Here::read(root, None);
    let mut window = json!({
        "verdict": held.verdict.name(),
        "age_seconds": known(held.time).map(|time| now().saturating_sub(time)),
        "open": [],
        "open_sites": [],
        "unasked": [],
        "error": null,
        "aborted_since": null,
        "default_branch": here.default.map(|(name, _)| name),
        "remotes": here.remotes,
        "last_advisory": last_advisory(lines),
        "notices": journal::open_notices(lines, held.commit.as_deref())
            .into_iter()
            .map(|notice| json!({"time": notice.time, "message": notice.text}))
            .collect::<Vec<Value>>(),
    });
    match held.verdict {
        Verdict::Red { open, unasked } => {
            window["open_sites"] = sites_of(lines, &open).into();
            window["open"] = open.into();
            window["unasked"] = unasked.into();
        }
        Verdict::Unjudged { error, .. } => window["error"] = error.into(),
        Verdict::Aborted { since } => window["aborted_since"] = known(since).into(),
        Verdict::Pending | Verdict::Green => {}
    }
    Some(window)
}

/// Where the newest Stop that recorded each open finding id found it, as file and line, or the
/// id where no Stop the journal holds recorded it. Findings that share an id take that Stop's
/// sites in order, so each keeps its own line. Spec 11.4.
fn sites_of(lines: &[Value], open: &[String]) -> Vec<String> {
    let found = newest_sites(lines, open.iter().map(String::as_str).collect());
    let mut taken: HashMap<&str, usize> = HashMap::new();
    open.iter()
        .map(|id| {
            let nth = taken.entry(id.as_str()).or_default();
            let sites = found
                .get(id.as_str())
                .map(Vec::as_slice)
                .unwrap_or_default();
            let site = sites.get(*nth).or(sites.last()).cloned();
            *nth += 1;
            site.unwrap_or_else(|| id.clone())
        })
        .collect()
}

/// Every site of each wanted id in the newest Stop that recorded it. One pass from the newest
/// line, which stops once every id has its sites.
fn newest_sites<'a>(lines: &'a [Value], wanted: HashSet<&str>) -> HashMap<&'a str, Vec<String>> {
    let mut found: HashMap<&str, Vec<String>> = HashMap::new();
    let stops = lines.iter().rev().filter(|line| line["kind"] == "stop");
    for findings in stops.filter_map(|line| line["result"]["findings"].as_array()) {
        if found.len() == wanted.len() {
            break;
        }
        let mut here: HashMap<&str, Vec<String>> = HashMap::new();
        for finding in findings {
            if let Some(id) = finding["id"].as_str().filter(|id| wanted.contains(id)) {
                here.entry(id).or_default().push(site(finding, id));
            }
        }
        for (id, sites) in here {
            found.entry(id).or_insert(sites);
        }
    }
    found
}

fn site(finding: &Value, id: &str) -> String {
    match (word(finding, "file"), finding["line"].as_u64()) {
        ("", _) => id.to_string(),
        (file, Some(line)) if line > 0 => format!("{file}:{line}"),
        (file, _) => file.to_string(),
    }
}

/// The last advisory Stop the journal records and its reason, also one that lost the lock and
/// so wrote no fresh stamp. Spec 11.4, 13.1.
fn last_advisory(lines: &[Value]) -> Value {
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "stop" && line["advisory"].is_string())
        .map_or(
            Value::Null,
            |line| json!({"time": line["time"], "reason": line["advisory"]}),
        )
}

/// The last Stop the journal records, which is history and not the tree as it stands.
fn last_stop(lines: &[Value]) -> Value {
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "stop")
        .map_or(
            Value::Null,
            |line| json!({"time": line["time"], "verdict": line["verdict"], "historical": true}),
        )
}

fn text(
    document: &Value,
    notes: &[String],
    newer: Option<&release::Version>,
    unmet: Option<&Expected>,
    out: &mut String,
) {
    let _ = writeln!(out, "klin {}", word(&document["klin"], "version"));
    if let Some(expected) = unmet {
        let _ = writeln!(out, "{}", expected.note(release::route(), None));
    }
    if let Some(tag) = newer {
        let news = release::News::Available(tag.clone());
        let _ = writeln!(
            out,
            "{}",
            release::Audience::Person.note(release::route(), &news, None)
        );
        if let Some(why) = release::refused(tag) {
            let _ = writeln!(out, "klin could not install it on its own: {why}");
        }
    }
    let config = &document["config"];
    match (config["path"].as_str(), config["error"].as_str()) {
        (Some(path), None) => {
            let _ = writeln!(out, "config: {path}, valid");
        }
        (Some(path), Some(error)) => {
            let _ = writeln!(out, "config: {path}, invalid: {error}");
        }
        (None, _) => {}
    }
    for note in notes {
        let _ = writeln!(out, "{note}");
    }
    integrations_text(document, out);
    local_text(document, out);
}

fn integrations_text(document: &Value, out: &mut String) {
    let rows = document["integrations"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    if rows.is_empty() {
        let _ = writeln!(out, "integration: none installed, and no host proven here");
    }
    for row in rows {
        let [host, scope, route, state, detail] =
            ["host", "scope", "route", "state", "detail"].map(|key| word(row, key));
        let _ = writeln!(
            out,
            "integration: {host} {scope} {route}: {state} — {detail}"
        );
    }
}

/// The state directory, the cache, the local window and the last Stop.
fn local_text(document: &Value, out: &mut String) {
    for (label, key) in [("state", "state_dir"), ("cache", "cache_dir")] {
        let _ = writeln!(out, "{label}: {}", document[key].as_str().unwrap_or("none"));
    }
    let window = &document["window"];
    window_text(window, out);
    let stop = &document["last_stop"];
    if let Some(verdict) = stop["verdict"].as_str() {
        let _ = writeln!(
            out,
            "last stop (historical): {verdict}{}",
            when(&stop["time"])
        );
    }
}

/// The stamp's verdict and age, and what keeps the window red: "nothing judged" for an
/// unjudged window, never green. Spec 6.6, 11.4.
fn window_text(window: &Value, out: &mut String) {
    let stamped = window["age_seconds"]
        .as_u64()
        .map(|age| format!(", stamped {}", turn::age(age)))
        .unwrap_or_default();
    let _ = match window["verdict"].as_str() {
        Some("unjudged") => writeln!(
            out,
            "window: nothing judged{stamped} — {}",
            word(window, "error")
        ),
        Some("aborted") => writeln!(
            out,
            "window: aborted{}{stamped} — klin failed during a stop, and \
             the next stop that measures replaces it",
            when(&window["aborted_since"])
        ),
        Some(verdict) => writeln!(out, "window: {verdict}{stamped}"),
        None => writeln!(out, "window: none, no session has opened one"),
    };
    history_text(window, out);
    for notice in window["notices"].as_array().into_iter().flatten() {
        let _ = writeln!(out, "  notice: {}", word(notice, "message"));
    }
    for (key, label) in [
        ("open_sites", "open finding"),
        ("unasked", "deleted test not asked about"),
    ] {
        for item in window[key].as_array().into_iter().flatten() {
            let _ = writeln!(out, "  {label}: {}", item.as_str().unwrap_or_default());
        }
    }
}

/// The default-branch ref the advisory rules read, and the last advisory Stop. Spec 6.6.
fn history_text(window: &Value, out: &mut String) {
    match window["default_branch"].as_str() {
        Some(branch) => {
            let _ = writeln!(out, "  default branch: {branch}");
        }
        None if window["remotes"] == true => {
            let _ = writeln!(out, "  {}", turn::NO_DEFAULT);
        }
        None => {}
    }
    let advisory = &window["last_advisory"];
    if let Some(reason) = advisory["reason"].as_str() {
        let _ = writeln!(
            out,
            "  last advisory stop: {reason}{}",
            when(&advisory["time"])
        );
    }
}

/// How long ago a journal time was, after a comma, and nothing where the time is unknown.
fn when(time: &Value) -> String {
    time.as_u64()
        .and_then(known)
        .map(|time| format!(", {}", turn::age(now().saturating_sub(time))))
        .unwrap_or_default()
}

/// A time a stamp or a journal line holds, where zero is the default of a missing one.
fn known(time: u64) -> Option<u64> {
    (time > 0).then_some(time)
}

fn word<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("")
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}
