//! `klin status`: the configuration, each host integration, the state directory and the local
//! window, read without running a check and without writing anything. Spec 11.4, 11.7.

use std::fmt::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::check::catalogue;
use crate::config::{Config, Discovered};
use crate::error::Error;
use crate::hooks::{self, Integration};
use crate::stamp::{Here, Verdict};
use crate::{journal, stamp, state};

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
    let document = document(&found, root, start);
    match args.json {
        true => {
            let _ = writeln!(out, "{document}");
        }
        false => text(&document, &found.said(), out),
    }
    Ok(0)
}

fn document(found: &Discovered, root: &Path, start: &Path) -> Value {
    let error = found
        .config
        .as_ref()
        .and_then(|_| Config::load(None, start, &catalogue::sections()).err());
    let at = state::dir(root);
    let (lines, _) = journal::read(root);
    json!({
        "schema_version": 1,
        "command": "status",
        "klin": {"version": env!("CARGO_PKG_VERSION")},
        "config": {
            "path": found.config.as_ref().map(|file| file.display().to_string()),
            "present": found.config.is_some(),
            "valid": error.is_none(),
            "error": error.map(|error| error.to_string()),
            "ignored": found.ignored_paths(),
        },
        "integrations": hooks::integrations(root).iter().map(integration).collect::<Vec<Value>>(),
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
    let mut window = json!({
        "verdict": held.verdict.name(),
        "age_seconds": now().saturating_sub(held.time),
        "open": [],
        "unasked": [],
        "error": null,
        "aborted_since": null,
        "default_branch": Here::read(root).default.map(|(name, _)| name),
        "last_advisory": last_advisory(lines),
        "notices": journal::open_notices(lines, held.commit.as_deref())
            .into_iter()
            .map(|(time, message)| json!({"time": time, "message": message}))
            .collect::<Vec<Value>>(),
    });
    match held.verdict {
        Verdict::Red { open, unasked } => {
            window["open"] = open.into();
            window["unasked"] = unasked.into();
        }
        Verdict::Unjudged { error, .. } => window["error"] = error.into(),
        Verdict::Aborted { since } => window["aborted_since"] = since.into(),
        Verdict::Pending | Verdict::Green => {}
    }
    Some(window)
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

fn text(document: &Value, notes: &[String], out: &mut String) {
    let _ = writeln!(out, "klin {}", word(&document["klin"], "version"));
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
        let _ = writeln!(out, "last stop (historical): {verdict} at {}", stop["time"]);
    }
}

/// The stamp's verdict and age, and what keeps the window red: "nothing judged" for an
/// unjudged window, never green. Spec 6.6, 11.4.
fn window_text(window: &Value, out: &mut String) {
    let age = &window["age_seconds"];
    let _ = match window["verdict"].as_str() {
        Some("unjudged") => writeln!(
            out,
            "window: nothing judged, stamped {age} s ago — {}",
            word(window, "error")
        ),
        Some("aborted") => writeln!(
            out,
            "window: aborted since {}, stamped {age} s ago — klin failed during a stop, and \
             the next stop that measures replaces it",
            window["aborted_since"]
        ),
        Some(verdict) => writeln!(out, "window: {verdict}, stamped {age} s ago"),
        None => writeln!(out, "window: none, no session has opened one"),
    };
    history_text(window, out);
    for notice in window["notices"].as_array().into_iter().flatten() {
        let _ = writeln!(out, "  notice: {}", word(notice, "message"));
    }
    for (key, label) in [
        ("open", "open finding"),
        ("unasked", "deleted test not asked about"),
    ] {
        for item in window[key].as_array().into_iter().flatten() {
            let _ = writeln!(out, "  {label}: {}", item.as_str().unwrap_or_default());
        }
    }
}

/// The default-branch ref the advisory rules read, and the last advisory Stop. Spec 6.6.
fn history_text(window: &Value, out: &mut String) {
    if let Some(branch) = window["default_branch"].as_str() {
        let _ = writeln!(out, "  default branch: {branch}");
    }
    let advisory = &window["last_advisory"];
    if let Some(reason) = advisory["reason"].as_str() {
        let _ = writeln!(
            out,
            "  last advisory stop: {reason} at {}",
            advisory["time"]
        );
    }
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
