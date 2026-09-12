use std::path::Path;

use crate::check::{Context, Sink};
use crate::config::Error;
use crate::markers::{self, Args, Kind, Language};
use crate::ratchet::{Evaluator, Values};
use crate::reference::{self, Key};

pub const SECTION: &str = "stubs";

/// The keys this section reads, which `klin reference` prints. Spec 5.4, 5.8.
const KEYS: &[Key] = &[
    reference::ROOTS.defaulting("the tree root"),
    reference::LANGUAGES
        .derived(
            "the same languages as `escapes`, less every language the stubs table holds no \
             rows for. This section refuses a `skip_rust_tests` key, because a stub inside an \
             inline test module is a stub",
        )
        .defaulting("none, and the section must then name `patterns`"),
    markers::PATTERNS,
    reference::EXCLUDE,
    reference::SKIP_DIRS,
];

const LABEL: &str = "stub";

const BODY: &str = "implement the body";
const NOTE: &str = "do the work the comment names, or record it in the tracker and delete the \
                    comment";

/// A comment marker in a language whose comments start with `//` or `/*`.
const SLASH: &str = r"(?://|/\*)[^\n]*\b(?:TODO|FIXME|XXX|HACK)\b";
/// The same, for a language whose comments start with `#`.
const HASH: &str = r"#[^\n]*\b(?:TODO|FIXME|XXX|HACK)\b";

/// The markers of spec 8.2 that one line states. A body only a parser can judge, such as `pass`
/// as the sole body of a function, is not here: `syntax::convention` reads it from the function
/// walk, and `reads_shapes` puts what it finds on the same sites. #114.
const LANGUAGES: &[Language] = &[
    Language {
        names: &["go"],
        suffixes: &[".go"],
        patterns: &[
            (
                "not implemented",
                r#"panic\(\s*"[^"]*(?i:not implemented)"#,
                BODY,
            ),
            ("comment marker", SLASH, NOTE),
        ],
    },
    Language {
        names: &["python"],
        suffixes: &[".py"],
        patterns: &[
            ("not implemented", r"\braise\s+NotImplementedError", BODY),
            ("comment marker", HASH, NOTE),
        ],
    },
    Language {
        names: &["rust"],
        suffixes: &[".rs"],
        patterns: &[
            ("not implemented", r"\b(?:todo|unimplemented)!\(", BODY),
            ("comment marker", SLASH, NOTE),
        ],
    },
    Language {
        names: &["javascript", "typescript"],
        suffixes: &[".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs"],
        patterns: &[
            (
                "not implemented",
                r#"throw new [A-Za-z]*Error\(\s*['"`][^'"`]*(?i:not implemented)"#,
                BODY,
            ),
            ("comment marker", SLASH, NOTE),
        ],
    },
];

/// Every language name this section selects a file set by, with the extensions each selects.
pub fn language_extensions() -> Vec<(&'static str, String)> {
    markers::language_extensions(&KIND)
}

pub const KIND: Kind = Kind {
    section: SECTION,
    languages: LANGUAGES,
    keys: KEYS,
    label: LABEL,
    skips_tests: false,
    skips_literals: true,
    reads_shapes: true,
    evaluator: Evaluator {
        metrics: &["count"],
        unit: "stub site(s)",
        condition: "where the code stands in for work nobody did",
        fix_advice: "Do what the marker stands in for. A placeholder an agent left behind is \
                     not work, and accepting one is a decision for a person, in the config, in \
                     a reviewed commit.",
        ceiling: None,
        format_metrics: show,
    },
};

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    markers::run(&KIND, args, start, out)
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    markers::gate(&KIND, at, out)
}

/// Whether the table holds rows for a language, named as the "languages" key names it.
pub fn holds_rows_for(language: &str) -> bool {
    LANGUAGES.iter().any(|held| held.names.contains(&language))
}

fn show(values: &Values) -> String {
    markers::show(LABEL, values)
}
