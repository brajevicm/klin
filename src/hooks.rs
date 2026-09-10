use std::fmt::Write;
use std::path::Path;

use serde_json::{Map, Value};

use crate::config::Error;
use crate::init;

/// A host klin writes hooks for: the directory that says a tree uses it, and the file it reads
/// its hooks from. A host whose adapter has not landed carries the ticket that lands it, and is
/// refused until then. Section 19.3.
struct Host {
    name: &'static str,
    marker: &'static str,
    file: Option<&'static str>,
    ticket: &'static str,
}

const HOSTS: &[Host] = &[
    Host {
        name: "claude",
        marker: ".claude",
        file: Some(".claude/settings.json"),
        ticket: "",
    },
    Host {
        name: "cursor",
        marker: ".cursor",
        file: None,
        ticket: "#67",
    },
    Host {
        name: "codex",
        marker: ".codex",
        file: None,
        ticket: "#68",
    },
];

/// klin's hook lines, one per event of section 9.2. The matcher names the tools the guard
/// reads, and an event with no matcher takes every call.
const ENTRIES: &[(&str, &str, &str)] = &[
    ("SessionStart", "", "klin radius"),
    ("UserPromptSubmit", "", "klin radius"),
    (
        "PreToolUse",
        "Write|Edit|MultiEdit|NotebookEdit|Bash",
        "klin guard",
    ),
    ("Stop", "", "klin gate --hook --changed"),
];

const HOOKS: &str = "hooks";

pub fn run(root: &Path, named: Option<&str>, out: &mut String) -> Result<u8, Error> {
    let hosts = wanted(root, named)?;
    if let Some(host) = refused(named, &hosts) {
        return Err(pending(host));
    }
    for host in &hosts {
        match host.file {
            Some(file) => wrote(&root.join(file), out)?,
            None => {
                let _ = writeln!(out, "klin: NOTE: {}", pending(host));
            }
        }
    }
    Ok(0)
}

/// A host klin has no adapter for is refused when `--host` named it, and when this tree names
/// no other host, because klin then has nothing to write. A tree that names it beside a host
/// klin does write takes the hooks it can and a note about the rest.
fn refused<'a>(named: Option<&str>, hosts: &[&'a Host]) -> Option<&'a Host> {
    let pending = hosts.iter().find(|host| host.file.is_none())?;
    let alone = hosts.iter().all(|host| host.file.is_none());
    (named.is_some() || alone).then_some(*pending)
}

/// The host `--host` names, or every host this tree root says it uses. A tree that names none
/// is refused rather than guessed at, because a hook file klin invented gates nothing.
fn wanted(root: &Path, named: Option<&str>) -> Result<Vec<&'static Host>, Error> {
    if let Some(name) = named {
        return match HOSTS.iter().find(|host| host.name == name) {
            Some(host) => Ok(vec![host]),
            None => Err(Error(format!(
                "--host {name} names no host klin knows — name one of {}",
                names()
            ))),
        };
    }
    let found: Vec<&Host> = HOSTS
        .iter()
        .filter(|host| root.join(host.marker).is_dir())
        .collect();
    match found.is_empty() {
        true => Err(Error(format!(
            "{}: no host klin knows has a directory here — name one with --host, one of {}",
            root.display(),
            names()
        ))),
        false => Ok(found),
    }
}

fn names() -> String {
    listed(HOSTS.iter())
}

/// The hosts klin can write today, which is what a person can name after a refusal.
fn built() -> String {
    listed(HOSTS.iter().filter(|host| host.file.is_some()))
}

fn listed<'a>(hosts: impl Iterator<Item = &'a Host>) -> String {
    hosts
        .map(|host| host.name)
        .collect::<Vec<&str>>()
        .join(", ")
}

fn pending(host: &Host) -> Error {
    Error(format!(
        "klin has no {} adapter yet, so it cannot write {}'s hooks — {} lands it. Until then \
         name a host klin writes: klin init --hooks --host {}",
        host.name,
        host.name,
        host.ticket,
        built()
    ))
}

/// klin's entries added to whatever the file already holds. An event klin shares with another
/// tool keeps that tool's entries, and an event that already calls klin is left as it is, so a
/// second run writes nothing.
fn wrote(file: &Path, out: &mut String) -> Result<(), Error> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|why| Error::unreadable(parent, why))?;
    }
    let mut settings = read(file)?;
    let held = settings
        .entry(HOOKS)
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(events) = held.as_object_mut() else {
        return Err(malformed(file, HOOKS));
    };
    let mut added = Vec::new();
    let mut held_already = Vec::new();
    for (event, matcher, command) in ENTRIES {
        let held = events
            .entry(*event)
            .or_insert_with(|| Value::Array(Vec::new()));
        let Some(entries) = held.as_array_mut() else {
            return Err(malformed(file, event));
        };
        if entries.iter().any(calls_klin) {
            held_already.push(*event);
            continue;
        }
        entries.push(entry(matcher, command));
        added.push(*event);
    }
    init::write(file, &settings)?;
    let _ = writeln!(out, "{}", said(file, &added, &held_already));
    Ok(())
}

/// What was added and what was left, because a run that added two entries of four reads like a
/// complete one otherwise.
fn said(file: &Path, added: &[&str], held: &[&str]) -> String {
    let kept = match held.is_empty() {
        true => String::new(),
        false => format!(
            " {} already calls klin, and klin left {} as it is.",
            held.join(", "),
            match held.len() {
                1 => "it",
                _ => "them",
            }
        ),
    };
    match added.is_empty() {
        true => format!("{}: nothing was added.{kept}", file.display()),
        false => format!(
            "{}: added klin's entry on {}. Commit it, so a teammate who clones gets the \
             hooks.{kept}",
            file.display(),
            added.join(", ")
        ),
    }
}

fn malformed(file: &Path, key: &str) -> Error {
    Error(format!(
        "{}: \"{key}\" is not the shape this host reads, so klin left it alone",
        file.display()
    ))
}

/// An entry that runs the klin binary. The test is the command's own words, not the text of
/// the entry: a hook that only mentions klin, such as a script under a directory named after
/// it, is another tool's entry, and reading it as klin's leaves the event ungated.
fn calls_klin(entry: &Value) -> bool {
    entry["hooks"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|hook| hook["command"].as_str())
        .any(runs_klin)
}

fn runs_klin(command: &str) -> bool {
    command
        .split_whitespace()
        .map(|word| word.trim_matches(['\'', '"']))
        .any(|word| word == "klin" || word.ends_with("/klin"))
}

fn entry(matcher: &str, command: &str) -> Value {
    let mut entry = Map::new();
    if !matcher.is_empty() {
        entry.insert("matcher".to_string(), matcher.into());
    }
    entry.insert(
        HOOKS.to_string(),
        serde_json::json!([{"type": "command", "command": command}]),
    );
    Value::Object(entry)
}

fn read(file: &Path) -> Result<Map<String, Value>, Error> {
    if !file.is_file() {
        return Ok(Map::new());
    }
    let text = std::fs::read_to_string(file).map_err(|why| Error::unreadable(file, why))?;
    match serde_json::from_str(&text).map_err(|why| Error::unreadable(file, why))? {
        Value::Object(held) => Ok(held),
        _ => Err(Error(format!(
            "{}: a host's settings are an object",
            file.display()
        ))),
    }
}
