//! `klin status`: the configuration, each host integration, the state directory and the local
//! window, read without running a check and without writing anything. Spec 11.4, 11.7.

use std::fmt::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::check::catalogue;
use crate::config::{self, Config, Discovered};
use crate::error::Error;
use crate::hooks::{self, Integration};
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
        false => text(&document, &config::notes(None, start), out),
    }
    Ok(0)
}

fn document(found: &Discovered, root: &Path, start: &Path) -> Value {
    let error = found
        .config
        .as_ref()
        .and_then(|_| Config::load(None, start, &catalogue::sections()).err());
    let at = state::dir(root);
    json!({
        "schema_version": 1,
        "command": "status",
        "klin": {"version": env!("CARGO_PKG_VERSION")},
        "config": {
            "path": found.config.as_ref().map(|file| file.display().to_string()),
            "present": found.config.is_some(),
            "valid": error.is_none(),
            "error": error.map(|error| error.to_string()),
            "ignored": config::ignored(start),
        },
        "integrations": hooks::integrations(root).iter().map(integration).collect::<Vec<Value>>(),
        "state_dir": at.as_ref().map(|at| at.display().to_string()),
        "cache_dir": at.as_ref().map(|at| at.join(state::CACHE).display().to_string()),
        "window": at.as_deref().and_then(window),
        "last_stop": last_stop(root),
    })
}

fn integration(one: &Integration) -> Value {
    json!({
        "host": one.host,
        "scope": one.scope,
        "route": one.route,
        "state": one.state,
        "detail": one.detail,
    })
}

/// The local window the stamp holds: its verdict and its age. It never claims that the working
/// tree passes. Spec 11.4.
fn window(at: &Path) -> Option<Value> {
    let held = stamp::read(at)?;
    Some(json!({
        "verdict": if held.green { "green" } else { "red" },
        "age_seconds": now().saturating_sub(held.time),
    }))
}

/// The last Stop the journal records, which is history and not the tree as it stands.
fn last_stop(root: &Path) -> Value {
    let (lines, _) = journal::read(root);
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
    match window["verdict"].as_str() {
        Some(verdict) => {
            let _ = writeln!(
                out,
                "window: {verdict}, stamped {} s ago",
                window["age_seconds"]
            );
        }
        None => {
            let _ = writeln!(out, "window: none, no session has opened one");
        }
    }
    let stop = &document["last_stop"];
    if let Some(verdict) = stop["verdict"].as_str() {
        let _ = writeln!(out, "last stop (historical): {verdict} at {}", stop["time"]);
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
