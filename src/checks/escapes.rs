use crate::checks::markers::{self, Kind, Language, TestCode, TestIdioms};
use crate::config::key::Key;
use crate::config::scope;
use crate::contract::check::{Context, Sink};
use crate::contract::ratchet::{Evaluator, Remedy};
use crate::sys::error::Error;
use crate::sys::record::Values;

pub const SECTION: &str = "escapes";

/// The keys this section reads, which `klin policy --reference` prints. Spec 5.4, 5.8.
const KEYS: &[Key] = &[scope::IN, scope::EXCEPT, markers::SKIP_TEST_IDIOMS];

const LABEL: &str = "escape";

const LANGUAGES: &[Language] = &[
    Language {
        names: &["go"],
        suffixes: &[".go"],
        patterns: &[
            ("nolint", r"//\s*nolint", ""),
            ("skipped test", r"\bt\.Skip(?:Now|f)?\(", ""),
        ],
        test_idioms: None,
    },
    Language {
        names: &["java"],
        suffixes: &[".java"],
        patterns: &[
            ("suppress warnings", r"@SuppressWarnings\(", ""),
            ("skipped test", r"@(?:Ignore|Disabled)\b", ""),
        ],
        test_idioms: None,
    },
    Language {
        names: &["kotlin"],
        suffixes: &[".kt", ".kts"],
        patterns: &[
            ("not-null assertion", r"!!", ""),
            ("suppress", r"@Suppress\(", ""),
            ("skipped test", r"@(?:Ignore|Disabled)\b", ""),
        ],
        test_idioms: None,
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
        test_idioms: None,
    },
    Language {
        names: &["ruby"],
        suffixes: &[".rb"],
        patterns: &[
            ("rubocop:disable", r"rubocop:disable", ""),
            ("skipped test", r"\bskip\b|\bxit\b|\bpending\b", ""),
        ],
        test_idioms: None,
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
                r"#(?:\s|//[^\n]*|/\*(?s:.)*?\*/)*\[(?:\s|//[^\n]*|/\*(?s:.)*?\*/)*(?:ignore|cfg_attr)\b",
                "",
            ),
        ],
        test_idioms: Some(TestIdioms {
            rows: &["unwrap", "expect"],
            code: TestCode::InlineModulesAndRoots,
        }),
    },
    Language {
        names: &["shell"],
        suffixes: &[".sh", ".bash", ".zsh"],
        patterns: &[
            ("errors ignored", r"\|\|\s*true\b|^\s*set\s+\+e\b", ""),
            ("shellcheck disable", r"shellcheck\s+disable", ""),
        ],
        test_idioms: None,
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
        test_idioms: None,
    },
    Language {
        names: &["javascript", "typescript"],
        suffixes: &[".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs"],
        patterns: &[
            ("any", r":\s*any\b|\bas\s+any\b|<any>", ""),
            ("ts-ignore", r"@ts-(?:ignore|nocheck)", ""),
            ("ts-expect-error", r"@ts-expect-error", ""),
            ("eslint-disable", r"eslint-disable", ""),
            ("non-null assertion", r"[\w)\]]!\.", ""),
            (
                "skipped test",
                r"\b(?:it|test|describe)\.(?:skip|only)\(|\bx(?:it|test|describe)\(",
                "",
            ),
            ("focused test", r"^[ \t]*f(?:it|describe)\(", ""),
        ],
        test_idioms: Some(TestIdioms {
            rows: &["ts-expect-error"],
            code: TestCode::Files,
        }),
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
    skips_literals: false,
    reads_shapes: false,
    reads_cfg_attr: true,
    keyed_by_row: &[],
    evaluator: Evaluator {
        metrics: &["count"],
        unit: "escape site(s)",
        condition: "where the code opts out of a check",
        fix_advice: Remedy::Fixed(
            "Fix what the escape hides: handle the error instead of unwrapping it, \
             address the lint instead of allowing it. Remove the skip, or fix what made the test fail. \
             Swallowing an error in place of the escape is not a fix. Accepting a new escape is a policy \
             decision for a person, in the config, in a reviewed commit.",
        ),
        ceiling: None,
        format_metrics: show,
        nested: None,
    },
};

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    markers::gate(&KIND, at, out)
}

fn show(values: &Values) -> String {
    markers::show(LABEL, values)
}
