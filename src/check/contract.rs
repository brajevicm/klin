//! What a check is, told once. A check is handed an immutable `Context` — who called it, which
//! gate it judges under, what base it compares against, what scope it may look at — and it
//! writes to an explicit `Sink`: the report text a person reads, and the `Records` the runner
//! turns into the 11.2 object. Nothing about a run reaches a check any other way, so the runner
//! keeps no side channel into it and a check keeps no state of its own. This module names no
//! check, so every check and the catalogue that registers them depend on it one way.

use std::fmt::Write;
use std::path::Path;

use serde_json::{Map, Value};

use crate::base::{self, Prior, Window};
use crate::changed::Change;
use crate::config::Config;
use crate::coverage::Coverage;
use crate::error::Error;
use crate::key::Key;
use crate::project::Project;
use crate::syntax::structural::{ExtractionCost, NameCost, Unchanged, footprint::Footprint};
use crate::{modules, surface};

/// The outcome of a file no grammar reads. The hook counts these to report the holes a
/// person must close, and nothing else in a run turns on it.
pub const UNPARSED: &str = "unparsed";

/// The outcome of a test the base holds that went in the window, which fails nothing. A stop the
/// hook lets end hands it to a person. Spec 8.2.
pub const DELETED: &str = "deleted";

/// The outcome of a file `before` measured and `after` did not, which a run records so a report
/// never reads a window it stopped measuring as a whole one. Spec 8.6.
pub const LOST: &str = "lost";

/// The outcome of a parser-readable file for which no semantic adapter exists. It is a hole in
/// a structural gate, not a green measurement. Spec 8.4, 8.6.
pub const NOT_MEASURED: &str = "not-measured";

/// The outcome of a derived ceiling whose recorded scope fell back to the whole repository or
/// differs from today's. The hook tells it, so a scope lag is never silent. Spec 5.4, ADR 0039.
pub const DERIVATION: &str = "derivation";

/// The outcome of a build whose command the shell could not find. The tool is absent, so the
/// tree is unmeasured rather than failing: the gates judge the source and the hook tells the
/// note, because the one action left is an install. ADR 0048.
pub const UNBUILT: &str = "unbuilt";

/// The outcome of a dependency form a module resolver supports and could not resolve. A green
/// layering run must not imply a resolution klin did not make. Spec 8.2.1, 8.6.
pub const UNRESOLVED: &str = "unresolved";

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
    /// The findings the ratchet passed, which the runner puts on the gate's row. `None` for a
    /// gate that never got that far. Spec 11.2.
    pub held: Option<u64>,
    /// How many of `held` a person-authored accepted entry passed rather than a base site, which
    /// is the one number the OK line's qualifier turns on. `None` for a gate that never got that
    /// far, and 0 for a ratcheting gate no accepted entry matched. Spec 11.2.
    pub accepted: Option<u64>,
    /// The structural facts the gate read over both trees: extracted by it, or shared from an
    /// earlier gate of the run. `None` for a gate that reads none. Spec 11.2.
    pub facts: Option<ExtractionCost>,
    /// The source contents a file-local gate read and parsed over both trees. `None` for a gate
    /// that records no content work.
    pub work: Option<ContentCost>,
    /// The declaration states `dead-symbols` built over both trees. `None` for a gate that
    /// builds none. Spec 11.2.
    pub states: Option<u64>,
    /// The name evidence `dead-symbols` and `reachability` built over both trees. `None` for a
    /// gate that resolves no names. Spec 11.2.
    pub names: Option<NameCost>,
    /// The parts of laying the whole base out, on the row of the gate that laid it out. `None`
    /// for every other gate. Spec 11.2.
    pub layout: Option<base::Layout>,
    /// What the facts `dead-symbols` held over both trees cost in population and bytes. `None`
    /// for a gate that records none. Spec 11.2.
    pub footprint: Option<Footprint>,
    /// The module graphs the gate built over both trees. `None` for a gate that builds none.
    pub graph: Option<modules::GraphCost>,
    /// The public surfaces the gate derived over both trees. `None` for a gate that derives
    /// none. Spec 11.2.
    pub surface: Option<surface::SurfaceCost>,
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

    /// The same run with another judgement scope, for a check that derives a narrower or wider
    /// effective scope from evidence the runner does not read. Nothing else moves.
    pub fn scoped<'b>(&self, only: Option<&'b [String]>) -> Context<'b>
    where
        Self: 'b,
    {
        Context {
            only,
            gate: self.gate,
            project: self.project,
            prior: self.prior,
            base: self.base,
            changes: self.changes,
            caller: self.caller,
            strict: self.strict,
            quiet: self.quiet,
        }
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

/// The base commit a gate judges against: the one the runner chose, or the one this gate
/// chooses for itself and names once in the report. Spec 6.1.
pub fn base_commit(root: &Path, at: &Context, out: &mut Sink) -> Result<String, Error> {
    match at.base {
        Some(commit) => Ok(commit.to_string()),
        None => Ok(announced(root, at, out)?.before),
    }
}

/// The base a gate the runner did not lay out chooses for itself, named once in the report.
pub fn announced(root: &Path, at: &Context, out: &mut Sink) -> Result<Window, Error> {
    let base = base::choose(root, at.strict)?;
    if at.context() {
        let _ = writeln!(out.text, "{}", base.line());
    }
    Ok(base)
}

/// The base laid out whole for this run: the runner's own when it laid the whole base out, which
/// a changed run never does, and otherwise the run's one checkout. Spec 8.4, ADR 0038.
pub fn whole_base<'a>(at: &Context<'a>, commit: &str) -> Result<&'a Prior, Error> {
    let laid = at.prior.filter(|_| at.changes.is_none());
    base::whole(at.project, laid, shared(at), commit)
}

/// The base's view of the files this run leaves alone, which only a changed run that is not
/// strict shares. Spec 8.4.
pub fn unchanged_base<'a>(
    at: &Context<'a>,
    prior: &'a Prior,
    commit: &str,
) -> Result<Option<Unchanged<'a>>, Error> {
    base::unchanged(at.project, shared(at), prior, commit)
}

fn shared<'a>(at: &Context<'a>) -> Option<&'a [Change]> {
    at.changes.filter(|_| !at.strict)
}

/// The base tree for a gate the runner did not lay out, such as a gate run by its own command.
pub fn own_base(at: &Context, out: &mut Sink) -> Result<Prior, Error> {
    let base = announced(at.project.root(), at, out)?;
    base::materialize(at.project, &base.before, None)
}

impl Sink<'_> {
    /// What every `OK:` line adds after what the gate judged, recorded for `--json` on the way
    /// past so one call per check carries both. Spec 8.6.
    pub fn covered(&mut self, coverage: &Coverage) -> String {
        self.record(|records| records.coverage = Some(coverage_record(coverage)));
        match coverage.not_measured {
            0 => format!(
                " ({} file(s) found, {} measured, {} excluded, {} unreadable)",
                coverage.found, coverage.measured, coverage.excluded, coverage.unreadable
            ),
            not_measured => format!(
                " ({} file(s) found, {} measured, {} not measured, {} excluded, {} unreadable)",
                coverage.found,
                coverage.measured,
                not_measured,
                coverage.excluded,
                coverage.unreadable
            ),
        }
    }
}

fn coverage_record(coverage: &Coverage) -> Value {
    let mut out = Map::new();
    out.insert("found".into(), coverage.found.into());
    out.insert("measured".into(), coverage.measured.into());
    out.insert("not_measured".into(), coverage.not_measured.into());
    out.insert("excluded".into(), coverage.excluded.into());
    out.insert("unreadable".into(), coverage.unreadable.into());
    Value::Object(out)
}

pub type Run = fn(&Context<'_>, &mut Sink<'_>) -> Result<u8, Error>;

/// One `derived:` or `pinned:` line and the `{section, key, value, rule}` entry beside a derived
/// one, built together so the two cannot say different things. Spec 11.2.
pub type Said = (String, Option<Value>);

/// The `{section, key, value, rule}` entry `--json` prints beside a `derived:` line. Spec 11.2.
pub fn derived_entry(section: &str, key: Option<&str>, value: Value, rule: &str) -> Value {
    serde_json::json!({ "section": section, "key": key, "value": value, "rule": rule })
}

/// The key every entry of a named section carries, whichever check reads the section.
pub const NAMED: Key = Key {
    name: "name",
    holds: "the gate's own name, which `--gate` takes",
    required: true,
    rule: None,
    default: "",
    shape: crate::key::Shape::String,
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
