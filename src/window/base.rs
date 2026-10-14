use std::borrow::Cow;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{Map, Value};

use crate::facts::tree::{self, Tree};
use crate::modules;
use crate::syntax::structural::cache::ReadCost;
use crate::syntax::structural::facts::Outcome;
use crate::syntax::structural::{Cache, selected_extensions};
use crate::sys::changed::{self, Change, blobs};
use crate::sys::error::Error;
use crate::sys::git::{Boolean, Repo, Staged};
use crate::sys::state;

/// What laying a base out reads from the run beside it: where the configuration sits, the
/// change set against the base commit, and the state directory the structural cache sits in.
/// The base reads each of them only when its layout needs it. Spec 8.4.
pub trait Run {
    fn root(&self) -> &Path;
    fn changes(&self, base: &str) -> Result<Cow<'_, [Change]>, Error>;
    fn state(&self) -> Option<&Path>;
}

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
    graph: modules::Shared,
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
            graph: modules::Shared::default(),
        }
    }

    /// The base's module graph, built once for every gate that reads it.
    pub fn graph(&self) -> &modules::Shared {
        &self.graph
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

    /// The base commit's structural cache, named under the state directory and timed into the
    /// layout, and `None` where klin keeps no state. Spec 8.4.
    pub fn cache(&self, dir: Option<&Path>, root: &Path, commit: &str) -> Option<Cache> {
        self.spent(
            |layout| &mut layout.cache_name,
            || structural_cache(dir, root, commit),
        )
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
}

impl Drop for Prior {
    fn drop(&mut self) {
        self.teardown();
    }
}

pub fn materialize(run: &impl Run, before: &str, scope: Option<&[Change]>) -> Result<Prior, Error> {
    let dir = temporary()?;
    match scope {
        Some(changes) => written(run, before, changes, dir),
        None => checked_out(run, before, dir, under_the_repository(run.root())?),
    }
}

/// The whole base for a check that resolves names against all of it. A run that hands over its
/// change set may take the layout that checks no whole commit out, and takes today's checkout
/// wherever that layout cannot serve it: no structural cache to read, or a git command that
/// refused. Optimization state decides the cost and never the verdict. Spec 8.4.
pub fn laid_out(run: &impl Run, before: &str, light: Option<&[Change]>) -> Result<Prior, Error> {
    let dir = temporary()?;
    let inside = under_the_repository(run.root())?;
    if let Some(changes) = light
        && let Some(laid) = lightly(run, before, changes, dir.path(), &inside)
    {
        return Ok(held(run, laid, dir, changes));
    }
    checked_out(run, before, dir, inside)
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
    run: &impl Run,
    before: &str,
    dir: tempfile::TempDir,
    inside: PathBuf,
) -> Result<Prior, Error> {
    let root = run.root();
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
    let changes = prior.spent(|layout| &mut layout.changes, || run.changes(before))?;
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
    run: &impl Run,
    before: &str,
    changes: &[Change],
    dir: &Path,
    inside: &Path,
) -> Option<Laid> {
    let mut layout = Layout::default();
    let started = Instant::now();
    let cache = structural_cache(run.state(), run.root(), before)?;
    layout.cache_name = started.elapsed();
    let mut read = ReadCost::default();
    let started = Instant::now();
    let outcomes = cache.read(&mut read)?;
    read.total = started.elapsed();

    let root = dir.join(inside);
    let started = Instant::now();
    let laid = (|| {
        let catalogue = staged(run.root(), dir, &root, before)?;
        let wanted = required(&catalogue, changes, &outcomes);
        let written = wanted.len();
        Repo::at(&root).checkout_index(&wanted)?;
        Some((catalogue, written))
    })();
    let Some((catalogue, written)) = laid else {
        abandoned(run.root(), dir);
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
            abandoned(run.root(), dir);
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
    read: ReadCost,
    layout: Layout,
}

/// A light layout made into the base every gate reads, with the outcomes the layout read handed
/// to the tree, so the view that asks later neither names nor reads the cache again.
fn held(run: &impl Run, laid: Laid, dir: tempfile::TempDir, changes: &[Change]) -> Prior {
    let tree = Tree::listed(&laid.root, laid.files);
    tree.extracted()
        .hold(laid.cache, laid.outcomes, changes, laid.read);
    let mut prior = Prior::new(tree, dir, Some(run.root().to_path_buf()));
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
            if lists && tree::reached(&entry.path) {
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
    run: &impl Run,
    before: &str,
    changes: &[Change],
    dir: tempfile::TempDir,
) -> Result<Prior, Error> {
    let root = run.root();
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

/// The structural cache of a base commit under the state directory, and `None` where klin
/// keeps no state or the cache cannot be named. Spec 8.4.
fn structural_cache(dir: Option<&Path>, root: &Path, commit: &str) -> Option<Cache> {
    let under = dir?.join(state::CACHE).join(state::STRUCTURAL);
    Cache::at(&under, commit, &checkout(root))
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

/// The pair of trees a run compares: which of the three kinds of 4.2 it is, the base commit
/// `before` names, and how that commit was chosen. `after` is the working tree. `notes` are what
/// choosing it found that fails nothing, and `unproven` is why a base equal to HEAD may hide
/// unpushed commits. Spec 6.5.
#[derive(Clone)]
pub struct Window {
    pub kind: Kind,
    pub before: String,
    pub how: String,
    pub derives: Option<String>,
    pub notes: Vec<String>,
    pub unproven: Option<String>,
}

/// The three window kinds of section 4.2. The hook judges a turn, `klin check` by hand and CI
/// on a pull request judge a branch, and CI on a push judges the push.
#[derive(Clone, Copy)]
pub enum Kind {
    Turn,
    Branch,
    Push,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Turn => "turn",
            Kind::Branch => "branch",
            Kind::Push => "push",
        }
    }
}

impl Window {
    /// A window with nothing found in choosing it: no note and a proven comparison.
    pub fn new(kind: Kind, before: String, how: String, derives: Option<String>) -> Window {
        Window {
            kind,
            before,
            how,
            derives,
            notes: Vec::new(),
            unproven: None,
        }
    }

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

/// The base of a `branch` or `push` window: the first candidate that resolves. A candidate that
/// is present and does not resolve is an error where more history fixes it, and a push `before`
/// a force-push rewrote away is a note. Spec 6.5.
pub fn choose(root: &Path) -> Result<Window, Error> {
    let head = resolve(root, "HEAD");
    let shallow = Repo::at(root)
        .text(&["rev-parse", "--is-shallow-repository"])
        .is_some_and(|said| said.trim() == "true");
    let mut tried: Vec<String> = Vec::new();
    let (candidates, mut notes) = candidates(root);
    for candidate in candidates {
        tried.push(candidate.named.clone());
        let found = candidate
            .references
            .iter()
            .find_map(|(reference, source)| Some((resolve(root, reference)?, *source)));
        let Some((commit, source)) = found else {
            notes.extend(candidate.absent(root, shallow)?);
            continue;
        };
        let mut base = Window::new(candidate.kind, commit.clone(), candidate.how, Some(commit));
        base.notes = notes;
        if head.as_deref() == Some(base.before.as_str()) && !changed::dirty(root) {
            return Ok(equal_to_head(root, source, base));
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

/// A base candidate that is present and that only more history brings back.
fn unfetched(named: &str, shallow: bool) -> Error {
    let clone = match shallow {
        true => "this shallow clone does not hold it",
        false => "this clone does not hold it",
    };
    Error(format!(
        "the base {named} does not resolve: {clone}. Fetch it, or check out the whole history, \
         as `fetch-depth: 0` does for actions/checkout."
    ))
}

/// One place a base can come from: the references that name it, the way a report says it, and
/// what a candidate that is present and does not resolve means.
struct Candidate {
    named: String,
    references: Vec<(String, Source)>,
    how: String,
    kind: Kind,
    absent: Absent,
}

impl Candidate {
    /// A candidate that does not resolve: an error where more history fixes it, a note for a
    /// push base a force-push rewrote away, and nothing for a merge-base a full clone lacks.
    /// Spec 6.5.
    fn absent(&self, root: &Path, shallow: bool) -> Result<Option<String>, Error> {
        match &self.absent {
            Absent::Fetch => Err(unfetched(&self.named, shallow)),
            Absent::Rewritten if shallow => Err(unfetched(&self.named, true)),
            Absent::Rewritten => Ok(Some(format!(
                "the push started from {}, which this repository no longer holds — a \
                 force-push rewrote it away, so this run compares against the merge-base",
                short(&self.named)
            ))),
            Absent::Unrelated(branch) if shallow && resolve(root, branch).is_some() => Err(
                unfetched(&format!("the merge-base with {}", self.named), true),
            ),
            Absent::Unrelated(_) => Ok(None),
        }
    }
}

/// What a candidate that does not resolve means. Spec 6.5.
enum Absent {
    /// The pull request target, which CI must fetch.
    Fetch,
    /// A push `before`, which a force-push can rewrite away in a full clone.
    Rewritten,
    /// The merge-base with a default branch, which a shallow clone may lack while the branch
    /// itself resolves.
    Unrelated(String),
}

/// Where a base candidate came from, which decides whether a base equal to HEAD hides work.
#[derive(Clone, Copy, PartialEq)]
enum Source {
    /// A remote reference or a push event's commit. The remote agrees with HEAD.
    Remote,
    /// A local reference, which says nothing about what the remote has.
    Local,
}

/// A clean working tree whose base is HEAD measures nothing. A remote source and a local one the
/// remote default branch holds pass. A local source that hides unpushed commits is unproven, and
/// with no remote at all the run passes with a note. Spec 6.5.
fn equal_to_head(root: &Path, source: Source, mut base: Window) -> Window {
    if source == Source::Remote {
        base.notes.push(format!(
            "the base is HEAD ({}) and the working tree matches it, so the trees are the same",
            base.how
        ));
        return base;
    }
    let Some((name, tip)) = remote_tip(root) else {
        base.notes.push(format!(
            "the base is HEAD ({}) and the working tree matches it, and no remote default \
             branch resolves, so no remote proves what to compare",
            base.how
        ));
        return base;
    };
    let Some(unpushed) = Repo::at(root).text(&[
        "log",
        "--oneline",
        "-n",
        &SHOWN.to_string(),
        &format!("{tip}..HEAD"),
    ]) else {
        base.unproven = Some(format!(
            "the base is HEAD ({}) and the working tree matches it, and git could not list what \
             {name} is missing, so klin cannot tell whether these commits are pushed",
            base.how
        ));
        return base;
    };
    let unpushed: Vec<&str> = unpushed.lines().filter(|line| !line.is_empty()).collect();
    if unpushed.is_empty() {
        return base;
    }
    base.unproven = Some(format!(
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
    ));
    base
}

/// How many unpushed commits an error names, so a long-lived branch is not a wall of text.
const SHOWN: usize = 10;

/// HEAD and whether the working tree differs from it, which the check document records as the
/// `after` tree. Spec 6.5, 11.7.
pub fn tree_record(root: &Path) -> Value {
    serde_json::json!({ "head": resolve(root, "HEAD"), "dirty": changed::dirty(root) })
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

fn candidates(root: &Path) -> (Vec<Candidate>, Vec<String>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();
    if let Some(target) = environment("GITHUB_BASE_REF") {
        out.push(Candidate {
            references: vec![
                (format!("refs/remotes/origin/{target}"), Source::Remote),
                (target.clone(), Source::Local),
            ],
            how: format!("the tip of {target}, the pull request target"),
            named: target,
            kind: Kind::Branch,
            absent: Absent::Fetch,
        });
    }
    if let Some(before) = pushed_from() {
        out.push(Candidate {
            references: vec![(before.clone(), Source::Remote)],
            named: before,
            how: "the commit this push started from".to_string(),
            kind: Kind::Push,
            absent: Absent::Rewritten,
        });
    }
    let defaults = default_branches(root);
    if out.is_empty() {
        let resolved = stacked_on(root, &defaults);
        notes.extend(resolved.notes);
        out.extend(resolved.stacked.map(Stacked::candidate));
    }
    out.extend(defaults.into_iter().map(merge_base_with));
    (out, notes)
}

/// The merge-base with one of the default branch's names.
fn merge_base_with(branch: String) -> Candidate {
    let (reference, source) = match remote_reference(&branch) {
        Some(remote) => (remote, Source::Remote),
        None => (branch.clone(), Source::Local),
    };
    Candidate {
        references: vec![(format!("{reference}...HEAD"), source)],
        how: format!("the merge-base with {branch}"),
        named: branch,
        kind: Kind::Branch,
        absent: Absent::Unrelated(reference),
    }
}

const REMOTES: &str = "refs/remotes/";

/// The branch this one is stacked on, by its full ref, and `None` where the default branch is
/// the base. The turn stamp records it, so a Stop tracks the same branch. Spec 6.5, 6.6.
pub fn stacked(root: &Path) -> Option<String> {
    stacked_on(root, &default_branches(root))
        .stacked
        .map(|stacked| stacked.reference)
}

/// The branch a stacked branch was cut from, and how klin learned it. Spec 6.5.
struct Stacked {
    reference: String,
    named: String,
    how: String,
}

impl Stacked {
    fn candidate(self) -> Candidate {
        let source = match self.reference.starts_with(REMOTES) {
            true => Source::Remote,
            false => Source::Local,
        };
        Candidate {
            references: vec![(format!("{}...HEAD", self.reference), source)],
            how: format!("the merge-base with {} ({})", self.named, self.how),
            named: self.named,
            kind: Kind::Branch,
            absent: Absent::Unrelated(self.reference),
        }
    }
}

/// What candidate 3 found: the branch HEAD is stacked on, if any, and the notes a person can act
/// on, a key that names no branch and a fork point several branches hold. Spec 6.5.
#[derive(Default)]
struct Resolved {
    stacked: Option<Stacked>,
    notes: Vec<String>,
}

/// The branch `branch.<name>.klin-base` names, or else the one inferred from the fork point. The
/// default branch is the first of its names whose remote ref resolves, and a fallback name past it
/// is an ordinary branch. A branch on the default branch, or with no name, is stacked on nothing.
/// Spec 6.5.
fn stacked_on(root: &Path, defaults: &[String]) -> Resolved {
    let repo = Repo::at(root);
    let Some(name) = branch_name(&repo) else {
        return Resolved::default();
    };
    let default_ref = defaults
        .iter()
        .filter_map(|branch| remote_reference(branch))
        .find(|reference| resolve(root, reference).is_some());
    if default_ref.as_deref() == Some(format!("{REMOTES}origin/{name}").as_str()) {
        return Resolved::default();
    }
    let key = format!("branch.{name}.klin-base");
    let mut notes = Vec::new();
    match keyed(&repo, &key) {
        Some(Ok(stacked)) => {
            return Resolved {
                stacked: Some(stacked),
                notes,
            };
        }
        Some(Err(note)) => notes.push(note),
        None => {}
    }
    let stacked = inferred(&repo, &name, default_ref.as_deref()).unwrap_or_else(|holders| {
        notes.push(ambiguous(&key, &holders));
        None
    });
    Resolved { stacked, notes }
}

/// The name of the branch HEAD is on, and `None` when HEAD is detached.
fn branch_name(repo: &Repo) -> Option<String> {
    let head = repo.text(&["symbolic-ref", "-q", "HEAD"])?;
    head.trim().strip_prefix("refs/heads/").map(str::to_string)
}

/// The branch the key names, `None` when the key is not set, and the note for a key that names
/// no branch or a branch that already holds HEAD, such as this branch's own remote copy, because
/// a base at HEAD would hide every commit of the branch.
fn keyed(repo: &Repo, key: &str) -> Option<Result<Stacked, String>> {
    let named = repo.text(&["config", "--get", key])?;
    let named = named.trim();
    let Some(reference) = full_name(repo, named) else {
        return Some(Err(format!(
            "{key} names {named}, which does not resolve, so klin infers the base"
        )));
    };
    if repo.is_ancestor("HEAD", &reference) == Some(true) {
        return Some(Err(format!(
            "{key} names {named}, which already holds HEAD and would hide this branch's \
             commits, so klin infers the base. Name the branch this one was cut from"
        )));
    }
    Some(Ok(Stacked {
        reference,
        named: named.to_string(),
        how: key.to_string(),
    }))
}

/// The one remote branch other than the default that holds the fork point. Where the default
/// branch holds it, or no branch does, there is none, and where several do, they are the error.
fn inferred(
    repo: &Repo,
    name: &str,
    default_ref: Option<&str>,
) -> Result<Option<Stacked>, Vec<String>> {
    let Some(holders) = holding_the_fork_point(repo, name) else {
        return Ok(None);
    };
    if holders
        .iter()
        .any(|held| Some(held.as_str()) == default_ref)
    {
        return Ok(None);
    }
    match holders.as_slice() {
        [held] => Ok(Some(Stacked {
            named: remote_name(held).to_string(),
            reference: held.clone(),
            how: "inferred".to_string(),
        })),
        [] => Ok(None),
        _ => Err(holders),
    }
}

/// The note for a fork point several remote branches hold.
fn ambiguous(key: &str, holders: &[String]) -> String {
    format!(
        "this branch forks from a commit that {} all hold, so klin judges against the default \
         branch. Set {key} to the branch it was cut from",
        holders
            .iter()
            .map(|held| remote_name(held))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// A remote-tracking ref without its `refs/remotes/` prefix, as `origin/main`.
fn remote_name(reference: &str) -> &str {
    reference.strip_prefix(REMOTES).unwrap_or(reference)
}

/// The remote branches other than this branch's own copies, `<remote>/<name>`, that hold the
/// fork point. Spec 6.5.
fn holding_the_fork_point(repo: &Repo, name: &str) -> Option<Vec<String>> {
    let fork = fork_point(repo, name)?;
    let holders = repo.text(&[
        "for-each-ref",
        "--contains",
        &fork,
        "--format=%(refname)",
        REMOTES,
    ])?;
    let own = |held: &str| {
        held.strip_prefix(REMOTES)
            .and_then(|remote| remote.split_once('/'))
            .is_some_and(|(_, branch)| branch == name || branch == "HEAD")
    };
    Some(
        holders
            .lines()
            .filter(|held| !own(held))
            .map(str::to_string)
            .collect(),
    )
}

/// The first parent of the oldest commit on HEAD's first-parent chain that no remote branch
/// other than this branch's own copies holds. Where one holds HEAD itself there is none, because
/// a base at HEAD would hide every commit of the branch, and with no remote branch there is none
/// and no history is walked.
fn fork_point(repo: &Repo, name: &str) -> Option<String> {
    let any = repo.text(&["for-each-ref", "--count=1", "--format=%(refname)", REMOTES])?;
    if any.trim().is_empty() {
        return None;
    }
    let exclude = format!("--exclude=*/{name}");
    let walked = repo.text(&[
        "rev-list",
        "--first-parent",
        "--parents",
        "HEAD",
        "--not",
        &exclude,
        "--exclude=*/HEAD",
        "--remotes",
    ])?;
    let oldest = walked.lines().last()?;
    oldest.split_whitespace().nth(1).map(str::to_string)
}

/// The full ref of the branch a name resolves to, local or remote-tracking, and `None` for a
/// name that is not a branch, such as a tag.
fn full_name(repo: &Repo, named: &str) -> Option<String> {
    let found = repo.text(&[
        "rev-parse",
        "--verify",
        "--quiet",
        "--symbolic-full-name",
        named,
    ])?;
    let found = found.trim();
    (found.starts_with("refs/heads/") || found.starts_with(REMOTES)).then(|| found.to_string())
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
