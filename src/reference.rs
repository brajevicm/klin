//! The configuration reference. Every check declares its keys beside the code that reads them,
//! and this module prints them as Markdown, so a person reads the keys without reading the
//! source. `docs/REFERENCE.md` holds the printed copy and a CLI test fails when the two differ,
//! so the committed copy cannot drift from the binary. A read takes the declared `Key` and not
//! a string of its own, so a key the reference does not print is a key no module can read.
//! Spec 5.8.

use std::collections::BTreeMap;
use std::fmt::Write;

use crate::config::{self, Error};
use crate::{check, doc_citations};

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
}

/// Every language name a section selects a file set by, with the extensions each name selects.
pub type Languages = fn() -> Vec<(&'static str, String)>;

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
};

pub const EXCLUDE: Key = Key {
    name: "exclude",
    holds: "globs on the basename and on the path from the tree root",
    required: false,
    rule: None,
    default: "nothing is excluded",
};

pub const SKIP_DIRS: Key = Key {
    name: "skip_dirs",
    holds: "directory names to skip beside the shared list",
    required: false,
    rule: None,
    default: "the shared list only",
};

const HEAD: &str = "| Key | Holds | Required | Source | Derivation rule | Default |";
const RULE: &str = "| --- | --- | --- | --- | --- | --- |";
const NONE: &str = "—";

pub fn run(out: &mut String) -> Result<u8, Error> {
    preamble(out);
    top_level(out);
    sections(out);
    languages(out);
    exclusion(out);
    ceilings(out);
    Ok(0)
}

fn preamble(out: &mut String) {
    let _ = writeln!(
        out,
        "# klin configuration reference\n\n\
         `klin reference` prints this page. `docs/REFERENCE.md` holds the printed copy, and a \
         test fails when the two differ, so the reference cannot drift from the binary. Do not \
         edit the copy by hand.\n\n\
         `klin.json` is a person's policy over facts klin discovers in the tree. `{{}}` is a \
         complete configuration: every Automatic check runs over what the tree holds and \
         derives what the file leaves out. A section pins a decision and leaves the rest to \
         derivation, and a run prints one `pinned:` or `derived:` line per value it used. The \
         file never describes the repository: roots, languages, documents, manifests, test \
         roots and build commands are facts. A key or field klin does not read is an error \
         naming it. A gate is excluded by setting its section to `false`."
    );
}

fn top_level(out: &mut String) {
    let _ = writeln!(out, "\n## Top-level keys\n");
    table(config::KEYS, out);
}

fn sections(out: &mut String) {
    let _ = writeln!(
        out,
        "\n## Sections\n\n\
         One key per gate, named for its section. Every check discovers what it applies to; its \
         object holds only a person's policy, and a section reads only the keys its own table \
         names."
    );
    for spec in check::CATALOGUE {
        let _ = writeln!(out, "\n### `{}`\n", spec.section);
        match spec.keys.is_empty() {
            true => {
                let _ = writeln!(
                    out,
                    "No keys: the section is absent, or `false` to exclude the gate."
                );
            }
            false => table(spec.keys, out),
        }
    }
}

fn table(keys: &[Key], out: &mut String) {
    let _ = writeln!(out, "{HEAD}\n{RULE}");
    for key in keys {
        let source = match key.rule {
            None => "pinned only",
            Some(_) => "derived when absent",
        };
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {source} | {} | {} |",
            key.name,
            cell(key.holds),
            yes(key.required),
            cell(key.rule.unwrap_or_default()),
            cell(key.default)
        );
    }
}

fn yes(required: bool) -> &'static str {
    match required {
        true => "yes",
        false => "no",
    }
}

/// One cell, with the pipe that would otherwise open a column escaped.
fn cell(text: &str) -> String {
    match text.is_empty() {
        true => NONE.to_string(),
        false => text.replace('|', "\\|"),
    }
}

fn languages(out: &mut String) {
    let _ = writeln!(
        out,
        "\n## Built-in language coverage\n\n\
         These tables report the source extensions each check discovers automatically. They \
         are capabilities of the binary, not selectors accepted in `klin.json`."
    );
    let named = check::CATALOGUE
        .iter()
        .filter_map(|spec| Some((spec.section, spec.languages?)));
    for (section, rows) in named {
        let _ = writeln!(out, "\n### `{section}`\n");
        let _ = writeln!(out, "| Name | Extensions |\n| --- | --- |");
        for (name, extensions) in rows() {
            let _ = writeln!(out, "| `{name}` | {extensions} |");
        }
    }
}

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

fn exclusion(out: &mut String) {
    let _ = writeln!(
        out,
        "\n## Scope and discovery\n\n\
         Every source check discovers supported files from one repository walk, skips the \
         fixed directory list {}, and drops files git ignores. `in` narrows a check to a \
         repository-relative path (or non-empty list) and `except` takes paths back out. Each \
         path names itself and everything below it; neither key accepts globs.\n\n\
         `complexity`, `escapes`, `stubs`, `dead_symbols`, `reachability`, `inventory` and \
         `lockfile` reject the retired `roots`, `languages`, `patterns`, `skip_dirs`, \
         `exclude`, `exclude_except`, `ceilings`, `name`, `path`, `pattern` and `manifests` \
         topology keys with a migration error. A file measured under the base scope and \
         omitted by today's scope is a NOTE in the hook and exit 2 under `--strict`.\n\n\
         `doc_size` maps a document path to its ceiling, and every document at the tree root it \
         does not name keeps a derived ceiling. `doc_citations` reads every Markdown file at the \
         tree root and resolves a citation against the whole tree; a citation names one of the \
         built-in extensions {}.",
        listed(&crate::files::default_skip_dirs()),
        listed(
            &doc_citations::EXTENSIONS
                .iter()
                .map(|extension| extension.to_string())
                .collect::<Vec<String>>()
        )
    );
}

fn listed(names: &[String]) -> String {
    names
        .iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<String>>()
        .join(", ")
}

fn ceilings(out: &mut String) {
    let _ = writeln!(
        out,
        "\n## Ceilings\n\n\
         A pinned ceiling is either a whole number or an object of dated steps:\n\n\
         ```json\n\
         \"complexity\": {{\n  \
         \"cc\": 12,\n  \
         \"lines\": {{ \"2026-09-08\": 90, \"2027-01-01\": 70, \"2027-07-01\": 60 }}\n\
         }}\n\
         ```\n\n\
         The run uses the lowest step whose date is on or before today, in UTC. A schedule with \
         no step yet due is an error. A run that uses a schedule prints the date it used beside \
         the ceiling. A derived ceiling is not monotone: it falls when simple functions arrive \
         and rises when simple functions leave, so a person who wants a ceiling that cannot \
         loosen pins one."
    );
}
