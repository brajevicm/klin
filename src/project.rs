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
use std::cell::OnceCell;
use std::path::{Path, PathBuf};

use crate::base::{self, Prior, Run, Window};
use crate::changed::{self, Change};
use crate::config::Config;
use crate::error::Error;
use crate::key::Section;
use crate::scope::{self, Moved, Moves};
use crate::tree::{self, Tree};
use crate::{stamp, survey, syntax};

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
    /// The base the run's window names, which the moved policy paths are read against.
    bound: Option<String>,
    /// What the change did to the paths the policy names, read on the first call after the
    /// build, so a file the build writes is in it. Spec 6.4, 7.3.
    moves: OnceCell<Moves>,
}

impl Project {
    /// The one load a run does: the config found from `start`, validated against the sections
    /// the runner passes, and a tree at its root with nothing read yet. Spec 5.1, 14.
    pub fn load(
        explicit: Option<&Path>,
        start: &Path,
        sections: &[Section],
    ) -> Result<Project, Error> {
        let config = Config::load(explicit, start, sections)?;
        Ok(Project {
            by_hand: true,
            ..Project::of(config, start)
        })
    }

    /// A run over a configuration already loaded.
    pub fn of(config: Config, start: &Path) -> Project {
        Project {
            tree: Tree::working(config.root()),
            config,
            start: start.to_path_buf(),
            changes: OnceCell::new(),
            whole_base: OnceCell::new(),
            facts: OnceCell::new(),
            derivation: OnceCell::new(),
            by_hand: false,
            bound: None,
            moves: OnceCell::new(),
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
                .then(|| base::choose(self.root()).ok())
                .flatten()
                .and_then(|window| window.derives)
                .or_else(|| stamp::unwindowed(self.root()))
        })
    }

    pub fn bind(&mut self, window: &Window) {
        let commit = window
            .derives
            .clone()
            .or_else(|| stamp::unwindowed(self.root()));
        self.derivation = OnceCell::from(commit);
        self.facts.take();
        self.bound = Some(window.before.clone());
        self.moves.take();
    }

    /// What the change did to the paths the policy names, which the run follows, and each file
    /// it renamed under a directory every walk skips. Nothing before a window is bound, and no
    /// scope move where no section states a scope. Spec 7.3.
    pub fn moves(&self) -> &Moves {
        self.moves.get_or_init(|| {
            let Some(base) = self.bound.as_deref() else {
                return Moves::default();
            };
            let Ok(changes) = self.changes(base) else {
                return Moves::default();
            };
            let mut moves = match scope::states_a_scope(&self.config) {
                true => self
                    .tree
                    .files()
                    .map(|files| scope::moved(&self.config, files, base, &changes))
                    .unwrap_or_default(),
                false => Moves::default(),
            };
            moves.extend(skipped(&changes));
            moves
        })
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
        self.facts().was_held(file)
    }
}

impl Run for Project {
    fn root(&self) -> &Path {
        Project::root(self)
    }

    fn changes(&self, base: &str) -> Result<Cow<'_, [Change]>, Error> {
        Project::changes(self, base)
    }

    fn state(&self) -> Option<&Path> {
        self.facts().state.as_deref()
    }
}

/// Each source file a rename took from a path a walk reaches to one under a directory every
/// walk skips, which no check measures in either tree. A file no language reads, such as test
/// data moved under `fixtures/`, is left out. Spec 7.3.
fn skipped(changes: &[Change]) -> impl Iterator<Item = Moved> + '_ {
    changes.iter().filter_map(|change| {
        let was = change.was.as_deref()?;
        let source = syntax::language_of(was).is_some();
        (source && tree::reached(was) && !tree::reached(&change.path)).then(|| Moved::Skipped {
            was: was.to_string(),
            path: change.path.clone(),
        })
    })
}
