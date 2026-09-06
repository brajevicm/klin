use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
}

pub fn run_from(cwd: &Path, args: &[&str]) -> Run {
    let done = Command::new(env!("CARGO_BIN_EXE_detent"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run detent");
    Run {
        code: done.status.code().expect("exit code"),
        out: String::from_utf8_lossy(&done.stdout).to_string()
            + &String::from_utf8_lossy(&done.stderr),
    }
}
