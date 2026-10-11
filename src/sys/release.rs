//! klin keeps itself current, or says how to update the plugin that carries it. A session
//! start, `klin status` and `klin setup` ask GitHub for the newest release at most once a day,
//! in a detached child, so nothing waits on the network. Under `KLIN_AUTO_UPDATE=1` the child
//! also runs the `klin-update` beside the binary. Every other command, a Stop among them, reads
//! only the cache. Only a binary the installer placed, with its `klin-update` beside it, asks,
//! updates or tells, and so does a plugin's binary, which the plugin wrapper marks with
//! `KLIN_PLUGIN`. A plugin's binary never updates itself: its host owns the plugin. Spec 11.9.

use std::fmt;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::sys::state::{self, Lock};
use crate::sys::write::{AtomicWrite, atomic_write};

const LATEST: &str = "https://github.com/brajevicm/klin/releases/latest";
const TAGGED: &str = "https://github.com/brajevicm/klin/releases/tag/v";
const DAY: u64 = 24 * 60 * 60;
const LATEST_FILE: &str = "latest.json";
/// Held by the one detached child that asks and updates, for as long as it runs.
const ASK_LOCK: &str = "release-ask.lock";
/// Held around every read and rewrite of a cache file, never across the network or a host's
/// delivery, so two klin processes never lose each other's change or tell one piece of news
/// twice.
const CACHE_LOCK: &str = "release-cache.lock";
/// How long a hook waits for the cache lock before it leaves the news for a later event.
const CACHE_WAIT: Duration = Duration::from_millis(500);
/// The person's own auto-update setting, which `klin update --auto` writes. A file reaches a hook
/// whatever environment the host passes it, which a shell profile does not.
const AUTO_FILE: &str = "auto-update";
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
    /// Each route keeps its own records, so a machine with both an installer's klin and a
    /// plugin's klin at two versions never reads the switch between them as an update.
    fn record(self, route: Route) -> &'static str {
        match (route, self) {
            (Route::Plugin, Audience::Person) => "release-plugin-person.json",
            (Route::Plugin, Audience::Agent) => "release-plugin-agent.json",
            (Route::Installer, Audience::Person) => "release-person.json",
            (Route::Installer, Audience::Agent) => "release-agent.json",
        }
    }

    /// The text for this audience on `route`. On a plugin's route, `host_step` is the step the
    /// host a hook event came from takes to update the plugin.
    pub fn note(self, route: Route, news: &News, host_step: Option<String>) -> String {
        match (self, news) {
            (Audience::Person, News::Available(tag)) => format!(
                "klin {tag} is available (you have {}). {}",
                env!("CARGO_PKG_VERSION"),
                route.step(host_step)
            ),
            (Audience::Agent, News::Available(tag)) if route == Route::Plugin => format!(
                "klin {tag} is available. Tell the person to update the klin plugin. Do not do \
                 it yourself."
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

/// The cache directory of a binary that may ask, update or tell, outside CI and with the check
/// on, and the route it reached the machine by. Every public entry builds it first, so the rule
/// is in one place.
struct Releases {
    dir: PathBuf,
    route: Route,
}

impl Releases {
    fn here() -> Option<Releases> {
        let route = Route::here()?;
        let on = !set("CI") && !set("KLIN_NO_UPDATE_CHECK");
        cache_dir()
            .filter(|_| on)
            .map(|dir| Releases { dir, route })
    }

    /// The short lock around one read and rewrite of a cache file.
    fn cache_lock(&self) -> Option<Lock> {
        locked(&self.dir, CACHE_LOCK, CACHE_WAIT)
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

    fn write(&self, dir: &Path) -> io::Result<()> {
        let held = json!({
            "tag": self.tag.as_ref().map(ToString::to_string),
            "checked": self.checked,
            "tried": self.tried.as_ref().map(ToString::to_string),
        });
        write(&dir.join(LATEST_FILE), held.to_string().as_bytes())
    }

    /// Read, change and rewrite `latest.json` under the cache lock, so a change another klin
    /// made in between is never lost. False when the lock or the write failed.
    fn change(dir: &Path, change: impl FnOnce(&mut Latest)) -> bool {
        let Some(_lock) = locked(dir, CACHE_LOCK, CACHE_WAIT) else {
            return false;
        };
        let mut latest = Latest::read(dir);
        change(&mut latest);
        latest.write(dir).is_ok()
    }

    fn stale(&self) -> bool {
        now().saturating_sub(self.checked) >= DAY
    }

    /// The newer release auto-update has not tried, so a failing updater runs once per release.
    /// Only the installer's route updates itself.
    fn untried(&self, route: Route) -> Option<&Version> {
        let installer = route == Route::Installer;
        self.tag
            .as_ref()
            .filter(|tag| installer && auto() && tag.is_newer() && self.tried.as_ref() != Some(tag))
    }
}

/// What one audience's record holds: the newest version it saw klin run as, and the release it
/// was last told of.
#[derive(Default)]
struct Record {
    seen: Option<Version>,
    told: Option<Version>,
}

impl Record {
    fn read(releases: &Releases, who: Audience) -> Record {
        let held = read_json(&releases.dir.join(who.record(releases.route)));
        Record {
            seen: version(&held["seen"]),
            told: version(&held["told"]),
        }
    }

    fn write(&self, releases: &Releases, who: Audience) -> io::Result<()> {
        let held = json!({
            "seen": self.seen.as_ref().map(ToString::to_string),
            "told": self.told.as_ref().map(ToString::to_string),
        });
        let file = releases.dir.join(who.record(releases.route));
        write(&file, held.to_string().as_bytes())
    }
}

/// How this binary reached the machine: through a host's plugin, whose wrapper sets
/// `KLIN_PLUGIN`, or through the installer, with `klin-update` beside it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Installer,
    Plugin,
}

impl Route {
    /// The wrapper's marker comes first, so a plugin's binary never runs an updater that
    /// happens to sit beside it.
    fn here() -> Option<Route> {
        if set("KLIN_PLUGIN") {
            Some(Route::Plugin)
        } else if updater_path().is_some() {
            Some(Route::Installer)
        } else {
            None
        }
    }

    /// The one step that installs a newer release on this route: `klin update`, or the plugin
    /// step of the host a hook event came from.
    fn step(self, host_step: Option<String>) -> String {
        match self {
            Route::Installer => "Run klin update.".to_string(),
            Route::Plugin => {
                host_step.unwrap_or_else(|| "Update the klin plugin in your host.".to_string())
            }
        }
    }
}

/// The route of this binary, or none when it may not ask, update or tell.
pub fn route() -> Option<Route> {
    Route::here()
}

/// From a session start, `klin status` or `klin setup`: start the detached child when the last
/// ask is a day old, or when auto-update holds a newer release it has not tried. The child
/// decides under its own lock, so two commands at once start at most one ask and one update.
pub fn refresh() {
    let Some(releases) = Releases::here() else {
        return;
    };
    let latest = Latest::read(&releases.dir);
    if !latest.stale() && latest.untried(releases.route).is_none() {
        return;
    }
    if let Ok(exe) = std::env::current_exe() {
        let _ = silent(Command::new(exe).arg(CHILD))
            .process_group(0)
            .spawn();
    }
}

/// The detached child, under the ask lock: the day's ask, which counts from its start so a
/// failed or slow ask is the day's ask, then under auto-update the updater, once for each newer
/// release. A child that finds the ask lock taken does nothing. Every change to the cache takes
/// the cache lock for that change alone.
pub fn child() {
    let Some(releases) = Releases::here() else {
        return;
    };
    let dir = &releases.dir;
    let Some(_ask) = locked(dir, ASK_LOCK, Duration::ZERO) else {
        return;
    };
    if Latest::read(dir).stale()
        && Latest::change(dir, |latest| latest.checked = now())
        && let Some(tag) = asked()
    {
        Latest::change(dir, |latest| latest.tag = Some(tag));
    }
    let route = releases.route;
    let mut claimed = None;
    let tried = Latest::change(dir, |latest| {
        claimed = latest.untried(route).cloned();
        if claimed.is_some() {
            latest.tried.clone_from(&claimed);
        }
    });
    if let (true, Some(_), Some(updater)) = (tried, claimed, updater_path()) {
        let _ = silent(&mut Command::new(updater)).status();
    }
}

/// Ask GitHub now and wait, whatever the cache holds, and keep the answer. `klin update
/// --check`.
pub fn fetch() -> Option<Version> {
    let tag = asked();
    if let (Some(dir), Some(tag)) = (cache_dir(), &tag) {
        Latest::change(&dir, |latest| {
            latest.tag = Some(tag.clone());
            latest.checked = now();
        });
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
    Record::read(&releases, Audience::Person)
        .seen
        .filter(Version::is_older)
}

/// The news `who` has not heard, and the route it reads it on: an update first, else a newer
/// release. A record keeps the newest version an audience saw, so neither a fresh install, nor
/// a downgrade, nor two hosts whose plugins run two versions ever reads as an update. The news
/// is not yet the audience's: `claim` makes it so at delivery.
pub fn news(who: Audience) -> Option<(Route, News)> {
    let releases = Releases::here()?;
    let current = Version::current()?;
    let record = Record::read(&releases, who);
    match record.seen {
        Some(seen) if seen < current => {
            let news = News::Updated {
                from: seen,
                to: current,
            };
            return Some((releases.route, news));
        }
        Some(_) => {}
        None => {
            let _lock = releases.cache_lock()?;
            let mut first = Record::read(&releases, who);
            first.seen.get_or_insert(current);
            let _ = first.write(&releases, who);
        }
    }
    Latest::read(&releases.dir)
        .tag
        .filter(|tag| tag.is_newer() && record.told.as_ref() != Some(tag))
        .map(|tag| (releases.route, News::Available(tag)))
}

/// Make `news` the audience's own, at the moment it is told: under the cache lock, record it,
/// and say whether this process is the one that tells it. Two hooks at once that read the same
/// news both try, and one of them tells it. False when the lock or the write failed, so the
/// news waits for a later event.
pub fn claim(who: Audience, route: Route, news: &News) -> bool {
    let Some(dir) = cache_dir() else {
        return false;
    };
    let releases = Releases { dir, route };
    let Some(_lock) = releases.cache_lock() else {
        return false;
    };
    let mut record = Record::read(&releases, who);
    let fresh = match news {
        News::Updated { to, .. } => record.seen.as_ref().is_none_or(|seen| seen < to),
        News::Available(tag) => record.told.as_ref() != Some(tag),
    };
    if !fresh {
        return false;
    }
    match news {
        News::Updated { to, .. } => record.seen = Some(to.clone()),
        News::Available(tag) => record.told = Some(tag.clone()),
    }
    record.write(&releases, who).is_ok()
}

/// Whether auto-update is on: `KLIN_AUTO_UPDATE=1` or `0` when set, else the setting
/// `klin update --auto` wrote, else off until releases are signed (#627).
fn auto() -> bool {
    match std::env::var_os("KLIN_AUTO_UPDATE") {
        Some(value) if value == "1" => true,
        Some(value) if value == "0" => false,
        _ => cache_dir()
            .and_then(|dir| std::fs::read_to_string(dir.join(AUTO_FILE)).ok())
            .is_some_and(|held| held.trim() == "on"),
    }
}

/// Whether this binary installs a newer release on its own now: the installer's, with the
/// check on, outside CI, and with auto-update on.
pub fn auto_update() -> bool {
    Releases::here().is_some_and(|releases| releases.route == Route::Installer) && auto()
}

const PLUGIN_SETTING: &str = "this klin came with a plugin, and its host updates the plugin. Turn \
                              on the host's own plugin auto-update instead.";
const NO_UPDATER: &str = "auto-update runs the klin-update the installer puts beside klin, and \
                          none is there.";

/// Write the person's auto-update setting. Only the installer's binary updates itself, so a
/// plugin's binary or one with no updater beside it says why the setting would change nothing.
/// A setting klin could not write is an error, never a claim that the switch moved.
pub fn set_auto(on: bool) -> Result<(), String> {
    match Route::here() {
        Some(Route::Installer) => {}
        Some(Route::Plugin) => return Err(PLUGIN_SETTING.to_string()),
        None => return Err(NO_UPDATER.to_string()),
    }
    let dir = cache_dir().ok_or("klin found no cache directory, so nothing was written.")?;
    let file = dir.join(AUTO_FILE);
    write(&file, if on { b"on\n" } else { b"off\n" })
        .map_err(|why| format!("{} could not be written: {why}", file.display()))
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

/// The advisory lock `name` in the cache directory, waiting up to `budget`.
fn locked(dir: &Path, name: &str, budget: Duration) -> Option<Lock> {
    std::fs::create_dir_all(dir).ok()?;
    state::lock_file(&dir.join(name), budget)
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

fn write(file: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    atomic_write(AtomicWrite {
        target: file,
        bytes,
        keep_mode_from: None,
    })
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}
