//! The one atomic content replacement klin writes with: a temporary file beside the target,
//! optionally carrying the target's permissions, renamed over it, and removed best-effort when
//! any step fails. A run that dies partway leaves the file it found rather than a truncated
//! one. ADR 0041.
//!
//! Two kinds of write stay outside this helper: the journal's append-only JSONL, which is
//! best-effort and not a replacement, and a base file's move to a renamed path, which is a
//! semantic move of an existing path and not new content overwriting a target.

use std::io;
use std::path::{Path, PathBuf};

/// One replacement: `bytes` written over `target` atomically, and, where `keep_mode_from`
/// names a path, carrying that path's permissions into the replacement.
pub struct AtomicWrite<'a> {
    pub target: &'a Path,
    pub bytes: &'a [u8],
    pub keep_mode_from: Option<&'a Path>,
}

/// The write, through a temporary neighbour, cleaned up best-effort on failure. The caller
/// translates the `io::Error` into its own user-facing behavior.
pub fn atomic_write(write: AtomicWrite<'_>) -> io::Result<()> {
    let beside = temporary(write.target);
    let outcome = place(&beside, write.bytes, write.keep_mode_from)
        .and_then(|()| std::fs::rename(&beside, write.target));
    if outcome.is_err() {
        let _ = std::fs::remove_file(&beside);
    }
    outcome
}

/// A temporary neighbour, with one writer per target assumed by klin's state lock.
fn temporary(target: &Path) -> PathBuf {
    target.with_extension("writing")
}

fn place(beside: &Path, bytes: &[u8], keep_mode_from: Option<&Path>) -> io::Result<()> {
    std::fs::write(beside, bytes)?;
    if let Some(from) = keep_mode_from {
        if let Ok(held) = std::fs::metadata(from) {
            let _ = std::fs::set_permissions(beside, held.permissions());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn target(dir: &Path) -> PathBuf {
        dir.join("state.json")
    }

    #[test]
    fn a_missing_target_is_created() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let at = target(dir.path());
        atomic_write(AtomicWrite {
            target: &at,
            bytes: b"one\n",
            keep_mode_from: None,
        })
        .expect("the write");
        assert_eq!(std::fs::read(&at).unwrap_or_default(), b"one\n");
    }

    #[test]
    fn an_existing_target_is_replaced_whole() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let at = target(dir.path());
        std::fs::write(&at, b"old\n").expect("the old file");
        atomic_write(AtomicWrite {
            target: &at,
            bytes: b"new\n",
            keep_mode_from: None,
        })
        .expect("the write");
        assert_eq!(std::fs::read(&at).unwrap_or_default(), b"new\n");
    }

    #[cfg(unix)]
    #[test]
    fn the_target_mode_is_kept_when_asked() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("a temporary directory");
        let at = target(dir.path());
        std::fs::write(&at, b"old\n").expect("the old file");
        std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o600))
            .expect("the narrow mode");
        atomic_write(AtomicWrite {
            target: &at,
            bytes: b"new\n",
            keep_mode_from: Some(&at),
        })
        .expect("the write");
        let mode = std::fs::metadata(&at)
            .expect("the replacement")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "the replacement kept the narrow mode");
    }

    #[test]
    fn a_failed_write_leaves_the_old_target_and_no_temporary() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let at = target(dir.path());
        std::fs::write(&at, b"old\n").expect("the old file");
        // A directory where the temporary file would be written makes the write fail.
        let blocked = temporary(&at);
        std::fs::create_dir(&blocked).expect("the block");
        let outcome = atomic_write(AtomicWrite {
            target: &at,
            bytes: b"new\n",
            keep_mode_from: None,
        });
        assert!(outcome.is_err(), "the blocked write failed");
        assert_eq!(std::fs::read(&at).unwrap_or_default(), b"old\n");
        assert!(blocked.is_dir(), "the blocking directory remains");
        assert!(!blocked.is_file(), "no temporary file was left behind");
    }

    #[test]
    fn a_failed_rename_is_cleaned_up_best_effort() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let at = target(dir.path());
        std::fs::write(&at, b"old\n").expect("the old file");
        // A directory sitting where the rename must place the file makes the rename fail after
        // the temporary was written.
        std::fs::remove_file(&at).expect("the old target");
        std::fs::create_dir(&at).expect("the block");
        let outcome = atomic_write(AtomicWrite {
            target: &at,
            bytes: b"new\n",
            keep_mode_from: None,
        });
        assert!(outcome.is_err(), "the rename onto a directory failed");
        assert!(at.is_dir(), "the directory is intact");
        assert!(
            !temporary(&at).exists(),
            "no temporary file was left behind"
        );
    }
}
