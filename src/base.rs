use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{Map, Value};

use crate::changed::{self, Change, blobs};
use crate::check::{Context, Sink};
use crate::error::Error;
use crate::git::{Boolean, Repo, Staged};
use crate::project::{self, Project, Tree};
use crate::state;
use crate::syntax::structural::{Cache, Outcome, Unchanged, same_grammar, selected_extensions};

const EMPTY: &str = "0000000000000000000000000000000000000000";

/// The `before` tree, laid out as a directory, so a gate measures it the way it measures the
/// working tree. A whole run checks the base out in a detached worktree. A scoped run writes
/// only the changed files, each at the path it has today, so a rename is not a tree of new debt.
pub struct Prior {
    dir: tempfile::TempDir,
    /// The base tree's files, read once however many gates measure it. ADR 0038.
    tree: Tree,
    from_worktree: Cell<Option<PathBuf>>,
    layout: Cell<Option<Layout>>,
    /// The path the base holds each file at that the change renamed, by the path it has today.
    renamed: HashMap<String, String>,
    added: HashSet<String>,
}

/// What laying the whole base out took, part by part, so a warm run's base cost is not one
/// number: laying the linked worktree out, looking the change set up, moving renamed files to
/// today's paths, and naming the structural cache. `worktree_add` holds every git command of
/// the layout, which is one checkout for the base checked out whole and four commands that
/// write only the files a gate reads for the base laid out light. The file list's own parts
/// come from the tree, and a light layout walks nothing, so both are zero. A scoped base
/// records none of this. Spec 11.2.
#[derive(Default, Clone, Copy)]
pub struct Layout {
    /// How many base files a light layout wrote from the index, and `None` for a base checked
    /// out whole, which writes every file the base commit holds.
    pub written: Option<usize>,
    pub worktree_add: Duration,
    pub changes: Duration,
    pub renames: Duration,
    pub cache_name: Duration,
    pub ignored: Duration,
    pub walk: Duration,
}

/// What removing the whole base's linked worktree took: `worktree remove --force` and
/// `worktree prune`. Spec 11.4.
#[derive(Default, Clone, Copy)]
pub struct Teardown {
    pub remove: Duration,
    pub prune: Duration,
}

impl Prior {
    fn new(tree: Tree, dir: tempfile::TempDir, from_worktree: Option<PathBuf>) -> Prior {
        Prior {
            tree,
            dir,
            from_worktree: Cell::new(from_worktree),
            layout: Cell::new(None),
            renamed: HashMap::new(),
            added: HashSet::new(),
        }
    }

    /// One part of the layout timed, added to what this base already recorded.
    fn spent<T>(&self, part: fn(&mut Layout) -> &mut Duration, work: impl FnOnce() -> T) -> T {
        let started = Instant::now();
        let out = work();
        self.add(part, started.elapsed());
        out
    }

    fn add(&self, part: fn(&mut Layout) -> &mut Duration, spent: Duration) {
        let mut layout = self.layout.take().unwrap_or_default();
        *part(&mut layout) += spent;
        self.layout.set(Some(layout));
    }

    /// What laying this base out took, handed over once to the first gate that asks, so the
    /// parts appear on one row of the run. `None` for a scoped base and on every later call.
    pub fn layout(&self) -> Option<Layout> {
        let mut layout = self.layout.take()?;
        let listed = self.tree.listing_cost();
        layout.ignored += listed.ignored;
        layout.walk += listed.walk;
        Some(layout)
    }

    /// The linked worktree removed and pruned now rather than when the run drops this base, and
    /// what that took. Nothing for a scoped base, or once it is removed.
    pub fn teardown(&self) -> Teardown {
        let Some(repository) = self.from_worktree.take() else {
            return Teardown::default();
        };
        let started = Instant::now();
        Repo::at(&repository).text(&[
            "worktree",
            "remove",
            "--force",
            &self.dir.path().to_string_lossy(),
        ]);
        let remove = started.elapsed();
        let started = Instant::now();
        Repo::at(&repository).text(&["worktree", "prune"]);
        Teardown {
            remove,
            prune: started.elapsed(),
        }
    }

    /// The base's copy of the directory the configuration sits in, which paths are relative to.
    pub fn root(&self) -> &Path {
        self.tree.root()
    }

    /// The base tree's file list, read once for every gate that measures it. ADR 0038.
    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// The path the base holds each file at that the change renamed, by the path it has today.
    pub fn renamed(&self) -> &HashMap<String, String> {
        &self.renamed
    }

    pub fn added(&self) -> &HashSet<String> {
        &self.added
    }

    fn changed_by(&mut self, changes: &[Change]) {
        self.renamed = changed::renamed(changes);
        self.added = changed::added(changes);
    }

    /// The files a measurement of this base could not read that the base could not read under
    /// its own path either. A file renamed from a path another grammar reads is left out: the
    /// layout read its base bytes under today's grammar, which is a reading the base never made.
    /// ADR 0021, spec 8.6.
    pub fn unread_either(&self, unreadable: &[String]) -> Vec<String> {
        unreadable
            .iter()
            .filter(|file| {
                self.renamed
                    .get(*file)
                    .is_none_or(|was| same_grammar(was, file))
            })
            .cloned()
            .collect()
    }
}

impl Drop for Prior {
    fn drop(&mut self) {
        self.teardown();
    }
}

pub fn materialize(
    project: &Project,
    before: &str,
    scope: Option<&[Change]>,
) -> Result<Prior, Error> {
    let dir = temporary()?;
    match scope {
        Some(changes) => written(project, before, changes, dir),
        None => checked_out(project, before, dir, under_the_repository(project.root())?),
    }
}

/// The whole base for a check that resolves names against all of it. A run that hands over its
/// change set may take the layout that checks no whole commit out, and takes today's checkout
/// wherever that layout cannot serve it: no structural cache to read, or a git command that
/// refused. Optimization state decides the cost and never the verdict. Spec 8.4.
pub fn laid_out(project: &Project, before: &str, light: Option<&[Change]>) -> Result<Prior, Error> {
    let dir = temporary()?;
    let inside = under_the_repository(project.root())?;
    if let Some(changes) = light
        && let Some(laid) = lightly(project, before, changes, dir.path(), &inside)
    {
        return Ok(held(project, laid, dir, changes));
    }
    checked_out(project, before, dir, inside)
}

fn temporary() -> Result<tempfile::TempDir, Error> {
    tempfile::Builder::new()
        .prefix("klin-base-")
        .tempdir()
        .map_err(|why| Error(format!("a directory for the base could not be made: {why}")))
}

fn short(commit: &str) -> &str {
    &commit[..7.min(commit.len())]
}

fn missing(before: &str, was: &str) -> Error {
    Error(format!(
        "the base commit {} holds no {was}, which git says it changed — the base and the working \
         tree disagree, so klin cannot judge this run",
        short(before)
    ))
}

fn checked_out(
    project: &Project,
    before: &str,
    dir: tempfile::TempDir,
    inside: PathBuf,
) -> Result<Prior, Error> {
    let root = project.root();
    let started = Instant::now();
    Repo::at(root)
        .text(&[
            "worktree",
            "add",
            "--detach",
            "--quiet",
            &dir.path().to_string_lossy(),
            before,
        ])
        .ok_or_else(|| {
            Error(format!(
                "the base commit {} could not be checked out to measure it — fetch history, or \
             give CI the full clone",
                short(before)
            ))
        })?;
    let at = dir.path().join(inside);
    let _ = std::fs::create_dir_all(&at);
    let mut prior = Prior::new(Tree::at(&at), dir, Some(root.to_path_buf()));
    prior.add(|layout| &mut layout.worktree_add, started.elapsed());
    let changes = prior.spent(|layout| &mut layout.changes, || project.changes(before))?;
    prior.changed_by(&changes);
    prior.spent(
        |layout| &mut layout.renames,
        || {
            for change in changes.iter() {
                let Some(was) = change.was.as_deref().filter(|was| *was != change.path) else {
                    continue;
                };
                move_within(prior.root(), was, &change.path)?;
            }
            Ok::<(), Error>(())
        },
    )?;
    Ok(prior)
}

/// Where the configuration sits inside the repository, since a worktree holds the whole tree.
fn under_the_repository(root: &Path) -> Result<PathBuf, Error> {
    let named = Repo::at(root)
        .text(&["rev-parse", "--show-toplevel"])
        .map(|found| PathBuf::from(found.trim()))
        .ok_or_else(|| {
            Error(format!(
                "{} is not inside a git repository, and klin measures the working tree against a \
                 base commit",
                root.display()
            ))
        })?;
    let top = named.canonicalize().unwrap_or(named);
    let here = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    Ok(here
        .strip_prefix(&top)
        .unwrap_or(Path::new(""))
        .to_path_buf())
}

/// The whole base laid out without checking the base commit out: a linked worktree registered
/// with no file on disk, the base commit read into its index, and only the base files a gate
/// may read written from that index. Every one of those files is written before any rename
/// moves it, so git converts each one under the base commit's own attribute topology, as a
/// checkout of the commit does. `None` where the run cannot take this layout, and where a
/// command failed the worktree is removed again, so nothing half-laid reaches a gate. Spec 8.4.
fn lightly(
    project: &Project,
    before: &str,
    changes: &[Change],
    dir: &Path,
    inside: &Path,
) -> Option<Laid> {
    let mut layout = Layout::default();
    let started = Instant::now();
    let under = project
        .facts()
        .state
        .as_deref()?
        .join(state::CACHE)
        .join(state::STRUCTURAL);
    let cache = Cache::at(&under, before, &checkout(project.root()))?;
    layout.cache_name = started.elapsed();
    let started = Instant::now();
    let outcomes = cache.read()?;
    let read = started.elapsed();

    let root = dir.join(inside);
    let started = Instant::now();
    let laid = (|| {
        let catalogue = staged(project.root(), dir, &root, before)?;
        let wanted = required(&catalogue, changes, &outcomes);
        let written = wanted.len();
        Repo::at(&root).checkout_index(&wanted)?;
        Some((catalogue, written))
    })();
    let Some((catalogue, written)) = laid else {
        abandoned(project.root(), dir);
        return None;
    };
    layout.worktree_add = started.elapsed();
    layout.written = Some(written);

    let started = Instant::now();
    for change in changes {
        let Some(was) = change.was.as_deref().filter(|was| *was != change.path) else {
            continue;
        };
        if move_within(&root, was, &change.path).is_err() {
            abandoned(project.root(), dir);
            return None;
        }
    }
    layout.renames = started.elapsed();

    Some(Laid {
        files: catalogue.listing(changes),
        root,
        cache,
        outcomes,
        read,
        layout,
    })
}

/// A base laid out light, before it becomes a `Prior`: where it sits, the files it lists, and
/// the structural cache the layout already named and read.
struct Laid {
    root: PathBuf,
    files: Vec<String>,
    cache: Cache,
    outcomes: HashMap<String, Outcome>,
    read: Duration,
    layout: Layout,
}

/// A light layout made into the base every gate reads, with the outcomes the layout read handed
/// to the tree, so the view that asks later neither names nor reads the cache again.
fn held(project: &Project, laid: Laid, dir: tempfile::TempDir, changes: &[Change]) -> Prior {
    let tree = Tree::listed(&laid.root, laid.files);
    tree.extracted()
        .hold(laid.cache, laid.outcomes, changes, laid.read);
    let mut prior = Prior::new(tree, dir, Some(project.root().to_path_buf()));
    prior.changed_by(changes);
    prior.layout.set(Some(laid.layout));
    prior
}

/// A light layout that could not be completed removed again, so the run checks the base out on
/// the same directory. A directory git never registered is left as it was.
fn abandoned(repository: &Path, dir: &Path) {
    let repo = Repo::at(repository);
    repo.text(&["worktree", "remove", "--force", &dir.to_string_lossy()]);
    repo.text(&["worktree", "prune"]);
}

/// The base commit registered as a linked worktree with nothing on disk, and its index listed
/// from the directory the configuration sits in, so a configuration root below the git top
/// level lists its own subtree by relative paths.
fn staged(repository: &Path, dir: &Path, root: &Path, before: &str) -> Option<Catalogue> {
    Repo::at(repository).text(&[
        "worktree",
        "add",
        "--detach",
        "--no-checkout",
        "--quiet",
        &dir.to_string_lossy(),
        before,
    ])?;
    let worktree = Repo::at(dir);
    worktree.read_tree(before)?;
    if told(&worktree, "core.sparseCheckout", false)? {
        return None;
    }
    std::fs::create_dir_all(root).ok()?;
    Catalogue::of(
        Repo::at(root).ls_files_stage()?,
        told(&worktree, "core.symlinks", true)?,
    )
}

/// What a boolean setting is worth to this layout: the value git read, the default where git
/// names none, and `None` where git refuses the value it holds, which sends the run to the
/// checkout rather than to a guess about the bytes git would write. Spec 8.4.
fn told(repo: &Repo, name: &str, default: bool) -> Option<bool> {
    match repo.boolean(name) {
        Boolean::Set(value) => Some(value),
        Boolean::Unset => Some(default),
        Boolean::Refused => None,
    }
}

/// The base commit's index as the light layout reads it: every path a checkout of the commit
/// would write, and the paths a tree lists, which is that set less what a walk never reaches.
struct Catalogue {
    written: Vec<String>,
    listed: Vec<String>,
}

/// The index modes a checkout of a commit writes.
const FILE: u32 = 0o100_644;
const EXECUTABLE: u32 = 0o100_755;
const SYMLINK: u32 = 0o120_000;
const GITLINK: u32 = 0o160_000;

impl Catalogue {
    /// One index reading, by mode. A file and an executable file are written and listed. A
    /// submodule is neither, because an uninitialized submodule is an empty directory that
    /// holds no file. A symbolic link is written, and it is listed only where `core.symlinks`
    /// is false, since git then writes the target path as a plain file that a walk lists, where
    /// otherwise it writes a link that a walk skips. `None` for any other mode and for an
    /// unmerged entry, which are shapes this layout does not prove equivalent to a checkout.
    fn of(entries: Vec<Staged>, symlinks: bool) -> Option<Catalogue> {
        let mut catalogue = Catalogue {
            written: Vec::new(),
            listed: Vec::new(),
        };
        for entry in entries {
            if entry.stage != 0 {
                return None;
            }
            let lists = match entry.mode {
                GITLINK => continue,
                FILE | EXECUTABLE => true,
                SYMLINK => !symlinks,
                _ => return None,
            };
            if lists && project::reached(&entry.path) {
                catalogue.listed.push(entry.path.clone());
            }
            catalogue.written.push(entry.path);
        }
        Some(catalogue)
    }

    /// The base tree's file list: the index's paths with every renamed file under the name it
    /// has today, where `move_within` left it, sorted as a walk sorts a tree.
    fn listing(&self, changes: &[Change]) -> Vec<String> {
        let renamed: HashMap<&str, &str> = changes
            .iter()
            .filter_map(|change| {
                let was = change.was.as_deref().filter(|was| *was != change.path)?;
                Some((was, change.path.as_str()))
            })
            .collect();
        let mut files: Vec<String> = self
            .listed
            .iter()
            .map(|path| match renamed.get(path.as_str()) {
                Some(now) => (*now).to_string(),
                None => path.clone(),
            })
            .collect();
        files.sort();
        files
    }
}

/// Every base path the light layout writes: the whole index, less the source files whose facts
/// the structural cache already holds, which no gate reads from disk. A file the change set
/// names is never served from the cache, so it is always written.
fn required<'a>(
    catalogue: &'a Catalogue,
    changes: &[Change],
    outcomes: &HashMap<String, Outcome>,
) -> Vec<&'a str> {
    let extensions = selected_extensions(&[]).unwrap_or_default();
    let changed: HashSet<&str> = changes
        .iter()
        .flat_map(|change| std::iter::once(change.path.as_str()).chain(change.was.as_deref()))
        .collect();
    let served: HashSet<&str> = catalogue
        .listed
        .iter()
        .map(String::as_str)
        .filter(|path| extensions.iter().any(|end| path.ends_with(end)))
        .filter(|path| outcomes.contains_key(*path) && !changed.contains(path))
        .collect();
    catalogue
        .written
        .iter()
        .map(String::as_str)
        .filter(|path| !served.contains(path))
        .collect()
}

fn move_within(root: &Path, was: &str, now: &str) -> Result<(), Error> {
    let (from, to) = (root.join(was), root.join(now));
    if !from.is_file() {
        return Ok(());
    }
    let moved = || Error(format!("the base's {was} could not be read at {now}"));
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|_| moved())?;
    }
    std::fs::rename(from, to).map_err(|_| moved())
}

fn written(
    project: &Project,
    before: &str,
    changes: &[Change],
    dir: tempfile::TempDir,
) -> Result<Prior, Error> {
    let root = project.root();
    let at = dir.path().to_path_buf();
    let mut prior = Prior::new(Tree::at(&at), dir, None);
    prior.changed_by(changes);
    let requested: Vec<(&str, &Change)> = changes
        .iter()
        .filter_map(|change| change.was.as_deref().map(|was| (was, change)))
        .collect();
    if requested.is_empty() {
        return Ok(prior);
    }
    let mut failure = None;
    let paths: Vec<&str> = requested.iter().map(|(was, _)| *was).collect();
    let mut delivered = 0;
    let completed = blobs(root, before, &paths, |was, bytes| {
        let change = requested.get(delivered).map(|(_, change)| *change);
        delivered += 1;
        if failure.is_some() {
            return;
        }
        let Some(bytes) = bytes else {
            failure = Some(missing(before, was));
            return;
        };
        let Some(change) = change else {
            failure = Some(missing(before, was));
            return;
        };
        let path = prior.root().join(&change.path);
        let unwritable = |why: &dyn std::fmt::Display| {
            Error(format!("{} could not be written: {why}", path.display()))
        };
        let result = (|| {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|why| unwritable(&why))?;
            }
            std::fs::write(&path, bytes).map_err(|why| unwritable(&why))
        })();
        if let Err(problem) = result {
            failure = Some(problem);
        }
    });
    if let Some(failure) = failure {
        return Err(failure);
    }
    if completed.is_none() {
        let was = requested
            .get(delivered)
            .map_or(requested[0].0, |(was, _)| *was);
        return Err(missing(before, was));
    }
    Ok(prior)
}

/// The base commit a gate judges against: the one the runner chose, or the one this gate
/// chooses for itself and names once in the report. Spec 6.1.
pub fn commit(root: &Path, at: &Context, out: &mut Sink) -> Result<String, Error> {
    match at.base {
        Some(commit) => Ok(commit.to_string()),
        None => Ok(announced(root, at, out)?.before),
    }
}

/// The base a gate the runner did not lay out chooses for itself, named once in the report.
pub fn announced(root: &Path, at: &Context, out: &mut Sink) -> Result<Window, Error> {
    let base = choose(root, at.strict)?;
    if at.context() {
        let _ = writeln!(out.text, "{}", base.line());
    }
    Ok(base)
}

/// The base laid out whole, for a check that resolves names against every file of it: the
/// runner's own when the runner laid the whole base out, and otherwise the run's one checkout,
/// which every such check shares. A changed run lays out only its changed files, whether or not
/// the check takes that run's scope, so the change set and not the judgement scope decides.
/// Spec 8.4, ADR 0038.
pub fn whole<'a>(at: &Context<'a>, commit: &str) -> Result<&'a Prior, Error> {
    match (at.prior, at.changes) {
        (Some(prior), None) => Ok(prior),
        _ => at
            .project
            .whole_base(commit, at.changes.filter(|_| !at.strict)),
    }
}

/// The base's view of the working tree's unchanged files, for a changed run that is not strict,
/// with the structural cache of the base commit where klin keeps state. Any other run shares
/// nothing and reads no cache. Spec 8.4.
pub fn unchanged<'a>(
    at: &Context<'a>,
    prior: &'a Prior,
    commit: &str,
) -> Result<Option<Unchanged<'a>>, Error> {
    let Some(changes) = at.changes.filter(|_| !at.strict) else {
        return Ok(None);
    };
    let dir = at.project.facts().state.as_deref();
    let cache = || {
        prior.spent(
            |layout| &mut layout.cache_name,
            || {
                let under = dir?.join(state::CACHE).join(state::STRUCTURAL);
                Cache::at(&under, commit, &checkout(at.project.root()))
            },
        )
    };
    Unchanged::new(prior.tree(), changes, cache).map(Some)
}

/// What the bytes of a base checkout depend on besides the commit: where the configuration sits,
/// git's configuration, and the attribute files git reads outside the tree. A change to any of
/// them names another structural cache. Spec 8.4.
fn checkout(root: &Path) -> Vec<u8> {
    let repo = Repo::at(root);
    let attributes = [
        repo.rev_parse_path("--git-common-dir")
            .map(|common| common.join("info/attributes")),
        repo.text(&["config", "--path", "--get", "core.attributesFile"])
            .map(|named| PathBuf::from(named.trim()))
            .or_else(global_attributes),
    ];
    let mut out = root.to_string_lossy().into_owned().into_bytes();
    out.push(0);
    out.extend(
        repo.text(&["config", "--list", "-z"])
            .unwrap_or_default()
            .bytes(),
    );
    for file in attributes.into_iter().flatten() {
        out.push(0);
        out.extend(std::fs::read(file).unwrap_or_default());
    }
    out
}

/// The attributes file git reads when `core.attributesFile` names none.
fn global_attributes() -> Option<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(config.join("git/attributes"))
}

/// The base the runner laid out, or the one `lay` lays out for this check into `own`, which the
/// caller keeps for as long as it reads the base.
pub fn laid<'p>(
    prior: Option<&'p Prior>,
    own: &'p mut Option<Prior>,
    lay: impl FnOnce() -> Result<Prior, Error>,
) -> Result<&'p Prior, Error> {
    match prior {
        Some(prior) => Ok(prior),
        None => Ok(own.insert(lay()?)),
    }
}

/// The base tree for a gate the runner did not lay out, such as a gate run by its own command.
pub fn own(at: &Context, out: &mut Sink) -> Result<Prior, Error> {
    let base = announced(at.project.root(), at, out)?;
    materialize(at.project, &base.before, None)
}

/// The pair of trees a run compares: which of the three kinds of 4.2 it is, the base commit
/// `before` names, and how that commit was chosen. `after` is the working tree.
#[derive(Clone)]
pub struct Window {
    pub kind: Kind,
    pub before: String,
    pub how: String,
    pub derives: Option<String>,
}

/// The three window kinds of section 4.2. The hook judges a turn, `klin gate` by hand and CI
/// on a pull request judge a branch, and CI on a push judges the push.
#[derive(Clone, Copy)]
pub enum Kind {
    Turn,
    Branch,
    Push,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Turn => "turn",
            Kind::Branch => "branch",
            Kind::Push => "push",
        }
    }
}

impl Window {
    pub fn short(&self) -> &str {
        short(&self.before)
    }

    pub fn line(&self) -> String {
        format!(
            "window: {} — base {}, {}",
            self.kind.name(),
            self.short(),
            self.how
        )
    }

    pub fn record(&self) -> Value {
        let mut out = Map::new();
        out.insert("kind".into(), self.kind.name().into());
        out.insert("before".into(), self.before.clone().into());
        out.insert("after".into(), "the working tree".into());
        out.insert("how".into(), self.how.clone().into());
        Value::Object(out)
    }
}

pub fn choose(root: &Path, strict: bool) -> Result<Window, Error> {
    let head = resolve(root, "HEAD");
    let mut tried: Vec<String> = Vec::new();
    for (reference, how, kind, source) in candidates(root) {
        tried.push(
            reference
                .split("...")
                .next()
                .unwrap_or(&reference)
                .to_string(),
        );
        let Some(commit) = resolve(root, &reference) else {
            continue;
        };
        let base = Window {
            kind,
            derives: Some(commit.clone()),
            before: commit,
            how,
        };
        if head.as_deref() == Some(base.before.as_str()) && !dirty(root) {
            return equal_to_head(root, strict, source, base);
        }
        return Ok(base);
    }
    Err(Error(format!(
        "no base commit to compare against in {} — klin measures the working tree against a \
         base commit, and none of these resolved: {}. Fetch history, or work on a branch.",
        root.display(),
        match tried.is_empty() {
            true => "nothing".to_string(),
            false => tried.join(", "),
        }
    )))
}

/// Where a base candidate came from, which decides whether a base equal to HEAD hides work.
#[derive(Clone, Copy, PartialEq)]
enum Source {
    /// A remote reference or a push event's commit. The remote agrees with HEAD.
    Remote,
    /// A local reference, which says nothing about what the remote has.
    Local,
}

/// A clean working tree whose base is HEAD measures nothing. That is a refusal only when klin
/// can show that commits are hidden from the remote, or when it cannot tell and CI is asking.
fn equal_to_head(root: &Path, strict: bool, source: Source, base: Window) -> Result<Window, Error> {
    if source == Source::Remote {
        return Ok(base);
    }
    let Some((name, tip)) = remote_tip(root) else {
        return cannot_tell(strict, base, "no remote default branch resolves");
    };
    let Some(unpushed) = Repo::at(root).text(&[
        "log",
        "--oneline",
        "-n",
        &SHOWN.to_string(),
        &format!("{tip}..HEAD"),
    ]) else {
        return cannot_tell(
            strict,
            base,
            &format!("git could not list what {name} is missing"),
        );
    };
    let unpushed: Vec<&str> = unpushed.lines().filter(|line| !line.is_empty()).collect();
    if unpushed.is_empty() {
        return Ok(base);
    }
    Err(Error(format!(
        "the base is HEAD ({}) and the working tree matches it, so a run would measure nothing, \
         and {} on this branch that {name} does not hold:\n{}\nPush this branch, or fetch the \
         remote reference the base names, so a run has a real diff to measure.",
        base.how,
        match unpushed.len() < SHOWN {
            true => format!("these {} commit(s) sit", unpushed.len()),
            false => format!("at least these {SHOWN} commits sit"),
        },
        unpushed
            .iter()
            .map(|line| format!("  {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    )))
}

/// How many unpushed commits an error names, so a long-lived branch is not a wall of text.
const SHOWN: usize = 10;

/// klin cannot see whether the work is pushed. CI asks for the refusal, a local run for its gates.
fn cannot_tell(strict: bool, base: Window, why: &str) -> Result<Window, Error> {
    if !strict {
        return Ok(base);
    }
    Err(Error(format!(
        "the base is HEAD ({}) and the working tree matches it, so a run would measure nothing, \
         and {why}, so klin cannot tell whether these commits are pushed — fetch the remote, or \
         drop --strict",
        base.how
    )))
}

/// The tip of the remote's default branch, which says what the remote already has.
fn remote_tip(root: &Path) -> Option<(String, String)> {
    default_branches(root).into_iter().find_map(|branch| {
        let commit = resolve(root, &remote_reference(&branch)?)?;
        Some((branch, commit))
    })
}

/// The remote-tracking reference that a name such as `origin/main` abbreviates. A local branch
/// may carry that literal name, and git resolves it first, so klin never calls a base remote
/// on the strength of the short name alone.
fn remote_reference(branch: &str) -> Option<String> {
    branch
        .strip_prefix("origin/")
        .map(|remote| format!("refs/remotes/origin/{remote}"))
}

fn dirty(root: &Path) -> bool {
    Repo::at(root)
        .text(&["status", "--porcelain"])
        .is_some_and(|listed| !listed.trim().is_empty())
}

fn candidates(root: &Path) -> Vec<(String, String, Kind, Source)> {
    let mut out = Vec::new();
    if let Some(target) = environment("GITHUB_BASE_REF") {
        for (reference, source) in [
            (format!("refs/remotes/origin/{target}"), Source::Remote),
            (target.clone(), Source::Local),
        ] {
            out.push((
                reference,
                format!("the tip of {target}, the pull request target"),
                Kind::Branch,
                source,
            ));
        }
    }
    if let Some(before) = pushed_from() {
        out.push((
            before,
            "the commit this push started from".to_string(),
            Kind::Push,
            Source::Remote,
        ));
    }
    for branch in default_branches(root) {
        let (reference, source) = match remote_reference(&branch) {
            Some(remote) => (remote, Source::Remote),
            None => (branch.clone(), Source::Local),
        };
        out.push((
            format!("{reference}...HEAD"),
            format!("the merge-base with {branch}"),
            Kind::Branch,
            source,
        ));
    }
    out
}

fn pushed_from() -> Option<String> {
    let path = environment("GITHUB_EVENT_PATH")?;
    let text = std::fs::read_to_string(path).ok()?;
    let event: Value = serde_json::from_str(&text).ok()?;
    let before = event.get("before")?.as_str()?;
    (before != EMPTY && !before.is_empty()).then(|| before.to_string())
}

fn environment(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn default_branches(root: &Path) -> Vec<String> {
    let named = Repo::at(root)
        .text(&["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
        .map(|found| found.trim().to_string())
        .filter(|found| !found.is_empty());
    named
        .into_iter()
        .chain(["origin/main", "origin/master", "main", "master"].map(String::from))
        .collect()
}

fn resolve(root: &Path, reference: &str) -> Option<String> {
    let found = match reference.split_once("...") {
        Some((branch, head)) => Repo::at(root).text(&["merge-base", branch, head])?,
        None => Repo::at(root).text(&[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{reference}^{{commit}}"),
        ])?,
    };
    let found = found.trim().to_string();
    (!found.is_empty()).then_some(found)
}
