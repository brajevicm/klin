#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct Tree {
    dir: tempfile::TempDir,
}

pub struct Run {
    pub code: i32,
    pub out: String,
    /// What the run wrote on stdout alone, which is what a redirect into a file captures.
    pub printed: String,
}

impl Run {
    /// Everything the run printed, as the one JSON object `--json` writes.
    pub fn json(&self) -> serde_json::Value {
        match serde_json::from_str(&self.out) {
            Ok(report) => report,
            Err(why) => panic!("{why} — the run printed:\n{}", self.out),
        }
    }

    pub fn says(&self, text: &str) -> bool {
        self.out.contains(text)
    }
}

impl Tree {
    /// A tree inside a repository whose base commit holds nothing, so every finding is new.
    pub fn new() -> Tree {
        let tree = Tree::bare();
        tree.git(&["init", "-q", "-b", "work"]);
        tree.import(&(made("main", "an empty base", "") + &on_the_branch()));
        tree
    }

    /// A tree that is not a repository.
    pub fn bare() -> Tree {
        Tree {
            dir: tempfile::tempdir().expect("temporary directory"),
        }
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.root().join(relative)
    }

    pub fn at(&self, relative: &str) -> String {
        self.path(relative).display().to_string()
    }

    pub fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directories");
        }
        fs::write(&path, text).expect("write");
        path
    }

    pub fn remove(&self, relative: &str) {
        let path = self.path(relative);
        assert!(fs::remove_file(&path).is_ok(), "remove {}", path.display());
    }

    pub fn words(&self, relative: &str, count: usize) -> PathBuf {
        self.write(relative, &(vec!["word"; count].join(" ") + "\n"))
    }

    pub fn run(&self, args: &[&str]) -> Run {
        run_from(self.root(), args)
    }

    pub fn run_without_path(&self, args: &[&str]) -> Run {
        spawn(self.root(), args, "", &[("PATH", "")])
    }

    pub fn run_with(&self, environment: &[(&str, &str)], args: &[&str]) -> Run {
        spawn(self.root(), args, "", environment)
    }

    pub fn git(&self, args: &[&str]) {
        let outcome = Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args(args)
            .output();
        let Ok(done) = outcome else {
            panic!("git {} could not run", args.join(" "))
        };
        assert!(
            done.status.success(),
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&done.stderr)
        );
    }

    pub fn revision(&self, reference: &str) -> String {
        let done = Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args(["rev-parse", "--verify", "--quiet", reference])
            .output();
        match done {
            Ok(done) if done.status.success() => {
                String::from_utf8_lossy(&done.stdout).trim().to_string()
            }
            _ => String::new(),
        }
    }

    /// What git sees as changed in the working tree, so a test can pin that klin wrote nothing.
    pub fn status(&self) -> String {
        let done = Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args(["status", "--porcelain"])
            .output();
        match done {
            Ok(done) => String::from_utf8_lossy(&done.stdout).to_string(),
            Err(why) => panic!("git status could not run: {why}"),
        }
    }

    pub fn base(&self) {
        if !self.path(".git").is_dir() {
            self.repository();
        }
        if self.level_with_main() {
            self.git(&["add", "-A"]);
            let root = format!(
                "from refs/heads/main^0\nM 040000 {} \"\"\n",
                self.read(&["write-tree"])
            );
            self.import(&(made("main", "the base", &root) + &on_the_branch()));
            return;
        }
        if !self.revision("main").is_empty() {
            self.git(&["checkout", "-q", "main"]);
        }
        self.commit("the base");
        self.git(&["checkout", "-q", "-B", "work"]);
        self.commit_empty("on the branch");
    }

    fn commit_empty(&self, message: &str) {
        self.git(&[
            "-c",
            "user.name=klin",
            "-c",
            "user.email=klin@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            message,
        ]);
    }

    pub fn commit(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&[
            "-c",
            "user.name=klin",
            "-c",
            "user.email=klin@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "-m",
            message,
        ]);
    }

    /// Where klin keeps its own state for this tree, by default under the git directory.
    pub fn state(&self, name: &str) -> PathBuf {
        self.path(&format!(".git/klin/{name}"))
    }

    /// One field of the turn stamp, as text, and empty when the stamp holds no such field.
    pub fn field(&self, name: &str) -> String {
        let text = fs::read_to_string(self.state("turn")).unwrap_or_default();
        let held: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        match held.get(name) {
            Some(serde_json::Value::String(text)) => text.clone(),
            Some(other) => other.to_string(),
            None => String::new(),
        }
    }

    pub fn repository(&self) {
        self.git(&["init", "-q", "-b", "main"]);
    }

    /// Whether the tree is on `work` and `work` holds the files of `main`, so a checkout of
    /// `main` would change nothing and the base can be written without one.
    fn level_with_main(&self) -> bool {
        let head = fs::read_to_string(self.path(".git/HEAD")).unwrap_or_default();
        let trees = self.read(&["rev-parse", "HEAD^{tree}", "main^{tree}"]);
        head == "ref: refs/heads/work\n"
            && matches!(trees.split_once('\n'), Some((ours, theirs)) if ours == theirs)
    }

    /// What one git command printed, trimmed, and empty when it failed.
    fn read(&self, args: &[&str]) -> String {
        match Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args(args)
            .output()
        {
            Ok(done) if done.status.success() => {
                String::from_utf8_lossy(&done.stdout).trim().to_string()
            }
            _ => String::new(),
        }
    }

    /// The commits a `git fast-import` stream names, written by one process where `git commit`
    /// takes one or two for each. A branch the stream moves back is moved anyway.
    fn import(&self, stream: &str) {
        let mut import = Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args(["fast-import", "--quiet", "--force", "--date-format=now"])
            .stdin(Stdio::piped())
            .spawn()
            .expect("git fast-import");
        import
            .stdin
            .take()
            .expect("stdin")
            .write_all(stream.as_bytes())
            .expect("the stream");
        assert!(import.wait().expect("git fast-import").success());
    }
}

/// One commit on `branch` for a `git fast-import` stream, dated now, with `rest` after its
/// message: a parent, and the files it changes. The message ends in a newline, as `git commit -m`
/// writes it.
fn made(branch: &str, message: &str, rest: &str) -> String {
    format!(
        "commit refs/heads/{branch}\ncommitter klin <klin@example.com> now\ndata {}\n{message}\n{rest}\n",
        message.len() + 1
    )
}

/// The empty commit `work` starts with, on the commit `main` holds in the stream.
fn on_the_branch() -> String {
    made("work", "on the branch", "from refs/heads/main\n")
}

/// A history whose percentile has a known answer: an opening commit of three lines in one
/// directory, then `small` commits that add one line in one directory, then `big` commits that
/// add ten lines in each of three. #92.
pub fn history(small: usize, big: usize) -> Tree {
    imported(
        std::iter::once(opening())
            .chain(small_commits(small))
            .chain(big_commits(big)),
    )
}

/// The same two kinds the other way round, so the big commits are the oldest and a sample that
/// stops short of them does not hold them.
pub fn history_from(big: usize, small: usize) -> Tree {
    imported(
        std::iter::once(opening())
            .chain(big_commits(big))
            .chain(small_commits(small)),
    )
}

/// One commit of a history: its message and the files it writes, as path and text.
struct Commit {
    message: &'static str,
    files: Vec<(String, String)>,
}

fn opening() -> Commit {
    Commit {
        message: "a first commit",
        files: vec![("README.md".into(), "one\ntwo\nthree\n".into())],
    }
}

fn small_commits(many: usize) -> impl Iterator<Item = Commit> {
    (0..many).map(|at| Commit {
        message: "a small commit",
        files: vec![(format!("small/{at}.txt"), "one line\n".into())],
    })
}

fn big_commits(many: usize) -> impl Iterator<Item = Commit> {
    (0..many).map(|at| Commit {
        message: "a big commit",
        files: ["a", "b", "c"]
            .map(|under| (format!("{under}/{at}.txt"), "line\n".repeat(10)))
            .to_vec(),
    })
}

/// A repository on `main` that holds `commits` in order, written by one `git fast-import`,
/// and checked out.
fn imported(commits: impl Iterator<Item = Commit>) -> Tree {
    let tree = Tree::bare();
    tree.repository();
    let mut stream = String::new();
    for Commit { message, files } in commits {
        let changed: String = files
            .iter()
            .map(|(path, text)| format!("M 100644 inline {path}\ndata {}\n{text}\n", text.len()))
            .collect();
        stream += &made("main", message, &changed);
    }
    tree.import(&stream);
    tree.git(&["reset", "-q", "--hard"]);
    tree
}

pub fn run_from(cwd: &Path, args: &[&str]) -> Run {
    feed(cwd, args, "")
}

pub fn run_from_with(cwd: &Path, environment: &[(&str, &str)], args: &[&str]) -> Run {
    spawn(cwd, args, "", environment)
}

pub fn feed(cwd: &Path, args: &[&str], stdin: &str) -> Run {
    spawn(cwd, args, stdin, &[])
}

pub fn feed_with(cwd: &Path, environment: &[(&str, &str)], args: &[&str], stdin: &str) -> Run {
    spawn(cwd, args, stdin, environment)
}

/// One empty home directory for the whole test binary, so a run reads the machine's own
/// host settings from nowhere and a test that wants a home names its own.
pub fn empty_home() -> &'static Path {
    static HOME: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    HOME.get_or_init(|| match tempfile::tempdir() {
        Ok(home) => home,
        Err(why) => panic!("a home directory could not be made: {why}"),
    })
    .path()
}

/// The binary under test: the one this build made, or the one `KLIN_BIN` names, so the
/// performance rows can be taken under an earlier release for a comparison.
pub fn binary() -> String {
    std::env::var("KLIN_BIN").unwrap_or_else(|_| env!("CARGO_BIN_EXE_klin").to_string())
}

/// A run of another klin binary, so a differential test can hold two builds to one output.
pub fn feed_as(klin: &str, cwd: &Path, args: &[&str], stdin: &str) -> Run {
    spawn_binary(klin, cwd, args, stdin, &[])
}

fn spawn(cwd: &Path, args: &[&str], stdin: &str, environment: &[(&str, &str)]) -> Run {
    spawn_binary(&binary(), cwd, args, stdin, environment)
}

fn spawn_binary(
    klin: &str,
    cwd: &Path,
    args: &[&str],
    stdin: &str,
    environment: &[(&str, &str)],
) -> Run {
    let mut command = Command::new(klin);
    command
        .args(args)
        .env("HOME", empty_home())
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, _) in std::env::vars().filter(|(name, _)| name.starts_with("GITHUB_")) {
        command.env_remove(name);
    }
    let mut child = command
        .envs(environment.iter().copied())
        .spawn()
        .expect("run klin");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    let done = child.wait_with_output().expect("wait for klin");
    let printed = String::from_utf8_lossy(&done.stdout).to_string();
    Run {
        code: done.status.code().expect("exit code"),
        out: printed.clone() + &String::from_utf8_lossy(&done.stderr),
        printed,
    }
}

/// The per-gate rows of a report: the check document's diagnostics, or the Stop hook's report.
pub fn gate_rows(report: &serde_json::Value) -> &serde_json::Value {
    match report["diagnostics"]["gates"].is_array() {
        true => &report["diagnostics"]["gates"],
        false => &report["gates"],
    }
}

/// The derived values a check document records in each measurement's basis.
pub fn derived(report: &serde_json::Value) -> Vec<serde_json::Value> {
    report["measurements"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|record| {
            record["basis"]["policy"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .collect()
}
