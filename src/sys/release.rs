//! klin keeps itself current. A session start, `klin status` and `klin setup` ask GitHub for the
//! newest release at most once a day, in a detached child, so nothing waits on the network.
//! Under `KLIN_AUTO_UPDATE=1` the child also runs the `klin-update` beside the binary. Every
//! other command, a Stop among them, reads only the cache. Only a binary the installer placed,
//! with its `klin-update` beside it, asks, updates or tells. Spec 11.9.

use std::fmt;
use std::fs::OpenOptions;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::sys::write::{AtomicWrite, atomic_write};

const LATEST: &str = "https://github.com/brajevicm/klin/releases/latest";
const TAGGED: &str = "https://github.com/brajevicm/klin/releases/tag/v";
const DAY: u64 = 24 * 60 * 60;
const LATEST_FILE: &str = "latest.json";
const LOCK_FILE: &str = "release.lock";
/// A lock older than this belongs to a child that died, and the next child takes it.
const LOCK_STALE: u64 = 10 * 60;
/// The hidden argument the detached child runs under. It is not a public command.
pub const CHILD: &str = "__release";

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

    fn is_older(&self) -> bool {
        Version::current().is_some_and(|own| *self < own)
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

/// What one audience has not heard yet: that this binary replaced an older one, or that a
/// newer release exists.
pub enum News {
    Updated { from: Version, to: Version },
    Available(Version),
}

/// Who hears the news: the person through a Stop's notice, the agent through the
/// session-start text. Each keeps its own record of the version it last saw and the release
/// it was last told of, so each hears every piece of news once.
#[derive(Clone, Copy)]
pub enum Audience {
    Person,
    Agent,
}

impl Audience {
    fn record(self) -> &'static str {
        match self {
            Audience::Person => "release-person.json",
            Audience::Agent => "release-agent.json",
        }
    }

    pub fn note(self, news: &News) -> String {
        match (self, news) {
            (Audience::Person, News::Available(tag)) => format!(
                "klin {tag} is available (you have {}). Run klin update.",
                env!("CARGO_PKG_VERSION")
            ),
            (Audience::Agent, News::Available(tag)) => format!(
                "klin {tag} is available. Tell the person to run klin update. Do not run it \
                 yourself."
            ),
            (Audience::Person, News::Updated { from, to }) => format!(
                "klin updated to {to} (from {from}). Release notes: {}",
                to.notes()
            ),
            (Audience::Agent, News::Updated { from, to }) => {
                format!("klin {to} judges this repository now (it was {from}).")
            }
        }
    }
}

/// From a session start, `klin status` or `klin setup`: start the detached child when the last
/// ask is a day old, or when auto-update holds a newer release it has not tried. The child
/// decides under its lock, so two commands at once start at most one ask and one update.
pub fn refresh() {
    let Some(dir) = cache_dir().filter(|_| enabled()) else {
        return;
    };
    let cached = read(&dir);
    if !stale(&cached) && untried(&cached).is_none() {
        return;
    }
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe)
            .arg(CHILD)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn();
    }
}

/// The detached child, under the cache's lock: the day's ask, which counts from its start so a
/// failed or slow ask is the day's ask, then under auto-update the updater, once for each newer
/// release. A child that finds the lock taken does nothing. Its exit code is never read.
pub fn child() -> u8 {
    let Some(dir) = cache_dir().filter(|_| enabled()) else {
        return 0;
    };
    let Some(_lock) = Lock::take(&dir) else {
        return 0;
    };
    let mut cached = read(&dir);
    if stale(&cached) {
        cached["checked"] = now().into();
        write(&dir.join(LATEST_FILE), cached.to_string().as_bytes());
        let _ = fetched(&dir);
        cached = read(&dir);
    }
    if let (Some(tag), Some(updater)) = (untried(&cached), updater_path()) {
        cached["tried"] = tag.to_string().into();
        write(&dir.join(LATEST_FILE), cached.to_string().as_bytes());
        let _ = Command::new(updater)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    0
}

/// The cache's lock file, made new so only one child holds it, and removed when it drops.
struct Lock(PathBuf);

impl Lock {
    fn take(dir: &Path) -> Option<Lock> {
        let file = dir.join(LOCK_FILE);
        let _ = std::fs::create_dir_all(dir);
        let old = std::fs::metadata(&file)
            .and_then(|held| held.modified())
            .ok()
            .and_then(|at| at.elapsed().ok())
            .is_some_and(|age| age.as_secs() >= LOCK_STALE);
        if old {
            let _ = std::fs::remove_file(&file);
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&file)
            .ok()
            .map(|_| Lock(file))
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Whether an integration of the repository at a root is behind this binary. The integration
/// scan lives in the command layer, which registers it here at start.
static BEHIND: OnceLock<fn(&Path) -> bool> = OnceLock::new();

pub fn register_behind(behind: fn(&Path) -> bool) {
    let _ = BEHIND.set(behind);
}

pub fn behind(root: &Path) -> bool {
    BEHIND.get().is_some_and(|behind| behind(root))
}

/// Ask GitHub now and wait, whatever the cache holds, and keep the answer. `klin update
/// --check`.
pub fn fetch() -> Option<Version> {
    cache_dir().map_or_else(asked, |dir| fetched(&dir))
}

/// The newest release the cache holds, on a binary that may tell of one.
pub fn known() -> Option<Version> {
    let dir = cache_dir().filter(|_| enabled())?;
    tag_of(&read(&dir))
}

/// The version the person saw before this binary replaced it, read without consuming it.
pub fn updated_from() -> Option<Version> {
    let dir = cache_dir().filter(|_| enabled())?;
    seen(&dir, Audience::Person).filter(Version::is_older)
}

/// The news `who` has not heard: an update first, else a newer release. The first sight of a
/// binary, and a binary older than the one last seen, records its version, so neither a fresh
/// install nor a downgrade is an update.
pub fn news(who: Audience) -> Option<News> {
    let dir = cache_dir().filter(|_| enabled())?;
    let current = Version::current()?;
    match seen(&dir, who) {
        Some(seen) if seen < current => {
            return Some(News::Updated {
                from: seen,
                to: current,
            });
        }
        Some(seen) if seen == current => {}
        _ => record(&dir, who, |held| held["seen"] = current.to_string().into()),
    }
    let told = read_record(&dir, who)["told"]
        .as_str()
        .and_then(Version::parse);
    tag_of(&read(&dir))
        .filter(|tag| tag.is_newer() && told.as_ref() != Some(tag))
        .map(News::Available)
}

/// Record that `who` heard `news`.
pub fn heard(who: Audience, news: &News) {
    let Some(dir) = cache_dir() else {
        return;
    };
    record(&dir, who, |held| match news {
        News::Updated { to, .. } => held["seen"] = to.to_string().into(),
        News::Available(tag) => held["told"] = tag.to_string().into(),
    });
}

fn enabled() -> bool {
    !set("CI") && !set("KLIN_NO_UPDATE_CHECK") && updater_path().is_some()
}

fn auto() -> bool {
    std::env::var_os("KLIN_AUTO_UPDATE").is_some_and(|value| value == "1")
}

fn set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.is_empty())
}

fn updater_path() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("klin-update")))
        .filter(|path| path.is_file())
}

/// The newer release auto-update has not tried, so a failing updater runs once per release.
fn untried(cached: &Value) -> Option<Version> {
    let tried = cached["tried"].as_str().and_then(Version::parse);
    tag_of(cached).filter(|tag| auto() && tag.is_newer() && tried.as_ref() != Some(tag))
}

/// `${KLIN_CACHE_DIR:-$HOME/.cache/klin}`, the directory the plugin wrapper uses.
fn cache_dir() -> Option<PathBuf> {
    std::env::var_os("KLIN_CACHE_DIR")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".cache/klin")))
}

fn stale(cached: &Value) -> bool {
    now().saturating_sub(cached["checked"].as_u64().unwrap_or_default()) >= DAY
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
            "10",
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

fn tag_of(cached: &Value) -> Option<Version> {
    cached["tag"].as_str().and_then(Version::parse)
}

fn seen(dir: &Path, who: Audience) -> Option<Version> {
    read_record(dir, who)["seen"]
        .as_str()
        .and_then(Version::parse)
}

fn record(dir: &Path, who: Audience, change: impl FnOnce(&mut Value)) {
    let mut held = read_record(dir, who);
    change(&mut held);
    write(&dir.join(who.record()), held.to_string().as_bytes());
}

fn read_record(dir: &Path, who: Audience) -> Value {
    read_json(&dir.join(who.record()))
}

fn read(dir: &Path) -> Value {
    read_json(&dir.join(LATEST_FILE))
}

fn read_json(file: &Path) -> Value {
    std::fs::read(file)
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
