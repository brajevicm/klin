use std::borrow::Borrow;
use std::collections::HashSet;
use std::ops::Deref;
use std::rc::Rc;

use crate::syntax::{LanguageId, Unparsed};

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
    /// Byte offset of this statement in its source, unique within the file.
    pub start_byte: u64,
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
    /// Byte offset of this statement in its source, unique within the file.
    pub start_byte: u64,
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
    /// True where a block holds the declaration with no module between them, so the module is
    /// an item of that block and no path outside it names it.
    pub in_block: bool,
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
}
