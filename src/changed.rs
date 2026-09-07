use std::path::Path;
use std::process::Command;

use crate::config::Error;

/// A file the working tree changed since the base, with the path it had at the base.
#[derive(Clone)]
pub struct Change {
    pub path: String,
    pub was: Option<String>,
}

pub fn files(root: &Path, base: &str) -> Result<Vec<Change>, Error> {
    let listed = git(
        root,
        &[
            "diff",
            "--name-status",
            "-M",
            "--diff-filter=d",
            "--relative",
            base,
            "--",
        ],
    )
    .ok_or_else(|| {
        Error(format!(
            "--changed needs a git repository, and git could not read {}",
            root.display()
        ))
    })?;
    let mut changes: Vec<Change> = listed.lines().filter_map(change).collect();
    let untracked = git(root, &["ls-files", "--others", "--exclude-standard"]).unwrap_or_default();
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

/// The bytes a file held at a commit, or None when the commit does not hold it.
pub fn blob(root: &Path, commit: &str, path: &str) -> Option<Vec<u8>> {
    let done = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["show", &format!("{commit}:{path}")])
        .output()
        .ok()?;
    done.status.success().then_some(done.stdout)
}

pub fn git(root: &Path, args: &[&str]) -> Option<String> {
    let done = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("-c")
        .arg("core.quotePath=false")
        .args(args)
        .output()
        .ok()?;
    done.status
        .success()
        .then(|| String::from_utf8_lossy(&done.stdout).into_owned())
}
