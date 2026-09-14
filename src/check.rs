//! What a check is, told once. A check is handed an immutable `Context` — who called it, which
//! gate it judges under, what base it compares against, what scope it may look at — and it
//! writes to an explicit `Sink`: the report text a person reads, and the `Records` the runner
//! turns into the 11.2 object. Nothing about a run reaches a check any other way, so the runner
//! keeps no side channel into it and a check keeps no state of its own.
//!
//! The `CATALOGUE` is the one table of the checks klin has. The runner takes its gates from it,
//! `config` takes the section names it accepts from it, and `reference` prints it. A new check
//! is one row here beside its Clap command, and a CLI test fails when only one of the two is
//! written. ADR 0036.

use serde_json::Value;

use crate::base::Prior;
use crate::changed::Change;
use crate::config::{Config, Error};
use crate::project::Project;
use crate::reference::{Key, Languages};
use crate::{
    complexity, conventions, dead_symbols, doc_citations, doc_size, escapes, inventory, lockfile,
    reachability, sarif, stubs, syntax,
};

/// The outcome of a file no grammar reads. The hook counts these to report the holes a
/// person must close, and nothing else in a run turns on it.
pub const UNPARSED: &str = "unparsed";

/// The outcome of a test the base holds that went in the window, which fails nothing. A stop the
/// hook lets end hands it to a person. Spec 8.2.
pub const DELETED: &str = "deleted";

/// The outcome of a parser-readable file for which no semantic adapter exists. It is a hole in
/// a structural gate, not a green measurement. Spec 8.4, 8.6.
pub const NOT_MEASURED: &str = "not-measured";

/// The outcome of a derived ceiling whose recorded scope fell back to the whole repository or
/// differs from today's. The hook tells it, so a scope lag is never silent. Spec 5.4, ADR 0039.
pub const DERIVATION: &str = "derivation";

/// The source-content work a file-local gate performed over its current and base trees. The
/// runner exposes this beside structural `facts` so performance rows can prove a changed run
/// did not read unchanged source.
#[derive(Default, Clone, Copy)]
pub struct ContentCost {
    pub reads: usize,
    pub parses: usize,
}

impl std::ops::Add for ContentCost {
    type Output = ContentCost;

    fn add(self, other: ContentCost) -> ContentCost {
        ContentCost {
            reads: self.reads + other.reads,
            parses: self.parses + other.parses,
        }
    }
}

/// Everything a run records about what it judged, which the runner prints as the one object of
/// spec 11.2 and the journal writes as the stop's line. There is one of these per gate, gathered
/// into one for the run. A check a person runs by hand has none, and records nothing.
#[derive(Default)]
pub struct Records {
    pub findings: Vec<Value>,
    pub notes: Vec<Value>,
    /// One row per gate the run judged, which only the runner fills in. Spec 11.2.
    pub gates: Vec<Value>,
    /// What scope the gate measured, which every check records once. Spec 11.2.
    pub coverage: Option<Value>,
    /// One `{section, key, value, rule}` entry per value the run derived. Spec 11.2.
    pub derived: Vec<Value>,
    pub derived_lines: Vec<String>,
    /// The count the check's own `OK:` line prints as held at the base, which the runner puts
    /// on the gate's row. `None` for a gate that never got that far. Spec 11.2.
    pub held: Option<u64>,
    /// The structural facts the gate read over both trees: extracted by it, or shared from an
    /// earlier gate of the run. `None` for a gate that reads none. Spec 11.2.
    pub facts: Option<syntax::structural::ExtractionCost>,
    /// The source contents a file-local gate read and parsed over both trees. `None` for a gate
    /// that records no content work.
    pub work: Option<ContentCost>,
}

/// Who ran this check. A person running one by hand gets the run's own context lines and no
/// records; the runner gets neither, because it prints the context once for the whole run; the
/// stop hook is the runner again, where a hole the agent cannot fix is a note and not a failure.
/// Spec 4.3, 8.2, 11.1.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Caller {
    Hand,
    Gate,
    Hook,
}

/// What a check needs of the base: nothing, the commit the window names, or that commit laid
/// out as a tree beside the working one. A check that needs the commit or the tree is also the
/// kind `--strict` reaches, because it has a comparison or an accepted list to judge. Spec
/// 4.6, 10.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Needs {
    Nothing,
    TheCommit,
    TheTree,
}

impl Needs {
    /// Whether the run resolves the base commit for this check, which is also whether
    /// `--strict` reaches it. Spec 4.6, 10.
    pub fn the_commit(self) -> bool {
        self >= Needs::TheCommit
    }

    /// Whether the run lays the base commit out as a tree for this check.
    pub fn the_tree(self) -> bool {
        self == Needs::TheTree
    }
}

/// What absence of a check's section means, which is the one thing a person needs to know
/// about a check before writing its section. Spec 4.6, ADR 0038.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    /// Absence means the check runs over the facts of the tree and derives its own policy.
    Automatic,
    /// Absence means the project has no such policy, so the check does not run.
    Policy,
    /// Absence means no external tool is configured, so the check does not run.
    Integration,
}

/// Everything one check is told, and nothing it writes. Borrowed for the length of the call, so
/// a check cannot keep any of it and cannot change it.
pub struct Context<'a> {
    /// The name of the gate being run, which the accepted list names.
    pub gate: &'a str,
    /// The one configuration the run loaded, and the facts of the working tree, read once for
    /// every check the run selects. ADR 0038.
    pub project: &'a Project,
    /// The base commit, already laid out as a tree by the runner.
    pub prior: Option<&'a Prior>,
    /// The base commit the runner chose, for a gate that reads the base tree out of git.
    pub base: Option<&'a str>,
    /// The files a scoped run judges, and `None` for a run that judges everything.
    pub only: Option<&'a [String]>,
    /// The authoritative before-to-after changes of a changed run, separate from its judgement
    /// scope. Direct checks and unscoped runs have no changed-run context.
    pub changes: Option<&'a [Change]>,
    pub caller: Caller,
    pub strict: bool,
    /// Print nothing on success: no `OK:` line, and nothing under it.
    pub quiet: bool,
}

impl Context<'_> {
    /// Whether this check says the run's own context for itself: the `window:` line and the
    /// `derived:` lines. The runner prints those once for the whole run, so only a check a
    /// person ran by hand says them here. Spec 4.3, 11.1.
    pub fn context(&self) -> bool {
        self.caller == Caller::Hand && !self.quiet && self.changes.is_none()
    }

    /// Whether the Stop hook runs this gate, so a hole the agent cannot fix is a note, not a
    /// failure.
    pub fn hook(&self) -> bool {
        self.caller == Caller::Hook
    }

    /// The configuration the run loaded, which every check reads its section from.
    pub fn config(&self) -> &Config {
        &self.project.config
    }
}

impl<'a> Context<'a> {
    /// A check a person ran by hand: no runner, so no base laid out for it and no scope from a
    /// window. A command states the rest.
    pub fn by_hand(gate: &'a str, project: &'a Project) -> Context<'a> {
        Context {
            gate,
            project,
            prior: None,
            base: None,
            only: None,
            changes: None,
            caller: Caller::Hand,
            strict: false,
            quiet: false,
        }
    }
}

/// Where a check writes: the report a person reads, and the records the runner keeps. A check a
/// person runs by hand has no records, and every `record` call on it does nothing.
pub struct Sink<'a> {
    pub text: &'a mut String,
    pub records: Option<&'a mut Records>,
}

impl<'a> Sink<'a> {
    /// A sink that only prints, for a check a person runs by hand. Nothing records the run, so
    /// every `record` call on it does nothing.
    pub fn unrecorded(text: &'a mut String) -> Sink<'a> {
        Sink {
            text,
            records: None,
        }
    }

    pub fn record(&mut self, add: impl FnOnce(&mut Records)) {
        if let Some(records) = self.records.as_deref_mut() {
            add(records);
        }
    }

    /// One value a check derived itself. Direct commands print it here; the runner keeps the
    /// line beside the record so it can place provenance before that gate's status row.
    pub fn provenance(&mut self, line: String, derived: Option<Value>) {
        match self.records.as_deref_mut() {
            Some(records) => {
                records.derived_lines.push(line);
                records.derived.extend(derived);
            }
            None => {
                self.text.push_str(&line);
                self.text.push('\n');
            }
        }
    }
}

pub type Run = fn(&Context<'_>, &mut Sink<'_>) -> Result<u8, Error>;

/// One `derived:` or `pinned:` line and the `{section, key, value, rule}` entry beside a derived
/// one, built together so the two cannot say different things. Spec 11.2.
pub type Said = (String, Option<Value>);

/// The `{section, key, value, rule}` entry `--json` prints beside a `derived:` line. Spec 11.2.
pub fn derived_entry(section: &str, key: Option<&str>, value: Value, rule: &str) -> Value {
    serde_json::json!({ "section": section, "key": key, "value": value, "rule": rule })
}

/// One row of the catalogue: one check, as the runner, the configuration, the reference and
/// the plan all read it.
pub struct Row {
    /// What `--gate` calls this check, which for two checks is not the name of the section
    /// they read.
    pub name: &'static str,
    pub section: &'static str,
    /// What the section's absence means: derive it, or run nothing. Spec 4.6.
    pub activation: Activation,
    /// The configuration keys the section reads, declared in the check's own module and printed
    /// by `klin reference`. Spec 5.8.
    pub keys: &'static [Key],
    /// The built-in language coverage this check reports, and none for a check that reads no
    /// programming language. This is capability, never configurable source topology. Spec 5.8.
    pub languages: Option<Languages>,
    /// Whether the tree holds what an Automatic check applies to when its section is absent,
    /// answered from facts alone and never from a derived number. Spec 4.6, ADR 0040.
    pub available: fn(&Project) -> bool,
    pub run: Run,
    pub needs: Needs,
    pub takes_scope: bool,
    /// Whether the section is a list of entries a person writes, each its own gate under its
    /// own `name`, rather than one section the whole check runs under. Spec 8.3.
    pub gate_per_entry: bool,
}

pub const CATALOGUE: &[Row] = &[
    Row {
        name: "doc-size",
        section: doc_size::SECTION,
        activation: Activation::Automatic,
        keys: doc_size::KEYS,
        languages: None,
        available: |project| !project.facts().found.documents.is_empty(),
        run: doc_size::gate,
        needs: Needs::Nothing,
        takes_scope: false,
        gate_per_entry: false,
    },
    Row {
        name: "doc-citations",
        section: doc_citations::SECTION,
        activation: Activation::Automatic,
        keys: doc_citations::KEYS,
        languages: None,
        available: |project| !project.facts().found.documents.is_empty(),
        run: doc_citations::gate,
        needs: Needs::TheTree,
        takes_scope: true,
        gate_per_entry: false,
    },
    Row {
        name: "lockfile",
        section: lockfile::SECTION,
        activation: Activation::Automatic,
        keys: lockfile::KEYS,
        languages: None,
        available: lockfile::applies,
        run: lockfile::gate,
        needs: Needs::TheTree,
        takes_scope: false,
        gate_per_entry: false,
    },
    Row {
        name: "escapes",
        section: escapes::SECTION,
        activation: Activation::Automatic,
        keys: escapes::KIND.keys,
        languages: Some(escapes::language_extensions),
        available: |project| !project.found_no_source_root(),
        run: escapes::gate,
        needs: Needs::TheTree,
        takes_scope: true,
        gate_per_entry: false,
    },
    Row {
        name: "stubs",
        section: stubs::SECTION,
        activation: Activation::Automatic,
        keys: stubs::KIND.keys,
        languages: Some(stubs::language_extensions),
        available: |project| !project.found_no_source_root(),
        run: stubs::gate,
        needs: Needs::TheTree,
        takes_scope: true,
        gate_per_entry: false,
    },
    Row {
        name: "inventory",
        section: inventory::SECTION,
        activation: Activation::Automatic,
        keys: inventory::KEYS,
        languages: None,
        available: inventory::applies,
        run: inventory::gate,
        needs: Needs::TheTree,
        takes_scope: true,
        gate_per_entry: false,
    },
    Row {
        name: "complexity",
        section: complexity::SECTION,
        activation: Activation::Automatic,
        keys: complexity::KEYS,
        languages: Some(syntax::language_extensions),
        available: |project| !project.found_no_source_root(),
        run: complexity::gate,
        needs: Needs::TheTree,
        takes_scope: true,
        gate_per_entry: false,
    },
    Row {
        name: "dead-symbols",
        section: dead_symbols::SECTION,
        activation: Activation::Automatic,
        keys: dead_symbols::KEYS,
        languages: Some(dead_symbols::language_extensions),
        available: |project| !project.found_no_source_root(),
        run: dead_symbols::gate,
        needs: Needs::TheTree,
        takes_scope: true,
        gate_per_entry: false,
    },
    Row {
        name: "reachability",
        section: reachability::SECTION,
        activation: Activation::Automatic,
        keys: reachability::KEYS,
        languages: Some(reachability::language_extensions),
        available: |project| !project.found_no_source_root(),
        run: reachability::gate,
        needs: Needs::TheTree,
        takes_scope: true,
        gate_per_entry: false,
    },
    Row {
        name: "conventions",
        section: conventions::SECTION,
        activation: Activation::Policy,
        keys: conventions::KEYS,
        languages: Some(syntax::pattern::language_extensions),
        available: |_| false,
        run: conventions::gate,
        needs: Needs::TheTree,
        takes_scope: true,
        gate_per_entry: false,
    },
    Row {
        name: "sarif",
        section: sarif::SECTION,
        activation: Activation::Integration,
        keys: sarif::KEYS,
        languages: None,
        available: |_| false,
        run: sarif::gate,
        needs: Needs::TheCommit,
        takes_scope: false,
        gate_per_entry: true,
    },
];

/// The section each check reads, which config.rs judges the top-level keys against.
pub fn sections() -> impl Iterator<Item = &'static str> {
    CATALOGUE.iter().map(|check| check.section)
}

/// The section behind a key that is what the command is called, so an error can say which of
/// the two the config wants. `None` for a check whose name is its section.
pub fn command_named(key: &str) -> Option<&'static str> {
    CATALOGUE
        .iter()
        .find(|check| check.name != check.section && check.name == key)
        .map(|check| check.section)
}

/// Every check by name, for the errors that list what a person may write.
pub fn names() -> impl Iterator<Item = &'static str> {
    CATALOGUE.iter().map(|check| check.name)
}

impl Row {
    /// Whether this check measures code, which is what a tree with no source root leaves it
    /// nothing to measure. Spec 10, 14.
    pub fn reads_code(&self) -> bool {
        self.activation == Activation::Automatic && self.languages.is_some()
    }
}

/// The key every entry of a named section carries, whichever check reads the section.
pub const NAMED: Key = Key {
    name: "name",
    holds: "the gate's own name, which `--gate` takes",
    required: true,
    rule: None,
    default: "",
};

/// The entries of a section a person writes entry by entry, each with the name its gate takes.
/// Such a section is a list, and an entry with no `name` is a config error naming the key,
/// because nothing in a tree says which tool the entry runs. The check that reads one entry
/// reads its own list through this, so a gate's name is the name the check judges under.
/// Spec 8.3.
pub fn named_entries(config: &Config, section: &str) -> Result<Vec<(String, Value)>, Error> {
    let held = config.required(section)?;
    let listed = held.as_array().ok_or_else(|| {
        Error(format!(
            "{}: \"{section}\" is a list of entries, each its own gate under its own \"name\"",
            config.file.display()
        ))
    })?;
    listed
        .iter()
        .map(|entry| {
            let name = entry
                .get(NAMED.name)
                .and_then(Value::as_str)
                .ok_or_else(|| config.missing(section, NAMED.name))?;
            Ok((name.to_string(), entry.clone()))
        })
        .collect()
}
