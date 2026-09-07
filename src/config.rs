use std::cell::RefCell;
use std::fmt;
use std::path::{Path, PathBuf};

use serde_json::Value;

const FILENAME: &str = "quality.json";

#[derive(Debug)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str(&self.0)
    }
}

impl Error {
    pub fn unreadable(path: &Path, problem: impl fmt::Display) -> Error {
        Error(format!("{} could not be read: {problem}", path.display()))
    }
}

#[derive(Default)]
pub struct Records {
    pub findings: Vec<Value>,
    pub notes: Vec<Value>,
}

pub struct Flags {
    pub config: Option<PathBuf>,
    pub quiet: bool,
    pub strict: bool,
    pub only: Option<Vec<String>>,
    pub records: Option<RefCell<Records>>,
    pub with: Option<(String, Value)>,
}

impl Flags {
    pub fn record(&self, add: impl FnOnce(&mut Records)) {
        if let Some(records) = &self.records {
            add(&mut records.borrow_mut());
        }
    }
}

pub struct Config {
    pub file: PathBuf,
    root: PathBuf,
    data: Value,
}

impl Config {
    pub fn load(explicit: Option<&Path>, start: &Path) -> Result<Config, Error> {
        let file = match explicit {
            Some(named) => absolute(named, start),
            None => find(start).ok_or_else(|| {
                Error(format!(
                    "no {FILENAME} at or above {} — write one at the repository root, \
                     listing the gates this project runs",
                    start.display()
                ))
            })?,
        };
        let text = std::fs::read_to_string(&file).map_err(|why| Error::unreadable(&file, why))?;
        let data = serde_json::from_str(&text).map_err(|why| Error::unreadable(&file, why))?;
        let root = file.parent().unwrap_or(Path::new("")).to_path_buf();
        Ok(Config { file, root, data })
    }

    pub fn open(flags: &Flags, start: &Path) -> Result<Config, Error> {
        let mut config = Config::load(flags.config.as_deref(), start)?;
        if let Some((section, values)) = &flags.with
            && let Some(data) = config.data.as_object_mut()
        {
            data.insert(section.clone(), values.clone());
        }
        Ok(config)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn section(&self, name: &str) -> Result<&Value, Error> {
        self.data
            .get(name)
            .ok_or_else(|| Error(format!("{} has no \"{name}\" section", self.file.display())))
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        let expanded = expand_home(relative);
        if expanded.is_absolute() {
            expanded
        } else {
            self.root.join(expanded)
        }
    }

    pub fn missing(&self, section: &str, key: &str) -> Error {
        Error(format!(
            "{}: a \"{section}\" entry has no \"{key}\"",
            self.file.display()
        ))
    }

    pub fn malformed(&self, section: &str, key: &str, must_be: &str) -> Error {
        Error(format!(
            "{}: a \"{section}\" entry's \"{key}\" must be {must_be}",
            self.file.display()
        ))
    }
}

fn find(start: &Path) -> Option<PathBuf> {
    let mut here = start.to_path_buf();
    loop {
        let candidate = here.join(FILENAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !here.pop() {
            return None;
        }
    }
}

fn absolute(path: &Path, start: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        start.join(path)
    }
}

fn expand_home(relative: &str) -> PathBuf {
    match relative
        .strip_prefix("~/")
        .and_then(|rest| std::env::home_dir().map(|home| home.join(rest)))
    {
        Some(expanded) => expanded,
        None => PathBuf::from(relative),
    }
}
