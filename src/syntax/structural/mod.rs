//! The semantic facts a structural check reads, and the one index it resolves names through.
//! A consumer here never sees a Tree-sitter node or a node kind: it sees declarations,
//! imports, module declarations and references, and a file it did not measure says so. Rust
//! and TypeScript are the structural languages of V1, and TSX is TypeScript. ADR 0035.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

use crate::config::Error;
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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
pub enum Outcome {
    Facts(FileFacts),
    /// A grammar read the file and no structural adapter reads its language.
    Unsupported(&'static str),
    /// The grammar rejected the text.
    Unparsed(Unparsed),
    /// No grammar here reads the path at all.
    Foreign,
}

/// What structural analysis makes of one path.
pub fn of(path: &str, source: &str) -> Result<Outcome, Error> {
    match parse(path, source)? {
        None => Ok(Outcome::Foreign),
        Some(Parsed::Rejected(file)) => Ok(Outcome::Unparsed(file)),
        Some(Parsed::Read(file)) => measured(&file),
    }
}

fn measured(file: &ParsedFile) -> Result<Outcome, Error> {
    Ok(match facts(file)? {
        Some(found) => Outcome::Facts(found),
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

/// Whether this declaration is the language's ordinary executable entry point, which a
/// dead-symbol consumer leaves out by default.
pub fn is_default_entry_point(file: &FileFacts, declaration: &Declaration) -> bool {
    matches!(
        (file.language, declaration.name.as_str()),
        (LanguageId::Rust, "main")
    )
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

/// What one language adapter states. A query names the node kinds that declare something, and
/// the capture name says what kind of thing; everything else is the handful of judgments a
/// query cannot make.
pub(crate) struct Adapter {
    pub patterns: &'static str,
    /// The node kinds that are a use of a name.
    pub identifiers: &'static [&'static str],
    /// The node kinds that turn a function into a method when one holds it.
    pub methods_in: &'static [&'static str],
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
        self.declarations.push(Declaration {
            name: text_of(name, self.source),
            kind: self.kind(capture, node),
            line: self.row(node),
            end: node.end_position().row as u64 + 1,
            text: self.text(node),
            externally_visible: (self.adapter.visible)(node),
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

/// Every file's facts under the measured roots, with one name index over them. Resolution here
/// is by name alone: no type inference and no import-aware lookup, so a reference resolves to
/// every declaration that spells it. That errs toward "referenced", which makes a structural
/// gate fail less and never more. ADR 0035, spec 8.4.
pub struct SourceIndex {
    files: Vec<FileFacts>,
    by_name: BTreeMap<String, Vec<(usize, usize)>>,
}

impl SourceIndex {
    pub fn of(mut files: Vec<FileFacts>) -> SourceIndex {
        files.sort_by(|a, b| a.file.cmp(&b.file));
        let mut by_name: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
        for (at, file) in files.iter().enumerate() {
            for (which, declaration) in file.declarations.iter().enumerate() {
                by_name
                    .entry(declaration.name.clone())
                    .or_default()
                    .push((at, which));
            }
        }
        SourceIndex { files, by_name }
    }

    pub fn files(&self) -> &[FileFacts] {
        &self.files
    }

    /// References with this name, in deterministic repository order. Resolution is deliberately
    /// name-only, so every declaration with the name sees the same sites.
    pub fn references(&self, name: &str) -> Vec<ReferenceSite<'_>> {
        let mut out = self
            .files
            .iter()
            .flat_map(|file| {
                file.references
                    .iter()
                    .filter(|reference| reference.name == name)
                    .map(|reference| ReferenceSite {
                        file: file.file.as_str(),
                        line: reference.line,
                    })
            })
            .collect::<Vec<_>>();
        out.sort();
        out.dedup();
        out
    }

    /// Every declaration of this name under the roots, in file order and then line order.
    pub fn declarations(&self, name: &str) -> Vec<Declared<'_>> {
        let Some(held) = self.by_name.get(name) else {
            return Vec::new();
        };
        held.iter()
            .filter_map(|(at, which)| self.at(*at, *which))
            .collect()
    }

    fn at(&self, at: usize, which: usize) -> Option<Declared<'_>> {
        let file = self.files.get(at)?;
        Some(Declared {
            file: &file.file,
            language: file.language,
            declaration: file.declarations.get(which)?,
        })
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

    fn measured_facts(path: &str, source: &str) -> FileFacts {
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
        let found: Vec<(&str, LanguageId, u64)> = index
            .declarations("refund")
            .iter()
            .map(|held| (held.file, held.language, held.declaration.line))
            .collect();
        let rust = LanguageId::Rust;
        assert_eq!(
            found,
            vec![("src/one.rs", rust, 1), ("src/two.rs", rust, 1)]
        );
        assert!(index.declarations("nothing").is_empty());
        assert_eq!(index.files().len(), 2);
    }
}
