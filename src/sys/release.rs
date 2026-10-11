//! The newest klin release. A person command asks GitHub for it at most once a day, and every
//! other command, a Stop among them, reads only the answer the cache holds. Only a binary the
//! installer placed, with its `klin-update` beside it, asks or tells, because only that route
//! can act on the answer. Spec 11.9.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::sys::write::{AtomicWrite, atomic_write};

const LATEST: &str = "https://github.com/brajevicm/klin/releases/latest";
const TAGGED: &str = "https://github.com/brajevicm/klin/releases/tag/v";
const DAY: u64 = 24 * 60 * 60;

/// Ask GitHub when the cache is older than a day, from a person command on an installer's
/// binary outside CI. A failed ask changes nothing.
pub fn refresh() {
    let Some(file) = cache_file().filter(|_| enabled()) else {
        return;
    };
    let checked = read(&file)["checked"].as_u64().unwrap_or_default();
    if now().saturating_sub(checked) >= DAY {
        let _ = fetched(&file);
    }
}

/// Ask GitHub now, whatever the cache holds, and keep the answer. `klin update --check`.
pub fn fetch() -> Option<String> {
    cache_file().map_or_else(asked, |file| fetched(&file))
}

/// The newest release the cache holds, on a binary that may tell of one.
pub fn known() -> Option<String> {
    enabled()
        .then(cache_file)
        .flatten()
        .and_then(|file| read(&file)["tag"].as_str().map(str::to_string))
}

/// The newest release when it is newer than this binary.
pub fn newer() -> Option<String> {
    known().filter(|tag| is_newer(tag))
}

/// The newer release the agent has not yet heard of, and the record that it heard. A Stop.
pub fn untold() -> Option<(String, impl FnOnce())> {
    let file = cache_file()?;
    let tag = newer().filter(|tag| read(&file)["told"].as_str() != Some(tag))?;
    let heard = tag.clone();
    Some((tag, move || {
        let mut cached = read(&file);
        cached["told"] = heard.into();
        write(&file, &cached);
    }))
}

pub fn notes(tag: &str) -> String {
    format!("{TAGGED}{tag}")
}

pub fn is_newer(tag: &str) -> bool {
    parsed(tag) > parsed(env!("CARGO_PKG_VERSION"))
}

fn enabled() -> bool {
    let set = |name| std::env::var_os(name).is_some_and(|value| !value.is_empty());
    !set("CI") && !set("KLIN_NO_UPDATE_CHECK") && beside_updater()
}

fn beside_updater() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("klin-update")))
        .is_some_and(|path| path.is_file())
}

/// `${KLIN_CACHE_DIR:-$HOME/.cache/klin}/latest.json`, the directory the plugin wrapper uses.
fn cache_file() -> Option<PathBuf> {
    let dir = std::env::var_os("KLIN_CACHE_DIR")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".cache/klin")))?;
    Some(dir.join("latest.json"))
}

fn fetched(file: &Path) -> Option<String> {
    let tag = asked()?;
    let mut cached = read(file);
    cached["tag"] = tag.clone().into();
    cached["checked"] = now().into();
    write(file, &cached);
    Some(tag)
}

/// The tag GitHub's latest-release page redirects to, through `curl`, so klin carries no HTTP
/// client. The redirect is read and not followed, which spends no API rate limit.
fn asked() -> Option<String> {
    let output = Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            "2",
            "-o",
            "/dev/null",
            "-w",
            "%{redirect_url}",
            LATEST,
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let url = String::from_utf8(output.stdout).ok()?;
    let tag = url.trim().strip_prefix(TAGGED)?;
    parsed(tag).map(|_| tag.to_string())
}

/// A release version as numbers, and none for anything but dot-separated digits, so a tag
/// never carries text klin did not expect into a message.
fn parsed(version: &str) -> Option<Vec<u64>> {
    version.split('.').map(|part| part.parse().ok()).collect()
}

fn read(file: &Path) -> Value {
    std::fs::read(file)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

fn write(file: &Path, cached: &Value) {
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = atomic_write(AtomicWrite {
        target: file,
        bytes: cached.to_string().as_bytes(),
        keep_mode_from: None,
    });
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}
