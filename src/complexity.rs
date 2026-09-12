use std::path::{Path, PathBuf};

use serde_json::Value;
use tree_sitter::Node;

use crate::base;
use crate::ceiling::{self, Ceiling};
use crate::config::{Config, Error, Flags};
use crate::coverage::{self, Files};
use crate::files;
use crate::ratchet::{self, Evaluator, Finding, Values};
use crate::reference::{self, Key};
use crate::syntax::{self, Language, LanguageId, Parsed, ParsedFile, Unparsed};

pub const SECTION: &str = "complexity";

/// The keys this section reads, which `klin reference` prints. Spec 5.4, 5.8.
pub const KEYS: &[Key] = &[
    reference::ROOTS.required(),
    reference::LANGUAGES
        .pinned()
        .defaulting("every language the table below names"),
    CC,
    LINES,
    reference::EXCLUDE,
    reference::SKIP_DIRS,
    reference::EXCLUDE_EXCEPT,
];

/// The keys of this section the survey supplies, named off the declarations above so the two
/// cannot spell one key differently. These are the keys above that carry a rule, restated
/// because the survey merges a section key by key and the reference only prints it. Spec 5.4.
pub const DERIVED: &[&str] = &[reference::ROOTS.name, CC.name, LINES.name];

/// The object both ceilings live in, which each key below names its path through.
pub const CEILINGS: &str = "ceilings";

pub const CC: Key = Key {
    name: "ceilings.cc",
    holds: "the cyclomatic complexity a function may not pass",
    required: true,
    rule: Some(
        "the 95th percentile of `cc` over every function under the derivation commit's roots, \
                rounded up to the next whole number, with a floor of 5, and the floor itself below 50 \
                functions",
    ),
    default: "",
};

pub const LINES: Key = Key {
    name: "ceilings.lines",
    holds: "the body length a function may not pass",
    required: true,
    rule: Some(
        "the 95th percentile of `lines`, by the same rule as `ceilings.cc`, with a floor of 25",
    ),
    default: "",
};

const ECMASCRIPT_DECISIONS: &[&str] = &[
    "if_statement",
    "while_statement",
    "do_statement",
    "for_statement",
    "for_in_statement",
    "switch_case",
    "catch_clause",
    "ternary_expression",
];

const ECMASCRIPT_OPERATORS: &[&str] = &["&&", "||", "??"];

const ACCESSOR_HOLDERS: &[&str] = &[
    "computed_property",
    "subscript_declaration",
    "property_declaration",
    "willset_didset_block",
];

const FALL_THROUGH_ARMS: &[(&str, &str)] = &[
    ("switch_label", "default"),
    ("switch_entry", "default_keyword"),
    ("when_entry", "else"),
];

const CATCH_ALL_PATTERNS: &[&str] = &["match_pattern", "case_pattern"];

/// What this check counts in one language: the node kinds that branch, and the operators that
/// branch without a node of their own. This is complexity's own policy, not syntax, which is
/// why it stays here and is keyed by the logical language rather than by a grammar variant.
/// ADR 0001, ADR 0035.
struct Metrics {
    decisions: &'static [&'static str],
    operators: &'static [&'static str],
}

const METRICS: &[(LanguageId, Metrics)] = &[
    (
        LanguageId::Rust,
        Metrics {
            decisions: &[
                "if_expression",
                "while_expression",
                "loop_expression",
                "for_expression",
                "match_arm",
                "try_expression",
            ],
            operators: &["&&", "||"],
        },
    ),
    (
        LanguageId::Python,
        Metrics {
            decisions: &[
                "if_statement",
                "elif_clause",
                "while_statement",
                "for_statement",
                "for_in_clause",
                "if_clause",
                "except_clause",
                "case_clause",
                "conditional_expression",
                "boolean_operator",
                "assert_statement",
            ],
            operators: &[],
        },
    ),
    (
        LanguageId::TypeScript,
        Metrics {
            decisions: ECMASCRIPT_DECISIONS,
            operators: ECMASCRIPT_OPERATORS,
        },
    ),
    (
        LanguageId::JavaScript,
        Metrics {
            decisions: ECMASCRIPT_DECISIONS,
            operators: ECMASCRIPT_OPERATORS,
        },
    ),
    (
        LanguageId::Go,
        Metrics {
            decisions: &[
                "if_statement",
                "for_statement",
                "expression_case",
                "type_case",
                "communication_case",
            ],
            operators: &["&&", "||"],
        },
    ),
    (
        LanguageId::Java,
        Metrics {
            decisions: &[
                "if_statement",
                "while_statement",
                "do_statement",
                "for_statement",
                "enhanced_for_statement",
                "switch_label",
                "catch_clause",
                "ternary_expression",
            ],
            operators: &["&&", "||"],
        },
    ),
    (
        LanguageId::Ruby,
        Metrics {
            decisions: &[
                "if",
                "elsif",
                "unless",
                "while",
                "until",
                "for",
                "when",
                "in_clause",
                "rescue",
                "conditional",
                "if_modifier",
                "unless_modifier",
                "while_modifier",
                "until_modifier",
                "rescue_modifier",
            ],
            operators: &["&&", "||", "and", "or"],
        },
    ),
    (
        LanguageId::Swift,
        Metrics {
            decisions: &[
                "if_statement",
                "guard_statement",
                "while_statement",
                "repeat_while_statement",
                "for_statement",
                "switch_entry",
                "catch_block",
                "ternary_expression",
                "conjunction_expression",
                "disjunction_expression",
                "nil_coalescing_expression",
            ],
            operators: &[],
        },
    ),
    (
        LanguageId::Kotlin,
        Metrics {
            decisions: &[
                "if_expression",
                "when_entry",
                "while_statement",
                "do_while_statement",
                "for_statement",
                "catch_block",
            ],
            operators: &["&&", "||", "?:"],
        },
    ),
];

/// What no table names, which is every language the parser registry holds and this check has
/// no metric table for. A function in one is counted, and nothing in it branches.
static NOTHING: Metrics = Metrics {
    decisions: &[],
    operators: &[],
};

fn metrics(id: LanguageId) -> &'static Metrics {
    METRICS
        .iter()
        .find(|(held, _)| *held == id)
        .map_or(&NOTHING, |(_, table)| table)
}

#[derive(clap::Args)]
pub struct Args {
    /// The klin.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Print nothing on success
    #[arg(long)]
    quiet: bool,
    /// Fail when an accepted entry matches nothing — what CI runs
    #[arg(long)]
    strict: bool,
    /// Judge only these repo-relative files, against only their functions at the base
    #[arg(long, num_args = 0.., value_name = "FILE")]
    only: Option<Vec<String>>,
}

struct Function {
    file: String,
    line: u64,
    end: u64,
    cc: u64,
    text: String,
    body: u64,
}

impl Function {
    fn length(&self) -> u64 {
        self.end - self.line + 1
    }

    fn over(&self, ceilings: &Ceilings) -> bool {
        self.cc > ceilings.cc.value || self.length() > ceilings.lines.value
    }

    fn finding(&self) -> Finding {
        let mut values = Values::new();
        values.insert("cc".into(), self.cc.into());
        values.insert("lines".into(), self.length().into());
        Finding {
            file: self.file.clone(),
            line: self.line,
            text: self.text.clone(),
            values,
            body: Some(self.body),
        }
    }
}

struct Ceilings {
    cc: Ceiling,
    lines: Ceiling,
}

/// One tree walked: its functions, the files no grammar read, and the files the walk reached,
/// which is what the gate's coverage counts.
struct Sweep {
    functions: Vec<Function>,
    unparsed: Vec<Unparsed>,
    files: Files,
}

#[derive(Clone)]
struct Selection {
    languages: Vec<&'static Language>,
    skip_dirs: Vec<String>,
    exclude: Vec<String>,
    exclude_except: Vec<PathBuf>,
}

struct Spec {
    roots: Vec<PathBuf>,
    selection: Selection,
    ceilings: Ceilings,
    gate_text: String,
    /// The two ceilings as one line, which every failure names beside its values. Spec 8.6.
    ceiling_text: String,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    evaluate(&flags(args), start, out)
}

pub fn gate(flags: &Flags, start: &Path, out: &mut String) -> Result<u8, Error> {
    evaluate(flags, start, out)
}

fn evaluate(flags: &Flags, start: &Path, out: &mut String) -> Result<u8, Error> {
    let config = Config::open(flags, start)?;
    let spec = spec(&config)?;
    config.say(flags, SECTION, out);
    let sweep = measure(&spec.roots, &spec.selection, config.root())?;
    let now = over(&sweep.functions, &spec);
    let judged = scoped(sweep.functions.iter().map(|function| &function.file), flags);
    let count = scoped(now.iter().map(|finding| &finding.file), flags);
    let said = sweep.files.coverage(flags.only.as_deref()).said(flags);
    let (prior, before) = at_the_base(&config, &spec, flags, out)?;
    let lost = sweep.files.lost(&before, &config, flags.only.as_deref());
    let code = evaluator(&spec).evaluate(
        now,
        prior,
        ratchet::accepted(&config, &flags.gate, evaluator(&spec).metrics)?,
        flags,
        &format!(
            "OK: {judged} function(s) judged, {count} over the gate{}, all held at the base{said}",
            ceiling::in_force(&[("cc", &spec.ceilings.cc), ("lines", &spec.ceilings.lines)])
        ),
        out,
    );
    let code = coverage::lost_said(&lost, flags, code, out);
    Ok(syntax::unread(&sweep.unparsed, flags, code, out))
}

fn at_the_base(
    config: &Config,
    spec: &Spec,
    flags: &Flags,
    out: &mut String,
) -> Result<(Vec<Finding>, Files), Error> {
    let owned;
    let prior = match flags.prior.as_deref() {
        Some(dir) => dir,
        None => {
            owned = base::own(config, flags, out)?;
            owned.root()
        }
    };
    let selection = Selection {
        exclude: files::base_exclusions(config, SECTION, prior, &spec.selection.exclude),
        ..spec.selection.clone()
    };
    let before = measure(&base::roots(&spec.roots, config, prior)?, &selection, prior)?;
    let mut found = over(&before.functions, spec);
    found.retain(|finding| config.was_held(&finding.file));
    Ok((found, before.files))
}

fn over(functions: &[Function], spec: &Spec) -> Vec<Finding> {
    functions
        .iter()
        .filter(|function| function.over(&spec.ceilings))
        .map(Function::finding)
        .collect()
}

fn flags(args: &Args) -> Flags {
    Flags {
        config: args.config.clone(),
        gate: SECTION.to_string(),
        prior: None,
        base: None,
        quiet: args.quiet,
        context: !args.quiet,
        strict: args.strict,
        hook: false,
        only: args.only.clone(),
        records: None,
        with: None,
    }
}

fn scoped<'a>(files: impl Iterator<Item = &'a String>, flags: &Flags) -> usize {
    match flags.only.as_deref() {
        Some(only) => files.filter(|file| only.contains(file)).count(),
        None => files.count(),
    }
}

fn evaluator(spec: &Spec) -> Evaluator<'_> {
    Evaluator {
        metrics: &["cc", "lines"],
        unit: "function(s)",
        condition: &spec.gate_text,
        ceiling: Some(&spec.ceiling_text),
        fix_advice: "Split the function so each piece is under the gate. Accepting new debt is a \
                     policy decision for a person, in the config, in a reviewed commit.",
        format_metrics: show,
    }
}

fn spec(config: &Config) -> Result<Spec, Error> {
    let section = ratchet::section(config, SECTION)?;
    let values = &section.values;
    let ceilings = ceilings(section.config, values)?;
    Ok(Spec {
        roots: files::roots(section.config, section.name, values, reference::ROOTS)?
            .ok_or_else(|| section.config.missing(section.name, reference::ROOTS.name))?,
        selection: selection(section.config, values)?,
        gate_text: format!(
            "over the complexity gate (cyclomatic > {}{} or body > {} lines{})",
            ceilings.cc.value,
            ceilings.cc.note(),
            ceilings.lines.value,
            ceilings.lines.note()
        ),
        ceiling_text: format!("cc {}, lines {}", ceilings.cc, ceilings.lines),
        ceilings,
    })
}

fn selection(config: &Config, section: &Values) -> Result<Selection, Error> {
    let named = files::strings(config, SECTION, section, reference::LANGUAGES)?;
    Ok(Selection {
        languages: languages(config, &named)?,
        skip_dirs: files::skip_dirs(config, SECTION, section)?,
        exclude: files::strings(config, SECTION, section, reference::EXCLUDE)?,
        exclude_except: files::roots(config, SECTION, section, reference::EXCLUDE_EXCEPT)?
            .unwrap_or_default(),
    })
}

fn languages(config: &Config, named: &[String]) -> Result<Vec<&'static Language>, Error> {
    if named.is_empty() {
        return Ok(syntax::LANGUAGES.iter().collect());
    }
    let mut out: Vec<&'static Language> = Vec::new();
    for name in named {
        let matching = syntax::LANGUAGES
            .iter()
            .filter(|language| language.names.contains(&name.as_str()));
        let mut found = false;
        for language in matching {
            found = true;
            if !out.iter().any(|held| std::ptr::eq(*held, language)) {
                out.push(language);
            }
        }
        if !found {
            return Err(unknown_language(config, name));
        }
    }
    Ok(out)
}

fn unknown_language(config: &Config, name: &str) -> Error {
    let mut known: Vec<&str> = syntax::LANGUAGES
        .iter()
        .flat_map(|language| language.names.iter().copied())
        .collect();
    known.sort_unstable();
    known.dedup();
    Error(format!(
        "{}: \"{SECTION}\" measures no language called \"{name}\" — one of: {}",
        config.file.display(),
        known.join(", ")
    ))
}

fn ceilings(config: &Config, section: &Values) -> Result<Ceilings, Error> {
    let listed = section
        .get(CEILINGS)
        .ok_or_else(|| config.missing(SECTION, CEILINGS))?
        .as_object()
        .ok_or_else(|| {
            config.malformed(
                SECTION,
                CEILINGS,
                &format!("an object of \"{}\" and \"{}\"", CC.inner(), LINES.inner()),
            )
        })?;
    let ceiling = |key: Key| {
        let value = listed
            .get(key.inner())
            .ok_or_else(|| config.missing(SECTION, key.name))?;
        ceiling::read(config, SECTION, key.name, value, "a whole number")
    };
    Ok(Ceilings {
        cc: ceiling(CC)?,
        lines: ceiling(LINES)?,
    })
}

fn measure(roots: &[PathBuf], selection: &Selection, repo_root: &Path) -> Result<Sweep, Error> {
    let extensions: Vec<&str> = selection
        .languages
        .iter()
        .flat_map(|language| language.extensions)
        .copied()
        .collect();
    let mut out = Vec::new();
    let mut unparsed = Vec::new();
    let wanted = files::Wanted {
        extensions: &extensions,
        skip_dirs: &selection.skip_dirs,
        exclude: &selection.exclude,
        exclude_except: &selection.exclude_except,
        skip_hidden: true,
    };
    let found = files::found(roots, &wanted)?;
    let mut read: Vec<String> = Vec::new();
    for file in found.kept {
        let name = file.to_string_lossy().to_string();
        let Some(language) = selection.languages.iter().find(|language| {
            language
                .extensions
                .iter()
                .any(|extension| name.ends_with(extension))
        }) else {
            continue;
        };
        out.extend(functions(&file, repo_root, language, &mut unparsed)?);
        read.push(files::relative(&file, repo_root));
    }
    out.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    read.retain(|file| !unparsed.iter().any(|unread| &unread.file == file));
    let files = Files {
        measured: read,
        excluded: found
            .excluded
            .iter()
            .map(|file| files::relative(file, repo_root))
            .collect(),
        unreadable: unparsed.iter().map(|file| file.file.clone()).collect(),
    };
    Ok(Sweep {
        functions: out,
        unparsed,
        files,
    })
}

fn functions(
    path: &Path,
    repo_root: &Path,
    language: &'static Language,
    unparsed: &mut Vec<Unparsed>,
) -> Result<Vec<Function>, Error> {
    let bytes = std::fs::read(path).map_err(|why| Error::unreadable(path, why))?;
    let source = String::from_utf8_lossy(&bytes).to_string();
    let file = files::relative(path, repo_root);
    match syntax::read(&file, &source, language)? {
        Parsed::Read(read) => Ok(parsed(&read)),
        Parsed::Rejected(refused) => {
            unparsed.push(refused);
            Ok(Vec::new())
        }
    }
}

/// Every function one parse holds, with the two numbers this check ratchets.
fn parsed(file: &ParsedFile) -> Vec<Function> {
    let at = Walked {
        language: file.language,
        metrics: metrics(file.language.id),
        file: file.path,
        source: file.source,
        lines: file.lines(),
    };
    let mut out = Vec::new();
    collect(file.root(), &at, &mut out);
    out
}

/// What one function comes to under this check, for a caller that measures a tree it does not
/// judge. The two numbers a ceiling names, and nothing about where the function sits.
pub struct Measured {
    pub cc: u64,
    pub lines: u64,
}

/// The cyclomatic complexity and body length of every function in one source text, for the
/// percentile the survey takes over the derivation commit. Nothing for a path no grammar here
/// reads, and nothing for a text the grammar rejects. Spec 5.4.
pub fn measured(path: &str, source: &str) -> Vec<Measured> {
    let Ok(Some(Parsed::Read(file))) = syntax::parse(path, source) else {
        return Vec::new();
    };
    parsed(&file)
        .iter()
        .map(|function| Measured {
            cc: function.cc,
            lines: function.length(),
        })
        .collect()
}

/// One tree being walked, with everything the walk reads off the language and this check.
struct Walked<'a> {
    language: &'static Language,
    metrics: &'static Metrics,
    file: &'a str,
    source: &'a str,
    lines: Vec<&'a str>,
}

fn collect(node: Node, at: &Walked, out: &mut Vec<Function>) {
    if at.language.functions.contains(&node.kind()) && !holds_a_body(node, at.language) {
        out.push(Function {
            file: at.file.to_string(),
            line: node.start_position().row as u64 + 1,
            end: node.end_position().row as u64 + 1,
            cc: 1 + decisions(node, at),
            text: site(node, &at.lines),
            body: ratchet::body_hash(node.utf8_text(at.source.as_bytes()).unwrap_or_default()),
        });
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, at, out);
    }
}

fn decisions(node: Node, at: &Walked) -> u64 {
    let mut count = 0;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if at.language.functions.contains(&child.kind()) {
            continue;
        }
        let table = if child.is_named() {
            at.metrics.decisions
        } else {
            at.metrics.operators
        };
        count += u64::from(table.contains(&child.kind()) && !falls_through(child))
            + decisions(child, at);
    }
    count
}

fn holds_a_body(node: Node, language: &Language) -> bool {
    ACCESSOR_HOLDERS.contains(&node.kind())
        && has_child(node, |kind| language.functions.contains(&kind))
}

fn falls_through(node: Node) -> bool {
    FALL_THROUGH_ARMS
        .iter()
        .any(|(arm, marker)| node.kind() == *arm && has_child(node, |kind| kind == *marker))
        || catches_all(node)
}

fn catches_all(node: Node) -> bool {
    let mut cursor = node.walk();
    let mut patterns = node
        .children(&mut cursor)
        .filter(|child| CATCH_ALL_PATTERNS.contains(&child.kind()));
    let Some(pattern) = patterns.next() else {
        return false;
    };
    patterns.next().is_none()
        && pattern.child_count() == 1
        && pattern.child(0).is_some_and(|only| only.kind() == "_")
}

fn site(node: Node, lines: &[&str]) -> String {
    let row = node.start_position().row;
    match holder_row(node) {
        Some(holder) if holder < row => {
            format!(
                "{} {}",
                syntax::line_at(lines, holder),
                syntax::line_at(lines, row)
            )
        }
        _ => syntax::line_at(lines, row),
    }
}

fn holder_row(node: Node) -> Option<usize> {
    let mut row = None;
    let mut above = node.parent();
    while let Some(holder) = above.filter(|above| ACCESSOR_HOLDERS.contains(&above.kind())) {
        row = Some(holder.start_position().row);
        above = holder.parent();
    }
    row
}

fn has_child(node: Node, wanted: impl Fn(&str) -> bool) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| wanted(child.kind()))
}

fn show(values: &Values) -> String {
    let number = |key: &str| {
        values
            .get(key)
            .and_then(Value::as_u64)
            .map_or("?".to_string(), |value| value.to_string())
    };
    format!("cc {}, {} lines", number("cc"), number("lines"))
}
