//! The one place klin launches a git process. `Repo` owns how git is invoked: `-C`, the
//! captured output, the UTF-8 conversion a caller asks for, the byte output a blob needs, and
//! the environment overrides klin's private index and authorship require. What a caller does
//! with the answer — which rev-parse flag finds the state directory, which diff semantics a
//! changed set parses — stays with the caller. ADR 0041.

use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// One repository at a root. The root is where git is told to run, which is the configuration
/// root klin measures, not necessarily the repository top.
pub struct Repo<'a> {
    root: &'a Path,
}

impl<'a> Repo<'a> {
    pub fn at(root: &'a Path) -> Repo<'a> {
        Repo { root }
    }

    /// The text git printed on success, and `None` when git could not run or refused the
    /// arguments. A successful command with no output is `Some` of an empty string, so callers
    /// retain the meaning of an empty answer.
    pub fn text(&self, args: &[&str]) -> Option<String> {
        self.text_with_env(args, &[])
    }

    /// The same, with environment overrides for the invocation, such as klin's private index.
    pub fn text_with_env(&self, args: &[&str], env: &[(&OsStr, &OsStr)]) -> Option<String> {
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(self.root)
            .args(["-c", "core.quotePath=false"])
            .args(args);
        for (name, value) in env {
            command.env(name, value);
        }
        let done = command.output().ok()?;
        done.status
            .success()
            .then(|| String::from_utf8_lossy(&done.stdout).into_owned())
    }

    /// The bytes of one path at a commit, and `None` when the commit does not hold it as a
    /// blob. The path is relative to `root`, which is what the changed set reports, so it is
    /// named that way to git.
    pub fn blob(&self, commit: &str, path: &str) -> Option<Vec<u8>> {
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(self.root)
            .args(["show", &format!("{commit}:./{path}")]);
        let done = command.output().ok()?;
        done.status.success().then_some(done.stdout)
    }

    /// The bytes of many paths at one commit, read through one git process and handed over one
    /// at a time in request order, so a cold survey pays one process rather than one per file.
    /// `None` for a path the commit does not hold as a blob, which is not an empty blob; an
    /// empty blob is `Some` of nothing. Nothing here reads the working tree. The whole read is
    /// `None` when git could not be run or stopped answering, and every path answered before
    /// that was already handed over.
    pub fn blobs(
        &self,
        commit: &str,
        paths: &[&str],
        mut each: impl FnMut(&str, Option<&[u8]>),
    ) -> Option<()> {
        if paths.is_empty() {
            return Some(());
        }
        let mut git = Reaped(
            Command::new("git")
                .arg("-C")
                .arg(self.root)
                .args(["cat-file", "--batch"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .ok()?,
        );
        let mut requests = git.0.stdin.take()?;
        let mut answers = BufReader::new(git.0.stdout.take()?);
        let mut held = Vec::new();
        for path in paths {
            writeln!(requests, "{commit}:./{path}").ok()?;
            let is_blob = answer(&mut answers, &mut held)?;
            each(path, is_blob.then_some(held.as_slice()));
        }
        Some(())
    }

    /// Every path a commit holds, in git's relative spelling, with NUL separators kept out of
    /// the caller's parsing.
    pub fn ls_tree_paths(&self, commit: &str) -> Option<Vec<String>> {
        let listed = self.text(&["ls-tree", "-r", "-z", "--name-only", commit])?;
        Some(
            listed
                .split('\0')
                .filter(|path| !path.is_empty())
                .map(str::to_string)
                .collect(),
        )
    }

    /// The paths git ignores below this root, without the trailing directory marker.
    pub fn ignored_paths(&self) -> Option<Vec<String>> {
        let listed = self.text(&[
            "ls-files",
            "--others",
            "--ignored",
            "--exclude-standard",
            "--directory",
        ])?;
        Some(
            listed
                .lines()
                .filter(|path| !path.is_empty())
                .map(|path| path.trim_end_matches('/').to_string())
                .collect(),
        )
    }

    /// Whether a reference resolves to a commit, printed by git. Empty output is a refusal,
    /// which is how `rev-parse --verify --quiet` answers.
    pub fn rev_parse(&self, args: &[&str]) -> Option<String> {
        self.text(&args_from("rev-parse", args))
            .and_then(|text| (!text.trim().is_empty()).then(|| text.trim().to_string()))
    }

    /// The path git answers a rev-parse path-format question with, and `None` where git
    /// refuses or names nothing.
    pub fn rev_parse_path(&self, flag: &str) -> Option<PathBuf> {
        let text = self.text(&["rev-parse", "--path-format=absolute", flag])?;
        (!text.trim().is_empty()).then(|| PathBuf::from(text.trim()))
    }

    /// The worktree list git prints, and `None` where git could not run. The caller parses the
    /// porcelain it needs.
    pub fn worktrees(&self) -> Option<String> {
        self.text(&["worktree", "list", "--porcelain"])
    }
}

/// A git process that is killed and reaped when the read ends, on success or failure, so no
/// zombie and no open pipe outlives the call.
struct Reaped(Child);

impl Drop for Reaped {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// One answer from `cat-file --batch`: `Some(true)` for a blob whose bytes now fill `held`,
/// `Some(false)` for a path the commit does not hold as a blob, and `None` when git stopped
/// answering.
fn answer(answers: &mut impl BufRead, held: &mut Vec<u8>) -> Option<bool> {
    let mut header = String::new();
    if answers.read_line(&mut header).ok()? == 0 {
        return None;
    }
    let mut fields = header.split_whitespace().rev();
    let size = fields.next().and_then(|size| size.parse::<usize>().ok());
    let (Some(size), Some(kind)) = (size, fields.next()) else {
        return Some(false);
    };
    held.clear();
    held.resize(size, 0);
    answers.read_exact(held).ok()?;
    answers.read_exact(&mut [0u8; 1]).ok()?;
    Some(kind == "blob")
}

fn args_from<'a>(first: &'a str, rest: &'a [&'a str]) -> Vec<&'a str> {
    let mut args = Vec::with_capacity(rest.len() + 1);
    args.push(first);
    args.extend_from_slice(rest);
    args
}

#[cfg(test)]
mod tests {
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

    fn repository(files: &[(&str, &[u8])]) -> (tempfile::TempDir, PathBuf) {
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
        // macOS hands a /var path back whose canonical form is /private/var, and git answers
        // in the canonical form, so the tests compare against what git itself resolves.
        let top = Repo::at(dir.path())
            .rev_parse_path("--show-toplevel")
            .expect("the repository top");
        (dir, top)
    }

    #[test]
    fn text_reads_a_root_with_spaces_and_utf8() {
        let (inner, _) = repository(&[("src/a.rs", b"fn a() {}\n")]);
        let outer = tempfile::tempdir().expect("a temporary directory");
        let root = outer.path().join("a dir with spaces");
        std::fs::rename(inner.path(), &root).expect("the moved repository");
        let repo = Repo::at(&root);
        assert_eq!(
            repo.text(&["ls-tree", "-r", "--name-only", "HEAD"]),
            Some("src/a.rs\n".to_string()),
            "a root with spaces runs"
        );
        assert_eq!(
            repo.text(&["rev-parse", "--verify", "--quiet", "no-such-ref"]),
            None,
            "a refused query is None"
        );
    }

    #[test]
    fn a_failed_command_is_none_and_success_is_the_text() {
        let (dir, _) = repository(&[(".keep", b"")]);
        let repo = Repo::at(dir.path());
        assert_eq!(repo.text(&["rev-parse", "--verify", "no-such-ref"]), None);
        assert_eq!(repo.text(&["status", "--porcelain"]), Some(String::new()));
    }

    #[test]
    fn blob_reads_bytes_and_distinguishes_missing_from_empty() {
        let (dir, _) = repository(&[("src/a.rs", b"fn a() {}\n"), ("src/empty.rs", b"")]);
        let repo = Repo::at(dir.path());
        assert_eq!(repo.blob("HEAD", "src/a.rs"), Some(b"fn a() {}\n".to_vec()));
        assert_eq!(repo.blob("HEAD", "src/empty.rs"), Some(Vec::new()));
        assert_eq!(repo.blob("HEAD", "src/missing.rs"), None);
    }

    #[test]
    fn blobs_read_one_process_for_many_paths_in_request_order() {
        let (dir, _) = repository(&[
            ("src/a.rs", b"fn a() {}\n"),
            ("src/empty.rs", b""),
            ("docs/with space.md", b"# hi\n"),
        ]);
        let repo = Repo::at(dir.path());
        let mut out = Vec::new();
        repo.blobs(
            "HEAD",
            &["src/empty.rs", "src/missing.rs", "src/a.rs"],
            |path, bytes| {
                out.push((path.to_string(), bytes.map(<[u8]>::to_vec)));
            },
        )
        .expect("git read the commit");
        assert_eq!(
            out,
            vec![
                ("src/empty.rs".to_string(), Some(Vec::new())),
                ("src/missing.rs".to_string(), None),
                ("src/a.rs".to_string(), Some(b"fn a() {}\n".to_vec())),
            ]
        );
    }

    #[test]
    fn blobs_refuse_a_root_git_cannot_open() {
        let repo = Repo::at(Path::new("/nonexistent/klin"));
        let mut seen = 0;
        let outcome = repo.blobs("HEAD", &["src/a.rs"], |_, _| seen += 1);
        assert!(outcome.is_none());
        assert_eq!(seen, 0);
    }

    #[test]
    fn an_env_override_reaches_the_invocation() {
        let (dir, top) = repository(&[("src/a.rs", b"fn a() {}\n")]);
        let repo = Repo::at(dir.path());
        let index = top.join("private-index");
        let value = index.as_os_str();
        let name = OsStr::new("GIT_INDEX_FILE");
        let listed = repo.text_with_env(&["add", "-A"], &[(name, value)]);
        assert_eq!(listed, Some(String::new()), "git add ran with the index");
        assert!(index.is_file(), "the private index was written");
    }

    #[test]
    fn worktrees_and_rev_parse_paths_answer_for_current_callers() {
        let (dir, top) = repository(&[("src/a.rs", b"fn a() {}\n")]);
        let repo = Repo::at(dir.path());
        let listed = repo.worktrees().expect("worktree list");
        assert!(listed.contains("worktree "), "{listed}");
        let found = repo
            .rev_parse_path("--show-toplevel")
            .expect("the top level");
        assert_eq!(found, top);
        let git_dir = repo.rev_parse_path("--absolute-git-dir").expect("git dir");
        assert!(git_dir.ends_with(".git"), "{git_dir:?}");
    }
}
