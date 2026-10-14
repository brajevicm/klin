use serde_json::Value;

use crate::checks::markers::{self, Kind, Language, TestCode, TestIdioms};
use crate::config::key::Key;
use crate::config::scope;
use crate::contract::check::{Context, Sink};
use crate::contract::project::Project;
use crate::contract::ratchet::{Evaluator, Remedy};
use crate::syntax::LanguageId;
use crate::sys::error::Error;
use crate::sys::record::Values;

pub const SECTION: &str = "escapes";

/// The keys this section reads, which `klin policy --reference` prints. Spec 5.4, 5.8.
const KEYS: &[Key] = &[scope::IN, scope::EXCEPT, markers::SKIP_TEST_IDIOMS];

const LABEL: &str = "escape";

const DOUBLE_CAST: &str = "double cast";
const DOUBLE_CAST_FIX: &str =
    "give the value the type it has: check its shape, or narrow it with a type guard";
const EMPTY_HANDLER: &str = "empty handler";
const BIOME_IGNORE: &str = "biome-ignore";
const ANALYZER_FIX: &str = "fix what the analyzer reports instead of silencing it";

/// The rows in the ride-along tier of ADR 0073, by their replay N on #572. Spec 9.6.
const RIDE_ALONG: &[&str] = &[BIOME_IGNORE];

const LANGUAGES: &[Language] = &[
    Language {
        languages: &[LanguageId::Go],
        patterns: &[
            ("nolint", r"//\s*nolint", ""),
            ("skipped test", r"\bt\.Skip(?:Now|f)?\(", ""),
        ],
        test_idioms: None,
    },
    Language {
        languages: &[LanguageId::Java],
        patterns: &[
            ("suppress warnings", r"@SuppressWarnings\(", ""),
            ("skipped test", r"@(?:Ignore|Disabled)\b", ""),
        ],
        test_idioms: None,
    },
    Language {
        languages: &[LanguageId::Kotlin],
        patterns: &[
            ("not-null assertion", r"!!", ""),
            ("suppress", r"@Suppress\(", ""),
            ("skipped test", r"@(?:Ignore|Disabled)\b", ""),
        ],
        test_idioms: None,
    },
    Language {
        languages: &[LanguageId::Python],
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
        languages: &[LanguageId::Ruby],
        patterns: &[
            ("rubocop:disable", r"rubocop:disable", ""),
            ("skipped test", r"\bskip\b|\bxit\b|\bpending\b", ""),
        ],
        test_idioms: None,
    },
    Language {
        languages: &[LanguageId::Rust],
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
        languages: &[LanguageId::Shell],
        patterns: &[
            ("errors ignored", r"\|\|\s*true\b|^\s*set\s+\+e\b", ""),
            ("shellcheck disable", r"shellcheck\s+disable", ""),
        ],
        test_idioms: None,
    },
    Language {
        languages: &[LanguageId::Swift],
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
        languages: &[LanguageId::TypeScript, LanguageId::JavaScript],
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
            (DOUBLE_CAST, r"\bas\s+unknown\s+as\b", DOUBLE_CAST_FIX),
            (
                "any generic argument",
                r"<\s*any\s*,|,\s*any\s*[,>]",
                "name the type the argument holds, or take `unknown` and narrow it",
            ),
            (
                BIOME_IGNORE,
                r"biome-ignore",
                "fix what the Biome rule reports instead of ignoring it",
            ),
            (
                "eslint config",
                r"/\*\s*eslint\s+[\w@/-]+\s*:",
                "fix what the ESLint rule reports instead of reconfiguring it in the file",
            ),
            ("NOSONAR", r"\bNOSONAR\b", ANALYZER_FIX),
            ("codescene disable", r"@codescene\(\s*disable", ANALYZER_FIX),
            (
                EMPTY_HANDLER,
                r"\bcatch\s*(?:\([^)]*\))?\s*\{\s*\}|\.catch\(\s*(?:(?:async\s+)?(?:\([^)]*\)|\w+)\s*=>|(?:async\s+)?function\s*\w*\s*\([^)]*\))\s*\{\s*\}\s*\)",
                "handle the error, or say in a comment inside the handler why ignoring it is right",
            ),
        ],
        test_idioms: Some(TestIdioms {
            rows: &["ts-expect-error", DOUBLE_CAST, EMPTY_HANDLER],
            code: TestCode::Files,
        }),
    },
];

/// Every language name this section selects a file set by, with the extensions each selects.
pub fn language_extensions() -> Vec<(&'static str, String)> {
    markers::language_extensions(&KIND)
}

/// How many files the escape gate judges in the working tree. Spec 11.6.
pub fn judges(project: &Project) -> Result<usize, Error> {
    markers::judges(&KIND, project)
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
    ride_along: RIDE_ALONG,
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
        rides_along,
    },
};

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    markers::gate(&KIND, at, out)
}

fn rides_along(values: &Values) -> bool {
    values
        .get(LABEL)
        .and_then(Value::as_str)
        .is_some_and(|name| RIDE_ALONG.contains(&name))
}

fn show(values: &Values) -> String {
    markers::show(LABEL, values)
}
