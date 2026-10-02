use crate::syntax::structural::facts::{Declaration, DeclarationKind};

/// What the derivations found over one tree: its surfaces, and each package or target they
/// derived no surface from.
#[derive(Default)]
pub struct Found {
    pub surfaces: Vec<Surface>,
    pub inapplicable: Vec<Inapplicable>,
}

/// The words a report prints for what an item is.
pub const FUNCTION: &str = "function";

pub const METHOD: &str = "method";

pub const TYPE: &str = "type";

pub const CONSTANT: &str = "constant";

pub const VARIABLE: &str = "variable";

pub const MODULE: &str = "module";

pub const NAMESPACE: &str = "namespace";

/// An item klin proves exposed and cannot classify: a re-export of another crate or package,
/// an enum variant re-exported by path, or an anonymous default export.
pub const ITEM: &str = "item";

/// How much of an item's contract klin proves.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Contract {
    /// A stable identity and a supported canonical declared signature.
    Measured(String),
    /// The item exists. Where the clause that exposes it is itself the contract klin can prove,
    /// that clause is here to be compared; otherwise only presence is judged.
    Opaque(Option<String>),
}

/// One thing a consumer addresses under a surface.
#[derive(Clone, Debug)]
pub struct Item {
    /// The path or name the consumer writes, without the surface's own name.
    pub path: String,
    pub kind: &'static str,
    /// Where the declaration behind the item sits, as explanation only.
    pub origin: Option<(String, u64)>,
    pub contract: Contract,
}

/// One consumer-facing surface: a library crate, or one export subpath of a package.
pub struct Surface {
    /// What a consumer names the surface by: the crate name, or the package name and subpath.
    pub id: String,
    /// The Rust package manifest that owns this surface; crate names need not be unique.
    pub manifest: Option<String>,
    pub language: &'static str,
    /// Where the surface was discovered, for a report.
    pub source: String,
    pub items: Vec<Item>,
    /// The files whose facts the surface is derived from, so a gate can say which unparsed
    /// files sit inside it.
    pub files: Vec<String>,
    pub holes: Vec<Hole>,
}

/// One form klin recognizes inside a supported surface and cannot resolve, so the surface is not
/// completely measured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hole {
    pub file: String,
    pub line: u64,
    pub text: String,
    pub why: String,
}

/// A package or target klin found and derived no surface from, and why.
pub struct Inapplicable {
    pub what: String,
    pub why: String,
}

impl Surface {
    pub fn item(&self, path: &str, kind: &str) -> Option<&Item> {
        self.items
            .iter()
            .find(|item| item.path == path && item.kind == kind)
    }
}

/// The word for a declaration's kind.
pub fn kind_word(kind: DeclarationKind) -> &'static str {
    match kind {
        DeclarationKind::Function => FUNCTION,
        DeclarationKind::Method => METHOD,
        DeclarationKind::Type => TYPE,
        DeclarationKind::Constant => CONSTANT,
        DeclarationKind::Variable => VARIABLE,
    }
}

/// One item from one declaration, exposed under `path`.
pub(super) fn declared(path: String, file: &str, declaration: &Declaration) -> Item {
    Item {
        path,
        kind: kind_word(declaration.kind),
        origin: Some((file.to_string(), declaration.line)),
        contract: match &declaration.signature {
            Some(signature) => Contract::Measured(signature.clone()),
            None => Contract::Opaque(None),
        },
    }
}

/// One item klin proves exposed by a clause and cannot classify further.
pub(super) fn opaque(
    path: String,
    kind: &'static str,
    file: &str,
    line: u64,
    clause: String,
) -> Item {
    Item {
        path,
        kind,
        origin: Some((file.to_string(), line)),
        contract: Contract::Opaque(Some(clause)),
    }
}
