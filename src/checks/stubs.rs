use crate::checks::markers::{self, Kind, Language};
use crate::config::key::Key;
use crate::config::scope;
use crate::contract::check::{Context, Sink};
use crate::contract::ratchet::{Evaluator, Remedy};
use crate::syntax::LanguageId;
use crate::sys::error::Error;
use crate::sys::record::Values;

pub const SECTION: &str = "stubs";

/// The keys this section reads, which `klin policy --reference` prints. Spec 5.4, 5.8.
const KEYS: &[Key] = &[scope::IN, scope::EXCEPT];

const LABEL: &str = "stub";

const BODY: &str = "implement the body";
const NOTE: &str = "do the work the comment names, or record it in the tracker and delete the \
                    comment";

/// The row whose matches in one file are one site, because a comment is not a declaration.
/// ADR 0064.
const MARKER: &str = "comment marker";

/// A comment marker in a language whose comments start with `//` or `/*`.
const SLASH: &str = r"(?://|/\*)[^\n]*\b(?:TODO|FIXME|XXX|HACK)\b";
/// The same, for a language whose comments start with `#`.
const HASH: &str = r"#[^\n]*\b(?:TODO|FIXME|XXX|HACK)\b";

/// The markers of spec 8.2 that one line states. A body only a parser can judge, such as `pass`
/// as the sole body of a function, is not here: `syntax::convention` reads it from the function
/// walk, and `reads_shapes` puts what it finds on the same sites. #114.
const LANGUAGES: &[Language] = &[
    Language {
        languages: &[LanguageId::Go],
        patterns: &[
            (
                "not implemented",
                r#"panic\(\s*"[^"]*(?i:not implemented)"#,
                BODY,
            ),
            (MARKER, SLASH, NOTE),
        ],
        test_idioms: None,
    },
    Language {
        languages: &[LanguageId::Python],
        patterns: &[
            ("not implemented", r"\braise\s+NotImplementedError", BODY),
            (MARKER, HASH, NOTE),
        ],
        test_idioms: None,
    },
    Language {
        languages: &[LanguageId::Rust],
        patterns: &[
            ("not implemented", r"\b(?:todo|unimplemented)!\(", BODY),
            (MARKER, SLASH, NOTE),
        ],
        test_idioms: None,
    },
    Language {
        languages: &[LanguageId::TypeScript, LanguageId::JavaScript],
        patterns: &[
            (
                "not implemented",
                r#"throw new [A-Za-z]*Error\(\s*['"`][^'"`]*(?i:not implemented)"#,
                BODY,
            ),
            (MARKER, SLASH, NOTE),
        ],
        test_idioms: None,
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
    skips_literals: true,
    reads_shapes: true,
    reads_cfg_attr: false,
    keyed_by_row: &[MARKER],
    evaluator: Evaluator {
        metrics: &["count"],
        unit: "stub site(s)",
        condition: "where the code stands in for work nobody did",
        fix_advice: Remedy::Fixed(
            "Do what the marker stands in for. A placeholder an agent left behind is \
             not work, and accepting one is a decision for a person, in the config, in \
             a reviewed commit.",
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
