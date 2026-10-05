use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::git::Repo;

/// klin's own state: the build stamp, the turn stamp, the journal and the cache. It lives
/// under the git directory, which git never tracks, never lists and never cleans, so klin
/// writes nothing the working tree can see. The guard reads this name too. ADR 0019, ADR 0032.
pub const DIR: &str = "klin";
pub const OVERRIDE: &str = "KLIN_STATE_DIR";
/// Names the tree a keyed directory under `KLIN_STATE_DIR` belongs to, so `cache clean --all`
/// can tell which entries outlived their repository.
pub const REPOSITORY: &str = "repository";
pub const CACHE: &str = "cache";
/// Where under the cache the base commits' structural outcomes are kept. Spec 8.4.
pub const STRUCTURAL: &str = "structural";

pub fn dir(root: &Path) -> Option<PathBuf> {
    match std::env::var_os(OVERRIDE) {
        Some(at) => Some(PathBuf::from(at).join(key(
            &Repo::at(root).rev_parse_path("--git-common-dir")?,
            root,
        ))),
        None => Some(
            Repo::at(root)
                .rev_parse_path("--absolute-git-dir")?
                .join(DIR),
        ),
    }
}

/// The state directory, created and proven writable. The caller reports the reason and carries
/// on: nothing klin writes for itself may block a stop.
pub fn ready(root: &Path) -> Result<PathBuf, String> {
    let Some(at) = dir(root) else {
        return Err(format!(
            "{} is not in a git repository, so klin has nowhere to keep its state",
            root.display()
        ));
    };
    prepared(&at, root)
}

/// The same, for a caller that resolved the directory already: `dir` runs a git subprocess, and
/// the guard holds its answer under a 50 millisecond budget (13).
pub fn prepared(at: &Path, root: &Path) -> Result<PathBuf, String> {
    if let Err(why) = std::fs::create_dir_all(at) {
        return Err(format!("{} could not be written: {why}", at.display()));
    }
    let named = at.join(REPOSITORY);
    if std::env::var_os(OVERRIDE).is_some() && !named.is_file() {
        let _ = std::fs::write(&named, worktree(root).display().to_string() + "\n");
    }
    Ok(at.to_path_buf())
}

fn key(common: &Path, root: &Path) -> String {
    let mut bytes = common.display().to_string().into_bytes();
    bytes.push(0);
    bytes.extend(worktree(root).display().to_string().into_bytes());
    format!("{:016x}", hash(&bytes))
}

fn worktree(root: &Path) -> PathBuf {
    Repo::at(root)
        .rev_parse_path("--show-toplevel")
        .unwrap_or_else(|| root.to_path_buf())
}

pub fn hash(bytes: &[u8]) -> u64 {
    let mut sum: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        sum ^= u64::from(*byte);
        sum = sum.wrapping_mul(0x100_0000_01b3);
    }
    sum
}

/// The advisory lock over a state transaction or private capture. Stops hold the state lock
/// through their verdict; prompts hold it through publishing the turn. Dropping it unlocks.
/// Spec 6.5.
pub struct Lock(std::fs::File);

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

const LOCK: &str = "lock";
const WAITED: Duration = Duration::from_millis(25);

/// The lock, or `None` when it could not be acquired within the caller's budget.
pub fn lock(at: &Path, budget: Duration) -> Option<Lock> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(at.join(LOCK))
        .ok()?;
    let until = Instant::now() + budget;
    loop {
        if file.try_lock().is_ok() {
            return Some(Lock(file));
        }
        if Instant::now() >= until {
            return None;
        }
        std::thread::sleep(WAITED);
    }
}

/// Where each copy of klin's hooks records the host event it took. Spec 9.8.
const CLAIMS: &str = "claims";
/// How long after the copy that took an event finished a copy of the same event still yields
/// to it. The host starts every copy of one event together and the next event only after all of
/// them answered, so the gap between copies is a process start, and two events that name the
/// same fields are a model turn apart.
/// ponytail: one fixed window, a copy whose wrapper downloads the binary for longer runs the
/// event again; record the copy's own start time if that shows up.
const SETTLED: Duration = Duration::from_secs(2);
/// How long a claim file outlives its event before a later claim removes it.
const KEPT: Duration = Duration::from_secs(3600);

/// One copy's hold on a host event, from before it acts until after. Dropping it stamps the
/// time the event finished and lets it go. Spec 9.8.
pub struct Claim(Option<std::fs::File>);

impl Drop for Claim {
    fn drop(&mut self) {
        if let Some(file) = &self.0 {
            let _ = file.set_modified(std::time::SystemTime::now());
            let _ = file.unlock();
        }
    }
}

/// The claim on the event `identity` names, or `None` when another copy of klin's hooks holds
/// it or finished it moments ago, and this copy yields. An event that names nothing, and a
/// state directory klin cannot write, are claimed by every copy: running an event twice is the
/// old failure, and running it never would drop a block. Spec 9.8.
pub fn claim(at: &Path, identity: &str) -> Option<Claim> {
    if identity.is_empty() {
        return Some(Claim(None));
    }
    let dir = at.join(CLAIMS);
    if std::fs::create_dir_all(&dir).is_err() {
        return Some(Claim(None));
    }
    let path = dir.join(format!("{:016x}", hash(identity.as_bytes())));
    swept(&dir, &path);
    let mut open = std::fs::OpenOptions::new();
    open.write(true);
    if let Ok(file) = open.clone().create_new(true).open(&path) {
        let _ = file.lock();
        return Some(Claim(Some(file)));
    }
    let Ok(file) = open.open(&path) else {
        return Some(Claim(None));
    };
    if file.try_lock().is_err() || settled_within(&file, SETTLED) {
        return None;
    }
    let _ = file.set_modified(std::time::SystemTime::now());
    Some(Claim(Some(file)))
}

/// The same, for a caller that has not resolved the state directory. A tree klin cannot keep
/// state for claims nothing, so every copy acts.
pub fn claimed(root: &Path, identity: &str) -> Option<Claim> {
    if identity.is_empty() {
        return Some(Claim(None));
    }
    match ready(root) {
        Ok(at) => claim(&at, identity),
        Err(_) => Some(Claim(None)),
    }
}

fn settled_within(file: &std::fs::File, window: Duration) -> bool {
    file.metadata()
        .and_then(|held| held.modified())
        .ok()
        .and_then(|at| at.elapsed().ok())
        .is_some_and(|age| age < window)
}

/// Every claim no copy holds that finished longer ago than any copy could still arrive, except
/// this event's own: another copy of it may have opened that file already, and a new file under
/// the same name would let both copies act.
fn swept(dir: &Path, own: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for path in entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path != own)
    {
        let Ok(file) = std::fs::OpenOptions::new().write(true).open(&path) else {
            continue;
        };
        if !settled_within(&file, KEPT) && file.try_lock().is_ok() {
            let _ = std::fs::remove_file(&path);
        }
    }
}
