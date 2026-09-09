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
}

impl Run {
    pub fn says(&self, text: &str) -> bool {
        self.out.contains(text)
    }
}

impl Tree {
    /// A tree inside a repository whose base commit holds nothing, so every finding is new.
    pub fn new() -> Tree {
        let tree = Tree::bare();
        tree.repository();
        tree.commit_empty("an empty base");
        tree.git(&["checkout", "-q", "-B", "work"]);
        tree.commit_empty("on the branch");
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

    pub fn repository(&self) {
        self.git(&["init", "-q", "-b", "main"]);
    }
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

fn spawn(cwd: &Path, args: &[&str], stdin: &str, environment: &[(&str, &str)]) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_klin"));
    command
        .args(args)
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
    Run {
        code: done.status.code().expect("exit code"),
        out: String::from_utf8_lossy(&done.stdout).to_string()
            + &String::from_utf8_lossy(&done.stderr),
    }
}
