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

/// The cache directory of a binary that may ask, update or tell: one the installer placed,
/// outside CI, with the check on. Every public entry builds it first, so the rule is in one
/// place.
struct Releases {
    dir: PathBuf,
}

impl Releases {
    fn here() -> Option<Releases> {
        Releases::any().filter(|_| enabled())
    }

    /// The cache directory whatever the rule says, for a person's `klin update --check`.
    fn any() -> Option<Releases> {
        cache_dir().map(|dir| Releases { dir })
    }
}

/// What `latest.json` holds: the newest tag GitHub named, when klin last asked, and the release
/// auto-update last tried.
#[derive(Default)]
struct Latest {
    tag: Option<Version>,
    checked: u64,
    tried: Option<Version>,
}

impl Latest {
    fn read(dir: &Path) -> Latest {
        let held = read_json(&dir.join(LATEST_FILE));
        Latest {
            tag: version(&held["tag"]),
            checked: held["checked"].as_u64().unwrap_or_default(),
            tried: version(&held["tried"]),
        }
    }

    fn write(&self, dir: &Path) {
        let held = json!({
            "tag": self.tag.as_ref().map(ToString::to_string),
            "checked": self.checked,
            "tried": self.tried.as_ref().map(ToString::to_string),
        });
        write(&dir.join(LATEST_FILE), held.to_string().as_bytes());
    }

    fn stale(&self) -> bool {
        now().saturating_sub(self.checked) >= DAY
    }

    /// The newer release auto-update has not tried, so a failing updater runs once per release.
    fn untried(&self) -> Option<&Version> {
        self.tag
            .as_ref()
            .filter(|tag| auto() && tag.is_newer() && self.tried.as_ref() != Some(tag))
    }
}

/// What one audience's record holds: the version it last saw klin run as, and the release it
/// was last told of.
#[derive(Default)]
struct Record {
    seen: Option<Version>,
    told: Option<Version>,
}

impl Record {
    fn read(dir: &Path, who: Audience) -> Record {
        let held = read_json(&dir.join(who.record()));
        Record {
            seen: version(&held["seen"]),
            told: version(&held["told"]),
        }
    }

    fn write(&self, dir: &Path, who: Audience) {
        let held = json!({
            "seen": self.seen.as_ref().map(ToString::to_string),
            "told": self.told.as_ref().map(ToString::to_string),
        });
        write(&dir.join(who.record()), held.to_string().as_bytes());
    }
}

/// From a session start, `klin status` or `klin setup`: start the detached child when the last
/// ask is a day old, or when auto-update holds a newer release it has not tried. The child
/// decides under its lock, so two commands at once start at most one ask and one update.
pub fn refresh() {
    let Some(releases) = Releases::here() else {
        return;
    };
    let latest = Latest::read(&releases.dir);
    if !latest.stale() && latest.untried().is_none() {
        return;
    }
    if let Ok(exe) = std::env::current_exe() {
        let _ = silent(Command::new(exe).arg(CHILD))
            .process_group(0)
            .spawn();
    }
}

/// The detached child, under the cache's lock: the day's ask, which counts from its start so a
/// failed or slow ask is the day's ask, then under auto-update the updater, once for each newer
/// release. A child that finds the lock taken does nothing.
pub fn child() {
    let Some(releases) = Releases::here() else {
        return;
    };
    let dir = &releases.dir;
    let Some(_lock) = Lock::take(dir) else {
        return;
    };
    let mut latest = Latest::read(dir);
    if latest.stale() {
        latest.checked = now();
        latest.write(dir);
        latest.tag = asked().or(latest.tag);
        latest.write(dir);
    }
    if let (Some(tag), Some(updater)) = (latest.untried().cloned(), updater_path()) {
        latest.tried = Some(tag);
        latest.write(dir);
        let _ = silent(&mut Command::new(updater)).status();
    }
}

/// The cache's lock file, made new so only one child holds it, and removed when it drops.
struct Lock(PathBuf);

impl Lock {
    /// A lock older than `LOCK_STALE` belongs to a child that died. Only one child can rename
    /// it away, so two children never both take over the same stale lock.
    fn take(dir: &Path) -> Option<Lock> {
        let file = dir.join(LOCK_FILE);
        let _ = std::fs::create_dir_all(dir);
        let old = std::fs::metadata(&file)
            .and_then(|held| held.modified())
            .ok()
            .and_then(|at| at.elapsed().ok())
            .is_some_and(|age| age.as_secs() >= LOCK_STALE);
        if old {
            let away = dir.join(format!("{LOCK_FILE}.{}", std::process::id()));
            if std::fs::rename(&file, &away).is_ok() {
                let _ = std::fs::remove_file(&away);
            }
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

/// Ask GitHub now and wait, whatever the cache holds, and keep the answer. `klin update
/// --check`.
pub fn fetch() -> Option<Version> {
    let tag = asked();
    if let (Some(releases), Some(tag)) = (Releases::any(), &tag) {
        let mut latest = Latest::read(&releases.dir);
        latest.tag = Some(tag.clone());
        latest.checked = now();
        latest.write(&releases.dir);
    }
    tag
}

/// The newest release the cache holds, on a binary that may tell of one.
pub fn known() -> Option<Version> {
    Latest::read(&Releases::here()?.dir).tag
}

/// The version the person saw before this binary replaced it, read without consuming it.
pub fn updated_from() -> Option<Version> {
    let releases = Releases::here()?;
    Record::read(&releases.dir, Audience::Person)
        .seen
        .filter(Version::is_older)
}

/// The news `who` has not heard: an update first, else a newer release. The first sight of a
/// binary, and a binary older than the one last seen, records its version, so neither a fresh
/// install nor a downgrade is an update.
pub fn news(who: Audience) -> Option<News> {
    let dir = Releases::here()?.dir;
    let current = Version::current()?;
    let mut record = Record::read(&dir, who);
    match record.seen.take() {
        Some(seen) if seen < current => {
            return Some(News::Updated {
                from: seen,
                to: current,
            });
        }
        Some(seen) if seen == current => record.seen = Some(seen),
        _ => {
            record.seen = Some(current);
            record.write(&dir, who);
        }
    }
    Latest::read(&dir)
        .tag
        .filter(|tag| tag.is_newer() && record.told.as_ref() != Some(tag))
        .map(News::Available)
}

/// Record that `who` heard `news`.
pub fn heard(who: Audience, news: &News) {
    let Some(releases) = Releases::any() else {
        return;
    };
    let mut record = Record::read(&releases.dir, who);
    match news {
        News::Updated { to, .. } => record.seen = Some(to.clone()),
        News::Available(tag) => record.told = Some(tag.clone()),
    }
    record.write(&releases.dir, who);
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

/// `${KLIN_CACHE_DIR:-$HOME/.cache/klin}`, the directory the plugin wrapper uses.
fn cache_dir() -> Option<PathBuf> {
    std::env::var_os("KLIN_CACHE_DIR")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".cache/klin")))
}

/// A process with no terminal of its own: nothing in, nothing out.
fn silent(command: &mut Command) -> &mut Command {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
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

fn version(held: &Value) -> Option<Version> {
    held.as_str().and_then(Version::parse)
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
