use std::path::{Path, PathBuf};
use std::process::Command;

/// klin's own state: the build stamp, and later the turn stamp and the survey cache. It lives
/// under the git directory, which git never tracks, never lists and never cleans, so klin
/// writes nothing the working tree can see. ADR 0019.
const DIR: &str = "klin";
pub const OVERRIDE: &str = "KLIN_STATE_DIR";
/// Names the tree a keyed directory under `KLIN_STATE_DIR` belongs to, so `cache clean --all`
/// can tell which entries outlived their repository.
pub const REPOSITORY: &str = "repository";
pub const CACHE: &str = "cache";

pub fn dir(root: &Path) -> Option<PathBuf> {
    let common = git(root, "--git-common-dir")?;
    match std::env::var_os(OVERRIDE) {
        Some(at) => Some(PathBuf::from(at).join(key(&common, root))),
        None => Some(git(root, "--absolute-git-dir")?.join(DIR)),
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
    if let Err(why) = std::fs::create_dir_all(&at) {
        return Err(format!("{} could not be written: {why}", at.display()));
    }
    let named = at.join(REPOSITORY);
    if std::env::var_os(OVERRIDE).is_some() && !named.is_file() {
        let _ = std::fs::write(&named, worktree(root).display().to_string() + "\n");
    }
    Ok(at)
}

fn key(common: &Path, root: &Path) -> String {
    let mut bytes = common.display().to_string().into_bytes();
    bytes.push(0);
    bytes.extend(worktree(root).display().to_string().into_bytes());
    format!("{:016x}", hash(&bytes))
}

fn worktree(root: &Path) -> PathBuf {
    git(root, "--show-toplevel").unwrap_or_else(|| root.to_path_buf())
}

fn git(root: &Path, flag: &str) -> Option<PathBuf> {
    let done = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--path-format=absolute", flag])
        .output()
        .ok()?;
    if !done.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&done.stdout).trim().to_string();
    (!text.is_empty()).then(|| PathBuf::from(text))
}

fn hash(bytes: &[u8]) -> u64 {
    let mut sum: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        sum ^= u64::from(*byte);
        sum = sum.wrapping_mul(0x100_0000_01b3);
    }
    sum
}
