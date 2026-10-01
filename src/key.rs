//! The key metadata every section declares and the reference prints: the shape of each key, the
//! keys every section shares, and the two keys one entry of the `build` list holds. It imports no
//! check and no catalogue, so a module that reads a key does not reach the reference. Spec 5.3.

use std::collections::BTreeMap;

/// Every language name a section selects a file set by, with the extensions each name selects.
pub type Languages = fn() -> Vec<(&'static str, String)>;

#[derive(Clone, Copy)]
pub enum Shape {
    String,
    Boolean,
    WholeNumber,
    Ceiling,
    Strings,
    StringOrList,
    Language(Languages),
    Build,
    Accepted,
    Radius,
    Journal,
    Layers,
}

#[derive(Clone, Copy)]
pub enum SectionShape {
    Object,
    DocumentMap,
    FalseOnly,
    Conventions,
    Sarif,
}

/// One configuration key, declared beside the code that reads it. `rule` is the rule klin
/// derives the key by when the configuration leaves it out, and none for a key only a person
/// pins. `default` is the value a run uses
/// when the key is absent, empty when there is none. Spec 5.4.
#[derive(Clone, Copy)]
pub struct Key {
    pub name: &'static str,
    /// What the key holds, in a person's words, so the reference stands without the source.
    pub holds: &'static str,
    pub required: bool,
    pub rule: Option<&'static str>,
    pub default: &'static str,
    pub shape: Shape,
}

/// The vocabulary of spec 5.3: the keys every section spells the same way and means the same
/// by. A section takes a row and states only what differs, so one meaning is written once.
pub const ROOTS: Key = Key {
    name: "roots",
    holds: "the directories the check reads",
    required: false,
    rule: Some(
        "the directories that hold source files of a known language, merged up to the \
                shallowest directory that holds nothing but source, over the derivation commit's \
                survey and a walk of the working tree",
    ),
    default: "",
    shape: Shape::Strings,
};

pub const LANGUAGES: Key = Key {
    name: "languages",
    holds: "the language names that choose the file set",
    required: false,
    rule: Some(
        "the languages of the files under `roots`, in the derivation commit and in the \
                working tree",
    ),
    default: "",
    shape: Shape::Strings,
};

pub const EXCLUDE: Key = Key {
    name: "exclude",
    holds: "globs on the basename and on the path from the tree root",
    required: false,
    rule: None,
    default: "nothing is excluded",
    shape: Shape::Strings,
};

pub const SKIP_DIRS: Key = Key {
    name: "skip_dirs",
    holds: "directory names to skip beside the shared list",
    required: false,
    rule: None,
    default: "the shared list only",
    shape: Shape::Strings,
};

/// The two keys one entry of the `build` list holds, which `config::BUILD` states.
pub const RUN: &str = "run";
pub const ROOT: &str = "root";

/// Every language name a table holds, with the extensions that name selects. Two rows under one
/// name, such as TypeScript and TSX, are one row here, because the name selects both.
pub fn extensions_by_name(
    rows: impl Iterator<Item = (&'static [&'static str], &'static [&'static str])>,
) -> Vec<(&'static str, String)> {
    let mut held: BTreeMap<&'static str, Vec<&str>> = BTreeMap::new();
    for (names, extensions) in rows {
        for name in names {
            let under = held.entry(name).or_default();
            for extension in extensions {
                if !under.contains(extension) {
                    under.push(extension);
                }
            }
        }
    }
    held.into_iter()
        .map(|(name, extensions)| {
            let listed: Vec<String> = extensions.iter().map(|at| format!("`{at}`")).collect();
            (name, listed.join(", "))
        })
        .collect()
}
