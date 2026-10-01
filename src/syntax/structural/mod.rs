//! The semantic facts a structural check reads, and the optional index that resolves names.
//! A consumer here never sees a Tree-sitter node or a node kind: it sees declarations, imports,
//! module declarations and references, and a file it did not measure says so. Rust and TypeScript
//! are the structural languages of V1, and TSX is TypeScript. ADR 0035.

use std::borrow::Borrow;
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::ops::{Add, Deref};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

use crate::changed::Change;
use crate::config::{Config, Error};
use crate::coverage::Files;
use crate::files::{self, Found};
use crate::project::Tree;
pub use crate::syntax::LanguageId;
use crate::syntax::convention;
use crate::syntax::{LANGUAGES, Language, Parsed, ParsedFile, Unparsed, line_at, parse, walk};

mod cache;
pub mod footprint;
mod rust;
mod typescript;

pub use cache::Cache;

/// What a declaration is, as far as V1 tells them apart. A language's own word for one is not
/// here: a Rust `struct` and a TypeScript `interface` are both a type.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum DeclarationKind {
    Function,
    Method,
    Type,
    Constant,
    Variable,
}

/// What a declaration's own syntax says about who may reach it. Rust writes `pub`, a restricted
/// `pub(crate)`, `pub(super)`, `pub(self)` or `pub(in ...)`, or nothing. TypeScript writes
/// `export` at the top of a file or nothing. Only `Public` can be part of a consumer's contract.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Visibility {
    Private,
    Restricted,
    Public,
}

pub struct Declaration {
    pub name: String,
    /// The names a destructuring pattern binds, each by its local name, and none where the
    /// declaration is named by one identifier.
    pub bindings: Box<[String]>,
    pub kind: DeclarationKind,
    pub line: u64,
    /// The last line the declaration covers, so a consumer can tell a reference written inside
    /// the declaration from one written outside it. A declaration with no body ends where it
    /// starts.
    pub end: u64,
    pub text: String,
    /// True where the syntax alone shows the declaration is exposed past the file that holds
    /// it. A doubt reads as exposed, so a dead-symbol check under-reports and never over-reports.
    pub externally_visible: bool,
    /// True where syntax or the shared test convention proves that the runtime or a framework
    /// calls this declaration without a source reference.
    pub entry_point: bool,
    /// The inline modules that hold the declaration, outermost first, and none at the top of a
    /// file.
    pub nesting: Vec<String>,
    /// True where a type body holds the declaration, as a Rust `impl` or `trait` holds its
    /// associated items, so it is no item of the module.
    pub associated: bool,
    /// What the declaration's own modifier says, with no doubt read either way.
    pub visibility: Visibility,
    /// The name a consumer of the module addresses the declaration by where it differs from
    /// `name`: TypeScript's `export default class Client` is addressed as `default`.
    pub exported_as: Option<String>,
    /// The type an inherent implementation adds this method to, for a language that writes
    /// methods outside the type's own body, as Rust's `impl Client { pub fn new() }` does. A
    /// trait's method, a trait implementation's method and a member of a class carry none.
    pub owner: Option<String>,
    /// The declared contract, canonical: no body, no comment, no attribute but a directly
    /// written `#[non_exhaustive]`, one space between tokens, and a parameter binding that is
    /// not contract written as `_`. `None` where the syntax is a form V1 does not canonicalize,
    /// and empty for a TypeScript implementation that follows its overloads. A type the
    /// language would infer is written as `?`, so an inferred contract is visibly partial and
    /// never fabricated.
    pub signature: Option<String>,
}

impl Declaration {
    /// Whether a destructuring pattern binds its names, so `name` holds the pattern text and
    /// `names` yields the bindings.
    pub fn destructures(&self) -> bool {
        !self.bindings.is_empty()
    }

    /// The names it declares: every name its pattern binds, and its own name where it binds
    /// none.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        let names: &[String] = if self.destructures() {
            &self.bindings
        } else {
            std::slice::from_ref(&self.name)
        };
        names.iter().map(String::as_str)
    }
}

/// One statement that exposes names past the module: Rust's `pub use` and `pub extern crate`,
/// and every TypeScript `export` that is not a declaration of its own. A leaf names what is
/// exposed and under which name. The module graph resolves a path or a specifier; nothing here
/// does.
pub struct Export {
    pub line: u64,
    pub text: String,
    /// The inline modules that hold the statement, outermost first.
    pub nesting: Vec<String>,
    /// The module specifier a TypeScript re-export names, and none for a local export or a Rust
    /// statement, whose leaves carry their own paths.
    pub source: Option<String>,
    /// True where the syntax proves only a type is exposed: TypeScript's `export type { T }`.
    pub type_only: bool,
    /// False for a form V1 recognizes as an export and cannot list the names of, such as
    /// TypeScript's `export = x` or an ambient module. A consumer reports it as a hole.
    pub supported: bool,
    pub leaves: Vec<ExportLeaf>,
    /// The canonical contract of what the statement declares where no declaration fact holds
    /// it: the namespace a TypeScript `export namespace N` declares. None for any other export.
    pub contract: Option<String>,
}

/// One name an export exposes. `path` is what is exposed as the source wrote it: a Rust leaf
/// path with `*` for a glob, a TypeScript local or source name, `*` for a star export, and
/// empty for an anonymous default export. `name` is the external name, and `None` for a glob
/// that exposes every name of its target.
pub struct ExportLeaf {
    pub path: String,
    pub name: Option<String>,
}

/// One Rust `extern crate`, whatever its visibility: the crate it names and the name it binds,
/// which is its alias where one is written. At the top of a crate root it puts that name in the
/// crate's extern prelude.
pub struct ExternCrate {
    /// The inline modules that hold the statement, outermost first.
    pub nesting: Vec<String>,
    pub name: String,
    pub alias: String,
}

/// One import, holding the specifier as it was written. The module graph resolves it to a file.
pub struct Import {
    pub line: u64,
    pub text: String,
    /// The inline modules that hold the import, outermost first, and none at the top of a file.
    pub nesting: Vec<String>,
    /// True where a function body holds the import, so it binds its names in that body and not
    /// in the module.
    pub in_function: bool,
    pub module: Option<String>,
    pub names: Vec<String>,
    /// Every path a Rust use tree names, one per leaf, its segments joined by `::`, with `self`
    /// in a list read as the path above it and a glob kept as `*`. A language whose import names
    /// a module specifier keeps none.
    pub paths: Vec<String>,
}

/// One module declaration, such as Rust's `mod foo;` or `mod foo { }`. A language without that
/// syntax declares none.
pub struct ModuleDecl {
    pub line: u64,
    pub text: String,
    pub name: String,
    /// The inline modules that hold the declaration, outermost first.
    pub nesting: Vec<String>,
    /// True where the declaration holds its module's body, so no file is named for it.
    pub inline: bool,
    /// The file the declaration names instead of its own name, where the language can say so.
    /// Rust writes it `#[path = "other.rs"]`. The module graph resolves either to a file.
    pub path: Option<String>,
    /// What the declaration's own modifier says: `pub mod` is `Public`, `mod` is `Private`.
    pub visibility: Visibility,
}

/// A path written outside every import that starts at the crate or at the module that holds it,
/// such as Rust's `crate::a::b` or `super::c`. A path that starts at any other name is not kept,
/// because a name alone may be an external crate or a local item.
pub struct QualifiedPath {
    pub line: u64,
    pub nesting: Vec<String>,
    pub path: String,
}

/// A thin, value-semantic owner of one canonical reference name.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Name(Rc<String>);

impl Name {
    /// A self-owning name for synthetic facts and other values outside a parsing pool.
    pub fn new(text: impl AsRef<str>) -> Name {
        Name(Rc::new(text.as_ref().to_owned()))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The physical canonical allocation, used only by representation diagnostics.
    pub(crate) fn allocation(&self) -> *const String {
        Rc::as_ptr(&self.0)
    }
}

impl Borrow<str> for Name {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Deref for Name {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

/// One tree or cache decode's local canonicalization pool.
#[derive(Default)]
pub(crate) struct Names {
    held: HashSet<Name>,
}

impl Names {
    pub(crate) fn intern(&mut self, text: &str) -> Name {
        if let Some(name) = self.held.get(text) {
            return name.clone();
        }
        let name = Name::new(text);
        self.held.insert(name.clone());
        name
    }
}

pub struct Reference {
    pub name: Name,
    pub line: u64,
}

/// One source location whose name the index can resolve to declarations.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ReferenceSite<'a> {
    pub file: &'a str,
    pub line: u64,
}

pub struct FileFacts {
    pub file: String,
    pub language: LanguageId,
    pub declarations: Vec<Declaration>,
    pub imports: Vec<Import>,
    pub module_declarations: Vec<ModuleDecl>,
    pub references: Vec<Reference>,
    pub paths: Vec<QualifiedPath>,
    pub exports: Vec<Export>,
    pub crates: Vec<ExternCrate>,
}

/// What one file came to under structural analysis. Three of the four outcomes are not a
/// measurement, and each says so in its own name, so no consumer can read an empty set of
/// facts as a file it measured. The fourth, a file the mode rule could not read, is an error
/// where every other check raises one. ADR 0003, ADR 0035, spec 8.6.
#[derive(Clone)]
pub enum Outcome {
    Facts(Rc<FileFacts>),
    /// A grammar read the file and no structural adapter reads its language.
    Unsupported(&'static str),
    /// The grammar rejected the text.
    Unparsed(Unparsed),
    /// No grammar here reads the path at all.
    Foreign,
}

pub struct Unsupported {
    pub file: String,
    pub language: &'static str,
}

/// One structural measurement over a discovered file set. Consumers receive the semantic facts
/// and explicit coverage outcomes; a name-resolving consumer asks for the index lazily, and none
/// parses files or reconstructs capability gaps.
pub struct Measurement {
    facts: Vec<Rc<FileFacts>>,
    index: OnceCell<SourceIndex>,
    pub unparsed: Vec<Unparsed>,
    pub unsupported: Vec<Unsupported>,
    pub files: Files,
    pub cost: ExtractionCost,
}

impl Measurement {
    /// The selected files' shared facts, sorted by path, without building the name-resolution
    /// index. `index().files()` holds the same files in the same order.
    pub fn facts(&self) -> &[Rc<FileFacts>] {
        &self.facts
    }

    /// The name-resolution index, built once only when a consumer asks for it.
    pub fn index(&self) -> &SourceIndex {
        self.index
            .get_or_init(|| SourceIndex::of(self.facts.clone()))
    }

    pub fn indexed(&self, cost: &mut TreeNameCost) -> &SourceIndex {
        let index = timed(&mut cost.index, || self.index());
        cost.files = index.files.len();
        cost.declarations = index.files.iter().map(|file| file.declarations.len()).sum();
        cost.references = index
            .names
            .values()
            .flat_map(HashMap::values)
            .map(|held| held.references.len())
            .sum();
        cost.distinct_names = index.names.values().map(HashMap::len).sum();
        index
    }
}

/// What a name-resolving gate's evidence cost: the base it laid out, each tree's part, and the
/// lost references `dead-symbols` explained. Spec 11.2.
#[derive(Default, Clone, Copy)]
pub struct NameCost {
    pub base: Duration,
    pub before: TreeNameCost,
    pub after: TreeNameCost,
    pub lost: Option<Duration>,
}

/// One tree's measurement, index and name queries, with what its index holds. Spec 11.2.
#[derive(Default, Clone, Copy)]
pub struct TreeNameCost {
    pub measure: Duration,
    pub index: Duration,
    pub query: Duration,
    pub files: usize,
    pub declarations: usize,
    pub references: usize,
    pub distinct_names: usize,
}

pub fn timed<T>(spent: &mut Duration, work: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let out = work();
    *spent += started.elapsed();
    out
}

/// Every outcome one tree's files came to, each file read, parsed and extracted on the first
/// request and held for the life of the tree, which is one run. Every structural check a run
/// selects reads the one extraction of a file, and still selects its own files and keeps their
/// facts, so a file one check leaves out never resolves a name for it because another check read
/// that file. A name-resolving measurement builds its own index lazily. A base tree may also hold
/// outcomes its structural cache kept from an earlier run, each taken the first time a check asks
/// for its file. ADR 0038.
#[derive(Default)]
pub struct Extracted {
    held: RefCell<HashMap<String, Outcome>>,
    cached: RefCell<HashMap<String, Outcome>>,
    names: RefCell<Names>,
    kept: OnceCell<Kept>,
}

/// The cache a base tree's outcomes are kept in, the paths whose base bytes the change set may
/// have moved and which the cache therefore never holds, and whether the cache is stale because
/// this run extracted an outcome it lacks.
struct Kept {
    cache: Cache,
    changed: HashSet<String>,
    stale: Cell<bool>,
    /// What reading the cache cost, when a caller read it before this tree held it, handed over
    /// once to the view that would otherwise have read it. Spec 11.2.
    read: Cell<Duration>,
}

impl Extracted {
    fn outcome(
        &self,
        path: &Path,
        file: &str,
        cost: &mut ExtractionCost,
    ) -> Result<Outcome, Error> {
        if let Some(held) = self.held.borrow().get(file) {
            cost.shared += 1;
            return Ok(held.clone());
        }
        let cached = self.cached.borrow_mut().remove(file);
        if let Some(outcome) = cached {
            cost.cached += 1;
            self.held
                .borrow_mut()
                .insert(file.to_string(), outcome.clone());
            return Ok(outcome);
        }
        let started = Instant::now();
        let bytes = std::fs::read(path).map_err(|why| Error::unreadable(path, why))?;
        cost.reads += 1;
        cost.parses += 1;
        let outcome = {
            let mut names = self.names.borrow_mut();
            of_with(file, &String::from_utf8_lossy(&bytes), &mut names)?
        };
        cost.extracted += 1;
        cost.time += started.elapsed();
        if let Some(kept) = self.kept.get().filter(|kept| !kept.changed.contains(file)) {
            kept.stale.set(true);
        }
        self.held
            .borrow_mut()
            .insert(file.to_string(), outcome.clone());
        Ok(outcome)
    }

    /// This tree's outcomes read from its cache, once for the run, less every path the change
    /// set names. The cache is named only when the tree holds none yet, and the time reading and
    /// decoding it took is the cost; naming it is the caller's part.
    fn keep(&self, cache: impl FnOnce() -> Option<Cache>, changes: &[Change]) -> ExtractionCost {
        if let Some(kept) = self.kept.get() {
            return ExtractionCost {
                cache_read: kept.read.take(),
                ..ExtractionCost::default()
            };
        }
        let Some(cache) = cache() else {
            return ExtractionCost::default();
        };
        let started = Instant::now();
        let outcomes = cache.read();
        self.held_from(cache, outcomes, changes, Duration::ZERO);
        ExtractionCost {
            cache_read: started.elapsed(),
            ..ExtractionCost::default()
        }
    }

    /// The same, for a caller that named and read the cache itself, because it needed the
    /// outcomes before this tree existed: the base laid out without a checkout reads them to
    /// know which of its files it must write. The read's cost is handed to the view that asks
    /// later, so the run records it once and in the same counter. Spec 8.4, 11.2.
    pub fn hold(
        &self,
        cache: Cache,
        outcomes: HashMap<String, Outcome>,
        changes: &[Change],
        read: Duration,
    ) {
        if self.kept.get().is_some() {
            return;
        }
        self.held_from(cache, Some(outcomes), changes, read);
    }

    fn held_from(
        &self,
        cache: Cache,
        outcomes: Option<HashMap<String, Outcome>>,
        changes: &[Change],
        read: Duration,
    ) {
        let changed: HashSet<String> = changes
            .iter()
            .flat_map(|change| std::iter::once(&change.path).chain(&change.was))
            .cloned()
            .collect();
        if let Some(outcomes) = outcomes {
            let held = self.held.borrow();
            self.cached.borrow_mut().extend(
                outcomes
                    .into_iter()
                    .filter(|(file, _)| !changed.contains(file) && !held.contains_key(file)),
            );
        }
        let _ = self.kept.set(Kept {
            cache,
            changed,
            stale: Cell::new(false),
            read: Cell::new(read),
        });
    }

    /// Every outcome this tree holds for a path the change set does not name, written to its
    /// cache when this run extracted one the cache lacked. The time the write took is the cost.
    fn publish(&self) -> ExtractionCost {
        let Some(kept) = self.kept.get().filter(|kept| kept.stale.replace(false)) else {
            return ExtractionCost::default();
        };
        let started = Instant::now();
        let (held, cached) = (self.held.borrow(), self.cached.borrow());
        let mut outcomes: Vec<(&str, &Outcome)> = held
            .iter()
            .chain(cached.iter())
            .filter(|(file, _)| !kept.changed.contains(*file))
            .map(|(file, outcome)| (file.as_str(), outcome))
            .collect();
        outcomes.sort_unstable_by_key(|(file, _)| *file);
        kept.cache.write(&outcomes);
        ExtractionCost {
            cache_write: started.elapsed(),
            ..ExtractionCost::default()
        }
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

/// What one measurement's files cost: how many it read, parsed and extracted itself, how many
/// an earlier measurement had already extracted, how many it took from the structural cache,
/// the time the first kind took, and the time reading and writing the cache took. A gate
/// records the sum over its two trees. Spec 11.2, 13.
#[derive(Default, Clone, Copy)]
pub struct ExtractionCost {
    pub reads: usize,
    pub parses: usize,
    pub extracted: usize,
    pub shared: usize,
    pub cached: usize,
    pub time: Duration,
    pub cache_read: Duration,
    pub cache_write: Duration,
}

impl Add for ExtractionCost {
    type Output = ExtractionCost;

    fn add(self, other: ExtractionCost) -> ExtractionCost {
        ExtractionCost {
            reads: self.reads + other.reads,
            parses: self.parses + other.parses,
            extracted: self.extracted + other.extracted,
            shared: self.shared + other.shared,
            cached: self.cached + other.cached,
            time: self.time + other.time,
            cache_read: self.cache_read + other.cache_read,
            cache_write: self.cache_write + other.cache_write,
        }
    }
}

/// What structural analysis makes of one path.
pub fn of(path: &str, source: &str) -> Result<Outcome, Error> {
    let mut names = Names::default();
    of_with(path, source, &mut names)
}

/// The same extraction with a caller-owned pool shared across a tree or a batch of blobs.
pub(crate) fn of_with(path: &str, source: &str, names: &mut Names) -> Result<Outcome, Error> {
    match parse(path, source)? {
        None => Ok(Outcome::Foreign),
        Some(Parsed::Rejected(file)) => Ok(Outcome::Unparsed(file)),
        Some(Parsed::Read(file)) => measured(&file, names),
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
    let found = files::found(tree, &[tree.root().to_path_buf()], &wanted)?;
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
            Outcome::Unsupported(language) => unsupported.push(Unsupported { file, language }),
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
        index: OnceCell::new(),
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

fn measured(file: &ParsedFile, names: &mut Names) -> Result<Outcome, Error> {
    Ok(match facts_with(file, names)? {
        Some(found) => Outcome::Facts(Rc::new(found)),
        None => Outcome::Unsupported(file.language.name),
    })
}

fn facts_with(file: &ParsedFile, names: &mut Names) -> Result<Option<FileFacts>, Error> {
    let Some(adapter) = adapter(file.language.id) else {
        return Ok(None);
    };
    harvest(file, adapter, names).map(Some)
}

/// The one structural registry. A new language is an adapter file and one arm here.
fn adapter(id: LanguageId) -> Option<&'static Adapter> {
    match id {
        LanguageId::Rust => Some(&rust::ADAPTER),
        LanguageId::TypeScript => Some(&typescript::ADAPTER),
        _ => None,
    }
}

/// The logical language a path is written in, by its extension, whether or not its grammar
/// accepts the file. The module graph asks this to know which resolvers a tree needs.
pub fn language_of(path: &str) -> Option<LanguageId> {
    crate::syntax::language_of(path).map(|language| language.id)
}

/// Whether one grammar reads both paths, so the base's bytes of a file renamed between them were
/// read under the grammar the base's own path selects.
pub fn same_grammar(was: &str, now: &str) -> bool {
    let grammar = |path| crate::syntax::language_of(path).map(|language| language.name);
    grammar(was) == grammar(now)
}

/// Whether any adapter reads this language, which is what a consumer asks before it counts a
/// file as one it could have measured.
pub fn supports(id: LanguageId) -> bool {
    adapter(id).is_some()
}

/// The languages a structural check can be configured for, each with the one name it is named
/// by. A grammar variant is not one of them: `tsx` selects a grammar for the `complexity`
/// table and can never name a structural language here, because a `.tsx` file is TypeScript.
/// Every structural consumer reads its `languages` key through this, so none of them holds a
/// language name of its own. ADR 0035.
pub fn languages() -> Vec<(&'static str, LanguageId)> {
    let mut out: Vec<(&'static str, LanguageId)> = Vec::new();
    for language in LANGUAGES.iter().filter(|row| supports(row.id)) {
        let named = out.iter().any(|(_, id)| *id == language.id);
        if let (false, Some(name)) = (named, language.names.first()) {
            out.push((name, language.id));
        }
    }
    out
}

/// The error a section raises for a language name no structural adapter measures, naming the
/// first such name and every name it could have used.
pub fn unknown_language(config: &Config, section: &str, named: &[String]) -> Error {
    let name = named
        .iter()
        .find(|name| selected_extensions(&[(*name).clone()]).is_none())
        .map_or("", String::as_str);
    Error(format!(
        "{}: \"{section}\" measures no language called \"{name}\" — one of: {}",
        config.file.display(),
        known_languages().join(", ")
    ))
}

/// The configured names and extensions of structural adapters, with grammar variants merged
/// under their logical language.
pub fn language_extensions() -> Vec<(&'static str, String)> {
    languages()
        .into_iter()
        .map(|(name, id)| {
            let extensions: Vec<String> = LANGUAGES
                .iter()
                .filter(|row| row.id == id)
                .flat_map(|row| row.extensions.iter().copied())
                .map(|extension| format!("`{extension}`"))
                .collect();
            (name, extensions.join(", "))
        })
        .collect()
}

/// Extensions selected by configured logical-language names. With no names, select every
/// structural adapter; an explicitly named parser language remains discoverable so measurement
/// can report that it has no structural adapter.
pub fn selected_extensions(names: &[String]) -> Option<Vec<&'static str>> {
    let supported: Vec<LanguageId> = languages().into_iter().map(|(_, id)| id).collect();
    let selected: Vec<&Language> = if names.is_empty() {
        LANGUAGES
            .iter()
            .filter(|language| supported.contains(&language.id))
            .collect()
    } else {
        let mut selected: Vec<&Language> = Vec::new();
        for name in names {
            let found: Vec<&Language> = LANGUAGES
                .iter()
                .filter(|language| language.names.first() == Some(&name.as_str()))
                .collect();
            if found.is_empty() {
                return None;
            }
            for language in found {
                if !selected.iter().any(|held| std::ptr::eq(*held, language)) {
                    selected.push(language);
                }
            }
        }
        selected
    };
    Some(
        selected
            .into_iter()
            .flat_map(|language| language.extensions.iter().copied())
            .collect(),
    )
}

pub fn known_languages() -> Vec<&'static str> {
    let mut known: Vec<&str> = LANGUAGES
        .iter()
        .filter_map(|language| language.names.first().copied())
        .collect();
    known.sort_unstable();
    known.dedup();
    known
}

/// What one language adapter states. A query names the node kinds that declare something, and
/// the capture name says what kind of thing; everything else is the handful of judgments a
/// query cannot make.
pub(crate) struct Adapter {
    pub patterns: &'static str,
    /// The node kinds that are a use of a name.
    pub identifiers: &'static [&'static str],
    /// The node kinds that turn a function into a method when one holds it.
    pub methods_in: &'static [&'static str],
    /// Names invoked by the runtime without a source reference.
    pub entry_points: &'static [&'static str],
    pub visible: fn(Node) -> bool,
    pub imported: fn(Node, &[u8]) -> Imported,
    /// The file a module declaration was remapped to, for a language that writes such a thing.
    pub remapped: fn(Node, &[u8]) -> Option<String>,
    /// The inline modules that hold a node, outermost first.
    pub nesting: fn(Node, &[u8]) -> Vec<String>,
    /// The segments of the whole path a node writes, and `None` for any other node, including a
    /// path inside a longer one.
    pub qualified: fn(Node, &[u8]) -> Option<Vec<String>>,
    /// The first segments a qualified path resolves from inside the crate. A path that starts
    /// with any other name is kept only where it names a module the file declares beside it.
    pub rooted: &'static [&'static str],
    /// The names a node writes inside a string that the language calls by that text, such as a
    /// function a Rust `serde` attribute names or a name a format string captures.
    pub quoted: fn(Node, &[u8]) -> Vec<String>,
    /// What a declaration's or a module declaration's own modifier says.
    pub visibility: fn(Node, &[u8]) -> Visibility,
    /// The external name a declaration is exported under where it differs from its own name.
    pub exported_as: fn(Node, &[u8]) -> Option<String>,
    /// The type an inherent implementation adds a method to.
    pub owner: fn(Node, &[u8]) -> Option<String>,
    /// The nodes that write each name a declaration's name binds where it is a destructuring
    /// pattern.
    pub destructured: fn(Node) -> Vec<Node>,
    /// The canonical declared contract of a declaration, and `None` for a form V1 does not
    /// canonicalize.
    pub contract: fn(Node, &[u8]) -> Option<String>,
    /// What an `@export` capture in the file at this path exposes, and `None` where the node
    /// exports nothing a declaration does not already say for itself.
    pub exported: fn(Node, &[u8], &str) -> Option<Exported>,
}

/// What one import states, before the shared reader puts it at a line. The specifier is kept
/// as it was written, and the module graph resolves it.
pub(crate) struct Imported {
    pub module: Option<String>,
    pub names: Vec<String>,
    pub paths: Vec<String>,
}

/// What one export statement states, before the shared reader puts it at a line.
pub(crate) struct Exported {
    pub source: Option<String>,
    pub type_only: bool,
    pub supported: bool,
    pub leaves: Vec<ExportLeaf>,
    pub contract: Option<String>,
}

/// How the canonical spelling treats one node: leave the subtree out, write this text for it
/// and go no deeper, or spell it token by token.
pub(crate) enum Spelling {
    Skip,
    Replace(String),
    Keep,
}

/// The canonical text of one node: every token the rule keeps, one space apart, with the
/// spacing a reader expects around punctuation. Comments never reach it, because a rule skips
/// them, and the rule decides what a body, an attribute or a binding name becomes.
pub(crate) fn spelled(node: Node, source: &[u8], rule: &dyn Fn(Node) -> Spelling) -> String {
    let mut tokens = Vec::new();
    collect_tokens(node, source, rule, &mut tokens);
    tidy(&tokens)
}

fn collect_tokens(
    node: Node,
    source: &[u8],
    rule: &dyn Fn(Node) -> Spelling,
    out: &mut Vec<String>,
) {
    match rule(node) {
        Spelling::Skip => {}
        Spelling::Replace(text) => out.push(text),
        Spelling::Keep if node.child_count() == 0 => out.push(text_of(node, source)),
        Spelling::Keep => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_tokens(child, source, rule, out);
            }
        }
    }
}

/// Tokens joined by one space, less the space a reader would not write: before a closing
/// bracket or a separator, after an opening bracket, around a path separator or a member dot,
/// and between a name and the bracket that opens its arguments. A separator left dangling
/// before a closing bracket, where a skipped token stood after it, is dropped.
fn tidy(tokens: &[String]) -> String {
    let tokens: Vec<&str> = tokens
        .iter()
        .map(String::as_str)
        .filter(|token| !token.is_empty())
        .collect();
    let mut out = String::new();
    let mut last: Option<&str> = None;
    for (at, token) in tokens.iter().enumerate() {
        if dangling(&tokens, at) {
            continue;
        }
        if !last.is_none_or(|last| glued(last, token)) {
            out.push(' ');
        }
        out.push_str(token);
        last = Some(token);
    }
    out
}

/// Whether the token at `at` is a separator nothing follows but a closing bracket or another
/// separator.
fn dangling(tokens: &[&str], at: usize) -> bool {
    const CLOSES: &[&str] = &[")", "]", "}", ">"];
    tokens[at] == ","
        && tokens
            .get(at + 1)
            .is_none_or(|next| CLOSES.contains(next) || *next == ",")
}

/// Whether no space stands between these two tokens.
fn glued(last: &str, token: &str) -> bool {
    const NO_SPACE_BEFORE: &[&str] = &[",", ";", ")", "]", ">", ":", "?", ".", "::", "!"];
    const NO_SPACE_AFTER: &[&str] = &["(", "[", "<", "&", "::", ".", "#", "*", "..."];
    const OPENS: &[&str] = &["(", "[", "<"];
    NO_SPACE_AFTER.contains(&last)
        || NO_SPACE_BEFORE.contains(&token)
        || (OPENS.contains(&token) && ends_a_name(last))
}

/// Whether a token is one an argument bracket attaches to directly: a name, a closing bracket
/// or a closing angle.
fn ends_a_name(token: &str) -> bool {
    token == ">"
        || token
            .chars()
            .last()
            .is_some_and(|last| last.is_alphanumeric() || matches!(last, '_' | ')' | ']'))
}

const METHOD: &str = "method";
const TYPE: &str = "type";
const CONSTANT: &str = "constant";
const VARIABLE: &str = "variable";
const IMPORT: &str = "import";
const MODULE: &str = "module";
const EXPORT: &str = "export";
const CRATE: &str = "crate";

type Held = OnceLock<Result<Query, String>>;

/// The cell one grammar variant's compiled query lives in, which is that language's own place
/// in the registry. A query belongs to the grammar it was compiled against, so TypeScript and
/// TSX hold one each, and no second table has to be kept in step with the registry.
fn held(language: &Language) -> Option<&'static Held> {
    static CELLS: [Held; LANGUAGES.len()] = [const { OnceLock::new() }; LANGUAGES.len()];
    CELLS.get(LANGUAGES.iter().position(|row| row.name == language.name)?)
}

/// The adapter's query against this grammar, compiled on first use and held for the life of
/// the process. A query is a constant of this binary, so one that will not compile is a fault
/// in klin and not in the tree it reads.
fn compiled(language: &'static Language, adapter: &Adapter) -> Result<&'static Query, Error> {
    let grammar = (language.grammar)();
    let built = held(language).map(|cell| {
        cell.get_or_init(|| Query::new(&grammar, adapter.patterns).map_err(|why| why.to_string()))
    });
    match built {
        Some(Ok(query)) => Ok(query),
        Some(Err(why)) => Err(refused(language, why)),
        None => Err(refused(language, "it is not in the language registry")),
    }
}

fn refused(language: &Language, why: &str) -> Error {
    Error(format!(
        "the {} structural query could not be compiled: {why}",
        language.name
    ))
}

fn harvest(
    file: &ParsedFile,
    adapter: &'static Adapter,
    names: &mut Names,
) -> Result<FileFacts, Error> {
    let query = compiled(file.language, adapter)?;
    let captures = query.capture_names();
    let mut reading = Reading::new(file, adapter, names);
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, file.root(), file.bytes());
    while let Some(matched) = matches.next() {
        for capture in matched.captures() {
            reading.take(captures[capture.index as usize], capture.node);
        }
    }
    Ok(reading.finish(file))
}

/// One file being read: what the query found so far, and what the reference pass must leave
/// alone because a declaration or an import already claimed it.
struct Reading<'a, 'b> {
    language: &'static Language,
    adapter: &'static Adapter,
    path: &'a str,
    source: &'a [u8],
    lines: Vec<&'a str>,
    declarations: Vec<Declaration>,
    imports: Vec<Import>,
    modules: Vec<ModuleDecl>,
    exports: Vec<Export>,
    crates: Vec<ExternCrate>,
    declared: BTreeSet<usize>,
    claimed: Vec<(usize, usize)>,
    names: &'b mut Names,
}

impl<'a, 'b> Reading<'a, 'b> {
    fn new(
        file: &ParsedFile<'a>,
        adapter: &'static Adapter,
        names: &'b mut Names,
    ) -> Reading<'a, 'b> {
        Reading {
            language: file.language,
            adapter,
            path: file.path,
            source: file.source.as_bytes(),
            lines: file.source.lines().collect(),
            declarations: Vec::new(),
            imports: Vec::new(),
            modules: Vec::new(),
            exports: Vec::new(),
            crates: Vec::new(),
            declared: BTreeSet::new(),
            claimed: Vec::new(),
            names,
        }
    }

    fn take(&mut self, capture: &str, node: Node) {
        match capture {
            IMPORT => self.import(node),
            MODULE => self.module(node),
            EXPORT => self.export(node),
            CRATE => self.extern_crate(node),
            _ => self.declaration(capture, node),
        }
    }

    /// One statement that exposes names, where the adapter says the node does so on its own.
    fn export(&mut self, node: Node) {
        let Some(found) = (self.adapter.exported)(node, self.source, self.path) else {
            return;
        };
        self.exports.push(Export {
            line: self.row(node),
            text: self.text(node),
            nesting: (self.adapter.nesting)(node, self.source),
            source: found.source,
            type_only: found.type_only,
            supported: found.supported,
            leaves: found.leaves,
            contract: found.contract,
        });
    }

    fn extern_crate(&mut self, node: Node) {
        let Some(name) = node.child_by_field_name("name") else {
            return;
        };
        let name = text_of(name, self.source);
        self.crates.push(ExternCrate {
            nesting: (self.adapter.nesting)(node, self.source),
            alias: node
                .child_by_field_name("alias")
                .map_or_else(|| name.clone(), |alias| text_of(alias, self.source)),
            name,
        });
    }

    fn import(&mut self, node: Node) {
        let found = (self.adapter.imported)(node, self.source);
        self.claimed.push((node.start_byte(), node.end_byte()));
        self.imports.push(Import {
            line: self.row(node),
            text: self.text(node),
            nesting: (self.adapter.nesting)(node, self.source),
            in_function: inside_a_function(node, self.language),
            module: found.module,
            names: found.names,
            paths: found.paths,
        });
    }

    /// One module declaration. An external one is claimed whole, as it always was, and an inline
    /// one claims nothing, so the uses its body holds stay references.
    fn module(&mut self, node: Node) {
        let Some(name) = node.child_by_field_name("name") else {
            return;
        };
        let inline = node.child_by_field_name("body").is_some();
        if !inline {
            self.claimed.push((node.start_byte(), node.end_byte()));
        }
        self.modules.push(ModuleDecl {
            line: self.row(node),
            text: self.text(node),
            name: text_of(name, self.source),
            nesting: (self.adapter.nesting)(node, self.source),
            inline,
            path: (!inline)
                .then(|| (self.adapter.remapped)(node, self.source))
                .flatten(),
            visibility: (self.adapter.visibility)(node, self.source),
        });
    }

    fn declaration(&mut self, capture: &str, node: Node) {
        let Some(name) = node.child_by_field_name("name") else {
            return;
        };
        let bindings = (self.adapter.destructured)(name);
        self.declared.insert(name.start_byte());
        self.declared.extend(bindings.iter().map(Node::start_byte));
        if inside_a_function(node, self.language) {
            return;
        }
        let name = text_of(name, self.source);
        let entry_point = self.adapter.entry_points.contains(&name.as_str());
        self.declarations.push(Declaration {
            name,
            bindings: bindings
                .into_iter()
                .map(|binding| text_of(binding, self.source))
                .collect(),
            kind: self.kind(capture, node),
            line: self.row(node),
            end: node.end_position().row as u64 + 1,
            text: self.text(node),
            externally_visible: (self.adapter.visible)(node),
            entry_point,
            nesting: (self.adapter.nesting)(node, self.source),
            associated: above(node, self.adapter.methods_in).is_some(),
            visibility: (self.adapter.visibility)(node, self.source),
            exported_as: (self.adapter.exported_as)(node, self.source),
            owner: (self.adapter.owner)(node, self.source),
            signature: (self.adapter.contract)(node, self.source),
        });
    }

    /// What the capture says the declaration is, and a function a type body holds is a method
    /// in a language that writes its methods that way.
    fn kind(&self, capture: &str, node: Node) -> DeclarationKind {
        match capture {
            METHOD => DeclarationKind::Method,
            TYPE => DeclarationKind::Type,
            CONSTANT => DeclarationKind::Constant,
            VARIABLE => DeclarationKind::Variable,
            _ if above(node, self.adapter.methods_in).is_some() => DeclarationKind::Method,
            _ => DeclarationKind::Function,
        }
    }

    fn row(&self, node: Node) -> u64 {
        node.start_position().row as u64 + 1
    }

    fn text(&self, node: Node) -> String {
        line_at(&self.lines, node.start_position().row)
    }

    /// Every use of a name the declarations and the imports did not already claim, every name a
    /// string calls by its text, and every qualified path outside them, from one walk. An import
    /// binding is not a reference to what it binds, so the whole import is stepped over, and
    /// neither is a declaration's name or a name its destructuring pattern binds. The head of a
    /// C-style `for` is a declaration. A name any other binding site writes — a parameter, a
    /// `for…in` or `for…of` head, a `catch` clause, an assignment, a field, a Rust `let` — is
    /// kept, because no adapter states those binding sites and keeping it errs toward
    /// "referenced", unless a pattern writes it as a shorthand such as `{ name }`, which is no
    /// identifier the adapter lists.
    fn uses(&mut self, root: Node) -> (Vec<Reference>, Vec<QualifiedPath>) {
        let mut references = Vec::new();
        let mut paths = Vec::new();
        walk(root, &mut |node| {
            if self.adapter.identifiers.contains(&node.kind())
                && !self.declared.contains(&node.start_byte())
                && !self.claimed(node)
            {
                references.push(Reference {
                    name: self
                        .names
                        .intern(node.utf8_text(self.source).unwrap_or_default()),
                    line: self.row(node),
                });
            } else if let Some(segments) =
                (self.adapter.qualified)(node, self.source).filter(|_| !self.claimed(node))
            {
                if let Some(path) = self.resolvable(node, &segments) {
                    paths.push(path);
                }
            } else {
                for name in (self.adapter.quoted)(node, self.source) {
                    references.push(Reference {
                        name: self.names.intern(&name),
                        line: self.row(node),
                    });
                }
            }
        });
        (references, paths)
    }

    /// The qualified path a node writes where it starts at a rooted segment or at a module the
    /// file declares at the path's own nesting. The nesting is read only for a path whose first
    /// segment could qualify.
    fn resolvable(&self, node: Node, segments: &[String]) -> Option<QualifiedPath> {
        let first = segments.first()?.as_str();
        let rooted = self.adapter.rooted.contains(&first);
        if !rooted && !self.modules.iter().any(|module| module.name == first) {
            return None;
        }
        let nesting = (self.adapter.nesting)(node, self.source);
        let declared = || {
            self.modules
                .iter()
                .any(|module| module.name == first && module.nesting == nesting)
        };
        (rooted || declared()).then(|| QualifiedPath {
            line: self.row(node),
            nesting,
            path: segments.join("::"),
        })
    }

    fn claimed(&self, node: Node) -> bool {
        self.claimed
            .iter()
            .any(|(from, to)| node.start_byte() >= *from && node.end_byte() <= *to)
    }

    fn finish(mut self, file: &ParsedFile) -> FileFacts {
        let tests = convention::tests_in(file);
        for declaration in &mut self.declarations {
            declaration.entry_point |= tests
                .iter()
                .any(|test| test.line == declaration.line && test.text == declaration.text);
        }
        let (mut references, mut paths) = self.uses(file.root());
        paths.sort_by(|a, b| (a.line, &a.path).cmp(&(b.line, &b.path)));
        self.declarations
            .sort_by(|a, b| (a.line, &a.name, a.kind).cmp(&(b.line, &b.name, b.kind)));
        self.imports.sort_by_key(|import| import.line);
        self.modules
            .sort_by(|a, b| (a.line, &a.name).cmp(&(b.line, &b.name)));
        self.exports.sort_by_key(|export| export.line);
        references.sort_by(|a, b| (a.line, &a.name).cmp(&(b.line, &b.name)));
        FileFacts {
            file: file.path.to_string(),
            language: file.language.id,
            declarations: self.declarations,
            imports: self.imports,
            module_declarations: self.modules,
            references,
            paths,
            exports: self.exports,
            crates: self.crates,
        }
    }
}

/// One declaration as the index reports it, with the file that holds it.
#[derive(Clone, Copy)]
pub struct Declared<'a> {
    pub file: &'a str,
    pub language: LanguageId,
    pub declaration: &'a Declaration,
}

/// Every file's facts under the measured roots, indexed by logical language and name. Resolution
/// within one language is name-only: no type inference and no import-aware lookup, so a
/// reference resolves to every declaration that spells it. The facts are the tree's one
/// extraction; the index over them belongs to the measurement that selected them. ADR 0035,
/// spec 8.4.
pub struct SourceIndex {
    files: Vec<Rc<FileFacts>>,
    names: HashMap<LanguageId, HashMap<Name, Sites>>,
}

/// Where one name of one language is declared and referenced, each list in file and line order.
/// A lookup reads one entry, so the order of the map it sits in reaches no consumer.
#[derive(Default)]
struct Sites {
    declarations: Vec<(usize, usize)>,
    references: Vec<(usize, u64)>,
}

impl SourceIndex {
    pub fn of(mut files: Vec<Rc<FileFacts>>) -> SourceIndex {
        files.sort_by(|a, b| a.file.cmp(&b.file));
        let mut names: HashMap<LanguageId, HashMap<Name, Sites>> = HashMap::new();
        for (at, file) in files.iter().enumerate() {
            let named = names.entry(file.language).or_default();
            for reference in &file.references {
                record_name(named, reference.name.clone(), |sites| {
                    sites.references.push((at, reference.line));
                });
            }
        }
        for (at, file) in files.iter().enumerate() {
            let named = names.entry(file.language).or_default();
            for (which, declaration) in file.declarations.iter().enumerate() {
                for name in declaration.names() {
                    record_text(named, name, |sites| {
                        sites.declarations.push((at, which));
                    });
                }
            }
        }
        for sites in names.values_mut().flat_map(HashMap::values_mut) {
            sites.references.sort_unstable();
            sites.references.dedup();
        }
        SourceIndex { files, names }
    }

    pub fn files(&self) -> &[Rc<FileFacts>] {
        &self.files
    }

    /// The facts of the file at this path, and `None` when the index holds no such file.
    pub fn file(&self, path: &str) -> Option<&FileFacts> {
        let at = self
            .files
            .binary_search_by(|held| held.file.as_str().cmp(path))
            .ok()?;
        self.files.get(at).map(Rc::as_ref)
    }

    /// The sites of one name in one logical language, and `None` when no file of that language
    /// declares or references it.
    fn sites(&self, language: LanguageId, name: &str) -> Option<&Sites> {
        self.names.get(&language)?.get(name)
    }

    /// References with this name and logical language, in file and line order, each site once.
    pub fn references(
        &self,
        language: LanguageId,
        name: &str,
    ) -> impl Iterator<Item = ReferenceSite<'_>> {
        self.sites(language, name)
            .into_iter()
            .flat_map(|sites| &sites.references)
            .map(|(at, line)| ReferenceSite {
                file: &self.files[*at].file,
                line: *line,
            })
    }

    /// Every declaration of this name and logical language, in file and line order. A
    /// destructuring declaration is listed under each name it binds, and its `name` is still the
    /// pattern text.
    pub fn declarations(
        &self,
        language: LanguageId,
        name: &str,
    ) -> impl Iterator<Item = Declared<'_>> {
        self.sites(language, name)
            .into_iter()
            .flat_map(|sites| &sites.declarations)
            .map(|(at, which)| {
                let file = &self.files[*at];
                Declared {
                    file: &file.file,
                    language: file.language,
                    declaration: &file.declarations[*which],
                }
            })
    }
}

/// One site recorded under its name, and the name copied only the first time the index meets it.
fn record_text(named: &mut HashMap<Name, Sites>, text: &str, into: impl FnOnce(&mut Sites)) {
    if let Some(sites) = named.get_mut(text) {
        into(sites);
        return;
    }
    let mut sites = Sites::default();
    into(&mut sites);
    named.insert(Name::new(text), sites);
}

fn record_name(named: &mut HashMap<Name, Sites>, name: Name, into: impl FnOnce(&mut Sites)) {
    if let Some(sites) = named.get_mut(&name) {
        into(sites);
        return;
    }
    let mut sites = Sites::default();
    into(&mut sites);
    named.insert(name, sites);
}

/// Whether a function body holds this node, which is what makes a declaration a local of that
/// function rather than a declaration of the file.
fn inside_a_function(node: Node, language: &Language) -> bool {
    above(node, language.functions).is_some()
}

/// The nearest node above this one whose kind is one of these.
pub(crate) fn above<'a>(node: Node<'a>, kinds: &[&str]) -> Option<Node<'a>> {
    let mut holder = node.parent();
    while let Some(found) = holder {
        if kinds.contains(&found.kind()) {
            return Some(found);
        }
        holder = found.parent();
    }
    None
}

/// The text one node covers, and the empty string when it is not valid UTF-8.
pub(crate) fn text_of(node: Node, source: &[u8]) -> String {
    node.utf8_text(source).unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUST: &str = r#"
use std::fmt::Write as Scribe;
use crate::pay::{Refund, refund};

mod ledger;

mod held {
    pub fn inside() -> u64 { 1 }
}

pub struct Charge {
    amount: u64,
}

enum Outcome {
    Paid,
}

pub trait Settle {
    fn settle(&self) -> u64;
}

type Money = u64;

pub const CEILING: u64 = 10;
static NAME: &str = "klin";

pub fn charge(at: u64) -> u64 {
    fn local(of: u64) -> u64 { of }
    local(at)
}

impl Charge {
    pub fn total(&self) -> u64 {
        self.amount
    }
}
"#;

    const TYPESCRIPT: &str = r#"
import { refund } from "./pay";
import Ledger from "../ledger";
export { total } from "./total";

export class Charge {
  total(): number {
    return 1;
  }
}

export interface Settle {
  settle(): number;
}

export type Money = number;

export enum Outcome {
  Paid,
}

export const CEILING = 10;
let name = "klin";

export function charge(at: number): number {
  const inside = at;
  return inside;
}
"#;

    fn measured_facts(path: &str, source: &str) -> Rc<FileFacts> {
        match of(path, source) {
            Ok(Outcome::Facts(found)) => found,
            _ => panic!("{path} was not measured"),
        }
    }

    fn declaration<'a>(facts: &'a FileFacts, name: &str) -> &'a Declaration {
        match facts.declarations.iter().find(|held| held.name == name) {
            Some(found) => found,
            None => panic!("{name} was not declared in {}", facts.file),
        }
    }

    fn kinds(facts: &FileFacts) -> Vec<(&str, DeclarationKind)> {
        facts
            .declarations
            .iter()
            .map(|held| (held.name.as_str(), held.kind))
            .collect()
    }

    fn referenced(facts: &FileFacts, name: &str) -> bool {
        facts.references.iter().any(|held| held.name == name)
    }

    #[test]
    fn rust_declares_every_kind_the_first_version_names() {
        let facts = measured_facts("src/pay.rs", RUST);
        assert_eq!(facts.language, LanguageId::Rust);
        let held = kinds(&facts);
        for wanted in [
            ("Charge", DeclarationKind::Type),
            ("Outcome", DeclarationKind::Type),
            ("Settle", DeclarationKind::Type),
            ("Money", DeclarationKind::Type),
            ("CEILING", DeclarationKind::Constant),
            ("NAME", DeclarationKind::Constant),
            ("charge", DeclarationKind::Function),
            ("inside", DeclarationKind::Function),
            ("total", DeclarationKind::Method),
            ("settle", DeclarationKind::Method),
        ] {
            assert!(held.contains(&wanted), "{wanted:?} missing from {held:?}");
        }
    }

    #[test]
    fn a_declaration_inside_a_function_body_is_a_local_and_not_a_declaration() {
        let facts = measured_facts("src/pay.rs", RUST);
        let names: Vec<&str> = facts
            .declarations
            .iter()
            .map(|held| held.name.as_str())
            .collect();
        assert!(!names.contains(&"local"), "{names:?}");
    }

    #[test]
    fn rust_says_what_leaves_the_file_and_what_does_not() {
        let facts = measured_facts("src/pay.rs", RUST);
        for name in ["Charge", "Settle", "CEILING", "charge", "total", "settle"] {
            assert!(declaration(&facts, name).externally_visible, "{name}");
        }
        for name in ["Outcome", "Money", "NAME"] {
            assert!(!declaration(&facts, name).externally_visible, "{name}");
        }
    }

    /// What #52 tells a reference inside a declaration from one outside it by.
    #[test]
    fn a_declaration_spans_from_its_line_to_its_end() {
        let source = "pub const CEILING: u64 = 10;\npub fn charge() -> u64 {\n    1\n}\n";
        let facts = measured_facts("src/pay.rs", source);
        let ceiling = declaration(&facts, "CEILING");
        assert_eq!((ceiling.line, ceiling.end), (1, 1));
        let charge = declaration(&facts, "charge");
        assert_eq!((charge.line, charge.end), (2, 4));
    }

    #[test]
    fn a_rust_module_declaration_keeps_the_file_an_attribute_sends_it_to() {
        let source = "#[path = \"other.rs\"]\nmod moved;\n\nmod plain;\n";
        let facts = measured_facts("src/pay.rs", source);
        let held: Vec<(&str, Option<&str>)> = facts
            .module_declarations
            .iter()
            .map(|found| (found.name.as_str(), found.path.as_deref()))
            .collect();
        assert_eq!(held, vec![("moved", Some("other.rs")), ("plain", None)]);
    }

    #[test]
    fn the_structural_languages_are_named_once_each_and_tsx_is_not_one() {
        assert_eq!(
            languages(),
            vec![
                ("rust", LanguageId::Rust),
                ("typescript", LanguageId::TypeScript)
            ]
        );
    }

    #[test]
    fn a_declaration_is_named_by_the_line_it_is_written_on() {
        let facts = measured_facts("src/pay.rs", RUST);
        let charge = declaration(&facts, "charge");
        assert_eq!(charge.text, "pub fn charge(at: u64) -> u64 {");
        assert_eq!(charge.line, row_of(RUST, "pub fn charge"));
    }

    fn row_of(source: &str, starts: &str) -> u64 {
        match source.lines().position(|line| line.starts_with(starts)) {
            Some(at) => at as u64 + 1,
            None => panic!("the fixture has no line starting {starts}"),
        }
    }

    #[test]
    fn rust_imports_keep_the_path_as_written_and_the_names_they_bind() {
        let facts = measured_facts("src/pay.rs", RUST);
        let paths: Vec<Option<&str>> = facts
            .imports
            .iter()
            .map(|held| held.module.as_deref())
            .collect();
        assert_eq!(
            paths,
            vec![
                Some("std::fmt::Write as Scribe"),
                Some("crate::pay::{Refund, refund}")
            ]
        );
        assert_eq!(facts.imports[0].names, vec!["Scribe".to_string()]);
        assert_eq!(
            facts.imports[1].names,
            vec!["Refund".to_string(), "refund".to_string()]
        );
        assert_eq!(facts.imports[0].text, "use std::fmt::Write as Scribe;");
    }

    #[test]
    fn rust_tells_an_external_module_declaration_from_an_inline_one() {
        let facts = measured_facts("src/pay.rs", RUST);
        let named: Vec<(&str, &str, bool)> = facts
            .module_declarations
            .iter()
            .map(|held| (held.name.as_str(), held.text.as_str(), held.inline))
            .collect();
        assert_eq!(
            named,
            vec![
                ("ledger", "mod ledger;", false),
                ("held", "mod held {", true)
            ]
        );
    }

    #[test]
    fn a_rust_module_declaration_names_the_inline_modules_that_hold_it() {
        let source = "mod plain;\nmod held {\n    #[path = \"x.rs\"]\n    mod moved;\n    mod nested {}\n}\n";
        let facts = measured_facts("src/pay.rs", source);
        let held: Vec<(&str, bool, Vec<&str>, Option<&str>)> = facts
            .module_declarations
            .iter()
            .map(|found| {
                (
                    found.name.as_str(),
                    found.inline,
                    found.nesting.iter().map(String::as_str).collect(),
                    found.path.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            held,
            vec![
                ("plain", false, vec![], None),
                ("held", true, vec![], None),
                ("moved", false, vec!["held"], Some("x.rs")),
                ("nested", true, vec!["held"], None),
            ]
        );
    }

    #[test]
    fn a_rust_import_keeps_its_nesting_and_every_path_its_use_tree_names() {
        let source = "use crate::a::{self, b::{C, d as e}, f::*};\nuse std::fmt;\nmod outer {\n    mod inner {\n        use super::super::g;\n    }\n}\n";
        let facts = measured_facts("src/pay.rs", source);
        let held: Vec<(Vec<&str>, Vec<&str>)> = facts
            .imports
            .iter()
            .map(|found| {
                (
                    found.nesting.iter().map(String::as_str).collect(),
                    found.paths.iter().map(String::as_str).collect(),
                )
            })
            .collect();
        assert_eq!(
            held,
            vec![
                (
                    vec![],
                    vec![
                        "crate::a",
                        "crate::a::b::C",
                        "crate::a::b::d",
                        "crate::a::f::*"
                    ]
                ),
                (vec![], vec!["std::fmt"]),
                (vec!["outer", "inner"], vec!["super::super::g"]),
            ]
        );
    }

    #[test]
    fn a_rust_path_from_the_crate_or_a_module_is_kept_outside_imports_and_visibility() {
        let source = "pub(crate) fn a() -> crate::b::Kind {\n    crate::b::make::<u8>();\n    super::c::run();\n    Self::new();\n    std::process::exit(0);\n}\npub(in crate::d) struct E;\nmod inner {\n    fn f() { self::g(); }\n}\nuse crate::h::i;\n";
        let facts = measured_facts("src/pay.rs", source);
        let held: Vec<(u64, Vec<&str>, &str)> = facts
            .paths
            .iter()
            .map(|found| {
                (
                    found.line,
                    found.nesting.iter().map(String::as_str).collect(),
                    found.path.as_str(),
                )
            })
            .collect();
        assert_eq!(
            held,
            vec![
                (1, vec![], "crate::b::Kind"),
                (2, vec![], "crate::b::make"),
                (3, vec![], "super::c::run"),
                (9, vec!["inner"], "self::g"),
            ]
        );
        assert!(used(&facts, "g", 9), "a use inside an inline module");
    }

    #[test]
    fn a_typescript_file_keeps_no_nesting_no_use_tree_paths_and_no_qualified_paths() {
        let facts = measured_facts("src/pay.ts", TYPESCRIPT);
        assert!(
            facts
                .imports
                .iter()
                .all(|found| found.nesting.is_empty() && found.paths.is_empty())
        );
        assert!(facts.paths.is_empty());
    }

    #[test]
    fn a_reference_is_a_use_and_never_a_declaration_or_an_import_binding() {
        let facts = measured_facts("src/pay.rs", RUST);
        assert!(used(&facts, "amount", row_of(RUST, "        self.amount")));
        assert!(referenced(&facts, "local"), "the call to the local");
        assert!(!referenced(&facts, "Scribe"), "an import binding");
        assert!(!referenced(&facts, "CEILING"), "a declaration name");
    }

    /// The conservative direction of ADR 0035, pinned so it stays a decision. A name a binding
    /// site other than a declaration writes reads as a reference, because no adapter states those
    /// binding sites. It keeps a declaration alive that nothing uses, which makes a structural
    /// gate fail less and never more.
    #[test]
    fn a_binding_site_reads_as_a_reference_and_errs_toward_referenced() {
        let facts = measured_facts("src/pay.rs", RUST);
        assert!(used(&facts, "amount", row_of(RUST, "    amount: u64,")));
    }

    fn used(facts: &FileFacts, name: &str, line: u64) -> bool {
        facts
            .references
            .iter()
            .any(|held| held.name == name && held.line == line)
    }

    #[test]
    fn typescript_declares_every_kind_the_first_version_names() {
        let facts = measured_facts("src/pay.ts", TYPESCRIPT);
        assert_eq!(facts.language, LanguageId::TypeScript);
        let held = kinds(&facts);
        for wanted in [
            ("Charge", DeclarationKind::Type),
            ("Settle", DeclarationKind::Type),
            ("Money", DeclarationKind::Type),
            ("Outcome", DeclarationKind::Type),
            ("CEILING", DeclarationKind::Constant),
            ("name", DeclarationKind::Variable),
            ("charge", DeclarationKind::Function),
            ("total", DeclarationKind::Method),
            ("settle", DeclarationKind::Method),
        ] {
            assert!(held.contains(&wanted), "{wanted:?} missing from {held:?}");
        }
    }

    #[test]
    fn typescript_says_an_export_leaves_the_file() {
        let facts = measured_facts("src/pay.ts", TYPESCRIPT);
        assert!(declaration(&facts, "charge").externally_visible);
        assert!(declaration(&facts, "total").externally_visible);
        assert!(!declaration(&facts, "name").externally_visible);
    }

    #[test]
    fn typescript_imports_and_re_exports_keep_the_specifier_and_the_names() {
        let facts = measured_facts("src/pay.ts", TYPESCRIPT);
        let paths: Vec<Option<&str>> = facts
            .imports
            .iter()
            .map(|held| held.module.as_deref())
            .collect();
        assert_eq!(
            paths,
            vec![Some("./pay"), Some("../ledger"), Some("./total")]
        );
        assert_eq!(facts.imports[0].names, vec!["refund".to_string()]);
        assert_eq!(facts.imports[1].names, vec!["Ledger".to_string()]);
        assert_eq!(facts.imports[2].names, vec!["total".to_string()]);
        assert!(facts.module_declarations.is_empty());
    }

    #[test]
    fn a_tsx_file_is_the_typescript_language_and_not_one_of_its_own() {
        let facts = measured_facts("src/view.tsx", "export const view = () => <p>hi</p>;\n");
        assert_eq!(facts.language, LanguageId::TypeScript);
        assert_eq!(declaration(&facts, "view").kind, DeclarationKind::Constant);
    }

    #[test]
    fn declarations_carry_runtime_and_test_entry_points_as_semantic_facts() {
        let rust = measured_facts(
            "src/lib.rs",
            "fn main() {}\n#[test]\nfn works() {}\nfn helper() {}\n",
        );
        assert!(declaration(&rust, "main").entry_point);
        assert!(declaration(&rust, "works").entry_point);
        assert!(!declaration(&rust, "helper").entry_point);

        let typescript = measured_facts("src/index.ts", "function main() {}\n");
        assert!(!declaration(&typescript, "main").entry_point);
    }

    #[test]
    fn a_parser_backed_language_with_no_adapter_is_not_measured() {
        assert!(!supports(LanguageId::Python));
        match of("src/pay.py", "def charge():\n    return 1\n") {
            Ok(Outcome::Unsupported(language)) => assert_eq!(language, "Python"),
            _ => panic!("a Python file read as measured"),
        }
    }

    #[test]
    fn a_text_the_grammar_rejects_is_not_measured() {
        match of("src/pay.rs", "fn charge( {\n") {
            Ok(Outcome::Unparsed(file)) => assert_eq!(file.language, "Rust"),
            _ => panic!("a rejected text read as measured"),
        }
    }

    #[test]
    fn a_path_no_grammar_reads_is_not_measured() {
        match of("README.md", "# klin\n") {
            Ok(Outcome::Foreign) => {}
            _ => panic!("a document read as measured"),
        }
    }

    #[test]
    fn the_index_resolves_a_name_to_every_declaration_of_it_in_one_order() {
        let one = "pub fn refund() {}\npub struct Refund;\n";
        let two = "pub fn refund() {}\n";
        let index = SourceIndex::of(vec![
            measured_facts("src/two.rs", two),
            measured_facts("src/one.rs", one),
        ]);
        let rust = LanguageId::Rust;
        let found: Vec<(&str, LanguageId, u64)> = index
            .declarations(rust, "refund")
            .map(|held| (held.file, held.language, held.declaration.line))
            .collect();
        assert_eq!(
            found,
            vec![("src/one.rs", rust, 1), ("src/two.rs", rust, 1)]
        );
        assert_eq!(index.declarations(rust, "nothing").count(), 0);
        assert_eq!(index.files().len(), 2);
    }

    #[test]
    fn the_index_resolves_a_name_to_every_reference_of_it_in_one_order() {
        let index = SourceIndex::of(vec![
            measured_facts(
                "src/two.ts",
                "export const a = refund() + refund();\nrefund();\n",
            ),
            measured_facts(
                "src/one.rs",
                "fn a() { refund(); other(); }\nfn b() { refund(); }\n",
            ),
        ]);
        let found: Vec<(&str, u64)> = index
            .references(LanguageId::Rust, "refund")
            .map(|site| (site.file, site.line))
            .collect();
        assert_eq!(found, vec![("src/one.rs", 1), ("src/one.rs", 2)]);
        assert_eq!(index.references(LanguageId::Rust, "other").count(), 1);
        assert_eq!(index.references(LanguageId::Rust, "nothing").count(), 0);
    }

    fn measurement_of(files: Vec<Rc<FileFacts>>) -> Measurement {
        Measurement {
            facts: files,
            index: OnceCell::new(),
            unparsed: Vec::new(),
            unsupported: Vec::new(),
            files: Files::default(),
            cost: ExtractionCost::default(),
        }
    }

    #[test]
    fn reading_facts_builds_no_name_index_and_holds_the_order_the_index_uses() {
        let measured = measurement_of(vec![
            Rc::new(synthetic("src/one.rs", LanguageId::Rust, &["a"])),
            Rc::new(synthetic("src/two.rs", LanguageId::Rust, &["b"])),
        ]);
        let read: Vec<&str> = measured.facts().iter().map(|f| f.file.as_str()).collect();
        assert_eq!(read, vec!["src/one.rs", "src/two.rs"]);
        assert!(measured.index.get().is_none(), "facts() built an index");
        let indexed: Vec<&str> = measured
            .index()
            .files()
            .iter()
            .map(|f| f.file.as_str())
            .collect();
        assert_eq!(indexed, read);
        assert!(measured.index.get().is_some());
    }

    fn synthetic(file: &str, language: LanguageId, names: &[&str]) -> FileFacts {
        FileFacts {
            file: file.to_string(),
            language,
            declarations: names
                .iter()
                .enumerate()
                .map(|(at, name)| Declaration {
                    name: name.to_string(),
                    bindings: Box::default(),
                    kind: DeclarationKind::Function,
                    line: at as u64 + 1,
                    end: at as u64 + 1,
                    text: format!("fn {name}"),
                    externally_visible: false,
                    entry_point: false,
                    nesting: Vec::new(),
                    associated: false,
                    visibility: Visibility::Private,
                    exported_as: None,
                    owner: None,
                    signature: None,
                })
                .collect(),
            imports: Vec::new(),
            module_declarations: Vec::new(),
            paths: Vec::new(),
            exports: Vec::new(),
            crates: Vec::new(),
            references: names
                .iter()
                .enumerate()
                .map(|(at, name)| Reference {
                    name: Name::new(format!("use_{name}")),
                    line: at as u64 + 1,
                })
                .collect(),
        }
    }

    fn many_files() -> SourceIndex {
        let mut files = Vec::new();
        for at in (0..400).rev() {
            let (language, suffix) = if at % 2 == 0 {
                (LanguageId::Rust, "rs")
            } else {
                (LanguageId::TypeScript, "ts")
            };
            let own = format!("only_{at}");
            let names: Vec<&str> = if at % 100 == 7 {
                vec!["shared", &own, "shared"]
            } else {
                vec![&own]
            };
            files.push(Rc::new(synthetic(
                &format!("src/f{at:04}.{suffix}"),
                language,
                &names,
            )));
        }
        SourceIndex::of(files)
    }

    const SHARED_SITES: [(&str, u64); 8] = [
        ("src/f0007.ts", 1),
        ("src/f0007.ts", 3),
        ("src/f0107.ts", 1),
        ("src/f0107.ts", 3),
        ("src/f0207.ts", 1),
        ("src/f0207.ts", 3),
        ("src/f0307.ts", 1),
        ("src/f0307.ts", 3),
    ];

    #[test]
    fn a_declaration_lookup_over_many_files_yields_only_its_own_sites_in_file_order() {
        let index = many_files();
        let shared: Vec<(&str, LanguageId, u64)> = index
            .declarations(LanguageId::TypeScript, "shared")
            .map(|held| (held.file, held.language, held.declaration.line))
            .collect();
        let expected: Vec<(&str, LanguageId, u64)> = SHARED_SITES
            .iter()
            .map(|(file, line)| (*file, LanguageId::TypeScript, *line))
            .collect();
        assert_eq!(shared, expected);
        let one: Vec<&str> = index
            .declarations(LanguageId::Rust, "only_42")
            .map(|held| held.file)
            .collect();
        assert_eq!(one, vec!["src/f0042.rs"]);
        assert_eq!(
            index.declarations(LanguageId::Rust, "use_only_42").count(),
            0
        );
    }

    #[test]
    fn a_reference_lookup_over_many_files_yields_only_its_own_sites_in_file_order() {
        let index = many_files();
        let used: Vec<(&str, u64)> = index
            .references(LanguageId::TypeScript, "use_shared")
            .map(|site| (site.file, site.line))
            .collect();
        assert_eq!(used, SHARED_SITES.to_vec());
        let one: Vec<&str> = index
            .references(LanguageId::Rust, "use_only_42")
            .map(|site| site.file)
            .collect();
        assert_eq!(one, vec!["src/f0042.rs"]);
        assert_eq!(index.references(LanguageId::Rust, "only_42").count(), 0);
        for _ in 0..3 {
            let again: Vec<u64> = index
                .references(LanguageId::TypeScript, "use_shared")
                .map(|site| site.line)
                .collect();
            assert_eq!(again, vec![1, 3, 1, 3, 1, 3, 1, 3]);
        }
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    fn measured_facts(path: &str, source: &str) -> Rc<FileFacts> {
        match of(path, source) {
            Ok(Outcome::Facts(found)) => found,
            _ => panic!("{path} was not measured"),
        }
    }

    fn signature(facts: &FileFacts, name: &str) -> String {
        let found = facts
            .declarations
            .iter()
            .find(|held| held.name == name)
            .unwrap_or_else(|| panic!("{name} was not declared in {}", facts.file));
        found
            .signature
            .clone()
            .unwrap_or_else(|| panic!("{name} has no signature"))
    }

    fn declaration<'a>(facts: &'a FileFacts, name: &str) -> &'a Declaration {
        match facts.declarations.iter().find(|held| held.name == name) {
            Some(found) => found,
            None => panic!("{name} was not declared in {}", facts.file),
        }
    }

    const RUST: &str = r#"
pub use crate::client::{Client as C, model::*, self};
pub(crate) use inner::x;
use std::fmt;
pub mod m { pub fn f() {} }
mod hidden;
pub(crate) mod restricted;
/// doc
#[derive(Debug)]
pub struct S<T: Clone> where T: Copy { pub a: T, b: u8 }
pub struct U(pub u8, u16);
pub enum E { A, B(u8) = 3, C { x: u8 } }
pub trait Tr: Send { fn f(&self) -> u8; fn g(&self) { } type A: Copy; const N: u8; }
pub const K: u8 = 1;
pub static mut ST: &str = "x";
pub type Al<T> = Vec<T>;
pub unsafe extern "C" fn ff<T>(x: T, (a, b): (u8, u8), mut y: &mut u8) -> u8 where T: Copy { 1 }
impl<T> S<T> { pub fn new(self: Box<Self>, n: u8) -> Self { *self } pub(crate) fn p() {} fn q() {} }
impl Tr for S<u8> { fn f(&self) -> u8 { 1 } }
"#;

    #[test]
    fn rust_tells_public_from_restricted_and_private() {
        let facts = measured_facts("src/lib.rs", RUST);
        assert_eq!(declaration(&facts, "S").visibility, Visibility::Public);
        assert_eq!(declaration(&facts, "new").visibility, Visibility::Public);
        assert_eq!(declaration(&facts, "p").visibility, Visibility::Restricted);
        assert_eq!(declaration(&facts, "q").visibility, Visibility::Private);
        let modules: Vec<(&str, Visibility)> = facts
            .module_declarations
            .iter()
            .map(|held| (held.name.as_str(), held.visibility))
            .collect();
        assert_eq!(
            modules,
            vec![
                ("m", Visibility::Public),
                ("hidden", Visibility::Private),
                ("restricted", Visibility::Restricted)
            ]
        );
    }

    #[test]
    fn rust_canonical_contracts_drop_bodies_attributes_names_and_private_fields() {
        let facts = measured_facts("src/lib.rs", RUST);
        assert_eq!(
            signature(&facts, "S"),
            "struct S<T: Clone> where T: Copy { a: T, .. }"
        );
        assert_eq!(signature(&facts, "U"), "struct U(u8, _);");
        assert_eq!(
            signature(&facts, "E"),
            "enum E { A, B(u8) = 3, C { x: u8 } }"
        );
        assert_eq!(
            signature(&facts, "Tr"),
            "trait Tr: Send { fn f(&self) -> u8; fn g(&self) { .. } type A: Copy; const N: u8; }"
        );
        assert_eq!(signature(&facts, "K"), "const K: u8;");
        assert_eq!(signature(&facts, "ST"), "static mut ST: &str;");
        assert_eq!(signature(&facts, "Al"), "type Al<T> = Vec<T>;");
        assert_eq!(
            signature(&facts, "ff"),
            "unsafe extern \"C\" fn ff<T>(_: T, _: (u8, u8), _: &mut u8) -> u8 where T: Copy;"
        );
        assert_eq!(
            signature(&facts, "new"),
            "fn new(self: Box<Self>, _: u8) -> Self;"
        );
    }

    #[test]
    fn a_rust_method_names_the_type_its_inherent_impl_adds_it_to() {
        let facts = measured_facts("src/lib.rs", RUST);
        assert_eq!(declaration(&facts, "new").owner.as_deref(), Some("S"));
        let trait_impl = facts
            .declarations
            .iter()
            .filter(|held| held.name == "f")
            .map(|held| held.owner.clone())
            .collect::<Vec<_>>();
        assert_eq!(trait_impl, vec![None, None, None]);
    }

    #[test]
    fn a_rust_pub_use_exposes_every_leaf_under_its_name_and_a_restricted_use_exposes_nothing() {
        let facts = measured_facts("src/lib.rs", RUST);
        type Leaves<'a> = Vec<(&'a str, Option<&'a str>)>;
        let exports: Vec<(Option<&str>, Leaves)> = facts
            .exports
            .iter()
            .map(|held| {
                (
                    held.source.as_deref(),
                    held.leaves
                        .iter()
                        .map(|leaf| (leaf.path.as_str(), leaf.name.as_deref()))
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            exports,
            vec![(
                None,
                vec![
                    ("crate::client::Client", Some("C")),
                    ("crate::client::model::*", None),
                    ("crate::client", Some("client")),
                ]
            )]
        );
        assert_eq!(
            facts.imports[0].paths,
            vec![
                "crate::client::Client",
                "crate::client::model::*",
                "crate::client"
            ]
        );
    }

    #[test]
    fn a_rust_declaration_keeps_the_inline_modules_that_hold_it() {
        let facts = measured_facts("src/lib.rs", RUST);
        assert_eq!(declaration(&facts, "f").nesting, vec!["m".to_string()]);
        assert!(declaration(&facts, "S").nesting.is_empty());
    }

    const TYPESCRIPT: &str = r#"
export function f<T extends X>(a: A, b?: B, ...rest: C[]): R { return 1; }
export function g(a) {}
export default function h(): void {}
export { a, b as c };
export { e, f as g2 } from "./m";
export * from "./n";
export * as ns from "./o";
export type { T } from "./p";
export const x: number = 1, y = 2;
export class K<T> extends B implements I { private p: T; #q = 1; protected r?: T; static s(): void {} constructor(public a: A, b: B) {} m<U>(u: U): R { return 1 } get gg(): T { return this.p } }
export interface I2<T> extends J { a: T; m(x: number): void; readonly ro: number; }
export enum En { A, B = 2, C = "c" }
export namespace NS { export const q = 1; }
export declare function dd(x: number): number;
export = something;
export default foo;
function local() {}
"#;

    #[test]
    fn typescript_tells_an_exported_declaration_and_its_external_name() {
        let facts = measured_facts("src/index.ts", TYPESCRIPT);
        assert_eq!(declaration(&facts, "f").visibility, Visibility::Public);
        assert_eq!(declaration(&facts, "local").visibility, Visibility::Private);
        assert_eq!(declaration(&facts, "q").visibility, Visibility::Private);
        assert_eq!(declaration(&facts, "s").visibility, Visibility::Private);
        assert_eq!(declaration(&facts, "dd").visibility, Visibility::Public);
        assert_eq!(
            declaration(&facts, "h").exported_as.as_deref(),
            Some("default")
        );
        assert_eq!(declaration(&facts, "f").exported_as, None);
    }

    #[test]
    fn typescript_canonical_contracts_drop_bodies_binding_names_and_private_members() {
        let facts = measured_facts("src/index.ts", TYPESCRIPT);
        assert_eq!(
            signature(&facts, "f"),
            "function f<T extends X>(_: A, _?: B, ..._: C[]): R"
        );
        assert_eq!(signature(&facts, "g"), "function g(_: ?): ?");
        assert_eq!(signature(&facts, "x"), "const x: number");
        assert_eq!(signature(&facts, "y"), "const y: ?");
        assert_eq!(
            signature(&facts, "K"),
            "class K<T> extends B implements I { constructor(public a: A, _: B); get gg(): T; m<U>(_: U): R; protected r?: T; static s(): void }"
        );
        assert_eq!(
            signature(&facts, "I2"),
            "interface I2<T> extends J { a: T; m(_: number): void; readonly ro: number }"
        );
        assert_eq!(signature(&facts, "En"), "enum En { A, B = 2, C = \"c\" }");
        assert_eq!(signature(&facts, "dd"), "function dd(_: number): number");
        assert!(declaration(&facts, "m").signature.is_none(), "a member");
    }

    #[test]
    fn typescript_exports_keep_their_clause_source_and_type_only_flag() {
        let facts = measured_facts("src/index.ts", TYPESCRIPT);
        type Leaves<'a> = Vec<(&'a str, Option<&'a str>)>;
        let exports: Vec<(Option<&str>, bool, bool, Leaves)> = facts
            .exports
            .iter()
            .map(|held| {
                (
                    held.source.as_deref(),
                    held.type_only,
                    held.supported,
                    held.leaves
                        .iter()
                        .map(|leaf| (leaf.path.as_str(), leaf.name.as_deref()))
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            exports,
            vec![
                (None, false, true, vec![("a", Some("a")), ("b", Some("c"))]),
                (
                    Some("./m"),
                    false,
                    true,
                    vec![("e", Some("e")), ("f", Some("g2"))]
                ),
                (Some("./n"), false, true, vec![("*", None)]),
                (Some("./o"), false, true, vec![("*", Some("ns"))]),
                (Some("./p"), true, true, vec![("T", Some("T"))]),
                (None, false, true, vec![("NS", Some("NS"))]),
                (None, false, false, vec![]),
                (None, false, true, vec![("foo", Some("default"))]),
            ]
        );
    }
}
