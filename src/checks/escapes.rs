use serde_json::Value;

use crate::checks::markers::{self, Kind, Language, TestCode, TestIdioms};
use crate::config::key::Key;
use crate::config::scope;
use crate::contract::check::{Context, Sink};
use crate::contract::project::Project;
use crate::contract::ratchet::{self, Evaluator, Remedy};
use crate::syntax::LanguageId;
use crate::sys::error::Error;
use crate::sys::record::Values;

pub const SECTION: &str = "escapes";

/// The keys this section reads, which `klin policy --reference` prints. Spec 5.4, 5.8.
const KEYS: &[Key] = &[scope::IN, scope::EXCEPT, markers::SKIP_TEST_IDIOMS];

const LABEL: &str = "escape";

const SHARED_REMEDY: &str = "Swallowing an error in place of the escape is not a fix. Accepting \
                             a new escape is a policy decision for a person, in the config, in a \
                             reviewed commit.";

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
    evaluator: Evaluator {
        metrics: &["count"],
        unit: "escape site(s)",
        condition: "where the code opts out of a check",
        fix_advice: Remedy::ByValues(remedy),
        ceiling: None,
        format_metrics: show,
        nested: None,
        rides_along: ratchet::every_row_blocks,
    },
};

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    markers::gate(&KIND, at, out)
}

fn show(values: &Values) -> String {
    markers::show(LABEL, values)
}

/// One remedy per kind of escape that failed, in the order each first failed, then what every
/// escape shares. A worsened site whose values name no kind gets the remedy of every kind.
/// Spec 10.6.
fn remedy(failed: &[&Values]) -> String {
    let mut said: Vec<&str> = Vec::new();
    for values in failed {
        let kind = kind_remedy(values.get(LABEL).and_then(Value::as_str));
        if !said.contains(&kind) {
            said.push(kind);
        }
    }
    said.push(SHARED_REMEDY);
    said.join(" ")
}

fn kind_remedy(kind: Option<&str>) -> &'static str {
    match kind {
        Some("unwrap" | "expect" | "force try" | "bare except" | "errors ignored") => {
            "Handle the error instead of unwrapping or ignoring it."
        }
        Some(
            "any" | "non-null assertion" | "not-null assertion" | "force unwrap" | "force cast"
            | "type ignore" | "ts-ignore" | "ts-expect-error" | "unchecked Sendable",
        ) => {
            "Give the value its real type, or narrow it with a check the type checker sees, \
             instead of asserting a type or silencing the type error."
        }
        Some(
            "nolint" | "suppress warnings" | "suppress" | "noqa" | "allow" | "eslint-disable"
            | "rubocop:disable" | "shellcheck disable" | "swiftlint:disable",
        ) => "Address the lint instead of allowing it.",
        Some("skipped test" | "expected failure") => {
            "Remove the skip, or fix what made the test fail."
        }
        Some("focused test") => "Remove the focus, so that every test runs.",
        Some("no cover") => "Cover the code with a test instead of excluding it from coverage.",
        Some("unsafe") => "Use a safe API in place of the unsafe block.",
        _ => {
            "Fix what the escape hides: handle the error instead of unwrapping it, address the \
             lint instead of allowing it. Remove the skip, or fix what made the test fail."
        }
    }
}
