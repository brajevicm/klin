//! A literal shaped like a live vendor token, and the fingerprint that stands in for it. The
//! `secrets` check counts with the strict pattern. The ratchet, `sys::out` and `hook::record`
//! redact with the loose one. Spec 9.1, 16.3, ADR 0075.

use std::borrow::Cow;
use std::sync::LazyLock;

use regex::Regex;

use crate::sys::error::Error;
use crate::sys::site;

/// One vendor's token: the name a report gives it, its prefix, the rest at its usual length, and
/// the rest as redaction reads it, which holds as few as 8 characters, so a key that a cut left
/// short is still replaced.
pub struct Vendor {
    pub name: &'static str,
    prefix: &'static str,
    rest: &'static str,
    cut: &'static str,
}

const VENDORS: &[Vendor] = &[
    Vendor {
        name: "OpenAI project key",
        prefix: "sk-proj-",
        rest: r"[A-Za-z0-9_-]{40,}",
        cut: r"[A-Za-z0-9_-]{8,}",
    },
    Vendor {
        name: "Anthropic key",
        prefix: "sk-ant-",
        rest: r"[a-z]+[0-9]{2}-[A-Za-z0-9_-]{80,}",
        cut: r"[A-Za-z0-9_-]{8,}",
    },
    Vendor {
        name: "Stripe live key",
        prefix: "sk_live_",
        rest: r"[A-Za-z0-9]{24,}",
        cut: r"[A-Za-z0-9]{8,}",
    },
    Vendor {
        name: "GitHub token",
        prefix: "ghp_",
        rest: r"[A-Za-z0-9]{36}\b",
        cut: r"[A-Za-z0-9]{8,}",
    },
    Vendor {
        name: "GitHub fine-grained token",
        prefix: "github_pat_",
        rest: r"[A-Za-z0-9_]{82}\b",
        cut: r"[A-Za-z0-9_]{8,}",
    },
    Vendor {
        name: "AWS access key",
        prefix: "AKIA",
        rest: r"[A-Z0-9]{16}\b",
        cut: r"[A-Z0-9]{8,}",
    },
    Vendor {
        name: "Slack token",
        prefix: "xox",
        rest: r"[abposr]-[0-9]{8,}-[A-Za-z0-9-]{20,}",
        cut: r"[abposr]-[A-Za-z0-9-]{8,}",
    },
    Vendor {
        name: "Google API key",
        prefix: "AIza",
        rest: r"[A-Za-z0-9_-]{35}",
        cut: r"[A-Za-z0-9_-]{8,}",
    },
    Vendor {
        name: "GitLab token",
        prefix: "glpat-",
        rest: r"[A-Za-z0-9_-]{20,}",
        cut: r"[A-Za-z0-9_-]{8,}",
    },
];

/// The words a documented example or a test token holds, which no live key does.
const PLACEHOLDERS: &[&str] = &["test", "xxx", "example"];

/// The gate whose site identity a fingerprint's hash shares, so a fingerprint stays the text of
/// a `secrets` site.
const SECTION: &str = "secrets";

type Pattern = LazyLock<Result<Regex, String>>;

/// One pattern for every vendor, so a text is scanned once, with one group per vendor in table
/// order. A token starts at a word boundary, so the `secrets` gate counts no key inside a longer
/// word.
static BOUNDED: Pattern = LazyLock::new(|| compile(r"\b", |vendor| vendor.rest));

/// The pattern redaction reads: no boundary before a token, so a key that a word character runs
/// into, such as the `n` of a newline that JSON escapes, is still replaced, and the short rest,
/// so a key that an excerpt or a clip cut before the text reached a boundary is replaced too.
static LOOSE: Pattern = LazyLock::new(|| compile("", |vendor| vendor.cut));

fn compile(before: &str, rest: fn(&Vendor) -> &'static str) -> Result<Regex, String> {
    let vendors: Vec<String> = VENDORS
        .iter()
        .map(|vendor| format!("({}{})", regex::escape(vendor.prefix), rest(vendor)))
        .collect();
    Regex::new(&format!("{before}(?:{})", vendors.join("|"))).map_err(|why| why.to_string())
}

/// The pattern, or the error that names the broken vendor table.
fn pattern(compiled: &'static Pattern) -> Result<&'static Regex, Error> {
    compiled.as_ref().map_err(|why| {
        Error(format!(
            "the {SECTION} vendor table is not a regular expression: {why}"
        ))
    })
}

/// A token's prefix and a hash of the whole token, which holds no part of the key after the
/// prefix.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Fingerprint(String);

impl Fingerprint {
    fn of(vendor: &Vendor, key: &str) -> Fingerprint {
        let hash = site::id(SECTION, vendor.name, key);
        Fingerprint(format!("{}...{hash}", vendor.prefix))
    }
}

impl From<Fingerprint> for String {
    fn from(fingerprint: Fingerprint) -> String {
        fingerprint.0
    }
}

/// One live-shaped token in a text: its vendor, its byte range, its fingerprint, and whether it
/// looks like an example. A token that holds a placeholder word, or that an ellipsis cuts short,
/// looks like an example, which the `secrets` gate does not count and redaction still replaces.
pub struct Live {
    pub vendor: &'static Vendor,
    pub start: usize,
    pub end: usize,
    pub fingerprint: Fingerprint,
    pub example: bool,
}

/// Each live-shaped token in the text, in order, or the error that names a broken vendor table.
pub fn live(text: &str) -> Result<impl Iterator<Item = Live> + '_, Error> {
    Ok(found(pattern(&BOUNDED)?, text).map(|mut token| {
        let key = &text[token.start..token.end];
        token.example = placeholder(key) || text[token.end..].starts_with("...");
        token
    }))
}

fn found<'t>(pattern: &'t Regex, text: &'t str) -> impl Iterator<Item = Live> + 't {
    pattern.captures_iter(text).filter_map(|captures| {
        let (at, key) = captures
            .iter()
            .skip(1)
            .enumerate()
            .find_map(|(at, group)| Some((at, group?)))?;
        let vendor = &VENDORS[at];
        Some(Live {
            vendor,
            start: key.start(),
            end: key.end(),
            fingerprint: Fingerprint::of(vendor, key.as_str()),
            example: false,
        })
    })
}

fn placeholder(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    PLACEHOLDERS.iter().any(|word| lower.contains(word))
}

/// The text with each live-shaped token replaced by its fingerprint, an example included, because
/// a live key may hold a placeholder word by chance. A text that holds none comes back as it
/// was. A broken vendor table also leaves the text as it was. The `secrets` gate reports that
/// table as a tool error, but only in a run that selects that gate.
pub fn redact(text: &str) -> Cow<'_, str> {
    let Ok(anywhere) = pattern(&LOOSE) else {
        return Cow::Borrowed(text);
    };
    let tokens = found(anywhere, text);
    let mut out = String::new();
    let mut from = 0;
    for token in tokens {
        if from == 0 {
            out.reserve(text.len());
        }
        out.push_str(&text[from..token.start]);
        out.push_str(&token.fingerprint.0);
        from = token.end;
    }
    match from {
        0 => Cow::Borrowed(text),
        _ => {
            out.push_str(&text[from..]);
            Cow::Owned(out)
        }
    }
}

/// The text with each live-shaped token replaced by its fingerprint, changed only where it holds
/// one.
pub fn redact_in_place(text: &mut String) {
    if let Cow::Owned(safe) = redact(text) {
        *text = safe;
    }
}
