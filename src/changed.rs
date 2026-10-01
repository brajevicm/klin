use std::path::Path;

use crate::config::Error;
use crate::git::Repo;

/// A file the working tree changed since the base, with the path it had at the base. A
/// deletion is a change: `inventory` judges the path the working tree no longer holds, so a
/// scoped run must have it in scope. Spec 4.5, 8.2.
#[derive(Clone)]
pub struct Change {
    pub path: String,
    pub was: Option<String>,
}

pub fn files(root: &Path, base: &str) -> Result<Vec<Change>, Error> {
    let repo = Repo::at(root);
    let listed = repo
        .text(&["diff", "--name-status", "-M", "--relative", base, "--"])
        .ok_or_else(|| {
            Error(format!(
                "--changed needs a git repository, and git could not read {}",
                root.display()
            ))
        })?;
    let mut changes: Vec<Change> = listed.lines().filter_map(change).collect();
    let untracked = repo
        .text(&["ls-files", "--others", "--exclude-standard"])
        .unwrap_or_default();
    changes.extend(
        untracked
            .lines()
            .filter(|name| !name.is_empty())
            .map(|name| Change {
                path: name.to_string(),
                was: None,
            }),
    );
    changes.sort_by(|a, b| a.path.cmp(&b.path));
    changes.dedup_by(|a, b| a.path == b.path);
    Ok(changes)
}

fn change(line: &str) -> Option<Change> {
    let mut fields = line.split('\t');
    let status = fields.next()?;
    let first = fields.next()?;
    match status.starts_with('R') {
        true => fields.next().map(|now| Change {
            path: now.to_string(),
            was: Some(first.to_string()),
        }),
        false => Some(Change {
            path: first.to_string(),
            was: (status != "A").then(|| first.to_string()),
        }),
    }
}

/// The bytes a file held at a commit, or None when the commit does not hold it. The path is
/// relative to `root`, which is what the changed set reports, so it is named that way to git.
pub fn blob(root: &Path, commit: &str, path: &str) -> Option<Vec<u8>> {
    Repo::at(root).blob(commit, path)
}

/// The bytes many files held at one commit, read through one git process and handed over one
/// at a time in request order, so a cold survey pays one process rather than one per file.
/// `None` for a path the commit does not hold as a blob, which is not an empty blob; an empty
/// blob is `Some` of nothing. Nothing here reads the working tree. The whole read is `None`
/// when git could not be run or stopped answering, and every path answered before that was
/// already handed over.
pub fn blobs(
    root: &Path,
    commit: &str,
    paths: &[&str],
    each: impl FnMut(&str, Option<&[u8]>),
) -> Option<()> {
    Repo::at(root).blobs(commit, paths, each)
}

pub fn added(changes: &[Change]) -> std::collections::HashSet<String> {
    changes
        .iter()
        .filter(|change| change.was.is_none())
        .map(|change| change.path.clone())
        .collect()
}

/// Every file the change set renamed, by its current path, with the path it had at the base.
pub fn renamed(changes: &[Change]) -> std::collections::HashMap<String, String> {
    changes
        .iter()
        .filter_map(|change| {
            let was = change.was.as_ref().filter(|was| **was != change.path)?;
            Some((change.path.clone(), was.clone()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    fn git_in(root: &Path, args: &[&str]) {
        let done = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .expect("git runs");
        assert!(
            done.status.success(),
            "{}",
            String::from_utf8_lossy(&done.stderr)
        );
    }

    fn repository(files: &[(&str, &[u8])]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("a temporary directory");
        git_in(dir.path(), &["init", "-q"]);
        git_in(dir.path(), &["config", "user.email", "t@example.com"]);
        git_in(dir.path(), &["config", "user.name", "t"]);
        for (name, bytes) in files {
            let path = dir.path().join(name);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("dirs");
            std::fs::write(path, bytes).expect("a file");
        }
        git_in(dir.path(), &["add", "-A"]);
        git_in(dir.path(), &["commit", "-q", "-m", "base"]);
        dir
    }

    fn read(root: &Path, paths: &[&str]) -> Vec<(String, Option<Vec<u8>>)> {
        let mut out = Vec::new();
        blobs(root, "HEAD", paths, |path, bytes| {
            out.push((path.to_string(), bytes.map(<[u8]>::to_vec)));
        })
        .expect("git read the commit");
        out
    }

    #[test]
    fn many_blobs_come_back_in_request_order_with_missing_and_empty_told_apart() {
        let dir = repository(&[
            ("src/a.rs", b"fn a() {}\n"),
            ("src/empty.rs", b""),
            ("docs/with space.md", b"# hi\n"),
            ("src/\u{e9}t\u{e9}.rs", b"fn ete() {}\n"),
        ]);
        let found = read(
            dir.path(),
            &[
                "src/empty.rs",
                "src/missing.rs",
                "src/a.rs",
                "docs/with space.md",
                "src/\u{e9}t\u{e9}.rs",
                "src",
            ],
        );
        assert_eq!(
            found,
            vec![
                ("src/empty.rs".to_string(), Some(Vec::new())),
                ("src/missing.rs".to_string(), None),
                ("src/a.rs".to_string(), Some(b"fn a() {}\n".to_vec())),
                ("docs/with space.md".to_string(), Some(b"# hi\n".to_vec())),
                (
                    "src/\u{e9}t\u{e9}.rs".to_string(),
                    Some(b"fn ete() {}\n".to_vec())
                ),
                ("src".to_string(), None),
            ]
        );
    }

    #[test]
    fn the_commit_is_read_and_never_the_working_tree() {
        let dir = repository(&[("src/a.rs", b"fn a() {}\n")]);
        std::fs::write(dir.path().join("src/a.rs"), b"fn changed() {}\n").expect("a write");
        std::fs::write(dir.path().join("src/new.rs"), b"fn new() {}\n").expect("a write");
        assert_eq!(
            read(dir.path(), &["src/a.rs", "src/new.rs"]),
            vec![
                ("src/a.rs".to_string(), Some(b"fn a() {}\n".to_vec())),
                ("src/new.rs".to_string(), None),
            ]
        );
    }

    #[test]
    fn a_batch_wider_than_one_pipe_buffer_reads_every_blob_in_order() {
        let large = vec![b'x'; 300_000];
        let held: Vec<(String, Vec<u8>)> = (0..400)
            .map(|at| {
                (
                    format!("src/f{at:04}.rs"),
                    format!("fn f{at}() {{}}\n").into_bytes(),
                )
            })
            .chain(std::iter::once(("src/large.rs".to_string(), large.clone())))
            .collect();
        let files: Vec<(&str, &[u8])> = held
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
            .collect();
        let dir = repository(&files);
        let paths: Vec<&str> = files.iter().map(|(name, _)| *name).collect();
        let found = read(dir.path(), &paths);
        assert_eq!(found.len(), paths.len());
        for ((name, bytes), (found_name, found_bytes)) in held.iter().zip(&found) {
            assert_eq!(name, found_name);
            assert_eq!(Some(bytes), found_bytes.as_ref());
        }
    }

    #[test]
    fn a_commit_git_does_not_hold_reads_as_every_path_missing() {
        let dir = repository(&[("src/a.rs", b"fn a() {}\n")]);
        let mut seen = Vec::new();
        let outcome = blobs(
            dir.path(),
            "no-such-commit",
            &["src/a.rs"],
            |path, bytes| {
                seen.push((path.to_string(), bytes.is_some()));
            },
        );
        assert!(outcome.is_some());
        assert_eq!(seen, vec![("src/a.rs".to_string(), false)]);
    }

    #[test]
    fn a_root_git_cannot_open_is_a_failure_and_hands_over_nothing() {
        let mut seen = 0;
        let outcome = blobs(
            Path::new("/nonexistent/klin"),
            "HEAD",
            &["src/a.rs"],
            |_, _| {
                seen += 1;
            },
        );
        assert!(outcome.is_none());
        assert_eq!(seen, 0);
    }
}
