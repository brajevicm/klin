use std::cell::OnceCell;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use serde_json::{Map, Value};
use tree_sitter::Node;

use crate::base;
use crate::ceiling::{self, Ceiling};
use crate::changed::Change;
use crate::check::contract::{self, ContentCost, Context, Line, Sink};
use crate::check::holes;
use crate::coverage::Files;
use crate::error::Error;
use crate::files;
use crate::key::Key;
use crate::project::Project;
use crate::ratchet::{self, Evaluator, Finding, Remedy};
use crate::record::{self, Values};
use crate::scope::{self, Scope};
use crate::syntax::{self, Language, LanguageId, Parsed, ParsedFile, Unparsed};
use crate::tree::Tree;
use crate::{cache, changed, survey};

pub const SECTION: &str = "complexity";
const CC_FLOOR: u64 = 10;
const LINES_FLOOR: u64 = 25;
const SAMPLE_SIZE: usize = 50;
const PERCENTILE: usize = 95;

/// The keys this section reads, which `klin policy --reference` prints. Spec 5.4, 5.8.
pub const KEYS: &[Key] = &[CC, LINES, TEST_LINES, scope::IN, scope::EXCEPT];

pub const CC: Key = Key {
    name: "cc",
    holds: "the cyclomatic complexity a function may not pass",
    required: false,
    rule: Some(
        "the 95th percentile of `cc` over every supported function selected by the compact \
         scope recorded at the derivation commit, rounded up to the next whole number, with a \
         floor of 10, and the floor itself below 50 functions",
    ),
    default: "",
    shape: crate::key::Shape::Ceiling,
};

pub const LINES: Key = Key {
    name: "lines",
    holds: "the body length a function outside test code may not pass",
    required: false,
    rule: Some(
        "the 95th percentile of `lines`, by the same rule as `cc`, test code included, with a \
         floor of 25",
    ),
    default: "",
    shape: crate::key::Shape::Ceiling,
};

pub const TEST_LINES: Key = Key {
    name: "test_lines",
    holds: "the body length a function in test code may not pass: in a test file of spec 5.4, \
            or in a Rust item marked `#[cfg(test)]`, such as an inline test module",
    required: false,
    rule: None,
    default: "test code is not judged on length",
    shape: crate::key::Shape::Ceiling,
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

const SUITE_CONTAINERS: &[&str] = &["describe", "context", "suite", "fdescribe", "xdescribe"];
const SUITE_BASES: &[&str] = &["describe", "context", "suite"];
const SUITE_DIRECT_METHODS: &[&str] = &["only", "skip"];
const SUITE_CALLBACK_WRAPPERS: &[&str] = &[
    "parenthesized_expression",
    "as_expression",
    "satisfies_expression",
    "non_null_expression",
    "type_assertion",
];

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

struct Function {
    file: String,
    line: u64,
    end: u64,
    cc: u64,
    text: Rc<str>,
    body: u64,
    test: bool,
}

impl Function {
    fn length(&self) -> u64 {
        self.end - self.line + 1
    }

    fn length_ceiling<'a>(&self, ceilings: &'a Ceilings) -> Option<&'a Ceiling> {
        match self.test {
            true => ceilings.test_lines.as_ref(),
            false => Some(&ceilings.lines),
        }
    }

    fn over(&self, ceilings: &Ceilings) -> bool {
        self.cc > ceilings.cc.value
            || self
                .length_ceiling(ceilings)
                .is_some_and(|ceiling| self.length() > ceiling.value)
    }

    fn finding(&self, ceilings: &Ceilings) -> Finding {
        let mut values = Values::new();
        values.insert(CC.name.into(), self.cc.into());
        if self.length_ceiling(ceilings).is_some() {
            let key = match self.test {
                true => TEST_LINES,
                false => LINES,
            };
            values.insert(key.name.into(), self.length().into());
        }
        Finding {
            file: self.file.clone(),
            line: self.line,
            text: self.text.to_string(),
            values,
            body: Some(self.body),
        }
    }
}

struct Ceilings {
    cc: Ceiling,
    lines: Ceiling,
    test_lines: Option<Ceiling>,
}

impl Ceilings {
    fn named(&self) -> Vec<(&'static str, &Ceiling)> {
        let mut named = vec![(CC.name, &self.cc), (LINES.name, &self.lines)];
        named.extend(
            self.test_lines
                .iter()
                .map(|ceiling| (TEST_LINES.name, ceiling)),
        );
        named
    }

    /// Every ceiling in force under its key, for the `OK:` line that names the dated steps.
    fn steps(&self) -> Vec<(&'static str, Ceiling)> {
        self.named()
            .into_iter()
            .map(|(key, ceiling)| (key, ceiling.clone()))
            .collect()
    }
}

/// One tree walked: its functions, the files no grammar read, and the files the walk reached,
/// which is what the gate's coverage counts.
struct Sweep {
    functions: Vec<Function>,
    unparsed: Vec<Unparsed>,
    files: Files,
    work: ContentCost,
}

#[derive(Clone)]
struct Selection {
    languages: Vec<&'static Language>,
    scope: Scope,
}

struct Spec {
    selection: Selection,
    ceilings: Ceilings,
    gate_text: String,
    /// The two ceilings as one line, which every failure names beside its values. Spec 8.6.
    ceiling_text: String,
    provenance: Provenance,
    notes: Notes,
}

type Provenance = Vec<(String, Option<Value>)>;
type Notes = Vec<(String, String)>;

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    let project = at.project;
    let spec = spec(project)?;
    for (line, value) in &spec.provenance {
        out.provenance(line.clone(), value.clone());
    }
    ratchet::noted_as(contract::DERIVATION, &spec.notes, out);
    let sweep = measure(
        project.tree(),
        &spec.selection,
        project.root(),
        at.changes.filter(|_| !at.strict),
        None,
    )?;
    let now = over(&sweep.functions, &spec);
    let tests = unjudged_tests(&sweep.functions, &spec.ceilings, at);
    let judged = scoped(sweep.functions.iter().map(|function| &function.file), at);
    let count = scoped(now.iter().map(|finding| &finding.file), at);
    let said = out.covered(&sweep.files.coverage(at.only));
    let mut owned = None;
    let laid = base::laid(at.prior, &mut owned, || contract::own_base(at))?;
    let (prior, before, before_work) = at_the_base(&spec, at, laid)?;
    out.record(|records| records.work = Some(sweep.work + before_work));
    let lost = sweep.files.lost(&before, project, at.only);
    let unjudged = tests.said(laid);
    let code = evaluator(&spec).evaluate(
        now,
        prior,
        ratchet::accepted_leaving_out(
            &project.config,
            at.gate,
            evaluator(&spec).metrics,
            &[LINES.name, TEST_LINES.name],
        )?,
        at,
        Line::new(
            contract::Measured::Complexity(contract::Complexity {
                judged,
                over: count,
                steps: spec.ceilings.steps(),
                unjudged: unjudged.0,
                arrived: unjudged.1,
            }),
            said,
        ),
        out,
    );
    let code = holes::lost_said(&lost, at, code, out);
    Ok(holes::unread_said(
        &sweep.unparsed,
        || laid.unread_either(&before.unreadable),
        at,
        code,
        out,
    ))
}

fn at_the_base(
    spec: &Spec,
    at: &Context,
    prior: &base::Prior,
) -> Result<(Vec<Finding>, Files, ContentCost), Error> {
    let project = at.project;
    let selection = Selection {
        scope: Scope::at_base(
            &project.config,
            SECTION,
            prior.root(),
            &spec.selection.scope,
        ),
        ..spec.selection.clone()
    };
    let before = measure(
        prior.tree(),
        &selection,
        prior.root(),
        None,
        Some(prior.renamed()),
    )?;
    let mut found = over(&before.functions, spec);
    found.retain(|finding| project.was_held(&finding.file));
    Ok((found, before.files, before.work))
}

fn over(functions: &[Function], spec: &Spec) -> Vec<Finding> {
    functions
        .iter()
        .filter(|function| function.over(&spec.ceilings))
        .map(|function| function.finding(&spec.ceilings))
        .collect()
}

struct Unjudged<'a> {
    functions: usize,
    files: BTreeSet<&'a str>,
}

fn unjudged_tests<'a>(
    functions: &'a [Function],
    ceilings: &Ceilings,
    at: &Context,
) -> Unjudged<'a> {
    let tests: Vec<&Function> = functions
        .iter()
        .filter(|function| function.test && function.length_ceiling(ceilings).is_none())
        .filter(|function| at.only.is_none_or(|only| only.contains(&function.file)))
        .collect();
    Unjudged {
        functions: tests.len(),
        files: tests
            .iter()
            .map(|function| function.file.as_str())
            .collect(),
    }
}

impl Unjudged<'_> {
    /// How many test functions were not judged on length, and the files among them a window
    /// added or renamed.
    fn said(&self, prior: &base::Prior) -> (usize, Vec<String>) {
        let arrived = self
            .files
            .iter()
            .filter(|file| prior.renamed().contains_key(**file) || prior.added().contains(**file))
            .map(|file| file.to_string())
            .collect();
        (self.functions, arrived)
    }
}

fn scoped<'a>(files: impl Iterator<Item = &'a String>, at: &Context) -> usize {
    match at.only {
        Some(only) => files.filter(|file| only.contains(file)).count(),
        None => files.count(),
    }
}

fn evaluator(spec: &Spec) -> Evaluator<'_> {
    Evaluator {
        metrics: &[CC.name, LINES.name, TEST_LINES.name],
        unit: "function(s)",
        condition: &spec.gate_text,
        ceiling: Some(&spec.ceiling_text),
        fix_advice: Remedy::Fixed(
            "Reduce the function's responsibility or decision complexity. Split at \
             coherent behavior boundaries, not into arbitrary helpers that only get \
             under the gate. Accepting new debt is a policy decision for a person, in \
             the config, in a reviewed commit.",
        ),
        format_metrics: show,
        nested: None,
    }
}

fn spec(project: &Project) -> Result<Spec, Error> {
    let config = &project.config;
    let values = config.policy(SECTION, KEYS)?;
    let selection = Selection {
        languages: syntax::LANGUAGES.iter().collect(),
        scope: Scope::read(config, SECTION, &values)?,
    };
    if selection.scope.has_in() && !applicable(project.tree(), &selection)? {
        return Err(Error(format!(
            "{}: \"{SECTION}\" has an \"in\" scope with no applicable file",
            config.file.display()
        )));
    }
    let resolved = ceilings(project, &values, &selection.scope)?;
    Ok(Spec {
        selection,
        gate_text: format!(
            "over the complexity gate (cyclomatic > {}{} or body > {} lines{}{})",
            resolved.ceilings.cc.value,
            resolved.ceilings.cc.note(),
            resolved.ceilings.lines.value,
            resolved.ceilings.lines.note(),
            resolved
                .ceilings
                .test_lines
                .as_ref()
                .map(|ceiling| format!(
                    ", or test body > {} lines{}",
                    ceiling.value,
                    ceiling.note()
                ))
                .unwrap_or_default()
        ),
        ceiling_text: resolved
            .ceilings
            .named()
            .iter()
            .map(|(key, ceiling)| format!("{key} {ceiling}"))
            .collect::<Vec<_>>()
            .join(", "),
        ceilings: resolved.ceilings,
        provenance: resolved.provenance,
        notes: resolved.notes,
    })
}

struct Resolved {
    ceilings: Ceilings,
    provenance: Provenance,
    notes: Notes,
}

fn pinned(
    project: &Project,
    key: Key,
    value: &Value,
) -> Result<(Ceiling, (String, Option<Value>)), Error> {
    let ceiling = ceiling::read(
        &project.config.file,
        SECTION,
        key.name,
        value,
        "a whole number",
    )?;
    let line = format!("pinned: {SECTION} {} {ceiling}", key.name);
    Ok((ceiling, (line, None)))
}

fn ceilings(project: &Project, section: &Values, scope: &Scope) -> Result<Resolved, Error> {
    let sample = OnceCell::new();
    let resolve = |key: Key, floor: u64, measure: fn(&Sample) -> u64| -> Result<_, Error> {
        if let Some(value) = section.get(key.name) {
            return pinned(project, key, value);
        }
        let (found, commit) = sample.get_or_init(|| derived_sample(project));
        let value = measure(found).max(floor);
        let rule = number_rule(value, floor, found.functions, commit.as_deref());
        let recorded = commit
            .as_ref()
            .map(|_| format!("; recorded scope: {}", found.scope.description()))
            .unwrap_or_default();
        let record = derived_value(key.name, value, &format!("{rule}{recorded}"));
        Ok((
            Ceiling { value, step: None },
            (
                format!("derived: {SECTION} {} {value} ({rule}){recorded}", key.name),
                Some(record),
            ),
        ))
    };
    let (cc, cc_said) = resolve(CC, CC_FLOOR, |found| found.cc)?;
    let (lines, lines_said) = resolve(LINES, LINES_FLOOR, |found| found.lines)?;
    let mut provenance = vec![cc_said, lines_said];
    let test_lines = match section.get(TEST_LINES.name) {
        Some(value) => {
            let (ceiling, said) = pinned(project, TEST_LINES, value)?;
            provenance.push(said);
            Some(ceiling)
        }
        None => None,
    };
    let file = config_name(&project.config.file);
    let notes = sample
        .get()
        .map(|(found, commit)| sample_notes(found, commit.as_deref(), scope, file))
        .unwrap_or_default();
    Ok(Resolved {
        ceilings: Ceilings {
            cc,
            lines,
            test_lines,
        },
        provenance,
        notes,
    })
}

#[derive(Clone, Default)]
struct Sample {
    cc: u64,
    lines: u64,
    functions: usize,
    scope: Scope,
    fallback: Option<String>,
}

/// The two ceilings the derivation commit gives, each with the `derived:` line that says where
/// it came from, which `init --pin` writes as policy. Nothing when there is no commit to sample,
/// because a floor alone is no measurement. Spec 5.7.
pub fn suggested(project: &Project) -> Vec<(&'static str, u64, String)> {
    let (found, Some(commit)) = derived_sample(project) else {
        return Vec::new();
    };
    [(CC, CC_FLOOR, found.cc), (LINES, LINES_FLOOR, found.lines)]
        .into_iter()
        .map(|(key, floor, measured)| {
            let value = measured.max(floor);
            let rule = number_rule(value, floor, found.functions, Some(&commit));
            (
                key.name,
                value,
                format!("derived: {SECTION} {} {value} ({rule})", key.name),
            )
        })
        .collect()
}

fn derived_sample(project: &Project) -> (Sample, Option<String>) {
    let Some((_, commit, at)) = project.source_derivation() else {
        return (Sample::default(), None);
    };
    let cached = at
        .and_then(|at| cache::read(at, commit, SECTION))
        .and_then(|value| read_sample(&value));
    let found = cached.unwrap_or_else(|| {
        let found = sample(project.root(), commit, &project.config.file);
        if let Some(at) = at {
            cache::write(at, commit, SECTION, kept_sample(&found));
        }
        found
    });
    (found, Some(commit.to_string()))
}

fn sample(root: &Path, commit: &str, config: &Path) -> Sample {
    let (scope, fallback) = recorded_scope(root, commit, config);
    let listed = survey::listed(root, commit).unwrap_or_default();
    let paths: Vec<&str> = listed
        .iter()
        .map(String::as_str)
        .filter(|path| scope.selects(path))
        .filter(|path| {
            syntax::LANGUAGES
                .iter()
                .any(|language| language.extensions.iter().any(|end| path.ends_with(end)))
        })
        .collect();
    let mut cc = Vec::new();
    let mut lines = Vec::new();
    let read = changed::blobs(root, commit, &paths, |path, bytes| {
        let Some(bytes) = bytes else { return };
        for function in measured(path, &String::from_utf8_lossy(bytes)) {
            cc.push(function.cc);
            lines.push(function.lines);
        }
    });
    if read.is_none() {
        cc.clear();
        lines.clear();
    }
    Sample {
        functions: cc.len(),
        cc: percentile(cc, CC_FLOOR),
        lines: percentile(lines, LINES_FLOOR),
        scope,
        fallback,
    }
}

fn config_name(config: &Path) -> &str {
    config
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("klin.json")
}

fn recorded_scope(root: &Path, commit: &str, config: &Path) -> (Scope, Option<String>) {
    match recorded(root, commit, config_name(config)) {
        Ok(scope) => (scope, None),
        Err(why) => (
            Scope::default(),
            Some(format!(
                "the recorded complexity policy could not be read as compact scope because \
                 {why}; the derived ceiling used the whole repository"
            )),
        ),
    }
}

/// The compact scope one commit's configuration records: the whole repository where the file,
/// the section, `in` and `except` are absent, and why it cannot be read where one is present.
fn recorded(root: &Path, commit: &str, name: &str) -> Result<Scope, &'static str> {
    let mut blob = None;
    let read = changed::blobs(root, commit, &[name], |_, bytes| {
        blob = Some(bytes.map(<[u8]>::to_vec));
    });
    let Some(bytes) = read
        .and(blob)
        .ok_or("git could not read its configuration blob")?
    else {
        return Ok(Scope::default());
    };
    let data: Value =
        serde_json::from_slice(&bytes).map_err(|_| "its configuration is not valid JSON")?;
    let config = data
        .as_object()
        .ok_or("its configuration is not an object")?;
    let Some(section) = config.get(SECTION) else {
        return Ok(Scope::default());
    };
    let fields = section
        .as_object()
        .ok_or("its complexity section is not a compact policy object")?;
    Scope::from_fields(fields).map_err(|_| "its complexity scope is malformed")
}

fn sample_notes(sample: &Sample, commit: Option<&str>, today: &Scope, file: &str) -> Notes {
    let mut notes = sample
        .fallback
        .iter()
        .map(|why| (file.to_string(), why.clone()))
        .collect::<Vec<_>>();
    if let Some(commit) = commit
        && sample.scope != *today
    {
        notes.push((
            file.to_string(),
            format!(
                "today's complexity scope ({}) differs from the scope recorded at {} ({}); the \
                 recorded scope derived the ceiling",
                today.description(),
                short(commit),
                sample.scope.description()
            ),
        ));
    }
    notes
}

fn percentile(mut values: Vec<u64>, floor: u64) -> u64 {
    if values.len() < SAMPLE_SIZE {
        return floor;
    }
    values.sort_unstable();
    floor.max(values[(values.len() * PERCENTILE).div_ceil(100) - 1])
}

fn number_rule(value: u64, floor: u64, functions: usize, commit: Option<&str>) -> String {
    let Some(commit) = commit else {
        return "the floor, with no commit to measure".into();
    };
    if value == floor {
        format!(
            "the floor of {floor}, over {} function(s) at {}",
            grouped(functions),
            short(commit)
        )
    } else {
        format!(
            "95th percentile of {} functions at {}, floor {floor}",
            grouped(functions),
            short(commit)
        )
    }
}

fn read_sample(value: &Value) -> Option<Sample> {
    let number = |key| value.get(key).and_then(Value::as_u64);
    let scope = Scope::from_fields(value.get("scope")?.as_object()?).ok()?;
    Some(Sample {
        cc: number("cc")?,
        lines: number("lines")?,
        functions: usize::try_from(number("functions")?).ok()?,
        scope,
        fallback: value
            .get("fallback")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn kept_sample(sample: &Sample) -> Value {
    let mut kept = Map::from_iter([
        ("cc".into(), sample.cc.into()),
        ("lines".into(), sample.lines.into()),
        ("functions".into(), sample.functions.into()),
        ("scope".into(), sample.scope.value()),
    ]);
    if let Some(fallback) = &sample.fallback {
        kept.insert("fallback".into(), fallback.clone().into());
    }
    Value::Object(kept)
}

fn derived_value(key: &str, value: u64, rule: &str) -> Value {
    Value::Object(Map::from_iter([
        ("section".into(), SECTION.into()),
        ("key".into(), key.into()),
        ("value".into(), value.into()),
        ("rule".into(), rule.into()),
    ]))
}

fn short(commit: &str) -> &str {
    commit.get(..7).unwrap_or(commit)
}

fn grouped(count: usize) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (at, digit) in digits.char_indices() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

fn measure(
    tree: &Tree,
    selection: &Selection,
    repo_root: &Path,
    changes: Option<&[Change]>,
    renamed: Option<&HashMap<String, String>>,
) -> Result<Sweep, Error> {
    let extensions: Vec<&str> = selection
        .languages
        .iter()
        .flat_map(|language| language.extensions)
        .copied()
        .collect();
    let wanted = files::Wanted {
        extensions: &extensions,
        skip_dirs: &files::default_skip_dirs(),
        exclude: &[],
        exclude_except: &[],
        skip_hidden: true,
    };
    let roots = [repo_root.to_path_buf()];
    let mut found = files::found(tree.root(), || tree.files(), &roots, &wanted)?;
    let mut scoped_out = Vec::new();
    found.kept.retain(|file| {
        let relative = files::relative(file, repo_root);
        if selection.scope.selects(&relative) {
            true
        } else {
            scoped_out.push(file.clone());
            false
        }
    });
    found.excluded.extend(scoped_out);
    let mut measured: Vec<String> = found
        .kept
        .iter()
        .map(|file| files::relative(file, repo_root))
        .collect();
    let test_file = test_files(tree, renamed);
    let (out, unparsed, work) =
        read_current(found.kept, selection, &test_file, repo_root, changes)?;
    measured.retain(|file| !unparsed.iter().any(|unread| &unread.file == file));
    let files = Files {
        measured,
        not_measured: Vec::new(),
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
        work,
    })
}

fn test_files<'a>(
    tree: &Tree,
    renamed: Option<&'a HashMap<String, String>>,
) -> impl Fn(&str) -> bool + 'a {
    let tests = survey::tests(tree);
    move |file| {
        tests.file_holds(
            renamed
                .and_then(|renamed| renamed.get(file))
                .map_or(file, String::as_str),
        )
    }
}

fn read_current(
    kept: Vec<PathBuf>,
    selection: &Selection,
    test_file: &dyn Fn(&str) -> bool,
    repo_root: &Path,
    changes: Option<&[Change]>,
) -> Result<(Vec<Function>, Vec<Unparsed>, ContentCost), Error> {
    let changed: Option<BTreeSet<&str>> =
        changes.map(|changes| changes.iter().map(|change| change.path.as_str()).collect());
    let mut out = Vec::new();
    let mut unparsed = Vec::new();
    let mut work = ContentCost::default();
    for file in kept {
        let name = file.to_string_lossy().to_string();
        let relative = files::relative(&file, repo_root);
        if changed
            .as_ref()
            .is_some_and(|changed| !changed.contains(relative.as_str()))
        {
            continue;
        }
        let Some(language) = selection.languages.iter().find(|language| {
            language
                .extensions
                .iter()
                .any(|extension| name.ends_with(extension))
        }) else {
            continue;
        };
        work.reads += 1;
        work.parses += 1;
        out.extend(functions(
            &file,
            repo_root,
            language,
            test_file,
            &mut unparsed,
        )?);
    }
    out.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok((out, unparsed, work))
}

fn applicable(tree: &Tree, selection: &Selection) -> Result<bool, Error> {
    Ok(tree.files()?.iter().any(|file| {
        selection.scope.inside(file)
            && selection
                .languages
                .iter()
                .any(|language| language.extensions.iter().any(|end| file.ends_with(end)))
    }))
}

fn functions(
    path: &Path,
    repo_root: &Path,
    language: &'static Language,
    test_file: &dyn Fn(&str) -> bool,
    unparsed: &mut Vec<Unparsed>,
) -> Result<Vec<Function>, Error> {
    let bytes = std::fs::read(path).map_err(|why| Error::unreadable(path, why))?;
    let source = String::from_utf8_lossy(&bytes).to_string();
    let file = files::relative(path, repo_root);
    match syntax::read(&file, &source, language)? {
        Parsed::Read(read) => {
            let mut found = parsed(&read);
            marked_as_tests(&mut found, test_file(&file), &read);
            Ok(found)
        }
        Parsed::Rejected(refused) => {
            unparsed.push(refused);
            Ok(Vec::new())
        }
    }
}

/// Every function one parse holds, with the two numbers this check ratchets.
fn parsed(file: &ParsedFile) -> Vec<Function> {
    let lines = file.lines();
    let mut sites = HashMap::new();
    let mut out = Vec::new();
    walk_functions(file, &mut |node, cc| {
        let row = node.start_position().row;
        let key = (holder_row(node).filter(|holder| *holder < row), row);
        let text = sites
            .entry(key)
            .or_insert_with(|| Rc::<str>::from(site(node, &lines)))
            .clone();
        out.push(Function {
            file: file.path.to_string(),
            line: row as u64 + 1,
            end: node.end_position().row as u64 + 1,
            cc,
            text,
            body: record::body_hash(node.utf8_text(file.bytes()).unwrap_or_default()),
            test: false,
        });
    });
    out
}

fn marked_as_tests(found: &mut [Function], test_file: bool, file: &ParsedFile) {
    let modules = match (test_file, file.language.id) {
        (false, LanguageId::Rust) if file.source.contains("test") => {
            syntax::convention::cfg_test_ranges(file.root(), file.source.as_bytes())
        }
        _ => Vec::new(),
    };
    for function in found {
        function.test = test_file
            || modules
                .iter()
                .any(|(from, to)| (*from..=*to).contains(&function.line));
    }
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
    let mut out = Vec::new();
    walk_functions(&file, &mut |node, cc| {
        out.push(Measured {
            cc,
            lines: (node.end_position().row - node.start_position().row + 1) as u64,
        });
    });
    out
}

/// One tree being walked, with everything the walk reads off the language and this check.
struct Walked<'a> {
    language: &'static Language,
    metrics: &'static Metrics,
    test_file: bool,
    source: &'a str,
}

/// The same function selection and metrics for survey samples and retained gate sites.
/// A sample never builds a Function or materializes its site text or body hash.
fn walk_functions(file: &ParsedFile, keep: &mut impl FnMut(Node, u64)) {
    let at = Walked {
        language: file.language,
        metrics: metrics(file.language.id),
        test_file: survey::marked(file.path),
        source: file.source,
    };
    syntax::walk(file.root(), &mut |node| {
        if at.language.functions.contains(&node.kind())
            && !holds_a_body(node, at.language)
            && !suite_callback(node, &at)
        {
            keep(node, 1 + decisions(node, &at));
        }
    });
}

fn suite_callback(node: Node, at: &Walked) -> bool {
    if !at.test_file
        || !matches!(
            at.language.id,
            LanguageId::TypeScript | LanguageId::JavaScript
        )
    {
        return false;
    }
    let mut callback = node;
    while let Some(parent) = callback
        .parent()
        .filter(|parent| SUITE_CALLBACK_WRAPPERS.contains(&parent.kind()))
    {
        callback = parent;
    }
    let Some(arguments) = callback
        .parent()
        .filter(|parent| parent.kind() == "arguments")
    else {
        return false;
    };
    arguments
        .parent()
        .filter(|parent| parent.kind() == "call_expression")
        .is_some_and(|call| suite_call(call, at.source))
}

fn suite_call(call: Node, source: &str) -> bool {
    let Some(callee) = call.child_by_field_name("function") else {
        return false;
    };
    let callee = unparenthesized(callee);
    match callee.kind() {
        "identifier" => {
            SUITE_CONTAINERS.contains(&callee.utf8_text(source.as_bytes()).unwrap_or_default())
        }
        "member_expression" => suite_member(callee, SUITE_DIRECT_METHODS, source),
        "call_expression" => callee
            .child_by_field_name("function")
            .is_some_and(|function| suite_member(function, &["each"], source)),
        _ => false,
    }
}

fn suite_member(node: Node, methods: &[&str], source: &str) -> bool {
    let node = unparenthesized(node);
    let Some(object) = node.child_by_field_name("object") else {
        return false;
    };
    let Some(property) = node.child_by_field_name("property") else {
        return false;
    };
    let object = unparenthesized(object);
    SUITE_BASES.contains(&object.utf8_text(source.as_bytes()).unwrap_or_default())
        && methods.contains(&property.utf8_text(source.as_bytes()).unwrap_or_default())
}

fn unparenthesized(mut node: Node) -> Node {
    while node.kind() == "parenthesized_expression" {
        let Some(inner) = node.named_child(0) else {
            break;
        };
        node = inner;
    }
    node
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
    let length = [(LINES.name, "lines"), (TEST_LINES.name, "test lines")]
        .iter()
        .find(|(key, _)| values.contains_key(*key))
        .map(|(key, unit)| format!(", {} {unit}", number(key)))
        .unwrap_or_default();
    format!("cc {}{length}", number(CC.name))
}
