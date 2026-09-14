//! What one run knows about the repository, composed once. `Config` is the policy a person
//! wrote. A `Tree` is one set of files as it stands: the working tree, or the base laid out
//! beside it. `Project` holds the one configuration a run loads, the working tree, and the
//! facts every check would otherwise compute again for itself: the changed set against the
//! base, and what the survey derives for the sections the config leaves out. Each of those is
//! computed on the first call that needs it and never again. Checks borrow what they need
//! through the `Context` and own no lifetime of their own. ADR 0038.

use std::borrow::Cow;
use std::cell::OnceCell;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::changed::{self, Change};
use crate::config::{Config, Error};
use crate::{files, scope, survey};

/// One language a file is classified as by its extension, which is a fact of the path and no
/// check's opinion. The escapes table names its own rows for each of these by the same name,
/// and the survey leaves out a language that table has no rows for. Spec 5.4.
pub struct Language {
    pub name: &'static str,
    pub suffixes: &'static [&'static str],
}

/// The languages the survey calls source, by extension. This is what a derived root is a
/// directory of, so it belongs below every check. Spec 5.4, ADR 0038.
pub const LANGUAGES: &[Language] = &[
    Language {
        name: "go",
        suffixes: &[".go"],
    },
    Language {
        name: "java",
        suffixes: &[".java"],
    },
    Language {
        name: "kotlin",
        suffixes: &[".kt", ".kts"],
    },
    Language {
        name: "python",
        suffixes: &[".py"],
    },
    Language {
        name: "ruby",
        suffixes: &[".rb"],
    },
    Language {
        name: "rust",
        suffixes: &[".rs"],
    },
    Language {
        name: "shell",
        suffixes: &[".sh", ".bash", ".zsh"],
    },
    Language {
        name: "swift",
        suffixes: &[".swift"],
    },
    Language {
        name: "typescript",
        suffixes: &[".ts", ".tsx", ".mts", ".cts"],
    },
    Language {
        name: "javascript",
        suffixes: &[".js", ".jsx", ".mjs", ".cjs"],
    },
];

/// The language a path is written in, by its extension, and `None` for a file no language
/// claims. This is what the survey calls source.
pub fn language_of(path: &str) -> Option<&'static str> {
    LANGUAGES
        .iter()
        .find(|language| {
            language
                .suffixes
                .iter()
                .any(|suffix| path.ends_with(suffix))
        })
        .map(|language| language.name)
}

/// One tree's files, read once. The list is every file under the root by its relative path,
/// sorted, less the default skip set, everything git ignores, and symbolic links. Hidden
/// directories are in it, because two checks read them, and a caller that skips them filters
/// the list. Nothing here reads a file's contents. Spec 4.1, 4.3.
pub struct Tree {
    root: PathBuf,
    files: OnceCell<Result<Vec<String>, String>>,
}

impl Tree {
    /// A tree at this root, with nothing read yet.
    pub fn at(root: &Path) -> Tree {
        Tree {
            root: root.to_path_buf(),
            files: OnceCell::new(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Every file, read on the first call and held for the run. A directory the walk could not
    /// read is an error naming it, as it was for every walk before. Spec 4.3, 14.
    pub fn files(&self) -> Result<&[String], Error> {
        self.files
            .get_or_init(|| files::listing(&self.root).map_err(|why| why.to_string()))
            .as_deref()
            .map_err(|why| Error(why.clone()))
    }

    /// The file list's name for a directory this tree holds, and `None` for one the walk did
    /// not reach: outside the root, not a directory, under a directory every walk skips, or
    /// behind a symbolic link, which the walk does not follow and a root named through one
    /// still reads.
    pub fn covers(&self, directory: &Path) -> Option<String> {
        if !directory.is_dir() {
            return None;
        }
        let inside = directory.strip_prefix(&self.root).ok()?;
        let named = inside.to_str()?;
        if named.split('/').any(files::skipped) || self.linked(inside) {
            return None;
        }
        Some(match named.is_empty() {
            true => scope::ROOT.to_string(),
            false => named.to_string(),
        })
    }
}

impl Tree {
    /// Whether any directory between the root and this one is a symbolic link.
    fn linked(&self, inside: &Path) -> bool {
        let mut at = self.root.clone();
        inside.components().any(|part| {
            at.push(part);
            at.symlink_metadata()
                .is_ok_and(|held| held.file_type().is_symlink())
        })
    }
}

/// One run: the configuration it loaded, the working tree, and the facts it computes once.
pub struct Project {
    pub config: Config,
    start: PathBuf,
    tree: Tree,
    changes: OnceCell<(String, Vec<Change>)>,
    derived: OnceCell<survey::Derived>,
}

impl Project {
    /// The one load a run does: the config found from `start`, validated, and a tree at its
    /// root with nothing read yet. Spec 5.1, 14.
    pub fn load(explicit: Option<&Path>, start: &Path) -> Result<Project, Error> {
        let config = Config::load(explicit, start)?;
        Ok(Project::of(config, start))
    }

    /// A run over a configuration already loaded.
    pub fn of(config: Config, start: &Path) -> Project {
        Project {
            tree: Tree::at(config.root()),
            config,
            start: start.to_path_buf(),
            changes: OnceCell::new(),
            derived: OnceCell::new(),
        }
    }

    /// The directory the run started in.
    pub fn start(&self) -> &Path {
        &self.start
    }

    /// The directory the configuration sits in, which every path is relative to.
    pub fn root(&self) -> &Path {
        self.config.root()
    }

    /// The working tree, whose file list is read once for every check. ADR 0038.
    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// The derivation commit's factual survey and cache directory, for a check that derives
    /// its own policy from them.
    pub fn source_derivation(&self) -> Option<(&survey::Survey, &str, Option<&Path>)> {
        let derived = self.derivation();
        derived
            .at_commit()
            .map(|(facts, commit)| (facts, commit, derived.state()))
    }

    /// The files the working tree changed against the base, computed once for the base the run
    /// judges against and borrowed by everything that asks after. A run has one base, so a
    /// second one is read and not kept.
    pub fn changes(&self, base: &str) -> Result<Cow<'_, [Change]>, Error> {
        match self.changes.get() {
            Some((held, changes)) if held == base => Ok(Cow::Borrowed(changes)),
            Some(_) => changed::files(self.root(), base).map(Cow::Owned),
            None => {
                let changes = changed::files(self.root(), base)?;
                let (_, held) = self.changes.get_or_init(|| (base.to_string(), changes));
                Ok(Cow::Borrowed(held))
            }
        }
    }

    /// The section a check reads: what the config pins, filled in from the survey for a key it
    /// leaves out. A section klin cannot derive and the config does not name is an error naming
    /// the key. Spec 5.1, 5.2.
    pub fn section(&self, name: &str) -> Result<&Value, Error> {
        if survey::keys(name).is_none() {
            return self.config.required(name);
        }
        if let Some(pinned) = self.config.pinned(name)
            && survey::pinned_whole(name, pinned)
        {
            return Ok(pinned);
        }
        match self.derivation().section(name) {
            Some(derived) => Ok(derived),
            None => self.config.required(name),
        }
    }

    /// Whether the survey supplies this section for a config that leaves it out. The facts
    /// answer for every section but `reachability`, whose families the derivation commit alone
    /// proves, so planning a run derives no ceiling and reads that one cached policy. Spec 5.4.
    pub fn supplies(&self, section: &str) -> bool {
        self.derivation().supplies(section)
    }

    /// Whether any derivable section is left for the survey to fill in. A config that states
    /// every one of them derives nothing, so nothing walks the tree for it.
    pub fn derives_anything(&self) -> bool {
        survey::derivable().any(|name| match self.config.pinned(name) {
            Some(pinned) => !survey::pinned_whole(name, pinned),
            None => true,
        })
    }

    /// Whether the survey found no source root in this tree. The caller asks only when a check
    /// that measures code takes its roots from the survey, so a config that names its own roots
    /// surveys nothing for this. Spec 10, 14.
    pub fn found_no_source_root(&self) -> bool {
        self.derivation().roots.is_empty()
    }

    /// Whether the derivation commit's survey held the path this finding sits under. A site the
    /// survey did not hold matches nothing in `before`, whatever `before` holds there, so a
    /// directory that becomes a root cannot bring inherited debt with it. Spec 7.1.
    pub fn was_held(&self, file: &str) -> bool {
        let unheld = match self.derived.get() {
            Some(derived) => &derived.unheld,
            None => return true,
        };
        !unheld.iter().any(|root| scope::under_or_at(file, root))
    }

    /// The `derived:` and `pinned:` lines of the run, for the sections named and for every
    /// section when none is. Empty without running the survey when the config pins every
    /// derivable section. Spec 4.3, 10.
    pub fn derived_said(&self, only: Option<&[&str]>) -> Vec<String> {
        if !self.derives_anything() {
            return Vec::new();
        }
        self.derivation().lines(only)
    }

    /// The same values as the `{section, key, value, rule}` entries `--json` prints. Spec 11.2.
    pub fn derived_values(&self, only: Option<&[&str]>) -> Vec<Value> {
        if !self.derives_anything() {
            return Vec::new();
        }
        self.derivation().values(only)
    }

    /// The lines about one section, which `--list` prints under the gate that reads it.
    pub fn said_about(&self, section: &str) -> Vec<String> {
        self.derived_said(Some(&[section]))
            .into_iter()
            .filter(|line| names(line, section))
            .collect()
    }

    /// The lines about one section, written out by a check a person ran by hand, and only when
    /// the run already derived: a check whose section the config pins whole derived nothing and
    /// says nothing. The gate runner prints its own once for the whole run. Spec 4.3.
    pub fn say(&self, section: &str, out: &mut String) {
        let Some(derived) = self.derived.get() else {
            return;
        };
        for line in derived
            .lines(Some(&[section]))
            .iter()
            .filter(|line| names(line, section))
        {
            out.push_str(line);
            out.push('\n');
        }
    }

    fn derivation(&self) -> &survey::Derived {
        self.derived
            .get_or_init(|| survey::derive(&self.tree, self.config.values()))
    }
}

/// Whether a `derived:` or `pinned:` line is about this section, so a check a person ran by
/// hand prints the values it used and not another gate's.
fn names(line: &str, section: &str) -> bool {
    line.split_once(": ")
        .and_then(|(_, rest)| rest.strip_prefix(section))
        .is_some_and(|rest| rest.starts_with(' '))
}
