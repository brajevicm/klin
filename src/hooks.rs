use std::fmt::Write;
use std::path::Path;

use serde_json::{Map, Value};

use crate::config::Error;
use crate::host::{ADAPTERS, Adapter, Filter, Hook, HookFile};
use crate::init;

/// The host `--host` names, or every host this tree root says it uses. A tree that names none
/// is refused rather than guessed at, because a hook file klin invented gates nothing.
fn wanted(root: &Path, named: Option<&str>) -> Result<Vec<&'static dyn Adapter>, Error> {
    if let Some(name) = named {
        return match ADAPTERS.iter().copied().find(|host| host.name() == name) {
            Some(host) => Ok(vec![host]),
            None => Err(Error(format!(
                "--host {name} names no host klin knows — name one of {}",
                names()
            ))),
        };
    }
    let found: Vec<&'static dyn Adapter> = ADAPTERS
        .iter()
        .copied()
        .filter(|host| root.join(host.marker()).is_dir())
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
    ADAPTERS
        .iter()
        .map(|host| host.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Every line klin writes resolves the binary before it runs it, and ends the hook when none
/// resolves. A person who uninstalls klin, or installs it where the hook's shell does not look,
/// would otherwise see a failed hook on every event of every session. Section 19.3.
fn line(arguments: &str) -> String {
    format!("command -v klin > /dev/null 2>&1 || exit 0; klin {arguments}")
}

const HOOKS: &str = "hooks";

pub fn run(root: &Path, named: Option<&str>, shared: bool, out: &mut String) -> Result<u8, Error> {
    let hosts = wanted(root, named)?;
    for host in hosts {
        hooked(host, root, shared, out)?;
    }
    Ok(0)
}

/// One host's file, or the note that klin cannot write it. A host whose plugin already
/// carries the entries gets neither, and is told which settings file enables it.
fn hooked(
    host: &'static dyn Adapter,
    root: &Path,
    shared: bool,
    out: &mut String,
) -> Result<(), Error> {
    let target = root.join(host.hook_file());
    match elsewhere(host, root, shared, &target) {
        Some(said) => {
            let _ = writeln!(out, "{said}");
            Ok(())
        }
        None => wrote(host, &target, shared, out),
    }
}

/// What already runs klin's hooks over the file klin would write, in klin's own words. Two
/// copies of the entries run klin twice on every event: two gates race for one turn stamp,
/// and the prompt counter moves by two. A plugin is one copy. A user-level install klin wrote
/// itself is the other, because a host reads its user file and the tree's together.
fn elsewhere(host: &dyn Adapter, root: &Path, shared: bool, target: &Path) -> Option<String> {
    if let Some(settings) = host.plugin_enabled(root, shared) {
        return Some(registered(&settings, target));
    }
    if shared {
        return None;
    }
    let user = std::env::home_dir()?.join(host.hook_file());
    holds_klin(&user).then(|| installed(&user, target))
}

/// A host file that already holds an entry of klin's on some event.
fn holds_klin(settings: &Path) -> bool {
    let Ok(held) = read(settings) else {
        return false;
    };
    held.get(HOOKS)
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(_, entries)| entries.as_array())
        .flatten()
        .any(calls_klin)
}

fn installed(user: &Path, file: &Path) -> String {
    format!(
        "{}: klin's hooks are installed for every repository already, so klin added nothing \
         to {}. Change them where they are: klin init --hooks --global.",
        user.display(),
        file.display()
    )
}

fn registered(settings: &Path, file: &Path) -> String {
    format!(
        "{}: klin's plugin is enabled here and carries the hooks itself, so klin added nothing \
         to {}. Disable the plugin first if you would rather the file held them.",
        settings.display(),
        file.display()
    )
}

/// klin's entries added to whatever the file already holds. An event klin shares with another
/// tool keeps that tool's entries, and an event that already calls klin is left as it is, so a
/// second run writes nothing.
fn wrote(host: &dyn Adapter, file: &Path, shared: bool, out: &mut String) -> Result<(), Error> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|why| Error::unreadable(parent, why))?;
    }
    let mut settings = read(file)?;
    if let HookFile::Flat { version } = host.hook_file_kind() {
        settings
            .entry("version")
            .or_insert_with(|| Value::from(version));
    }
    let held = settings
        .entry(HOOKS)
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(events) = held.as_object_mut() else {
        return Err(malformed(file, HOOKS));
    };
    let (added, held_already) = merged(host, file, events)?;
    init::write(file, &settings)?;
    let _ = writeln!(out, "{}", said(file, shared, &added, &held_already));
    Ok(())
}

/// The events klin's entry was added on, and the events that already called klin.
fn merged(
    host: &dyn Adapter,
    file: &Path,
    events: &mut Map<String, Value>,
) -> Result<(Vec<&'static str>, Vec<&'static str>), Error> {
    let mut added = Vec::new();
    let mut held_already = Vec::new();
    for hook in host.hooks() {
        let held = events
            .entry(hook.event.to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        let Some(entries) = held.as_array_mut() else {
            return Err(malformed(file, hook.event));
        };
        if entries.iter().any(calls_klin) {
            held_already.push(hook.event);
            continue;
        }
        entries.push(entry(host, hook, &line(hook.arguments)));
        added.push(hook.event);
    }
    Ok((added, held_already))
}

/// What one entry filters by, in the host's matcher syntax: nothing, or the tools the guard
/// reads. The hook table says which, per event.
fn matcher(host: &dyn Adapter, hook: &Hook) -> &'static str {
    match hook.filter {
        Filter::Every => "",
        Filter::Tools => host.matcher(),
    }
}

/// What was added and what was left, because a run that added two entries of four reads like a
/// complete one otherwise.
fn said(file: &Path, shared: bool, added: &[&str], held: &[&str]) -> String {
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
            "{}: added klin's entry on {}.{}{kept}",
            file.display(),
            added.join(", "),
            match shared {
                true => " It covers every repository you open.",
                false => " Commit it, so a teammate who clones gets the hooks.",
            }
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
    if let Some(command) = entry.get("command").and_then(Value::as_str)
        && runs_klin(command)
    {
        return true;
    }
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

/// One entry in the host's own shape: the matcher the event filters by, when it filters, and
/// the command either beside it or under the host's nested `hooks` array.
fn entry(host: &dyn Adapter, hook: &Hook, command: &str) -> Value {
    let mut entry = Map::new();
    let matcher = matcher(host, hook);
    if !matcher.is_empty() {
        entry.insert("matcher".to_string(), matcher.into());
    }
    match host.hook_file_kind() {
        HookFile::Flat { .. } => {
            entry.insert("command".to_string(), command.into());
        }
        HookFile::Nested => {
            entry.insert(
                HOOKS.to_string(),
                serde_json::json!([{"type": "command", "command": command}]),
            );
        }
    }
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
