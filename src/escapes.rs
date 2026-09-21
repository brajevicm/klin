use std::path::Path;

use crate::check::{Context, Sink};
use crate::config::Error;
use crate::markers::{self, Args, Kind, Language};
use crate::ratchet::{Evaluator, Values};
use crate::reference::Key;
use crate::scope;

pub const SECTION: &str = "escapes";

/// The keys this section reads, which `klin reference` prints. Spec 5.4, 5.8.
const KEYS: &[Key] = &[scope::IN, scope::EXCEPT, markers::SKIP_RUST_TESTS];

const LABEL: &str = "escape";

const LANGUAGES: &[Language] = &[
    Language {
        names: &["go"],
        suffixes: &[".go"],
        patterns: &[
            ("nolint", r"//\s*nolint", ""),
            ("skipped test", r"\bt\.Skip(?:Now|f)?\(", ""),
        ],
    },
    Language {
        names: &["java"],
        suffixes: &[".java"],
        patterns: &[
            ("suppress warnings", r"@SuppressWarnings\(", ""),
            ("skipped test", r"@(?:Ignore|Disabled)\b", ""),
        ],
    },
    Language {
        names: &["kotlin"],
        suffixes: &[".kt", ".kts"],
        patterns: &[
            ("not-null assertion", r"!!", ""),
            ("suppress", r"@Suppress\(", ""),
            ("skipped test", r"@(?:Ignore|Disabled)\b", ""),
        ],
    },
    Language {
        names: &["python"],
        suffixes: &[".py"],
        patterns: &[
            ("type ignore", r"#\s*type:\s*ignore", ""),
            ("noqa", r"#\s*noqa\b", ""),
            ("no cover", r"#\s*pragma:\s*no cover", ""),
            (
                "skipped test",
                r"pytest\.mark\.skip\b|pytest\.skip\(|unittest\.skip|@skip\b",
                "",
            ),
            ("expected failure", r"pytest\.mark\.xfail\b", ""),
            ("bare except", r"^\s*except\s*:", ""),
        ],
    },
    Language {
        names: &["ruby"],
        suffixes: &[".rb"],
        patterns: &[
            ("rubocop:disable", r"rubocop:disable", ""),
            ("skipped test", r"\bskip\b|\bxit\b|\bpending\b", ""),
        ],
    },
    Language {
        names: &["rust"],
        suffixes: &[".rs"],
        patterns: &[
            ("unwrap", r"\.unwrap\(\)", ""),
            ("expect", r"\.expect\(", ""),
            ("unsafe", r"\bunsafe\s*\{", ""),
            ("allow", r"#!?\[allow\(", ""),
            ("skipped test", r"#\[ignore\b", ""),
        ],
    },
    Language {
        names: &["shell"],
        suffixes: &[".sh", ".bash", ".zsh"],
        patterns: &[
            ("errors ignored", r"\|\|\s*true\b|^\s*set\s+\+e\b", ""),
            ("shellcheck disable", r"shellcheck\s+disable", ""),
        ],
    },
    Language {
        names: &["swift"],
        suffixes: &[".swift"],
        patterns: &[
            ("force try", r"\btry!", ""),
            ("force cast", r"\bas!", ""),
            ("force unwrap", r"[\w)\]]!(?:\.|\s*[,;)\]]|$)", ""),
            ("swiftlint:disable", r"swiftlint:disable", ""),
            ("unchecked Sendable", r"@unchecked\s+Sendable", ""),
            ("skipped test", r"\bXCTSkip|\bthrow\s+XCTSkip", ""),
        ],
    },
    Language {
        names: &["javascript", "typescript"],
        suffixes: &[".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs"],
        patterns: &[
            ("any", r":\s*any\b|\bas\s+any\b|<any>", ""),
            ("ts-ignore", r"@ts-(?:ignore|expect-error|nocheck)", ""),
            ("eslint-disable", r"eslint-disable", ""),
            ("non-null assertion", r"[\w)\]]!\.", ""),
            (
                "skipped test",
                r"\b(?:it|test|describe)\.(?:skip|only)\(|\bx(?:it|test|describe)\(",
                "",
            ),
            ("focused test", r"^[ \t]*f(?:it|describe)\(", ""),
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
    skips_tests: true,
    test_idioms: &["unwrap", "expect"],
    skips_literals: false,
    reads_shapes: false,
    evaluator: Evaluator {
        metrics: &["count"],
        unit: "escape site(s)",
        condition: "where the code opts out of a check",
        fix_advice: "Fix what the escape hides: handle the error instead of unwrapping it, \
                     address the lint instead of allowing it. Accepting a new escape is a policy \
                     decision for a person, in the config, in a reviewed commit.",
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

fn show(values: &Values) -> String {
    markers::show(LABEL, values)
}
