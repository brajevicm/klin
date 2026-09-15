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

/// The advisory lock one stop holds over the state directory, from before it measures until
/// after it writes its verdict, so two stops in one worktree run in order and the last verdict
/// describes the last tree. Dropping it unlocks. Spec 6.5.
pub struct Lock(std::fs::File);

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

const LOCK: &str = "lock";
const WAITED: Duration = Duration::from_millis(25);

/// The lock, or `None` when another stop still held it when the budget ran out. A caller that
/// gets `None` measures anyway and writes no verdict.
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
