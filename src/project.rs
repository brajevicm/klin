//! What one run knows about the repository, composed once. `Config` is the policy a person
//! wrote. A `Tree` is one set of files as it stands: the working tree, or the base laid out
//! beside it. `Project` holds the one configuration a run loads, the working tree, and the
//! facts every check would otherwise compute again for itself: the changed set against the
//! base, the base laid out whole for the checks that resolve names against it, and the survey
//! of the derivation commit beside the working tree's. Each of those is computed on the first
//! call that needs it and never again. A check reads the facts and resolves its own policy;
//! nothing here manufactures a section. Checks borrow what they need through the `Context` and
//! own no lifetime of their own. ADR 0038, ADR 0040.

use std::borrow::Cow;
use std::cell::{Cell, OnceCell};
use std::path::{Path, PathBuf};

use crate::base::{self, Prior, Window};
use crate::changed::{self, Change};
use crate::config::{Config, Error};
use crate::syntax::structural::Extracted;
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
/// the list. The list reads no file's contents. A structural check's files are read, parsed and
/// extracted when a check first asks for them, and held with the tree for the run. Spec 4.1,
/// 4.3, ADR 0038.
pub struct Tree {
    root: PathBuf,
    files: OnceCell<Result<Vec<String>, String>>,
    listing: Cell<files::Listing>,
    extracted: Extracted,
}

impl Tree {
    /// A tree at this root, with nothing read yet.
    pub fn at(root: &Path) -> Tree {
        Tree {
            root: root.to_path_buf(),
            files: OnceCell::new(),
            listing: Cell::new(files::Listing::default()),
            extracted: Extracted::default(),
        }
    }

    /// A tree at this root whose file list a caller already knows, because it read the list
    /// from somewhere other than a walk: the base laid out without a checkout reads it from the
    /// base commit's index. The list carries the same rules a walk gives it. Spec 4.3, 8.4.
    pub fn listed(root: &Path, files: Vec<String>) -> Tree {
        let tree = Tree::at(root);
        let _ = tree.files.set(Ok(files));
        tree
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Each file's structural outcome, extracted once for every check that selects the file.
    pub fn extracted(&self) -> &Extracted {
        &self.extracted
    }

    /// The test roots of this tree alone, off its one file list. Spec 5.4.
    pub fn test_roots(&self) -> Vec<String> {
        survey::test_roots_of(self)
    }

    /// Every file, read on the first call and held for the run. A directory the walk could not
    /// read is an error naming it, as it was for every walk before. Spec 4.3, 14.
    pub fn files(&self) -> Result<&[String], Error> {
        self.files
            .get_or_init(|| {
                files::listing(&self.root)
                    .map(|(files, cost)| {
                        self.listing.set(cost);
                        files
                    })
                    .map_err(|why| why.to_string())
            })
            .as_deref()
            .map_err(|why| Error(why.clone()))
    }

    /// What reading the file list cost, handed over once: a second call, or a call before the
    /// list is read, is zero. Spec 11.2.
    pub fn listing_cost(&self) -> files::Listing {
        self.listing.take()
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

/// Whether a walk of a tree reaches a file at this path: it descends no directory of the
/// default skip set. A walk keeps a file whatever the file itself is called, so only the
/// directories above it decide. This is `Tree::covers` for a file the tree does not list yet,
/// which is what the base laid out from an index has. Spec 4.3.
pub fn reached(path: &str) -> bool {
    path.rsplit_once('/')
        .is_none_or(|(parents, _)| !parents.split('/').any(files::skipped))
}

/// One run: the configuration it loaded, the working tree, and the facts it computes once.
pub struct Project {
    pub config: Config,
    start: PathBuf,
    tree: Tree,
    changes: OnceCell<(String, Vec<Change>)>,
    whole_base: OnceCell<(String, Prior)>,
    facts: OnceCell<survey::Facts>,
    derivation: OnceCell<Option<String>>,
    by_hand: bool,
}

impl Project {
    /// The one load a run does: the config found from `start`, validated, and a tree at its
    /// root with nothing read yet. Spec 5.1, 14.
    pub fn load(explicit: Option<&Path>, start: &Path) -> Result<Project, Error> {
        let config = Config::load(explicit, start)?;
        Ok(Project {
            by_hand: true,
            ..Project::of(config, start)
        })
    }

    /// A run over a configuration already loaded.
    pub fn of(config: Config, start: &Path) -> Project {
        Project {
            tree: Tree::at(config.root()),
            config,
            start: start.to_path_buf(),
            changes: OnceCell::new(),
            whole_base: OnceCell::new(),
            facts: OnceCell::new(),
            derivation: OnceCell::new(),
            by_hand: false,
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

    /// What the derivation commit and the working tree say about the repository, read on the
    /// first call and held for the run. Spec 4.3.
    pub fn facts(&self) -> &survey::Facts {
        self.facts
            .get_or_init(|| survey::facts(&self.tree, self.derivation().as_deref()))
    }

    fn derivation(&self) -> &Option<String> {
        self.derivation.get_or_init(|| {
            self.by_hand
                .then(|| base::choose(self.root(), false).ok())
                .flatten()
                .and_then(|window| window.derives)
                .or_else(|| survey::unwindowed(self.root()))
        })
    }

    pub fn bind(&mut self, window: &Window) {
        let commit = window
            .derives
            .clone()
            .or_else(|| survey::unwindowed(self.root()));
        self.derivation = OnceCell::from(commit);
        self.facts.take();
    }

    /// The derivation commit's factual survey and cache directory, for a check that derives
    /// its own policy from them.
    pub fn source_derivation(&self) -> Option<(&survey::Survey, &str, Option<&Path>)> {
        let facts = self.facts();
        facts
            .at_commit()
            .map(|(held, commit)| (held, commit, facts.state.as_deref()))
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

    /// The base commit laid out whole, once for every check that resolves names against the
    /// whole base, and removed when the run ends. A run judges one base, so a second commit is
    /// refused rather than laid out beside the first. `light` is the change set of a run that
    /// may take the layout that checks no whole commit out, and `None` for every run that takes
    /// today's checkout. ADR 0038, Spec 8.4.
    pub fn whole_base(&self, commit: &str, light: Option<&[Change]>) -> Result<&Prior, Error> {
        if let Some((held, prior)) = self.whole_base.get() {
            return match held == commit {
                true => Ok(prior),
                false => Err(Error(format!(
                    "a run judges one base, and {commit} is a second one beside {held}"
                ))),
            };
        }
        let prior = base::laid_out(self, commit, light)?;
        Ok(&self
            .whole_base
            .get_or_init(|| (commit.to_string(), prior))
            .1)
    }

    /// The whole base's worktree removed now, before the run's journal line is written, rather
    /// than when the run drops it, and what the removal took. Nothing when no whole base was
    /// laid out, or it was removed already. Spec 11.4.
    pub fn teardown_base(&self) -> base::Teardown {
        self.whole_base
            .get()
            .map_or_else(base::Teardown::default, |(_, prior)| prior.teardown())
    }

    /// Whether the survey found no source root in this tree. Spec 10, 14.
    pub fn found_no_source_root(&self) -> bool {
        self.facts().found.roots.is_empty()
    }

    /// Whether the derivation commit's survey held the path this finding sits under. A site the
    /// survey did not hold matches nothing in `before`, whatever `before` holds there, so a
    /// directory that becomes a root cannot bring inherited debt with it. The facts are read
    /// whichever gates run, so the answer does not depend on the selection. Spec 7.1.
    pub fn was_held(&self, file: &str) -> bool {
        !self
            .facts()
            .unheld
            .iter()
            .any(|root| scope::under_or_at(file, root))
    }
}
