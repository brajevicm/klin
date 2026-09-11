use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::config::Error;
use crate::host::{ADAPTERS, Adapter};
use crate::init;

/// A host klin writes hooks for. A built host is its adapter. A pending host is one klin knows
/// the name of, carries the ticket that lands its adapter, and is refused until then.
/// Section 19.3.
#[derive(Clone, Copy)]
enum Port {
    Built(&'static dyn Adapter),
    Pending {
        name: &'static str,
        marker: &'static str,
        ticket: &'static str,
    },
}

impl Port {
    fn name(&self) -> &'static str {
        match self {
            Port::Built(host) => host.name(),
            Port::Pending { name, .. } => name,
        }
    }

    fn marker(&self) -> &'static str {
        match self {
            Port::Built(host) => host.marker(),
            Port::Pending { marker, .. } => marker,
        }
    }

    fn built(&self) -> Option<&'static dyn Adapter> {
        match self {
            Port::Built(host) => Some(*host),
            Port::Pending { .. } => None,
        }
    }
}

const PENDING: &[Port] = &[Port::Pending {
    name: "cursor",
    marker: ".cursor",
    ticket: "#67",
}];

fn ports() -> impl Iterator<Item = Port> {
    ADAPTERS
        .iter()
        .map(|host| Port::Built(*host))
        .chain(PENDING.iter().copied())
}

/// klin's hook lines, one per event of section 9.2. The guard's event takes the host's matcher
/// for the tools the guard reads, and every other event takes every call.
const ENTRIES: &[(&str, &str)] = &[
    ("SessionStart", "radius"),
    ("UserPromptSubmit", "radius"),
    (GUARDED, "guard"),
    ("Stop", "gate --hook --changed"),
];
const GUARDED: &str = "PreToolUse";

/// Every line klin writes resolves the binary before it runs it, and ends the hook when none
/// resolves. A person who uninstalls klin, or installs it where the hook's shell does not look,
/// would otherwise see a failed hook on every event of every session. Section 19.3.
fn line(arguments: &str) -> String {
    format!("command -v klin > /dev/null 2>&1 || exit 0; klin {arguments}")
}

const HOOKS: &str = "hooks";

/// klin's name in a host's list of enabled plugins.
const PLUGIN: &str = "klin";

pub fn run(root: &Path, named: Option<&str>, shared: bool, out: &mut String) -> Result<u8, Error> {
    let hosts = wanted(root, named)?;
    if let Some(host) = refused(named, &hosts) {
        return Err(pending(&host, shared));
    }
    for host in &hosts {
        hooked(host, root, shared, out)?;
    }
    Ok(0)
}

/// One host's file, or the note that klin cannot write it. A host whose plugin already
/// carries the entries gets neither, and is told which settings file enables it.
fn hooked(port: &Port, root: &Path, shared: bool, out: &mut String) -> Result<(), Error> {
    let Some(host) = port.built() else {
        let _ = writeln!(out, "klin: NOTE: {}", pending(port, shared));
        return Ok(());
    };
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
    if let Some(settings) = enabling(host, root, shared) {
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

/// A host klin has no adapter for is refused when `--host` named it, and when this tree names
/// no other host, because klin then has nothing to write. A tree that names it beside a host
/// klin does write takes the hooks it can and a note about the rest.
fn refused(named: Option<&str>, ports: &[Port]) -> Option<Port> {
    let pending = ports.iter().find(|port| port.built().is_none())?;
    let alone = ports.iter().all(|port| port.built().is_none());
    (named.is_some() || alone).then_some(*pending)
}

/// The host `--host` names, or every host this tree root says it uses. A tree that names none
/// is refused rather than guessed at, because a hook file klin invented gates nothing.
fn wanted(root: &Path, named: Option<&str>) -> Result<Vec<Port>, Error> {
    if let Some(name) = named {
        return match ports().find(|port| port.name() == name) {
            Some(port) => Ok(vec![port]),
            None => Err(Error(format!(
                "--host {name} names no host klin knows — name one of {}",
                names()
            ))),
        };
    }
    let found: Vec<Port> = ports()
        .filter(|port| root.join(port.marker()).is_dir())
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
    listed(ports())
}

/// The hosts klin can write today, which is what a person can name after a refusal.
fn built() -> String {
    listed(ports().filter(|port| port.built().is_some()))
}

fn listed(ports: impl Iterator<Item = Port>) -> String {
    ports
        .map(|port| port.name())
        .collect::<Vec<&str>>()
        .join(", ")
}

/// The refusal names the command that works today, and keeps `--global` when that is the
/// install the person asked for, because the per-tree form writes into a repository.
fn pending(port: &Port, shared: bool) -> Error {
    let ticket = match port {
        Port::Pending { ticket, .. } => ticket,
        Port::Built(_) => "",
    };
    Error(format!(
        "klin has no {} adapter yet, so it cannot write {}'s hooks — {} lands it. Until then \
         name a host klin writes: klin init --hooks{} --host {}",
        port.name(),
        port.name(),
        ticket,
        match shared {
            true => " --global",
            false => "",
        },
        built()
    ))
}

/// The settings file that enables klin's plugin for the file klin is about to write, if one
/// does. The plugin carries the same entries, so a second copy of them runs klin twice on
/// every event: two gates race for the turn stamp, and the prompt counter moves by two.
///
/// A write into a tree is covered by that tree's settings, the local settings beside them and
/// the user's. A write into the home directory is covered by the user's alone, because a
/// plugin one repository enables gates that repository and not the machine. A host with no
/// plugin key has no plugin of klin's to look for.
fn enabling(host: &dyn Adapter, root: &Path, shared: bool) -> Option<PathBuf> {
    let key = host.plugin_key()?;
    let file = host.hook_file();
    let mut looked = Vec::new();
    if !shared {
        looked.push(root.join(file));
        looked.push(root.join(file.replace(".json", ".local.json")));
    }
    if let Some(home) = std::env::home_dir() {
        looked.push(home.join(file));
    }
    looked
        .into_iter()
        .find(|settings| lists_klin(settings, key))
}

fn lists_klin(settings: &Path, key: &str) -> bool {
    let Ok(held) = read(settings) else {
        return false;
    };
    held.get(key)
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .any(|(named, on)| named_klin(named) && on.as_bool().unwrap_or(false))
}

/// The name a host lists the plugin under is klin's own name, or that name and the marketplace
/// it came from. A plugin whose name only starts with klin's is another plugin.
fn named_klin(named: &str) -> bool {
    named == PLUGIN
        || named
            .strip_prefix(PLUGIN)
            .is_some_and(|rest| rest.starts_with('@'))
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
    for (event, command) in ENTRIES {
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
        entries.push(entry(matcher(host, event), &line(command)));
        added.push(*event);
    }
    Ok((added, held_already))
}

/// The guard's event takes the host's tool matcher, and every other event takes every call.
fn matcher(host: &dyn Adapter, event: &str) -> &'static str {
    match event == GUARDED {
        true => host.matcher(),
        false => "",
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
