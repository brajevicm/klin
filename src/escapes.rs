use std::path::Path;

use crate::check::{Context, Sink};
use crate::config::Error;
use crate::markers::{self, Args, Kind, Language};
use crate::ratchet::{Evaluator, Remedy, Values};
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
            (
                "skipped test",
                r"#\[ignore\b|#\[cfg_attr\((?:\s|all|any|not|[(),])*,\s*ignore\b",
                "",
            ),
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
    stands,
    evaluator: Evaluator {
        metrics: &["count"],
        unit: "escape site(s)",
        condition: "where the code opts out of a check",
        fix_advice: Remedy::Fixed(
            "Fix what the escape hides: handle the error instead of unwrapping it, \
             address the lint instead of allowing it. Accepting a new escape is a policy \
             decision for a person, in the config, in a reviewed commit.",
        ),
        ceiling: None,
        format_metrics: show,
        nested: None,
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

/// Whether a match stands as a site. A `cfg_attr` that skips a test stands only where its
/// predicate holds on every target, which the pattern cannot decide. Spec 8.2.
fn stands(found: &str) -> bool {
    found.strip_prefix("#[cfg_attr(").is_none_or(|rest| {
        predicate(rest).is_some_and(|(holds, after)| holds && after.trim_start().starts_with(','))
    })
}

/// The value of the cfg predicate at the front of this text, made of `all`, `any` and `not`
/// alone, with the text after it, and `None` where no such predicate leads it.
fn predicate(text: &str) -> Option<(bool, &str)> {
    let text = text.trim_start();
    let name = ["all", "any", "not"]
        .into_iter()
        .find(|name| text.starts_with(name))?;
    let mut rest = text[name.len()..].trim_start().strip_prefix('(')?;
    let mut values = Vec::new();
    while let Some((value, after)) = predicate(rest) {
        values.push(value);
        rest = after.trim_start();
        rest = rest.strip_prefix(',').unwrap_or(rest);
    }
    let value = match (name, values.as_slice()) {
        ("all", _) => values.iter().all(|held| *held),
        ("any", _) => values.iter().any(|held| *held),
        (_, [only]) => !only,
        _ => return None,
    };
    Some((value, rest.trim_start().strip_prefix(')')?))
}
