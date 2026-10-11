//! The newest klin release. A person command asks GitHub for it at most once a day, and every
//! other command, a Stop among them, reads only the answer the cache holds. Only a binary the
//! installer placed, with its `klin-update` beside it, asks or tells, because only that route
//! can act on the answer. Spec 11.9.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::sys::write::{AtomicWrite, atomic_write};

const LATEST: &str = "https://github.com/brajevicm/klin/releases/latest";
const TAGGED: &str = "https://github.com/brajevicm/klin/releases/tag/v";
const DAY: u64 = 24 * 60 * 60;
const LATEST_FILE: &str = "latest.json";

/// A release version as numbers. Only dot-separated digits parse, so a version never carries
/// text klin did not expect into a message.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(Vec<u64>);

impl Version {
    fn parse(text: &str) -> Option<Version> {
        text.split('.')
            .map(|part| part.parse().ok())
            .collect::<Option<_>>()
            .map(Version)
    }

    /// This build's version, without a pre-release suffix such as `-beta.1`.
    fn current() -> Option<Version> {
        let own = env!("CARGO_PKG_VERSION");
        Version::parse(own.split('-').next().unwrap_or(own))
    }

    /// Whether this release is newer than the build. False when the build's own version does
    /// not parse, so an unknown build never hears of an older release.
    pub fn is_newer(&self) -> bool {
        Version::current().is_some_and(|own| *self > own)
    }

    pub fn notes(&self) -> String {
        format!("{TAGGED}{self}")
    }
}

impl fmt::Display for Version {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.0.iter().map(u64::to_string).collect();
        out.write_str(&parts.join("."))
    }
}

/// Who hears of a newer release: the person through a Stop's notice, the agent through the
/// session-start text. Each keeps its own record, so each hears of a release once.
#[derive(Clone, Copy)]
pub enum Audience {
    Person,
    Agent,
}

impl Audience {
    fn record(self) -> &'static str {
        match self {
            Audience::Person => "release-told",
            Audience::Agent => "release-told-agent",
        }
    }

    pub fn note(self, tag: &Version) -> String {
        let version = env!("CARGO_PKG_VERSION");
        match self {
            Audience::Person => format!(
                "klin {tag} is available (you have {version}). Run klin update, then klin setup."
            ),
            Audience::Agent => format!(
                "klin {tag} is available. Tell the person to run klin update, then klin setup. \
                 Do not run it yourself."
            ),
        }
    }
}

/// Ask GitHub when the last ask is a day old, from a person command on an installer's binary
/// outside CI. A failed ask still counts as the day's ask, so an offline machine waits on
/// `curl` once a day and not on every command.
pub fn refresh() {
    let Some(dir) = cache_dir().filter(|_| enabled()) else {
        return;
    };
    let checked = read(&dir)["checked"].as_u64().unwrap_or_default();
    if now().saturating_sub(checked) >= DAY {
        let _ = fetched(&dir);
    }
}

/// Ask GitHub now, whatever the cache holds, and keep the answer. `klin update --check`.
pub fn fetch() -> Option<Version> {
    cache_dir().map_or_else(asked, |dir| fetched(&dir))
}

/// The newest release the cache holds, on a binary that may tell of one.
pub fn known() -> Option<Version> {
    let dir = cache_dir().filter(|_| enabled())?;
    read(&dir)["tag"].as_str().and_then(Version::parse)
}

/// The newer release `who` has not yet heard of.
pub fn untold(who: Audience) -> Option<Version> {
    let dir = cache_dir()?;
    let told = std::fs::read_to_string(dir.join(who.record())).unwrap_or_default();
    known().filter(|tag| tag.is_newer() && Version::parse(&told).as_ref() != Some(tag))
}

/// Record that `who` heard of `tag`, in a file of its own, so it never races the ask's write.
pub fn heard(who: Audience, tag: &Version) {
    if let Some(dir) = cache_dir() {
        write(&dir.join(who.record()), tag.to_string().as_bytes());
    }
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

/// `${KLIN_CACHE_DIR:-$HOME/.cache/klin}`, the directory the plugin wrapper uses.
fn cache_dir() -> Option<PathBuf> {
    std::env::var_os("KLIN_CACHE_DIR")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".cache/klin")))
}

fn fetched(dir: &Path) -> Option<Version> {
    let tag = asked();
    let mut cached = read(dir);
    if let Some(tag) = &tag {
        cached["tag"] = tag.to_string().into();
    }
    cached["checked"] = now().into();
    write(&dir.join(LATEST_FILE), cached.to_string().as_bytes());
    tag
}

/// The tag GitHub's latest-release page redirects to, through `curl`, so klin carries no HTTP
/// client. The redirect is read and not followed, which spends no API rate limit.
fn asked() -> Option<Version> {
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
    Version::parse(url.trim().strip_prefix(TAGGED)?)
}

fn read(dir: &Path) -> Value {
    std::fs::read(dir.join(LATEST_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

fn write(file: &Path, bytes: &[u8]) {
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = atomic_write(AtomicWrite {
        target: file,
        bytes,
        keep_mode_from: None,
    });
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}
