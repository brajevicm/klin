//! The person's approval of one `klin.json` edit, given in the chat and with no command. The
//! guard records a request, the person replies with its code, and the same edit then passes
//! once. Spec 10.8.2.

use std::collections::BTreeSet;
use std::hash::{BuildHasher, RandomState};
use std::path::Path;

use serde_json::{Map, Value};

use crate::sys::write::{AtomicWrite, atomic_write};
use crate::sys::{jsonc, state};

/// The one request the state directory holds. The agent can read that directory, so the file
/// holds a hash of the code and never the code.
const FILE: &str = "approval";
const PHRASE: &str = "approve";
/// Letters and digits a person does not misread for one another.
const ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTVWXYZ";
const LENGTH: usize = 4;

/// The file the edit leaves, the hash of its code, whether the person approved it, and the
/// identity of the tool call that used it. Every copy of klin's hooks for that one call passes.
struct Request {
    result: String,
    code: u64,
    approved: bool,
    used: Option<String>,
}

/// A fresh request for the edit that leaves `result`, in place of any request before it, and
/// its code. `None` when the state directory cannot hold it.
pub fn requested(at: &Path, result: &str) -> Option<String> {
    let code = fresh();
    let request = Request {
        result: result.to_string(),
        code: state::hash(code.as_bytes()),
        approved: false,
        used: None,
    };
    write(at, &request).then_some(code)
}

/// Whether the person approved the edit that leaves `result`, for this call or another copy of
/// klin's hooks for it. The first call uses the approval, and any other call is denied.
pub fn allows(at: &Path, result: &str, identity: &str) -> bool {
    let Some(mut request) = read(at) else {
        return false;
    };
    if !request.approved || request.result != result {
        return false;
    }
    match &request.used {
        Some(used) => !identity.is_empty() && used == identity,
        None => {
            request.used = Some(identity.to_string());
            write(at, &request)
        }
    }
}

/// A prompt opens a turn: an approval from the turn before expires, and a prompt that is
/// exactly `approve <code>` approves the pending request. A prompt that only holds the phrase,
/// such as a host's message that wraps it in tags, approves nothing.
pub fn prompted(at: &Path, prompt: &str) {
    let Some(mut request) = read(at) else {
        return;
    };
    if request.approved {
        cleared(at);
    } else if approves(prompt, request.code) {
        request.approved = true;
        write(at, &request);
    }
}

/// The turn ended, so an approval given in it expires. A request the person has not answered
/// stays.
pub fn ended(root: &Path) {
    if let Some(at) = state::dir(root)
        && read(&at).is_some_and(|request| request.approved)
    {
        cleared(&at);
    }
}

fn cleared(at: &Path) {
    let _ = std::fs::remove_file(at.join(FILE));
}

/// What the person reads: what the edit changes in the configuration, and the reply that
/// allows it.
pub fn message(current: &str, result: &str, code: &str) -> String {
    format!(
        "klin: the agent wants to change klin.json.\n{}To allow this one change, reply with \
         exactly: {PHRASE} {code}",
        summary(current, result)
    )
}

fn approves(prompt: &str, code: u64) -> bool {
    match prompt.split_whitespace().collect::<Vec<_>>().as_slice() {
        [phrase, given] => {
            phrase.eq_ignore_ascii_case(PHRASE)
                && state::hash(given.to_ascii_uppercase().as_bytes()) == code
        }
        _ => false,
    }
}

/// `RandomState` draws its keys from the system's random source.
fn fresh() -> String {
    let seed = RandomState::new().hash_one(std::time::SystemTime::now());
    seed.to_le_bytes()[..LENGTH]
        .iter()
        .map(|byte| char::from(ALPHABET[usize::from(*byte) % ALPHABET.len()]))
        .collect()
}

fn summary(current: &str, result: &str) -> String {
    let Ok(new) = jsonc::parse(result.as_bytes()) else {
        return "  The new file is not valid JSON, so klin cannot read it.\n".to_string();
    };
    let old = jsonc::parse(current.as_bytes()).unwrap_or_else(|_| Value::Object(Map::new()));
    let mut lines = Vec::new();
    differences("", Some(&old), Some(&new), &mut lines);
    if lines.is_empty() {
        lines.push("no setting changes, only the text of the file".to_string());
    }
    lines.iter().map(|line| format!("  {line}\n")).collect()
}

/// One line per setting the edit adds, removes or changes, by its dotted path.
fn differences(path: &str, old: Option<&Value>, new: Option<&Value>, lines: &mut Vec<String>) {
    let named = Some(path)
        .filter(|path| !path.is_empty())
        .unwrap_or("the whole file");
    match (old, new) {
        (Some(Value::Object(old)), Some(Value::Object(new))) => fields(path, old, new, lines),
        (Some(Value::Array(old)), Some(Value::Array(new))) => listed(named, old, new, lines),
        (Some(old), Some(new)) if old != new => lines.push(format!("{named}: {old} -> {new}")),
        (None, Some(new)) => lines.push(format!("adds {named}: {new}")),
        (Some(old), None) => lines.push(format!("removes {named} (was {old})")),
        _ => {}
    }
}

/// A key that is not a plain word prints as a JSON string, so the agent's text cannot start a
/// line of what the person reads.
fn fields(path: &str, old: &Map<String, Value>, new: &Map<String, Value>, lines: &mut Vec<String>) {
    for key in old.keys().chain(new.keys()).collect::<BTreeSet<_>>() {
        let named = match key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-$".contains(c))
        {
            true => key.clone(),
            false => Value::from(key.as_str()).to_string(),
        };
        let inner = match path {
            "" => named,
            path => format!("{path}.{named}"),
        };
        differences(&inner, old.get(key), new.get(key), lines);
    }
}

/// A list changes by the entries it gains and loses, so removing the first entry of `accepted`
/// names one entry and not every one after it.
fn listed(path: &str, old: &[Value], new: &[Value], lines: &mut Vec<String>) {
    let removed = without(old, new);
    let added = without(new, old);
    if removed.is_empty() && added.is_empty() && old != new {
        lines.push(format!("reorders {path}"));
    }
    lines.extend(
        removed
            .iter()
            .map(|gone| format!("removes from {path}: {gone}")),
    );
    lines.extend(added.iter().map(|came| format!("adds to {path}: {came}")));
}

/// The entries of `from` that `other` does not hold, counting each entry once.
fn without<'a>(from: &'a [Value], other: &[Value]) -> Vec<&'a Value> {
    let mut left: Vec<&Value> = other.iter().collect();
    from.iter()
        .filter(|entry| match left.iter().position(|held| held == entry) {
            Some(at) => {
                left.swap_remove(at);
                false
            }
            None => true,
        })
        .collect()
}

fn read(at: &Path) -> Option<Request> {
    let text = std::fs::read_to_string(at.join(FILE)).ok()?;
    let held: Value = serde_json::from_str(&text).ok()?;
    Some(Request {
        result: held["result"].as_str()?.to_string(),
        code: held["code"].as_u64()?,
        approved: held["approved"].as_bool()?,
        used: held["used"].as_str().map(str::to_string),
    })
}

fn write(at: &Path, request: &Request) -> bool {
    let mut held = serde_json::json!({
        "result": request.result,
        "code": request.code,
        "approved": request.approved,
    });
    if let Some(used) = &request.used {
        held["used"] = used.as_str().into();
    }
    atomic_write(AtomicWrite {
        target: &at.join(FILE),
        bytes: (held.to_string() + "\n").as_bytes(),
        keep_mode_from: None,
    })
    .is_ok()
}
