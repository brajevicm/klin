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
    pub fn new() -> Tree {
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

    pub fn commit(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&[
            "-c",
            "user.name=detent",
            "-c",
            "user.email=detent@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "-m",
            message,
        ]);
    }

    pub fn repository(&self) {
        self.git(&["init", "-q", "-b", "main"]);
    }
}

pub fn run_from(cwd: &Path, args: &[&str]) -> Run {
    feed(cwd, args, "")
}

pub fn feed(cwd: &Path, args: &[&str], stdin: &str) -> Run {
    spawn(cwd, args, stdin, &[])
}

fn spawn(cwd: &Path, args: &[&str], stdin: &str, environment: &[(&str, &str)]) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_detent"))
        .args(args)
        .envs(environment.iter().copied())
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run detent");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    let done = child.wait_with_output().expect("wait for detent");
    Run {
        code: done.status.code().expect("exit code"),
        out: String::from_utf8_lossy(&done.stdout).to_string()
            + &String::from_utf8_lossy(&done.stderr),
    }
}
