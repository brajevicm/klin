//! The one parser owner. Every check that reads source through a grammar comes here for the
//! language a path belongs to, the grammar that reads it, the parse, and the file no grammar
//! read. A check keeps its own policy — what it counts and what it refuses — and holds no
//! Tree-sitter node kinds of another language's grammar. ADR 0003, ADR 0035.

use std::fmt::Write;

use serde_json::Value;
use tree_sitter::{Node, Parser, Tree};

use crate::config::{Error, Flags, UNPARSED};
use crate::ratchet::Values;
use crate::reference;

pub mod convention;
/// Dead until the first structural consumer lands (#51, #52). An expectation rather than an
/// allowance, so the compiler names this line the moment the module has a caller.
#[cfg_attr(not(test), expect(dead_code))]
pub mod structural;

/// One logical language, which is what a structural consumer names. A grammar variant is not
/// one: a `.tsx` file is TypeScript, and TSX is never a language of its own above this module.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum LanguageId {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Go,
    Java,
    Ruby,
    Swift,
    Kotlin,
}

/// One grammar-backed language: the logical language it belongs to, what a failure calls it,
/// the names a config selects it by, the files it selects, the grammar that reads them, and
/// the nodes that are functions in it.
pub struct Language {
    pub id: LanguageId,
    pub name: &'static str,
    pub names: &'static [&'static str],
    pub extensions: &'static [&'static str],
    pub grammar: fn() -> tree_sitter::Language,
    pub functions: &'static [&'static str],
}

fn rust() -> tree_sitter::Language {
    tree_sitter_rust::LANGUAGE.into()
}

fn python() -> tree_sitter::Language {
    tree_sitter_python::LANGUAGE.into()
}

fn typescript() -> tree_sitter::Language {
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
}

fn tsx() -> tree_sitter::Language {
    tree_sitter_typescript::LANGUAGE_TSX.into()
}

fn javascript() -> tree_sitter::Language {
    tree_sitter_javascript::LANGUAGE.into()
}

fn go() -> tree_sitter::Language {
    tree_sitter_go::LANGUAGE.into()
}

fn java() -> tree_sitter::Language {
    tree_sitter_java::LANGUAGE.into()
}

fn ruby() -> tree_sitter::Language {
    tree_sitter_ruby::LANGUAGE.into()
}

fn swift() -> tree_sitter::Language {
    tree_sitter_swift::LANGUAGE.into()
}

fn kotlin() -> tree_sitter::Language {
    tree_sitter_kotlin_ng::LANGUAGE.into()
}

const ECMASCRIPT_FUNCTIONS: &[&str] = &[
    "function_declaration",
    "function_expression",
    "generator_function",
    "generator_function_declaration",
    "arrow_function",
    "method_definition",
];

pub const LANGUAGES: &[Language] = &[
    Language {
        id: LanguageId::Rust,
        name: "Rust",
        names: &["rust"],
        extensions: &[".rs"],
        grammar: rust,
        functions: &["function_item"],
    },
    Language {
        id: LanguageId::Python,
        name: "Python",
        names: &["python"],
        extensions: &[".py"],
        grammar: python,
        functions: &["function_definition"],
    },
    Language {
        id: LanguageId::TypeScript,
        name: "TypeScript",
        names: &["typescript"],
        extensions: &[".ts", ".mts", ".cts"],
        grammar: typescript,
        functions: ECMASCRIPT_FUNCTIONS,
    },
    Language {
        id: LanguageId::TypeScript,
        name: "TSX",
        names: &["typescript", "tsx"],
        extensions: &[".tsx"],
        grammar: tsx,
        functions: ECMASCRIPT_FUNCTIONS,
    },
    Language {
        id: LanguageId::JavaScript,
        name: "JavaScript",
        names: &["javascript"],
        extensions: &[".js", ".jsx", ".mjs", ".cjs"],
        grammar: javascript,
        functions: ECMASCRIPT_FUNCTIONS,
    },
    Language {
        id: LanguageId::Go,
        name: "Go",
        names: &["go"],
        extensions: &[".go"],
        grammar: go,
        functions: &["function_declaration", "method_declaration", "func_literal"],
    },
    Language {
        id: LanguageId::Java,
        name: "Java",
        names: &["java"],
        extensions: &[".java"],
        grammar: java,
        functions: &[
            "method_declaration",
            "constructor_declaration",
            "compact_constructor_declaration",
            "static_initializer",
            "lambda_expression",
        ],
    },
    Language {
        id: LanguageId::Ruby,
        name: "Ruby",
        names: &["ruby"],
        extensions: &[".rb"],
        grammar: ruby,
        functions: &["method", "singleton_method"],
    },
    Language {
        id: LanguageId::Swift,
        name: "Swift",
        names: &["swift"],
        extensions: &[".swift"],
        grammar: swift,
        functions: &[
            "function_declaration",
            "init_declaration",
            "deinit_declaration",
            "subscript_declaration",
            "computed_property",
            "computed_getter",
            "computed_setter",
            "willset_clause",
            "didset_clause",
        ],
    },
    Language {
        id: LanguageId::Kotlin,
        name: "Kotlin",
        names: &["kotlin"],
        extensions: &[".kt", ".kts"],
        grammar: kotlin,
        functions: &[
            "function_declaration",
            "anonymous_function",
            "secondary_constructor",
            "anonymous_initializer",
            "getter",
            "setter",
        ],
    },
];

/// One file no grammar read, which every gate that parses names and refuses. ADR 0003.
pub struct Unparsed {
    pub file: String,
    pub language: &'static str,
}

/// The parse of one source text, which owns the tree its nodes are read out of.
pub struct ParsedFile<'a> {
    pub path: &'a str,
    pub source: &'a str,
    pub language: &'static Language,
    tree: Tree,
}

impl ParsedFile<'_> {
    pub fn root(&self) -> Node<'_> {
        self.tree.root_node()
    }

    pub fn lines(&self) -> Vec<&str> {
        self.source.lines().collect()
    }

    pub fn bytes(&self) -> &[u8] {
        self.source.as_bytes()
    }

    /// Every node this language calls a function, in source order.
    pub fn functions(&self) -> Vec<Node<'_>> {
        let mut out = Vec::new();
        walk(self.root(), &mut |node| {
            if self.language.functions.contains(&node.kind()) {
                out.push(node);
            }
        });
        out
    }
}

/// What one grammar made of one source text: a tree, or the file it refused.
pub enum Parsed<'a> {
    Read(ParsedFile<'a>),
    Rejected(Unparsed),
}

/// The parse of one path, and `None` when no grammar here reads the path. Nothing above this
/// module builds a `Parser`. Spec 8.4.
pub fn parse<'a>(path: &'a str, source: &'a str) -> Result<Option<Parsed<'a>>, Error> {
    match language_of(path) {
        Some(language) => read(path, source, language).map(Some),
        None => Ok(None),
    }
}

/// The parse of one path under a language the caller already chose, which is what a check that
/// selects its own language set does.
pub fn read<'a>(
    path: &'a str,
    source: &'a str,
    language: &'static Language,
) -> Result<Parsed<'a>, Error> {
    let read = tree_of(source, language)?.filter(|tree| !tree.root_node().has_error());
    Ok(match read {
        Some(tree) => Parsed::Read(ParsedFile {
            path,
            source,
            language,
            tree,
        }),
        None => Parsed::Rejected(Unparsed {
            file: path.to_string(),
            language: language.name,
        }),
    })
}

/// The tree a grammar made of a text, error nodes and all, for a reader that wants whatever
/// could be read rather than a verdict on the file. Nothing here is an unparsed file: ADR 0003
/// belongs to `read`, and a reader that comes this way says so in its own words.
pub fn tolerant<'a>(path: &'a str, source: &'a str) -> Option<ParsedFile<'a>> {
    let language = language_of(path)?;
    Some(ParsedFile {
        path,
        source,
        language,
        tree: tree_of(source, language).ok().flatten()?,
    })
}

fn tree_of(source: &str, language: &Language) -> Result<Option<Tree>, Error> {
    let mut parser = Parser::new();
    parser.set_language(&(language.grammar)()).map_err(|why| {
        Error(format!(
            "the {} grammar could not be loaded: {why}",
            language.name
        ))
    })?;
    Ok(parser.parse(source, None))
}

/// The grammar-backed language a path belongs to, and `None` when no grammar here reads it.
pub fn language_of(path: &str) -> Option<&'static Language> {
    LANGUAGES.iter().find(|language| {
        language
            .extensions
            .iter()
            .any(|extension| path.ends_with(extension))
    })
}

/// The file extensions the named languages carry, and every language's when none are named, so
/// a survey samples exactly the files a gate would measure. Spec 5.4.
pub fn extensions(named: &[String]) -> Vec<&'static str> {
    LANGUAGES
        .iter()
        .filter(|language| {
            named.is_empty()
                || language
                    .names
                    .iter()
                    .any(|name| named.iter().any(|want| want == name))
        })
        .flat_map(|language| language.extensions.iter().copied())
        .collect()
}

/// Every language name the table holds, with the extensions that name selects.
pub fn language_extensions() -> Vec<(&'static str, String)> {
    reference::extensions_by_name(
        LANGUAGES
            .iter()
            .map(|language| (language.names, language.extensions)),
    )
}

/// What a gate does about the files no grammar read: a NOTE in the hook, and exit 2 outside
/// it, because an agent cannot fix a grammar and a file klin cannot read is a hole in the
/// ratchet. ADR 0003, spec 14.
pub fn unread(unparsed: &[Unparsed], flags: &Flags, code: u8, out: &mut String) -> u8 {
    let only = flags.only.as_deref();
    let named: Vec<&Unparsed> = unparsed
        .iter()
        .filter(|file| only.is_none_or(|only| only.contains(&file.file)))
        .collect();
    if named.is_empty() {
        return code;
    }
    match flags.hook {
        true => {
            noted(&named, flags, out);
            code
        }
        false => {
            refused(&named, flags, out);
            2
        }
    }
}

fn noted(named: &[&Unparsed], flags: &Flags, out: &mut String) {
    let _ = writeln!(
        out,
        "NOTE: {} file(s) the grammar could not parse, so nothing in them was measured:",
        named.len()
    );
    for file in named {
        let rejected = rejected(file);
        let _ = writeln!(out, "  {}  {rejected}", file.file);
        flags.record(|records| records.notes.push(unparsed_site(file, &rejected)));
    }
    let _ = writeln!(out, "{REMEDY}");
}

fn refused(named: &[&Unparsed], flags: &Flags, out: &mut String) {
    let _ = writeln!(
        out,
        "FAIL: {} file(s) the grammar could not parse, so nothing in them was measured:",
        named.len()
    );
    for file in named {
        let rejected = rejected(file);
        let _ = writeln!(out, "  {}  {rejected}", file.file);
        flags.record(|records| records.findings.push(unparsed_site(file, &rejected)));
    }
    let _ = writeln!(out, "{REMEDY}");
}

const REMEDY: &str = "A file klin cannot read is a hole in the ratchet. Update the grammar, or \
                      exclude the file and accept that nothing measures it.";

fn rejected(file: &Unparsed) -> String {
    format!("the {} grammar rejected it", file.language)
}

fn unparsed_site(file: &Unparsed, rejected: &str) -> Value {
    let mut out = Values::new();
    out.insert("outcome".into(), UNPARSED.into());
    out.insert("file".into(), file.file.clone().into());
    out.insert("text".into(), rejected.into());
    Value::Object(out)
}

/// The line at this row, trimmed, which is the text every site in klin is named by.
pub fn line_at(lines: &[&str], row: usize) -> String {
    lines.get(row).unwrap_or(&"").trim().to_string()
}

/// Every node of one tree, in source order, given to a reader that keeps what it wants.
pub fn walk<'t>(node: Node<'t>, keep: &mut impl FnMut(Node<'t>)) {
    keep(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, keep);
    }
}
