//! The semantic facts a structural check reads, and the one index it resolves names through.
//! A consumer here never sees a Tree-sitter node or a node kind: it sees declarations,
//! imports, module declarations and references, and a file it did not measure says so. Rust
//! and TypeScript are the structural languages of V1, and TSX is TypeScript. ADR 0035.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::ops::Add;
use std::path::Path;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

use crate::config::{Config, Error};
use crate::coverage::Files;
use crate::files::{self, Found};
use crate::project::Tree;
use crate::syntax::convention;
use crate::syntax::{
    LANGUAGES, Language, LanguageId, Parsed, ParsedFile, Unparsed, line_at, parse, walk,
};

mod rust;
mod typescript;

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

pub struct Declaration {
    pub name: String,
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
}

/// One import, holding the specifier as it was written. Resolving it to a file is #50's work.
pub struct Import {
    pub line: u64,
    pub text: String,
    pub module: Option<String>,
    pub names: Vec<String>,
}

/// One external module declaration, such as Rust's `mod foo;`. A language without that syntax
/// declares none.
pub struct ModuleDecl {
    pub line: u64,
    pub text: String,
    pub name: String,
    /// The file the declaration names instead of its own name, where the language can say so.
    /// Rust writes it `#[path = "other.rs"]`. Resolving either to a file is #50's work.
    pub path: Option<String>,
}

pub struct Reference {
    pub name: String,
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

/// One structural measurement over a discovered file set. Consumers receive the semantic index
/// and explicit coverage outcomes; none of them parses files or reconstructs capability gaps.
pub struct Measurement {
    pub index: SourceIndex,
    pub unparsed: Vec<Unparsed>,
    pub unsupported: Vec<Unsupported>,
    pub files: Files,
    pub cost: ExtractionCost,
}

/// Every outcome one tree's files came to, each file read, parsed and extracted on the first
/// request and held for the life of the tree, which is one run. Every structural check a run
/// selects reads the one extraction of a file, and still selects its own files and builds its
/// own index from them, so a file one check leaves out never resolves a name for it because
/// another check read that file. ADR 0038.
#[derive(Default)]
pub struct Extracted(RefCell<HashMap<String, Outcome>>);

impl Extracted {
    fn outcome(
        &self,
        path: &Path,
        file: &str,
        cost: &mut ExtractionCost,
    ) -> Result<Outcome, Error> {
        if let Some(held) = self.0.borrow().get(file) {
            cost.shared += 1;
            return Ok(held.clone());
        }
        let started = Instant::now();
        let bytes = std::fs::read(path).map_err(|why| Error::unreadable(path, why))?;
        let outcome = of(file, &String::from_utf8_lossy(&bytes))?;
        cost.extracted += 1;
        cost.time += started.elapsed();
        self.0
            .borrow_mut()
            .insert(file.to_string(), outcome.clone());
        Ok(outcome)
    }
}

/// What one measurement's files cost: how many it read, parsed and extracted itself, how many
/// an earlier measurement of the same tree had already extracted, and the time the first kind
/// took. A gate records the sum over its two trees. Spec 11.2, 13.
#[derive(Default, Clone, Copy)]
pub struct ExtractionCost {
    pub extracted: usize,
    pub shared: usize,
    pub time: Duration,
}

impl Add for ExtractionCost {
    type Output = ExtractionCost;

    fn add(self, other: ExtractionCost) -> ExtractionCost {
        ExtractionCost {
            extracted: self.extracted + other.extracted,
            shared: self.shared + other.shared,
            time: self.time + other.time,
        }
    }
}

/// What structural analysis makes of one path.
pub fn of(path: &str, source: &str) -> Result<Outcome, Error> {
    match parse(path, source)? {
        None => Ok(Outcome::Foreign),
        Some(Parsed::Rejected(file)) => Ok(Outcome::Unparsed(file)),
        Some(Parsed::Read(file)) => measured(&file),
    }
}

/// The found files of one tree measured, each through the tree's one extraction of it.
pub fn measure(found: Found, tree: &Tree) -> Result<Measurement, Error> {
    let repo_root = tree.root();
    let mut facts = Vec::new();
    let mut unparsed = Vec::new();
    let mut unsupported = Vec::new();
    let mut measured = Vec::new();
    let mut cost = ExtractionCost::default();
    for path in found.kept {
        let file = files::relative(&path, repo_root);
        match tree.extracted().outcome(&path, &file, &mut cost)? {
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
    let not_measured = unsupported.iter().map(|file| file.file.clone()).collect();
    Ok(Measurement {
        index: SourceIndex::of(facts),
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

fn measured(file: &ParsedFile) -> Result<Outcome, Error> {
    Ok(match facts(file)? {
        Some(found) => Outcome::Facts(Rc::new(found)),
        None => Outcome::Unsupported(file.language.name),
    })
}

/// The facts one parsed file comes to, and `None` when no adapter reads its language.
pub fn facts(file: &ParsedFile) -> Result<Option<FileFacts>, Error> {
    let Some(adapter) = adapter(file.language.id) else {
        return Ok(None);
    };
    harvest(file, adapter).map(Some)
}

/// The one structural registry. A new language is an adapter file and one arm here.
fn adapter(id: LanguageId) -> Option<&'static Adapter> {
    match id {
        LanguageId::Rust => Some(&rust::ADAPTER),
        LanguageId::TypeScript => Some(&typescript::ADAPTER),
        _ => None,
    }
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
}

/// What one import states, before the shared reader puts it at a line. The specifier is kept
/// as it was written, because resolving it to a file is #50's work.
pub(crate) struct Imported {
    pub module: Option<String>,
    pub names: Vec<String>,
}

const METHOD: &str = "method";
const TYPE: &str = "type";
const CONSTANT: &str = "constant";
const VARIABLE: &str = "variable";
const IMPORT: &str = "import";
const MODULE: &str = "module";

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

fn harvest(file: &ParsedFile, adapter: &'static Adapter) -> Result<FileFacts, Error> {
    let query = compiled(file.language, adapter)?;
    let names = query.capture_names();
    let mut reading = Reading::new(file, adapter);
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, file.root(), file.bytes());
    while let Some(matched) = matches.next() {
        for capture in matched.captures() {
            reading.take(names[capture.index as usize], capture.node);
        }
    }
    Ok(reading.finish(file))
}

/// One file being read: what the query found so far, and what the reference pass must leave
/// alone because a declaration or an import already claimed it.
struct Reading<'a> {
    language: &'static Language,
    adapter: &'static Adapter,
    source: &'a [u8],
    lines: Vec<&'a str>,
    declarations: Vec<Declaration>,
    imports: Vec<Import>,
    modules: Vec<ModuleDecl>,
    declared: BTreeSet<usize>,
    claimed: Vec<(usize, usize)>,
}

impl<'a> Reading<'a> {
    fn new(file: &ParsedFile<'a>, adapter: &'static Adapter) -> Reading<'a> {
        Reading {
            language: file.language,
            adapter,
            source: file.source.as_bytes(),
            lines: file.source.lines().collect(),
            declarations: Vec::new(),
            imports: Vec::new(),
            modules: Vec::new(),
            declared: BTreeSet::new(),
            claimed: Vec::new(),
        }
    }

    fn take(&mut self, capture: &str, node: Node) {
        match capture {
            IMPORT => self.import(node),
            MODULE => self.module(node),
            _ => self.declaration(capture, node),
        }
    }

    fn import(&mut self, node: Node) {
        let found = (self.adapter.imported)(node, self.source);
        self.claimed.push((node.start_byte(), node.end_byte()));
        self.imports.push(Import {
            line: self.row(node),
            text: self.text(node),
            module: found.module,
            names: found.names,
        });
    }

    fn module(&mut self, node: Node) {
        let Some(name) = node.child_by_field_name("name") else {
            return;
        };
        self.claimed.push((node.start_byte(), node.end_byte()));
        self.modules.push(ModuleDecl {
            line: self.row(node),
            text: self.text(node),
            name: text_of(name, self.source),
            path: (self.adapter.remapped)(node, self.source),
        });
    }

    fn declaration(&mut self, capture: &str, node: Node) {
        let Some(name) = node.child_by_field_name("name") else {
            return;
        };
        self.declared.insert(name.start_byte());
        if inside_a_function(node, self.language) {
            return;
        }
        let name = text_of(name, self.source);
        let entry_point = self.adapter.entry_points.contains(&name.as_str());
        self.declarations.push(Declaration {
            name,
            kind: self.kind(capture, node),
            line: self.row(node),
            end: node.end_position().row as u64 + 1,
            text: self.text(node),
            externally_visible: (self.adapter.visible)(node),
            entry_point,
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

    /// Every use of a name the declarations and the imports did not already claim. An import
    /// binding is not a reference to what it binds, so the whole import is stepped over. A name
    /// a binding site writes — a parameter, a `let`, a field — is kept, because no adapter
    /// states its language's binding sites in V1 and keeping it errs toward "referenced".
    fn references(&self, root: Node) -> Vec<Reference> {
        let mut out = Vec::new();
        let claimed = |node: Node| {
            self.claimed
                .iter()
                .any(|(from, to)| node.start_byte() >= *from && node.end_byte() <= *to)
        };
        walk(root, &mut |node| {
            if self.adapter.identifiers.contains(&node.kind())
                && !self.declared.contains(&node.start_byte())
                && !claimed(node)
            {
                out.push(Reference {
                    name: text_of(node, self.source),
                    line: self.row(node),
                });
            }
        });
        out
    }

    fn finish(mut self, file: &ParsedFile) -> FileFacts {
        let tests = convention::tests_in(file);
        for declaration in &mut self.declarations {
            declaration.entry_point |= tests
                .iter()
                .any(|test| test.line == declaration.line && test.text == declaration.text);
        }
        let mut references = self.references(file.root());
        self.declarations
            .sort_by(|a, b| (a.line, &a.name, a.kind).cmp(&(b.line, &b.name, b.kind)));
        self.imports.sort_by_key(|import| import.line);
        self.modules
            .sort_by(|a, b| (a.line, &a.name).cmp(&(b.line, &b.name)));
        references.sort_by(|a, b| (a.line, &a.name).cmp(&(b.line, &b.name)));
        FileFacts {
            file: file.path.to_string(),
            language: file.language.id,
            declarations: self.declarations,
            imports: self.imports,
            module_declarations: self.modules,
            references,
        }
    }
}

/// One declaration as the index reports it, with the file that holds it.
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
    names: HashMap<LanguageId, HashMap<String, Sites>>,
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
        let mut names: HashMap<LanguageId, HashMap<String, Sites>> = HashMap::new();
        for (at, file) in files.iter().enumerate() {
            let named = names.entry(file.language).or_default();
            for (which, declaration) in file.declarations.iter().enumerate() {
                record(named, &declaration.name, |sites| {
                    sites.declarations.push((at, which));
                });
            }
            for reference in &file.references {
                record(named, &reference.name, |sites| {
                    sites.references.push((at, reference.line));
                });
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

    /// Every declaration of this name and logical language, in file and line order.
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
fn record(named: &mut HashMap<String, Sites>, name: &str, into: impl FnOnce(&mut Sites)) {
    match named.get_mut(name) {
        Some(sites) => into(sites),
        None => {
            let mut sites = Sites::default();
            into(&mut sites);
            named.insert(name.to_string(), sites);
        }
    }
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
    fn rust_keeps_an_external_module_declaration_and_leaves_an_inline_one() {
        let facts = measured_facts("src/pay.rs", RUST);
        let named: Vec<(&str, &str)> = facts
            .module_declarations
            .iter()
            .map(|held| (held.name.as_str(), held.text.as_str()))
            .collect();
        assert_eq!(named, vec![("ledger", "mod ledger;")]);
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
    /// site writes reads as a reference, because no adapter states its language's binding sites
    /// in V1. It keeps a declaration alive that nothing uses, which makes a structural gate
    /// fail less and never more.
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

    fn synthetic(file: &str, language: LanguageId, names: &[&str]) -> FileFacts {
        FileFacts {
            file: file.to_string(),
            language,
            declarations: names
                .iter()
                .enumerate()
                .map(|(at, name)| Declaration {
                    name: name.to_string(),
                    kind: DeclarationKind::Function,
                    line: at as u64 + 1,
                    end: at as u64 + 1,
                    text: format!("fn {name}"),
                    externally_visible: false,
                    entry_point: false,
                })
                .collect(),
            imports: Vec::new(),
            module_declarations: Vec::new(),
            references: names
                .iter()
                .enumerate()
                .map(|(at, name)| Reference {
                    name: format!("use_{name}"),
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
