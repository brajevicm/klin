//! klin keeps itself current, or says how to update the plugin that carries it. A session
//! start, `klin status` and `klin setup` ask GitHub for the newest release at most once a day,
//! in a detached child, so nothing waits on the network. When the person turned auto-update
//! on, the child also installs a newer release whose signature checks out. Every other command, a Stop among
//! them, reads only the cache. Only a binary the release workflow built asks, updates on its own
//! or tells, and so does a plugin's binary, which the plugin wrapper marks with `KLIN_PLUGIN`.
//! `klin update` installs over any binary but a plugin's, whose host owns it. Spec 11.8, 11.9.

use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::sys::signed;
use crate::sys::state::{self, Lock};
use crate::sys::write::{AtomicWrite, atomic_write};

const LATEST: &str = "https://github.com/brajevicm/klin/releases/latest";
const TAGGED: &str = "https://github.com/brajevicm/klin/releases/tag/v";
const DAY: u64 = 24 * 60 * 60;
const LATEST_FILE: &str = "latest.json";
/// Held by the one detached child that asks and updates, for as long as it runs, and by
/// `klin update`, so two installs never run at once.
const ASK_LOCK: &str = "release-ask.lock";
/// Held around every read and rewrite of a cache file, never across the network or a host's
/// delivery, so two klin processes never lose each other's change or tell one piece of news
/// twice.
const CACHE_LOCK: &str = "release-cache.lock";
/// How long `klin update` waits for an install or an ask in progress to end.
const INSTALL_WAIT: Duration = Duration::from_secs(60);
/// How long a hook waits for the cache lock before it leaves the news for a later event.
const CACHE_WAIT: Duration = Duration::from_millis(500);
/// The person's own auto-update setting, which `klin update --auto` writes. A file reaches a hook
/// whatever environment the host passes it, which a shell profile does not.
const AUTO_FILE: &str = "auto-update";
/// The hidden argument the detached child runs under. It is not a public command.
pub const CHILD: &str = "__release";
/// The child's flag that names the expected version it asks early for.
const EARLY: &str = "--expected";
/// Whether the release workflow built this binary, which it marks with `KLIN_RELEASE_BUILD` at
/// compile time. A debug build reads `KLIN_TEST_RELEASE_BUILD` at run time instead, so a test
/// stands for one.
const RELEASE_BUILD: bool =
    matches!(option_env!("KLIN_RELEASE_BUILD"), Some(value) if !value.is_empty());
/// The shape of the `klin` key, which the schema and the Action read too.
pub const EXPECTED_PATTERN: &str = r"^>=[0-9]+\.[0-9]+\.[0-9]+$";
/// The words a configuration error uses for that shape.
pub const EXPECTED_SHAPE: &str =
    "\">=\" and a version of three dot-separated numbers, such as \">=0.5.4\"";

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

/// The lowest klin a repository expects, as its `klin.json` states it under `klin`: `>=` and a
/// release version of three dot-separated numbers. Spec 5.3, 11.9.
#[derive(Clone, PartialEq, Eq)]
pub struct Expected(Version);

impl FromStr for Expected {
    type Err = &'static str;

    /// `>=` and three dot-separated numbers, or the shape it should have.
    fn from_str(text: &str) -> Result<Expected, &'static str> {
        text.strip_prefix(">=")
            .filter(|version| {
                version
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'.')
            })
            .and_then(Version::parse)
            .filter(|version| version.0.len() == 3)
            .map(Expected)
            .ok_or(EXPECTED_SHAPE)
    }
}

impl Expected {
    /// Whether this build meets it. True when the build's own version does not parse, so an
    /// unknown build is never told to update.
    pub fn met(&self) -> bool {
        Version::current().is_none_or(|own| own >= self.0)
    }

    /// Whether `tag` is a release that meets it.
    fn met_by(&self, tag: Option<&Version>) -> bool {
        tag.is_some_and(|tag| *tag >= self.0)
    }

    /// What the person reads when this build does not meet it, with the step that installs a
    /// newer klin on `route`.
    pub fn note(&self, route: Route, host_step: Option<String>) -> String {
        format!(
            "This repository expects klin {self} (you have {}). {}",
            env!("CARGO_PKG_VERSION"),
            route.step(host_step)
        )
    }

    pub fn version(&self) -> &Version {
        &self.0
    }
}

impl fmt::Display for Expected {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, ">={}", self.0)
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
    /// Each route keeps its own records, so a machine with both a release build and a
    /// plugin's klin at two versions never reads the switch between them as an update. A build
    /// from source keeps no record, because it never tells, and names the release build's.
    fn record(self, route: Route) -> &'static str {
        match (route, self) {
            (Route::Plugin, Audience::Person) => "release-plugin-person.json",
            (Route::Plugin, Audience::Agent) => "release-plugin-agent.json",
            (Route::Release | Route::Source, Audience::Person) => "release-person.json",
            (Route::Release | Route::Source, Audience::Agent) => "release-agent.json",
        }
    }

    /// Whether this audience hears of the same available release again each day until the
    /// person updates. The agent forgets between sessions, so it is told daily. The person sees
    /// the release in `klin status` and is told once.
    fn daily(self) -> bool {
        matches!(self, Audience::Agent)
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
        let route = Route::here();
        let on = route != Route::Source && !set("CI") && !set("KLIN_NO_UPDATE_CHECK");
        cache_dir()
            .filter(|_| on)
            .map(|dir| Releases { dir, route })
    }

    /// The short lock around one read and rewrite of a cache file.
    fn cache_lock(&self) -> Option<Lock> {
        locked(&self.dir, CACHE_LOCK, CACHE_WAIT)
    }
}

/// What `latest.json` holds: the newest tag GitHub named, when klin last asked, the release
/// auto-update last tried, why that release was not installed, whether a later try may pass,
/// and the expected version klin last asked early for.
#[derive(Default)]
struct Latest {
    tag: Option<Version>,
    checked: u64,
    tried: Option<Version>,
    refused: Option<String>,
    retry: bool,
    asked_early: Option<Version>,
}

impl Latest {
    fn read(dir: &Path) -> Latest {
        let held = read_json(&dir.join(LATEST_FILE));
        Latest {
            tag: version(&held["tag"]),
            checked: held["checked"].as_u64().unwrap_or_default(),
            tried: version(&held["tried"]),
            refused: held["refused"].as_str().map(str::to_string),
            retry: held["retry"].as_bool().unwrap_or_default(),
            asked_early: version(&held["asked_early"]),
        }
    }

    fn write(&self, dir: &Path) -> io::Result<()> {
        let held = json!({
            "tag": self.tag.as_ref().map(ToString::to_string),
            "checked": self.checked,
            "tried": self.tried.as_ref().map(ToString::to_string),
            "refused": self.refused,
            "retry": self.retry,
            "asked_early": self.asked_early.as_ref().map(ToString::to_string),
        });
        write(&dir.join(LATEST_FILE), held.to_string().as_bytes())
    }

    /// Read and change `latest.json` under the cache lock, so a change another klin made in
    /// between is never lost, and rewrite it when `change` returns a value. That value, or none
    /// when the change made nothing, or the lock or the write failed.
    fn change<T>(dir: &Path, change: impl FnOnce(&mut Latest) -> Option<T>) -> Option<T> {
        let _lock = locked(dir, CACHE_LOCK, CACHE_WAIT)?;
        let mut latest = Latest::read(dir);
        let changed = change(&mut latest)?;
        latest.write(dir).ok().map(|()| changed)
    }

    fn stale(&self) -> bool {
        now().saturating_sub(self.checked) >= DAY
    }

    /// Keep `tag` when it is newer than the tag already cached. An answer that arrives late,
    /// after a newer one was saved, never moves the cache back.
    fn learn(&mut self, tag: Version) {
        if self.tag.as_ref().is_none_or(|known| *known < tag) {
            self.tag = Some(tag);
        }
    }

    /// Whether auto-update asks now, without waiting for the day's ask: the repository expects
    /// a klin newer than this build and than the cached release, and klin has not asked early
    /// for that version yet. A version no release meets is asked for once, and the day's ask
    /// finds it later.
    fn early(&self, route: Route, wanted: &Expected) -> bool {
        route == Route::Release
            && auto()
            && !wanted.met()
            && !wanted.met_by(self.tag.as_ref())
            && self.asked_early.as_ref() != Some(wanted.version())
    }

    /// The newer release auto-update has not tried, so a failing updater runs once per release.
    /// Only a release build that is not a plugin's updates itself.
    fn untried(&self, route: Route) -> Option<&Version> {
        let release = route == Route::Release;
        self.tag
            .as_ref()
            .filter(|tag| release && auto() && tag.is_newer() && self.tried.as_ref() != Some(tag))
    }
}

/// What one audience's record holds: the newest version it saw klin run as, and the release it
/// was last told of.
#[derive(Clone, Default)]
struct Record {
    seen: Option<Version>,
    told: Option<Version>,
    told_at: u64,
}

impl Record {
    fn read(releases: &Releases, who: Audience) -> Record {
        let held = read_json(&releases.dir.join(who.record(releases.route)));
        Record {
            seen: version(&held["seen"]),
            told: version(&held["told"]),
            told_at: held["told_at"].as_u64().unwrap_or_default(),
        }
    }

    /// Record `news` as heard by `who`, and say whether it was news: an update to a version
    /// newer than any it saw, or a release it was not told of. An audience that hears daily is
    /// told of the same release again once a day has passed. The one place the rule lives, for
    /// the hook that reads the news and the hook that claims it.
    fn hear(&mut self, who: Audience, news: &News) -> bool {
        let due = |record: &Record, tag: &Version| {
            record.told.as_ref() != Some(tag)
                || (who.daily() && now().saturating_sub(record.told_at) >= DAY)
        };
        match news {
            News::Updated { to, .. } if self.seen.as_ref().is_none_or(|seen| seen < to) => {
                self.seen = Some(to.clone());
                true
            }
            News::Available(tag) if due(self, tag) => {
                self.told = Some(tag.clone());
                self.told_at = now();
                true
            }
            _ => false,
        }
    }

    fn write(&self, releases: &Releases, who: Audience) -> io::Result<()> {
        let held = json!({
            "seen": self.seen.as_ref().map(ToString::to_string),
            "told": self.told.as_ref().map(ToString::to_string),
            "told_at": self.told_at,
        });
        let file = releases.dir.join(who.record(releases.route));
        write(&file, held.to_string().as_bytes())
    }
}

/// How this binary reached the machine: through a host's plugin, whose wrapper sets
/// `KLIN_PLUGIN`, as a release build, from the installer or an archive, or built from source.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Release,
    Plugin,
    Source,
}

impl Route {
    /// The wrapper's marker comes first, because a plugin runs a release build too.
    fn here() -> Route {
        if set("KLIN_PLUGIN") {
            Route::Plugin
        } else if RELEASE_BUILD || (cfg!(debug_assertions) && set("KLIN_TEST_RELEASE_BUILD")) {
            Route::Release
        } else {
            Route::Source
        }
    }

    /// The one step that installs a newer release on this route: `klin update`, or the plugin
    /// step of the host a hook event came from.
    fn step(self, host_step: Option<String>) -> String {
        match self {
            Route::Release | Route::Source => "Run klin update.".to_string(),
            Route::Plugin => {
                host_step.unwrap_or_else(|| "Update the klin plugin in your host.".to_string())
            }
        }
    }
}

/// The route of this binary.
pub fn route() -> Route {
    Route::here()
}

/// From a session start, `klin status` or `klin setup`: start the detached child when the last
/// ask is a day old, when auto-update holds a newer release it has not tried, or when
/// auto-update asks early for the klin the repository `expected`. The child
/// decides under its own lock, so two commands at once start at most one ask and one update.
/// Before it starts, each audience's record holds this binary's version, so an update the child
/// runs before any hook still tells which version it replaced.
pub fn refresh(expected: Option<&Expected>) {
    let Some(releases) = Releases::here() else {
        return;
    };
    let latest = Latest::read(&releases.dir);
    let early = expected.filter(|wanted| latest.early(releases.route, wanted));
    if !latest.stale() && latest.untried(releases.route).is_none() && early.is_none() {
        return;
    }
    for who in [Audience::Person, Audience::Agent] {
        first_sight(&releases, who);
    }
    if let Ok(exe) = installed() {
        let mut child = Command::new(exe);
        child.arg(CHILD).args(
            early
                .map(|wanted| [EARLY.to_string(), wanted.version().to_string()])
                .into_iter()
                .flatten(),
        );
        let _ = silent(&mut child).process_group(0).spawn();
    }
}

/// The detached child, under the ask lock: the day's ask, which counts from its start so a
/// failed or slow ask is the day's ask, or the early ask for the expected version its argument
/// names, then under auto-update the signed install, once for each
/// newer release. A refused install keeps its reason for `klin status`. A child that finds the
/// ask lock taken does nothing. Every change to the cache takes the cache lock for that change
/// alone.
pub fn child() {
    let Some(releases) = Releases::here() else {
        return;
    };
    let dir = &releases.dir;
    let Some(_ask) = locked(dir, ASK_LOCK, Duration::ZERO) else {
        return;
    };
    let early = std::env::args()
        .skip_while(|arg| arg != EARLY)
        .nth(1)
        .as_deref()
        .and_then(Version::parse);
    ask_today(dir, early);
    install_untried(dir, releases.route);
}

/// The day's ask, which counts from its start. A release whose last try failed in a way that
/// may pass is tried again with the day's ask, so a failure that passes costs a day, and one
/// that does not pass is tried once a day at most. An `early` ask for an expected version runs
/// once for that version, whatever the day's ask did.
fn ask_today(dir: &Path, early: Option<Version>) {
    let latest = Latest::read(dir);
    let early = early.filter(|wanted| latest.asked_early.as_ref() != Some(wanted));
    if (latest.stale() || early.is_some())
        && Latest::change(dir, |latest| {
            latest.checked = now();
            if early.is_some() {
                latest.asked_early.clone_from(&early);
            }
            if latest.retry {
                latest.tried = None;
                latest.retry = false;
            }
            Some(())
        })
        .is_some()
        && let Some(tag) = asked()
    {
        Latest::change(dir, |latest| {
            latest.learn(tag);
            Some(())
        });
    }
}

/// Under auto-update, install the newer release not tried yet, unless the binary on disk is
/// already that release or newer, and keep why it failed.
/// The attempt counts as one that may pass until it ends, so a child killed during it leaves
/// the release to the next day's ask.
fn install_untried(dir: &Path, route: Route) {
    let Ok(exe) = installed() else {
        return;
    };
    let claimed = Latest::change(dir, |latest| {
        let tag = latest.untried(route).cloned()?;
        latest.tried = Some(tag.clone());
        latest.refused = None;
        latest.retry = true;
        Some(tag)
    });
    let Some(tag) = claimed.filter(|tag| older_on_disk(&exe, tag)) else {
        return;
    };
    if let Err(failure) = signed::install(&tag.to_string(), &exe) {
        Latest::change(dir, |latest| {
            latest.refused = Some(failure.to_string());
            latest.retry = failure.retry();
            Some(())
        });
    }
}

/// Whether the klin at `exe` is older than `tag`. Another klin may have replaced it since this
/// process started, so the file itself answers. A klin whose version it cannot read counts as
/// older.
fn older_on_disk(exe: &Path, tag: &Version) -> bool {
    Command::new(exe)
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|text| {
            let own = text.trim().strip_prefix("klin ")?;
            Version::parse(own.split('-').next().unwrap_or(own))
        })
        .is_none_or(|on_disk| on_disk < *tag)
}

/// Why auto-update did not install `tag`, when it tried and was refused.
pub fn refused(tag: &Version) -> Option<String> {
    let latest = Latest::read(&Releases::here()?.dir);
    latest
        .refused
        .filter(|_| latest.tried.as_ref() == Some(tag))
}

/// `klin update`: ask GitHub now, and install the newest release when it is newer than this
/// binary and its signature checks out. The release installed, or none when this build is the
/// newest. Any binary but a plugin's updates, a build from source too. One in a directory klin
/// cannot write says so before it asks.
pub fn update() -> Result<Option<Version>, String> {
    if Route::here() == Route::Plugin {
        return Err(PLUGIN_UPDATE.to_string());
    }
    let exe = installed()
        .and_then(|exe| exe.canonicalize())
        .map_err(|why| format!("klin could not find itself: {why}"))?;
    writable(&exe)?;
    let cache = cache_dir().ok_or("klin found no cache directory to hold its install lock.")?;
    let _install = locked(&cache, ASK_LOCK, INSTALL_WAIT).ok_or(BUSY)?;
    let Some(tag) = newer_than(&exe)? else {
        return Ok(None);
    };
    signed::install(&tag.to_string(), &exe).map_err(|failure| failure.to_string())?;
    Ok(Some(tag))
}

/// The newest release, asked of GitHub now, when it is newer than this build and than the klin
/// at `exe`, which another klin may have replaced since this one started.
fn newer_than(exe: &Path) -> Result<Option<Version>, String> {
    let tag = asked().ok_or("klin could not ask GitHub for the newest release.")?;
    Ok(Some(tag).filter(|tag| tag.is_newer() && older_on_disk(exe, tag)))
}

/// Ask GitHub now and wait, whatever the cache holds, and keep the answer: the tag GitHub
/// named, and whether the cache saved it. `klin update --check`.
pub fn fetch() -> Option<(Version, bool)> {
    let tag = asked()?;
    let saved = cache_dir()
        .and_then(|dir| {
            Latest::change(&dir, |latest| {
                latest.learn(tag.clone());
                latest.checked = now();
                Some(())
            })
        })
        .is_some();
    Some((tag, saved))
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
    if record.seen.is_none() {
        first_sight(&releases, who)?;
    }
    let updated = record
        .seen
        .clone()
        .filter(|seen| *seen < current)
        .map(|from| News::Updated { from, to: current });
    let news = updated.or_else(|| {
        Latest::read(&releases.dir)
            .tag
            .filter(Version::is_newer)
            .map(News::Available)
    })?;
    record
        .clone()
        .hear(who, &news)
        .then_some((releases.route, news))
}

/// Record this binary's version for `who` when its record holds none, so an update that runs
/// before any hook still reads as an update from this version. None when the lock failed.
fn first_sight(releases: &Releases, who: Audience) -> Option<()> {
    let current = Version::current()?;
    let _lock = releases.cache_lock()?;
    let mut record = Record::read(releases, who);
    if record.seen.is_none() {
        record.seen = Some(current);
        let _ = record.write(releases, who);
    }
    Some(())
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
    record.hear(who, news) && record.write(&releases, who).is_ok()
}

/// Whether auto-update is on: `KLIN_AUTO_UPDATE=1` or `0` when set, else the setting
/// `klin update --auto` wrote, else off: a person opts in to a binary that replaces itself.
fn auto() -> bool {
    match std::env::var_os("KLIN_AUTO_UPDATE") {
        Some(value) if value == "1" => true,
        Some(value) if value == "0" => false,
        _ => cache_dir()
            .and_then(|dir| std::fs::read_to_string(dir.join(AUTO_FILE)).ok())
            .is_some_and(|held| held.trim() == "on"),
    }
}

/// Whether this binary installs a newer release on its own now: a release build, with the
/// check on, outside CI, and with auto-update on.
pub fn auto_update() -> bool {
    Releases::here().is_some_and(|releases| releases.route == Route::Release) && auto()
}

const PLUGIN_SETTING: &str = "this klin came with a plugin, and its host updates the plugin. Turn \
                              on the host's own plugin auto-update instead.";
const SOURCE_BUILD: &str = "this klin was built from source, and only a release build asks for \
                            releases or installs one on its own. Run klin update by hand.";
const BUSY: &str = "another klin is still asking for or installing a release after a minute. Run \
                    klin update again when it ends.";
const PLUGIN_UPDATE: &str = "this klin came with a plugin, and its host updates the plugin.";
const INSTALLER: &str = "curl --proto '=https' --tlsv1.2 -LsSf \
                         https://github.com/brajevicm/klin/releases/latest/download/\
                         klin-installer.sh | sh";

/// Whether klin may put a new binary beside `exe`. The error names the installer, which
/// installs where the person can write. It names no `sudo`: a root klin would write root's files
/// into the person's cache.
fn writable(exe: &Path) -> Result<(), String> {
    signed::writable(exe).map_err(|why| {
        format!(
            "klin cannot write {}, where this klin is installed: {why}. Install klin where you \
             can write with the installer, and remove this klin: {INSTALLER}",
            exe.parent().unwrap_or(Path::new("/")).display()
        )
    })
}

/// The first other `klin` PATH finds before the binary `klin update` replaced, which a shell
/// and klin's hooks run instead of it. None when PATH finds this one first, or none at all. Like
/// a shell, it skips a relative entry and a file that is not executable.
pub fn shadowed() -> Option<PathBuf> {
    let exe = installed().ok()?.canonicalize().ok()?;
    let first = std::env::split_paths(&std::env::var_os("PATH")?)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join("klin"))
        .find(|path| {
            std::fs::metadata(path).is_ok_and(|held| held.is_file() && held.mode() & 0o111 != 0)
        })?;
    (first.canonicalize().ok()? != exe).then_some(first)
}

/// Write the person's auto-update setting. Only a release build asks for releases, and a
/// plugin's host owns the plugin, so any other binary says why the setting would change nothing.
/// A setting klin could not write is an error, never a claim that the switch moved.
pub fn set_auto(on: bool) -> Result<(), String> {
    match Route::here() {
        Route::Release => {}
        Route::Plugin => return Err(PLUGIN_SETTING.to_string()),
        Route::Source => return Err(SOURCE_BUILD.to_string()),
    }
    let dir = cache_dir().ok_or("klin found no cache directory, so nothing was written.")?;
    let file = dir.join(AUTO_FILE);
    write(&file, if on { b"on\n" } else { b"off\n" })
        .map_err(|why| format!("{} could not be written: {why}", file.display()))
}

fn set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.is_empty())
}

/// Where this binary is installed. Linux names a running binary that another process replaced
/// on disk with a ` (deleted)` suffix, and the klin now installed is at the path without it.
pub fn installed() -> io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    Ok(
        match exe.as_os_str().as_bytes().strip_suffix(b" (deleted)") {
            Some(path) => PathBuf::from(OsStr::from_bytes(path)),
            None => exe,
        },
    )
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
