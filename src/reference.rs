//! The configuration reference. Every check declares its keys beside the code that reads them,
//! and this module prints them as Markdown, so a person reads the keys without reading the
//! source. `docs/REFERENCE.md` holds the printed copy and a CLI test fails when the two differ,
//! so the committed copy cannot drift from the binary. A key a module reads and does not
//! declare here is still invisible, so a new key is two edits. Spec 5.8.

use std::collections::BTreeMap;
use std::fmt::Write;

use crate::config::{self, Error};
use crate::{complexity, escapes, gate, markers, stubs};

/// One configuration key, declared beside the code that reads it. `rule` is the rule the survey
/// derives the key by, empty for a key only a person pins. `default` is the value a run uses
/// when the key is absent, empty when there is none. Spec 5.4.
#[derive(Clone, Copy)]
pub struct Key {
    pub name: &'static str,
    /// What the key holds, in a person's words, so the reference stands without the source.
    pub holds: &'static str,
    pub required: bool,
    pub rule: &'static str,
    pub default: &'static str,
}

impl Key {
    /// The same key, required of a section that states itself.
    pub const fn required(self) -> Key {
        Key {
            required: true,
            ..self
        }
    }

    /// The same key with the value a run uses when a section leaves it out.
    pub const fn defaulting(self, default: &'static str) -> Key {
        Key { default, ..self }
    }

    /// The same key under a rule of this section's own, where the shared one does not hold.
    pub const fn derived(self, rule: &'static str) -> Key {
        Key { rule, ..self }
    }
}

/// The vocabulary of spec 5.3: the keys every section spells the same way and means the same
/// by. A section takes a row and states only what differs, so one meaning is written once.
pub const ROOTS: Key = Key {
    name: "roots",
    holds: "the directories the check reads",
    required: false,
    rule: "the directories that hold source files of a known language, merged up to the \
           shallowest directory that holds nothing but source, over the derivation commit's \
           survey and a walk of the working tree",
    default: "",
};

pub const LANGUAGES: Key = Key {
    name: "languages",
    holds: "the language names that choose the file set",
    required: false,
    rule: "the languages of the files under `roots`, in the derivation commit and in the \
           working tree",
    default: "",
};

pub const EXCLUDE: Key = Key {
    name: "exclude",
    holds: "globs on the basename and on the path from the tree root",
    required: false,
    rule: "",
    default: "nothing is excluded",
};

pub const SKIP_DIRS: Key = Key {
    name: "skip_dirs",
    holds: "directory names to skip beside the shared list",
    required: false,
    rule: "",
    default: "the shared list only",
};

pub const EXCLUDE_EXCEPT: Key = Key {
    name: "exclude_except",
    holds: "the files an `exclude` glob must not drop",
    required: false,
    rule: "",
    default: "nothing is kept back",
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
         `klin.json` is optional. A tree that has none is gated over the sections the survey \
         supplies. A section may pin some keys and leave the rest to derivation, and a run \
         prints one `pinned:` or `derived:` line per value it used. A top-level key klin does \
         not read is an error naming the key, and a key inside a section that klin does not \
         read is not refused, so a misspelled key inside a section measures nothing in silence. \
         A gate is excluded by setting its section to `false`."
    );
}

fn top_level(out: &mut String) {
    let _ = writeln!(out, "\n## Top-level keys\n");
    table(config::keys(), out);
}

fn sections(out: &mut String) {
    let _ = writeln!(
        out,
        "\n## Sections\n\n\
         One key per gate, named for its section. The sections share key names: `roots`, \
         `languages`, `exclude`, `skip_dirs` and `ceilings` mean the same thing everywhere. A \
         section reads only the keys its own table names."
    );
    for (section, keys) in gate::catalogue() {
        let _ = writeln!(out, "\n### `{section}`\n");
        table(keys, out);
    }
}

fn table(keys: &[Key], out: &mut String) {
    let _ = writeln!(out, "{HEAD}\n{RULE}");
    for key in keys {
        let source = match key.rule.is_empty() {
            true => "pinned only",
            false => "derived when absent",
        };
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {source} | {} | {} |",
            key.name,
            cell(key.holds),
            yes(key.required),
            cell(key.rule),
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
        "\n## Language names\n\n\
         The checks share language names and not file sets. A name selects the extensions of \
         its own check's table, and the tables differ. A section that names no language \
         measures every language `complexity` knows, and one that names none for `escapes` or \
         `stubs` must name `patterns` instead."
    );
    let named = [
        ("complexity", complexity::language_extensions()),
        ("escapes", markers::language_extensions(&escapes::KIND)),
        ("stubs", markers::language_extensions(&stubs::KIND)),
    ];
    for (section, rows) in named {
        let _ = writeln!(out, "\n### `{section}`\n");
        let _ = writeln!(out, "| Name | Extensions |\n| --- | --- |");
        for (name, extensions) in rows {
            let _ = writeln!(out, "| `{name}` | {extensions} |");
        }
    }
}

/// Every language name a table holds, with the extensions that name selects. Two rows under one
/// name, such as TypeScript and TSX, are one row here, because the name selects both.
pub fn extensions_by_name(
    rows: impl Iterator<Item = (&'static [&'static str], &'static [&'static str])>,
) -> Vec<(String, String)> {
    let mut held: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
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
            (name.to_string(), listed.join(", "))
        })
        .collect()
}

fn exclusion(out: &mut String) {
    let _ = writeln!(
        out,
        "\n## Exclusion\n\n\
         Every gate skips a fixed list of directories: {}. `skip_dirs` adds to that list and \
         replaces nothing in it.\n\n\
         Every gate also drops what git ignores.\n\n\
         `complexity` and `inventory` skip every directory whose name starts with a dot. \
         `escapes`, `stubs` and `doc_citations` read one, so a source file under a dot \
         directory is judged by those checks and not by these.\n\n\
         An `exclude` glob is matched against the basename of a file, and against the whole \
         path as the walk holds it, which is the absolute path. A glob written from the tree \
         root, such as `src/generated/*`, therefore matches nothing, and `*/generated/*` is the \
         form that works.\n\n\
         `exclude_except` names the files an `exclude` glob would otherwise drop, and it \
         answers `exclude` globs only. It cannot bring back a file under a skipped directory, \
         and it cannot bring back a file of an extension the check does not measure. Only \
         `complexity` reads `exclude_except`.\n\n\
         A file that `before` measured and `after` does not is a NOTE in the hook and exit 2 \
         under `--strict`, so an exclusion added this window is visible.",
        listed(&crate::files::default_skip_dirs())
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
         \"ceilings\": {{\n  \
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
