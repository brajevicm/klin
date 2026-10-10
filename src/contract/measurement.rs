//! One structural measurement of a tree: the files a gate selected, each through the tree's one
//! extraction, or the base's where the file is unchanged against it. This composes a `Tree`, the
//! base laid out beside it, the change set and the structural cache, so it sits above `project`
//! and `base`, and the facts it reads stay below both. ADR 0038, ADR 0065, spec 8.4.

use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

use crate::contract::check::{Context, Sink};
use crate::contract::coverage::Files;
use crate::facts::files::{self, Found};
use crate::facts::tree::Tree;
use crate::syntax::Unparsed;
use crate::syntax::structural::facts::{FileFacts, Outcome, Unsupported};
use crate::syntax::structural::{
    Cache, ExtractionCost, NameCost, NameSet, SourceIndex, TreeNameCost, file_at,
    selected_extensions, timed,
};
use crate::sys::changed::Change;
use crate::sys::error::Error;
use crate::window::base::{Layout, Prior, Run};

/// One structural measurement over a discovered file set. Consumers receive the semantic facts
/// and explicit coverage outcomes; a name-resolving consumer builds the index when it judges, and none
/// parses files or reconstructs capability gaps.
pub struct Measurement {
    facts: Vec<Rc<FileFacts>>,
    pub unparsed: Vec<Unparsed>,
    pub unsupported: Vec<Unsupported>,
    pub files: Files,
    pub cost: ExtractionCost,
}

impl Measurement {
    /// The selected files' shared facts, sorted by path, without building the name-resolution
    /// index. `indexed(..).files()` holds the same files in the same order.
    pub fn facts(&self) -> &[Rc<FileFacts>] {
        &self.facts
    }

    /// The facts of the selected file at this path, found without building the index.
    pub fn file(&self, path: &str) -> Option<&FileFacts> {
        file_at(&self.facts, path)
    }

    /// A name-resolution index over only the `wanted` names, or every name, with what building
    /// it took and what it holds counted into the tree's name cost. Spec 11.2.
    pub fn indexed(&self, cost: &mut TreeNameCost, wanted: Option<&NameSet>) -> SourceIndex {
        let index = timed(&mut cost.index, || {
            SourceIndex::of(self.facts.clone(), wanted)
        });
        index.tally(cost);
        index
    }
}

/// The whole base tree beside the working tree, with the paths a changed run's `Change` set
/// names. Git says a working-tree file outside that set holds the base's bytes at the same path,
/// so its outcome is the base extraction's, and the two trees share one set of facts for it. A
/// path the base does not list under the same name is extracted from the working tree. The base's
/// outcomes come from its structural cache where an earlier run kept them. Spec 8.4.
pub struct Unchanged<'a> {
    base: &'a Tree,
    listed: &'a [String],
    changed: HashSet<&'a str>,
    cost: ExtractionCost,
}

impl<'a> Unchanged<'a> {
    /// The view over this base tree and change set, with the base's outcomes read from the cache
    /// `cache` names, when the tree holds none yet. Spec 8.4.
    pub fn new(
        base: &'a Tree,
        changes: &'a [Change],
        cache: impl FnOnce() -> Option<Cache>,
    ) -> Result<Unchanged<'a>, Error> {
        let cost = base.extracted().keep(cache, changes);
        Ok(Unchanged {
            base,
            listed: base.files()?,
            changed: changes.iter().map(|change| change.path.as_str()).collect(),
            cost,
        })
    }

    /// The base's outcomes written to its cache once both trees are measured, and what reading
    /// and writing the cache cost this view.
    pub fn publish(self) -> ExtractionCost {
        self.cost + self.base.extracted().publish()
    }

    /// The base tree and its copy of this working-tree file, when the file is unchanged.
    fn copy(&self, file: &str) -> Option<(&'a Tree, PathBuf)> {
        let listed = self.listed.binary_search_by(|held| held.as_str().cmp(file));
        (listed.is_ok() && !self.changed.contains(file))
            .then(|| (self.base, self.base.root().join(file)))
    }
}

/// Every structural file of a tree, measured, because a gate that reads a module graph or a
/// public surface needs every module a path may reach. The selection is every structural
/// language under the default skip set, and no scope narrows it.
pub fn measure_all(tree: &Tree, unchanged: Option<&Unchanged>) -> Result<Measurement, Error> {
    let extensions = selected_extensions(&[]).unwrap_or_default();
    let skip_dirs = files::default_skip_dirs();
    let wanted = files::Wanted {
        extensions: &extensions,
        skip_dirs: &skip_dirs,
        exclude: &[],
        exclude_except: &[],
        skip_hidden: true,
    };
    let found = files::found(
        tree.root(),
        || tree.files(),
        &[tree.root().to_path_buf()],
        &wanted,
    )?;
    measure(found, tree, unchanged)
}

/// The found files of one tree measured, each through the tree's one extraction of it, or the
/// base's extraction where the file is unchanged against that base.
pub fn measure(
    found: Found,
    tree: &Tree,
    unchanged: Option<&Unchanged>,
) -> Result<Measurement, Error> {
    let repo_root = tree.root();
    let mut facts = Vec::new();
    let mut unparsed = Vec::new();
    let mut unsupported = Vec::new();
    let mut measured = Vec::new();
    let mut cost = ExtractionCost::default();
    for path in found.kept {
        let file = files::relative(&path, repo_root);
        let (source, path) = unchanged
            .and_then(|held| held.copy(&file))
            .unwrap_or((tree, path));
        match source.extracted().outcome(&path, &file, &mut cost)? {
            Outcome::Facts(found) => {
                measured.push(found.file.clone());
                facts.push(found);
            }
            Outcome::Unsupported(_) => unsupported.push(Unsupported { file }),
            Outcome::Unparsed(file) => unparsed.push(file),
            Outcome::Foreign => {}
        }
    }
    let excluded = found
        .excluded
        .iter()
        .map(|file| files::relative(file, repo_root))
        .collect();
    let unreadable = unparsed.iter().map(|file| file.file.clone()).collect();
    facts.sort_by(|a, b| a.file.cmp(&b.file));
    let not_measured = unsupported.iter().map(|file| file.file.clone()).collect();
    Ok(Measurement {
        facts,
        unparsed,
        unsupported,
        files: Files {
            measured,
            not_measured,
            excluded,
            unreadable,
        },
        cost,
    })
}

/// The base's view of the working tree's unchanged files, for a changed run that is not strict,
/// with the structural cache of the base commit where klin keeps state. `shared` holds the
/// changes of such a run, and any other run shares nothing and reads no cache. Spec 8.4.
fn unchanged<'a>(
    run: &impl Run,
    shared: Option<&'a [Change]>,
    prior: &'a Prior,
    commit: &str,
) -> Result<Option<Unchanged<'a>>, Error> {
    let Some(changes) = shared else {
        return Ok(None);
    };
    let dir = run.state();
    Unchanged::new(prior.tree(), changes, || {
        prior.cache(dir, run.root(), commit)
    })
    .map(Some)
}

/// The base and the working tree a structural check judges, each measured, with what the check
/// kept from the base. `prior` is the base laid out whole.
pub struct Sides<'a, T> {
    pub prior: &'a Prior,
    pub before: Measurement,
    pub kept: T,
    pub after: Measurement,
}

/// What a name-resolving check counts: the name cost its measurements add to, and the base
/// layout it records, taken from `prior` once. Spec 11.2.
pub struct Counted<'n> {
    pub names: &'n mut NameCost,
    pub layout: &'n mut Option<Layout>,
}

/// Every structural file of both trees measured, over one base extraction in a changed run that
/// is not strict. A check that reads a module graph or a public surface judges every file of both
/// trees whatever its scope. Spec 8.4.
pub fn sides_all<'a>(
    at: &Context<'a>,
    commit: &str,
    out: &mut Sink,
) -> Result<Sides<'a, ()>, Error> {
    let tree = at.project.tree();
    sides(
        at,
        commit,
        None,
        |unchanged| measure_all(tree, unchanged),
        |prior| Ok((measure_all(prior.tree(), None)?, ())),
        out,
    )
}

/// The two trees measured for a name-resolving check, with the time each step took added to
/// `counted.names` and the base layout taken into `counted.layout` before either tree is
/// measured. `measure_after` measures the working tree, and `measure_before` the base with what
/// the check keeps from it. Spec 8.4, spec 11.2.
pub fn sides_counted<'a, T>(
    at: &Context<'a>,
    commit: &str,
    counted: Counted,
    measure_after: impl FnOnce(Option<&Unchanged>) -> Result<Measurement, Error>,
    measure_before: impl FnOnce(&'a Prior) -> Result<(Measurement, T), Error>,
    out: &mut Sink,
) -> Result<Sides<'a, T>, Error> {
    sides(
        at,
        commit,
        Some(counted),
        measure_after,
        measure_before,
        out,
    )
}

/// The two trees measured, with the extraction cost of both recorded as `facts`. A changed run
/// that is not strict measures them over one base extraction: the working tree takes the base's
/// facts for every file its `Change` set leaves out, and extracts only the files it changed.
/// Strict and whole runs extract both trees. Only `counted` takes the base layout, which
/// `Prior::layout` hands out once. ADR 0038, ADR 0042, spec 8.4.
fn sides<'a, T>(
    at: &Context<'a>,
    commit: &str,
    counted: Option<Counted>,
    measure_after: impl FnOnce(Option<&Unchanged>) -> Result<Measurement, Error>,
    measure_before: impl FnOnce(&'a Prior) -> Result<(Measurement, T), Error>,
    out: &mut Sink,
) -> Result<Sides<'a, T>, Error> {
    let mut untimed = NameCost::default();
    let (names, layout) = match counted {
        Some(Counted { names, layout }) => (names, Some(layout)),
        None => (&mut untimed, None),
    };
    let prior = timed(&mut names.base, || whole_base(at, commit))?;
    let unchanged = timed(&mut names.base, || {
        unchanged(at.project, at.changes, prior, commit)
    })?;
    if let Some(layout) = layout {
        *layout = prior.layout();
    }
    let mut after = timed(&mut names.after.measure, || {
        measure_after(unchanged.as_ref())
    })?;
    let (before, kept) = timed(&mut names.before.measure, || measure_before(prior))?;
    after.cost = after.cost + unchanged.map_or_else(ExtractionCost::default, Unchanged::publish);
    out.record(|records| records.facts = Some(before.cost + after.cost));
    Ok(Sides {
        prior,
        before,
        kept,
        after,
    })
}

/// The base laid out whole for this run: the runner's own when it laid the whole base out, which
/// a changed run never does, and otherwise the run's one checkout. Spec 8.4, ADR 0038.
fn whole_base<'a>(at: &Context<'a>, commit: &str) -> Result<&'a Prior, Error> {
    match at.prior.filter(|_| at.changes.is_none()) {
        Some(prior) => Ok(prior),
        None => at.project.whole_base(commit, at.changes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::structural;

    fn measurement_of(files: Vec<Rc<FileFacts>>) -> Measurement {
        Measurement {
            facts: files,
            unparsed: Vec::new(),
            unsupported: Vec::new(),
            files: Files::default(),
            cost: ExtractionCost::default(),
        }
    }

    #[test]
    fn the_index_holds_the_facts_in_the_order_facts_reads_them() {
        let measured = measurement_of(vec![
            facts("src/one.rs", "fn a() {}\n"),
            facts("src/two.rs", "fn b() {}\n"),
        ]);
        let read: Vec<&str> = measured.facts().iter().map(|f| f.file.as_str()).collect();
        assert_eq!(read, vec!["src/one.rs", "src/two.rs"]);
        let index = measured.indexed(&mut TreeNameCost::default(), None);
        let indexed: Vec<&str> = index.files().iter().map(|f| f.file.as_str()).collect();
        assert_eq!(indexed, read);
    }

    fn facts(file: &str, source: &str) -> Rc<FileFacts> {
        match structural::of(file, source) {
            Ok(Outcome::Facts(found)) => found,
            _ => panic!("{file} gave no facts"),
        }
    }
}
