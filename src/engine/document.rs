//! The check document of spec 11.7, and the one loop that fills it. `klin check` and the Stop
//! each run every selected gate through `CheckDocument::ran`, which records what each gate found in
//! the document's own types. The text a person reads and the JSON are views of those records.
//! Spec 7, 11.3, 11.7, 13.1.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;
use std::path::PathBuf;

use serde_json::{Map, Value, json};

use crate::config::file::MEASUREMENT_LOST;
use crate::config::scope::{Moved, Moves};
use crate::contract;
use crate::contract::check::{
    Activation, Caller, Cause, Class, Context, Debt, Failed, Hole, Incomplete, NO_SOURCE_ROOT,
    Plain, RIDE_ALONG, Ratchet, Reason, Records, Sink, Told, UNBUILT,
};
use crate::contract::coverage::Coverage;
use crate::contract::holes::{self, Seen, Unmeasured};
use crate::contract::project::Project;
use crate::engine::catalogue;
use crate::engine::plan::{Gate, Plan, State, every_check};
use crate::engine::render::{self, Finding, GateRecords, Note, Review, Slot};
use crate::sys::changed::Change;
use crate::sys::error::{Error, ErrorKind, Fault};
use crate::window::base::{self, Prior, Window};
use crate::{engine::diagnostics, facts::survey, sys::clock};

const INVENTORY: &str = "inventory";

/// What a run writes: the text of `klin check`, its JSON document, or the report of the Stop hook.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Text,
    Json,
    Stop,
}

/// What one run of the runner is asked: by `check` or by the Stop hook.
#[derive(Default)]
pub struct Args {
    pub config: Option<PathBuf>,
    pub gates: Vec<String>,
    pub changed: bool,
    pub view: View,
    /// Whether the run requested the footprint's counters. Spec B.13.
    pub footprint: bool,
    /// Whether the run leaves every integration alone, so it runs no project command, as the
    /// baseline of `setup` does. Spec 11.2.
    pub without_integrations: bool,
}

impl Args {
    pub fn json(&self) -> bool {
        self.view == View::Json
    }

    pub fn at_stop(&self) -> bool {
        self.view == View::Stop
    }

    pub fn text(&self) -> bool {
        self.view == View::Text
    }
}

/// What this run judges the working tree against: the base commit, laid out, and the files
/// a scoped run looks at.
#[derive(Default)]
pub struct Against<'a> {
    pub base: Option<Window>,
    pub changes: Option<Cow<'a, [Change]>>,
    pub scope: Option<Vec<String>>,
    pub prior: Option<Prior>,
    /// Why the base tree could not be laid out, which fails only the gates that read it.
    /// Spec 7.3.
    pub unlaid: Option<String>,
}

/// A judgement of spec 7.1, in the order aggregation takes the worst of.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
enum Judgement {
    #[default]
    Pass,
    Review,
    Fail,
}

impl Judgement {
    fn name(self) -> &'static str {
        match self {
            Judgement::Pass => "pass",
            Judgement::Review => "review",
            Judgement::Fail => "fail",
        }
    }
}

fn measurement_word(incomplete: bool) -> &'static str {
    match incomplete {
        true => "incomplete",
        false => "complete",
    }
}

fn execution_word(error: bool) -> &'static str {
    match error {
        true => "error",
        false => "ok",
    }
}

/// The three axes of one gate, or of a whole run. Spec 4.5, 7.5.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct Axes {
    judgement: Judgement,
    incomplete: bool,
    error: bool,
}

impl Axes {
    fn and(self, other: Axes) -> Axes {
        Axes {
            judgement: self.judgement.max(other.judgement),
            incomplete: self.incomplete || other.incomplete,
            error: self.error || other.error,
        }
    }

    /// The one state that wins: an error, then a failing finding, then a hole, then a review
    /// item. Spec 7.4, 11.3.
    fn worst(self) -> Worst {
        match self {
            Axes { error: true, .. } => Worst::Error,
            Axes {
                judgement: Judgement::Fail,
                ..
            } => Worst::Fail,
            Axes {
                incomplete: true, ..
            } => Worst::Incomplete,
            Axes {
                judgement: Judgement::Review,
                ..
            } => Worst::Review,
            _ => Worst::Ok,
        }
    }

    /// ERROR 2 > FAIL 1 > INCOMPLETE 3 > success 0. Spec 7.4.
    fn exit(self) -> u8 {
        match self.worst() {
            Worst::Error => 2,
            Worst::Fail => 1,
            Worst::Incomplete => 3,
            Worst::Review | Worst::Ok => 0,
        }
    }

    /// The row word: the first of `ERR`, `FAIL`, `INCOMPLETE`, `REVIEW`, `ok`. The Stop's row
    /// says only what its agent acts on, so an incomplete or reviewed gate there is `ok`.
    /// Spec 9.5, 11.3.
    fn state(self, view: View) -> &'static str {
        match (self.worst(), view) {
            (Worst::Error, _) => "ERR ",
            (Worst::Fail, _) => "FAIL",
            (Worst::Incomplete, View::Text | View::Json) => "INCOMPLETE",
            (Worst::Review, View::Text | View::Json) => "REVIEW",
            (Worst::Incomplete | Worst::Review, View::Stop) | (Worst::Ok, _) => "ok  ",
        }
    }

    /// Whether the Stop asks this gate's question and judges nothing else of it: its only
    /// failure is a question. Spec 9.2.
    fn only_asks(self, view: View, asks: bool) -> bool {
        asks && matches!((self.worst(), view), (Worst::Fail, View::Stop))
    }

    /// The row word the report prints: `state`, and `ASK` in place of `FAIL` for a gate that
    /// only asks. Spec 9.2.
    fn row(self, view: View, asks: bool) -> &'static str {
        match self.only_asks(view, asks) {
            true => "ASK ",
            false => self.state(view),
        }
    }

    /// The word a gate's diagnostics row carries, which names only what an agent acts on.
    fn status(self) -> &'static str {
        self.state(View::Stop).trim_end()
    }

    fn measurement(self) -> &'static str {
        measurement_word(self.incomplete)
    }

    fn execution(self) -> &'static str {
        execution_word(self.error)
    }
}

/// The state of a gate or run that the exit code and the row word both follow.
#[derive(Clone, Copy)]
enum Worst {
    Error,
    Fail,
    Incomplete,
    Review,
    Ok,
}

/// One gate's axes from its exit code and its result. A gate that erred still carries the
/// findings it validated, so they still fail it. A finding of a ride-along row fails nothing on
/// its own: it is a review item, and the ratchet leaves it out of the exit code. Spec 7.5, 9.6.
fn axes_of(code: u8, shown: &GateRecords) -> Axes {
    let failing = || shown.findings.iter().filter(|finding| finding.failing());
    let rode = failing().any(|finding| finding.rides_along)
        || shown.reviews.iter().any(|review| review.kind == RIDE_ALONG);
    let fails = code == 1 || failing().any(|finding| !finding.rides_along);
    Axes {
        judgement: match (fails, shown.reviews.is_empty() && !rode) {
            (true, _) => Judgement::Fail,
            (false, false) => Judgement::Review,
            (false, true) => Judgement::Pass,
        },
        incomplete: !shown.holes.is_empty(),
        error: code == 2,
    }
}

/// One gate's run: its exit code, what it told, what it cost, and how long it took in ms.
struct Ran {
    code: u8,
    told: Vec<Told>,
    records: Records,
    ms: u64,
}

/// The names of the gates whose run failed.
fn failed_gates<'a>(wanted: &[&'a Gate], ran: &[Result<Ran, &str>]) -> Vec<&'a str> {
    wanted
        .iter()
        .zip(ran)
        .filter(|(_, ran)| matches!(ran, Ok(ran) if ran.code == 1))
        .map(|(gate, _)| gate.name.as_str())
        .collect()
}

/// One gate's run, or why the base could not be laid out for it.
fn run<'a>(
    args: &Args,
    gate: &Gate,
    project: &Project,
    against: &'a Against,
) -> Result<Ran, &'a str> {
    match (&against.unlaid, gate.check.needs.the_tree()) {
        (Some(why), true) => Err(why),
        _ => {
            let (ran, ms) = clock::timed(|| one(args, gate, project, against));
            Ok(Ran { ms, ..ran })
        }
    }
}

fn one(args: &Args, gate: &Gate, project: &Project, against: &Against) -> Ran {
    let mut told = Vec::new();
    let mut records = Records::default();
    let at = Context {
        gate: &gate.name,
        project,
        prior: against.prior.as_ref(),
        base: against.base.as_ref().map(|base| base.before.as_str()),
        only: against.scope.as_deref().filter(|_| gate.check.takes_scope),
        changes: against.changes.as_deref(),
        footprint: args.footprint,
        caller: match args.view {
            View::Stop => Caller::Hook,
            View::Text | View::Json => Caller::Gate,
        },
    };
    let outcome = (gate.check.run)(
        &at,
        &mut Sink {
            told: &mut told,
            records: &mut records,
        },
    );
    let code = match outcome {
        Ok(code) => code,
        Err(problem) => {
            records.errors.push(problem.to_string());
            told.push(Plain::Error(problem.to_string()).into());
            2
        }
    };
    Ran {
        code,
        told,
        records,
        ms: 0,
    }
}

/// What a gate told once every gate ran, and the records of it. Spec 10.6, 11.7.
fn shown(
    args: &Args,
    (gate, project): (&Gate, &Project),
    told: Vec<Told>,
    failed: &[&str],
) -> (Vec<Told>, GateRecords) {
    let mut told = beside(told, failed);
    if !args.at_stop() {
        told = reviewed(told);
    }
    let mut shown = render::records(&gate.name, &told);
    moved_out(project.moves(), gate.check.section, &mut shown.findings);
    (told, shown)
}

/// A remedy told beside another gate becomes a remedy, and joins the remedy of each failure it
/// names, where that gate failed in the same run and one of those failures failed. Elsewhere it
/// is dropped. Spec 10.6.
fn beside(mut told: Vec<Told>, failed: &[&str]) -> Vec<Told> {
    let kept = kept_beside(&told, failed);
    for item in &mut told {
        if let Told::Ratchet(
            Ratchet::New { failed: found, .. }
            | Ratchet::Worse { failed: found, .. }
            | Ratchet::Asked { failed: found, .. },
        ) = item
        {
            joined(found, &kept);
        }
    }
    told.into_iter()
        .filter_map(|item| match item {
            Told::Plain(Plain::Beside { key, remedy, .. }) => kept
                .contains(&(key, remedy))
                .then(|| Plain::Remedy(remedy.to_string()).into()),
            other => Some(other),
        })
        .collect()
}

/// The key and remedy of each remedy told beside a gate that failed, where a failure holds its
/// key.
fn kept_beside(told: &[Told], failed: &[&str]) -> Vec<(&'static str, &'static str)> {
    let fails = |key: &str| {
        told.iter().any(|item| match item {
            Told::Ratchet(
                Ratchet::New { failed: found, .. }
                | Ratchet::Worse { failed: found, .. }
                | Ratchet::Asked { failed: found, .. },
            ) => found.iter().any(|finding| finding.values.contains_key(key)),
            _ => false,
        })
    };
    told.iter()
        .filter_map(|item| match item {
            Told::Plain(Plain::Beside { gate, key, remedy })
                if failed.contains(gate) && fails(key) =>
            {
                Some((*key, *remedy))
            }
            _ => None,
        })
        .collect()
}

/// Each kept remedy joins the remedy of every failure that holds its key.
fn joined(found: &mut [Failed], kept: &[(&str, &str)]) {
    for finding in found {
        for (key, remedy) in kept {
            if finding.values.contains_key(*key) {
                finding.fix_advice.push(' ');
                finding.fix_advice.push_str(remedy);
            }
        }
    }
}

/// At `klin check`, a finding of a ride-along row is a review item, and a remedy told only for
/// such findings goes with them. ADR 0073, spec 9.6.
fn reviewed(told: Vec<Told>) -> Vec<Told> {
    let split: Vec<Told> = told
        .into_iter()
        .flat_map(|mut item| {
            let riders = match &mut item {
                Told::Ratchet(said) => said.ride_along(),
                _ => None,
            };
            let emptied = matches!(&item, Told::Ratchet(said) if said.emptied());
            (!emptied)
                .then_some(item)
                .into_iter()
                .chain(riders.map(Told::from))
        })
        .collect();
    let blocks = split.iter().any(|item| {
        matches!(
            item,
            Told::Ratchet(Ratchet::New { .. } | Ratchet::Worse { .. } | Ratchet::Asked { .. })
        )
    });
    split
        .into_iter()
        .filter(|item| blocks || !matches!(item, Told::Plain(Plain::Remedy(_))))
        .collect()
}

/// A Stop's result of a gate whose only failures are of ride-along rows, split into what it
/// tells on its own and those failures with their remedy. Spec 9.6.
fn riding(told: Vec<Told>) -> (Vec<Told>, Vec<Told>) {
    told.into_iter().partition(|item| {
        !matches!(
            item,
            Told::Ratchet(Ratchet::New { .. } | Ratchet::Worse { .. } | Ratchet::Asked { .. })
                | Told::Plain(Plain::Remedy(_))
        )
    })
}

/// A finding at a file the change moved out of its gate's scope, which keeps the scope of its
/// base path, carries `moved_out_of_scope`, which no ratchet compares. Spec 7.3.
fn moved_out(moves: &Moves, section: &str, findings: &mut [Finding]) {
    let out = moves.out_of(section);
    for finding in findings
        .iter_mut()
        .filter(|finding| out.contains(&finding.file.as_str()))
    {
        finding
            .values
            .insert("moved_out_of_scope".into(), true.into());
    }
}

/// How many files of the run's scope a capability's coverage counts, in the words of the
/// document. Spec 7.2, 11.7.
#[derive(Clone, Copy, Default)]
struct Counts {
    found: usize,
    measured: usize,
    not_read: usize,
    excluded: usize,
    gaps: usize,
    limits: usize,
}

impl Counts {
    fn of(coverage: &Coverage) -> Counts {
        Counts {
            found: coverage.found,
            measured: coverage.measured,
            excluded: coverage.excluded,
            ..Counts::default()
        }
    }

    fn json(&self) -> Value {
        json!({
            "found": self.found,
            "measured": self.measured,
            "not_read": self.not_read,
            "excluded": self.excluded,
            "gaps": self.gaps,
            "limits": self.limits,
        })
    }
}

/// One row of the document's capabilities. Spec 11.7.
struct Capability {
    name: String,
    /// The catalogue check the row runs, which for a `sarif` entry is not its name.
    check: &'static str,
    debt: Debt,
    kind: &'static str,
    placement: &'static [&'static str],
    state: State,
    judgement: Option<Judgement>,
    incomplete: Option<bool>,
    error: bool,
    coverage: Option<Counts>,
    claim: &'static str,
    held: Option<u64>,
    accepted: Option<u64>,
}

impl Capability {
    /// The row of a gate the run selected, with its axes and what its run recorded.
    fn active(gate: &Gate, axes: Axes, records: &Records) -> Capability {
        Capability {
            name: gate.name.clone(),
            check: gate.check.name,
            debt: gate.check.judges.debt,
            kind: gate.check.kind(),
            placement: gate.check.placement.names(),
            state: State::Active,
            judgement: Some(axes.judgement),
            incomplete: Some(axes.incomplete),
            error: axes.error,
            coverage: records.coverage.as_ref().map(Counts::of),
            claim: coverage_claim(gate.check),
            held: records.held,
            accepted: records.accepted,
        }
    }

    /// The row of a capability that does not apply to this tree, with the axes of a selector
    /// that named it.
    fn inapplicable(check: &catalogue::Row, named: Option<Axes>) -> Capability {
        Capability {
            name: check.name.to_string(),
            check: check.name,
            debt: check.judges.debt,
            kind: check.kind(),
            placement: check.placement.names(),
            state: State::NotApplicable,
            judgement: None,
            incomplete: named.map(|axes| axes.incomplete),
            error: false,
            coverage: None,
            claim: coverage_claim(check),
            held: None,
            accepted: None,
        }
    }

    /// The built-in row of the lost files, held where an accepted entry names the file.
    fn lost(axes: Axes, accepted: u64) -> Capability {
        Capability {
            name: MEASUREMENT_LOST.to_string(),
            check: MEASUREMENT_LOST,
            debt: Debt::Held,
            kind: "built-in",
            placement: &["stop", "check"],
            state: State::Active,
            judgement: Some(axes.judgement),
            incomplete: Some(axes.incomplete),
            error: axes.error,
            coverage: None,
            claim: "verified",
            held: Some(accepted),
            accepted: Some(accepted),
        }
    }

    fn count_unmeasured(&mut self, sorted: &[Unmeasured]) {
        if let Some(counts) = self.coverage.as_mut() {
            counts.gaps = counted(sorted, Class::Opened, &self.name);
            counts.limits = counted(sorted, Class::Limit, &self.name);
        }
    }

    fn json(&self) -> Value {
        json!({
            "name": self.name,
            "kind": self.kind,
            "placement": self.placement,
            "state": self.state.name(),
            "judgement": self.judgement.map(Judgement::name),
            "measurement": self.incomplete.map(measurement_word),
            "execution": execution_word(self.error),
            "coverage": self.coverage.as_ref().map(Counts::json),
            "coverage_claim": self.claim,
            "held": self.held,
            "accepted": self.accepted,
        })
    }
}

/// An error of the document: its kind, the gate it names or none for the run, and its words.
/// Spec 7.3, 11.7.
struct Problem {
    kind: &'static str,
    check: Option<String>,
    message: String,
}

impl Problem {
    fn of_run(kind: ErrorKind, message: &str) -> Problem {
        Problem::new(kind, None, message)
    }

    fn new(kind: ErrorKind, check: Option<&str>, message: &str) -> Problem {
        Problem {
            kind: kind.name(),
            check: check.map(str::to_string),
            message: message.to_string(),
        }
    }

    fn json(&self) -> Value {
        json!({ "kind": self.kind, "check": self.check, "message": self.message })
    }
}

/// How many files the gates of a run measured: all of them, and the ones a gate that reads code
/// measured.
#[derive(Default, Clone, Copy)]
struct Measured {
    files: u64,
    code: u64,
}

impl Measured {
    fn add(&mut self, check: &catalogue::Row, records: &Records) {
        let measured = records
            .coverage
            .map_or(0, |coverage| coverage.measured as u64);
        self.files += measured;
        if check.reads_code() {
            self.code += measured;
        }
    }
}

/// One capability as `setup` reports it: the findings the ratchet passed, which are the floor,
/// the ones it did not, and how many files it measured of the ones it found where it counts
/// files. Spec 11.2.
pub struct Floor {
    pub gate: String,
    /// The catalogue check the gate runs, which for a `sarif` entry is not its name.
    pub check: &'static str,
    pub standing: Standing,
    pub held: usize,
    pub failing: usize,
    pub coverage: Option<FilesMeasured>,
}

/// How many files a capability measured of the ones it found. Spec 11.2.
#[derive(Clone, Copy)]
pub struct FilesMeasured {
    pub measured: usize,
    pub found: usize,
}

/// A gate the run lists in no capability row: one a person set to `false`, or an integration
/// the run left alone. Spec 11.2.
struct Unrun {
    gate: String,
    check: &'static str,
}

impl Unrun {
    fn of(gate: &Gate) -> Unrun {
        Unrun {
            gate: gate.name.clone(),
            check: gate.check.name,
        }
    }

    fn floor(&self, standing: Standing) -> Floor {
        Floor {
            gate: self.gate.clone(),
            check: self.check,
            standing,
            held: 0,
            failing: 0,
            coverage: None,
        }
    }
}

/// How one capability stood in the run `setup` reads its floor from. Spec 11.2.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    Measured,
    /// The capability does not run in this tree, or a person set it to `false`.
    Off,
    /// An integration, whose command the run left alone.
    Skipped,
}

/// What a run found, beside the first error that kept it from judging everything. What it found
/// stays valid when the error is there. Spec 16.2.
pub struct Partial<T> {
    pub found: Vec<T>,
    pub erred: Option<Error>,
}

impl<T> Partial<T> {
    /// A run the error stopped before it found anything.
    pub fn stopped(why: Error) -> Partial<T> {
        Partial {
            found: Vec::new(),
            erred: Some(why),
        }
    }
}

/// Where a failing finding is: the check that found it, its file, line and text. A gate's name
/// is left out, so a renamed `sarif` entry keeps its sites. Spec 16.2.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Site<'a> {
    check: &'a str,
    file: &'a str,
    line: Option<u64>,
    text: Option<&'a str>,
}

/// The failing sites of a run, each with how many findings fail there, since findings can share
/// a site: the same text twice on one line. Spec 16.2.
pub type Sites<'a> = HashMap<Site<'a>, usize>;

/// What one run came to, gathered for the check document and its text. Spec 11.7.
#[derive(Default)]
pub struct CheckDocument {
    config: Option<Value>,
    window: Option<Value>,
    tree: Option<Value>,
    axes: Axes,
    /// Whether a run-scope error stopped the run before any capability measured. Spec 7.3.
    stopped: bool,
    /// The checks a person set to `false`, which the document lists nowhere. Spec 11.2.
    excluded: Vec<Unrun>,
    /// The integrations the run left alone, which the document lists nowhere. Spec 11.2.
    skipped: Vec<Unrun>,
    capabilities: Vec<Capability>,
    findings: Vec<Finding>,
    notes: Vec<Note>,
    measurements: Vec<Value>,
    holes: Vec<Incomplete>,
    /// The files the selected gates measured, all of them and the ones gates that read code
    /// measured, which decide the hole of a whole run that measured nothing. Spec 7.2.
    measured: Measured,
    not_measured: BTreeSet<String>,
    /// What each gate could not measure, as it saw it, for the run to sort once. Spec 7.2.
    seen: Vec<(String, String, Seen)>,
    /// The derived build commands the Stop ran before it measured, in the run's basis. Spec 8.1.
    built: Vec<Value>,
    reviews: Vec<Review>,
    problems: Vec<Problem>,
    gates: Vec<Value>,
    /// How many things the Stop tells a person beside the findings it blocks on: the notes its
    /// gates told that the hook tells, and each NOTE line it printed of the run itself. A stop
    /// nothing blocks still tells them. Spec 8.2, 9.5.
    told: usize,
    /// Whether a file the base parsed is lost to a parse, which a person may hold where the
    /// grammar lags. Spec 7.2.
    grammar_lag: bool,
    /// The Stop text of each gate whose only failures are of ride-along rows, which the Stop
    /// prints only when another finding blocks it. Spec 9.6.
    riding: Vec<String>,
    /// How many gates failed only on a question, which the Stop asks and does not judge.
    /// Spec 9.2.
    asked: usize,
}

/// The findings a capability's ratchet passed that are debt, as its check counts debt.
/// Spec 11.2.
fn held_debt(row: &Capability) -> usize {
    let passed = match row.debt {
        Debt::Held => row.held,
        Debt::Accepted => row.accepted,
    };
    passed.map_or(0, |count| count as usize)
}

impl CheckDocument {
    /// The configuration a run judged under, as the check document names it. Spec 11.7.
    pub fn config_of(project: &Project) -> Value {
        json!({
            "path": project.config.file.display().to_string(),
            "present": project.config.written(),
        })
    }

    /// The document a Stop gathers: the configuration it ran under and the commands it built.
    pub fn stopping(project: &Project, built: Vec<Value>) -> CheckDocument {
        CheckDocument {
            config: Some(CheckDocument::config_of(project)),
            built,
            ..CheckDocument::default()
        }
    }

    /// The document of a Stop whose build failed, so no capability measured. Spec 6.4, 13.1.
    pub fn unbuilt(config: Value, window: Option<&Window>, failure: &str) -> CheckDocument {
        CheckDocument {
            config: Some(config),
            window: window.map(Window::record),
            stopped: true,
            notes: vec![Note::of_run("build", failure)],
            ..CheckDocument::default()
        }
    }

    /// The document of a Stop that stopped before any capability measured, with the error that
    /// stopped it. Spec 7.3, 13.1.
    pub fn stopped_by(config: Value, window: Option<&Window>, fault: Fault) -> CheckDocument {
        let mut doc = CheckDocument {
            config: Some(config),
            window: window.map(Window::record),
            ..CheckDocument::default()
        };
        doc.stop(fault);
        doc
    }

    pub fn set_config(&mut self, config: Value) {
        self.config = Some(config);
    }

    pub fn config_or(&mut self, config: impl FnOnce() -> Value) {
        self.config.get_or_insert_with(config);
    }

    pub fn note(&mut self, note: Note) {
        self.notes.push(note);
    }

    pub fn review(&mut self, review: Review) {
        self.reviews.push(review);
    }

    /// The findings `wanted` fail under `project`'s policy, the lost files among them, at a site
    /// beyond what `failing` holds, run apart from any document a person reads, and why the run
    /// could not judge everything: its first error, or else the gates it left incomplete. Each
    /// site `failing` holds cancels one finding there. Spec 16.2.
    pub fn failing_under(
        args: &Args,
        (project, wanted): (&Project, &[&Gate]),
        against: &Against,
        failing: &Sites,
    ) -> Partial<Finding> {
        let mut apart = CheckDocument::default();
        apart.judged(args, (project, wanted), against, &mut String::new());
        let erred = apart.first_problem().or_else(|| apart.first_incomplete());
        let mut left = failing.clone();
        let found = apart
            .findings
            .iter()
            .filter(|finding| finding.failing())
            .filter(|finding| match left.get_mut(&apart.site(finding)) {
                Some(count) if *count > 0 => {
                    *count -= 1;
                    false
                }
                _ => true,
            })
            .cloned()
            .collect();
        Partial { found, erred }
    }

    /// Where a finding is, by the catalogue check of its gate, or by its kind for a row of its
    /// own such as `measurement-lost`.
    fn site<'a>(&'a self, finding: &'a Finding) -> Site<'a> {
        let check = finding
            .check
            .as_deref()
            .and_then(|gate| self.capabilities.iter().find(|row| row.name == gate))
            .map_or(finding.kind, |row| row.check);
        Site {
            check,
            file: &finding.file,
            line: finding.line,
            text: finding.text.as_deref(),
        }
    }

    /// The gates whose measurement is incomplete, named in one error.
    fn first_incomplete(&self) -> Option<Error> {
        let incomplete: Vec<&str> = self
            .capabilities
            .iter()
            .filter(|row| row.incomplete == Some(true))
            .map(|row| row.name.as_str())
            .collect();
        (!incomplete.is_empty()).then(|| {
            Error(format!(
                "{} could not measure everything it found",
                incomplete.join(", ")
            ))
        })
    }

    /// The first error of the run, a check's included.
    fn first_problem(&self) -> Option<Error> {
        let problem = self.problems.first()?;
        Some(Error(match &problem.check {
            Some(check) => format!("{check}: {}", problem.message),
            None => problem.message.clone(),
        }))
    }

    /// A run-scope error, which stops the run before any capability measures. Spec 7.3.
    pub fn stop(&mut self, fault: Fault) {
        self.stopped = true;
        self.problems
            .push(Problem::of_run(fault.kind, &fault.error.to_string()));
    }

    /// What `setup` says the run holds as its floor: each capability in the run's order, then
    /// each check a person set to `false`, then each integration the run left alone. The first
    /// error of the run, a check's included, where one kept any capability from measuring, and
    /// why the measurement is incomplete where it is, because then no floor is known. Spec 11.2.
    pub fn floor(&self) -> Result<Vec<Floor>, Error> {
        if let Some(error) = self.first_problem() {
            return Err(error);
        }
        if self.totals().incomplete {
            return Err(Error(match self.holes.first() {
                Some(hole) => hole.text.clone(),
                None => "a check could not measure everything it found, which klin check names"
                    .to_string(),
            }));
        }
        let ran = self.capabilities.iter().map(|row| Floor {
            gate: row.name.clone(),
            check: row.check,
            standing: match row.state {
                State::Active => Standing::Measured,
                _ => Standing::Off,
            },
            held: held_debt(row),
            failing: self.failing(&row.name),
            coverage: row.coverage.as_ref().map(|counts| FilesMeasured {
                measured: counts.measured,
                found: counts.found,
            }),
        });
        let excluded = self.excluded.iter().map(|gate| gate.floor(Standing::Off));
        let skipped = self
            .skipped
            .iter()
            .map(|gate| gate.floor(Standing::Skipped));
        Ok(ran.chain(excluded).chain(skipped).collect())
    }

    /// The integrations a run left alone, which `setup` names as blind spots. Spec 11.2.
    pub fn skip(&mut self, gates: &[&Gate]) {
        self.skipped = gates.iter().map(|gate| Unrun::of(gate)).collect();
    }

    fn failing(&self, gate: &str) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.failing() && finding.check.as_deref() == Some(gate))
            .count()
    }

    /// How many gates failed, counting the built-in row of the lost files. Spec 10.4.
    pub fn failed(&self) -> usize {
        let failed = |row: &&Capability| row.judgement == Some(Judgement::Fail);
        self.capabilities.iter().filter(failed).count()
    }

    /// How many gates a Stop's report prints as failed: the gates that failed on a finding and,
    /// where any gate failed, each gate whose only failures ride along. A gate that failed only
    /// on a question counts as asked. Spec 9.2, 9.6.
    pub fn failed_in_report(&self) -> usize {
        match self.failed_on_findings() {
            0 => 0,
            failed => failed + self.riding.len(),
        }
    }

    /// How many gates failed on a finding, and not only on a question. Spec 9.2.
    pub fn failed_on_findings(&self) -> usize {
        self.failed() - self.asked
    }

    /// How many gates could not run, and how many run-scope errors there were. Spec 10.4.
    pub fn errored(&self) -> usize {
        let erred = self.capabilities.iter().filter(|row| row.error).count();
        erred + self.problems.iter().filter(|p| p.check.is_none()).count()
    }

    pub fn told(&self) -> usize {
        self.told
    }

    /// How many failed gates failed only on a question. Spec 9.2.
    pub fn asked(&self) -> usize {
        self.asked
    }

    pub fn grammar_lag(&self) -> bool {
        self.grammar_lag
    }

    /// Every site the run failed, with how many of its findings fail there. Spec 16.2.
    pub fn failing_sites(&self) -> Sites<'_> {
        let mut sites = Sites::new();
        for finding in self.findings.iter().filter(|finding| finding.failing()) {
            *sites.entry(self.site(finding)).or_default() += 1;
        }
        sites
    }

    /// The site id of every finding the run failed, which a stop that blocks records as asked.
    /// Spec 8.2, 9.2.
    pub fn reported(&self) -> Vec<String> {
        self.findings
            .iter()
            .filter(|finding| finding.failing())
            .map(|finding| finding.id.clone())
            .collect()
    }

    /// The deleted tests among the findings, as sites, which klin has not asked about yet.
    /// Spec 8.2, 9.2.
    pub fn unasked(&self) -> Vec<String> {
        self.findings
            .iter()
            .filter(|finding| finding.check.as_deref() == Some(INVENTORY))
            .map(Finding::site)
            .collect()
    }

    /// The Stop's document once every gate ran: the note of a build that could not run and the
    /// note of a tree with no source root. Spec 11.7, 13.1.
    pub fn stopped_with(&mut self, unbuilt: Option<&str>, rootless: Option<&str>) {
        self.told += usize::from(unbuilt.is_some()) + usize::from(rootless.is_some());
        self.notes
            .extend(unbuilt.map(|unbuilt| Note::of_run(UNBUILT, unbuilt)));
        self.notes
            .extend(rootless.map(|said| Note::of_run(NO_SOURCE_ROOT, said)));
    }

    /// The notes the Stop prints of a moved pinned path, one whose files went with no rename or
    /// that selects nothing in either tree, and of a file moved under a skipped directory. A pin
    /// whose files were all renamed is followed in silence, and `klin check` names it. Spec 7.3.
    pub fn gone_moves(&mut self, project: &Project, wanted: &[&Gate], out: &mut String) {
        let gone = selected(project.moves(), wanted).filter(|moved| moved.gone());
        for said in gone.filter_map(Moved::said) {
            let _ = writeln!(out, "  NOTE: {said}");
            self.told += 1;
        }
    }

    /// Every selected gate, the rows of what a selector named that does not apply, and the hole
    /// of a whole run that measured nothing. Spec 7.2, 11.7.
    pub fn ran(
        &mut self,
        args: &Args,
        project: &Project,
        (plan, wanted, unsupported): (&Plan, &[&Gate], Vec<&'static catalogue::Row>),
        against: &Against,
        out: &mut String,
    ) {
        self.windowed(args, (project, wanted), against.base.as_ref(), out);
        self.judged(args, (project, wanted), against, out);
        if !args.changed
            && let Some(hole) = unmeasured_run(args, (plan, wanted), project, self.measured)
        {
            self.holes.push(hole);
        }
        for check in unsupported {
            self.unsupported(args, check, out);
        }
        if args.gates.is_empty() {
            self.not_applicable(plan);
        }
        if self.failed_on_findings() > 0 {
            out.extend(self.riding.iter().map(String::as_str));
        }
    }

    /// Every selected gate's result, and what the gates together could not measure, sorted once
    /// into the `measurement-lost` row, review items and coverage notes. Spec 7.2, 10.6.
    fn judged(
        &mut self,
        args: &Args,
        (project, wanted): (&Project, &[&Gate]),
        against: &Against,
        out: &mut String,
    ) {
        self.gates(args, (project, wanted), against, out);
        let base = against.base.as_ref().map(|base| base.before.as_str());
        let seen = std::mem::take(&mut self.seen);
        let sorted = holes::sorted(project, base, seen);
        self.settled(args, (project, wanted, against), &sorted, out);
    }

    /// Every selected gate runs first, and then each one's result goes into the document, so a
    /// remedy told beside another gate knows whether that gate failed. Spec 10.6.
    fn gates(
        &mut self,
        args: &Args,
        (project, wanted): (&Project, &[&Gate]),
        against: &Against,
        out: &mut String,
    ) {
        let ran: Vec<_> = wanted
            .iter()
            .map(|gate| run(args, gate, project, against))
            .collect();
        let failed = failed_gates(wanted, &ran);
        for (gate, ran) in wanted.iter().zip(ran) {
            match ran {
                Ok(ran) => self.took(args, (gate, project, against), ran, &failed, out),
                Err(why) => self.unlaid(args, gate, why, out),
            }
        }
    }

    /// What the run could not measure, sorted once over every gate: the error of a sort git
    /// could not finish, the files the gates could not measure, and the files of a language each
    /// gate does not read. Spec 7.2, 11.7.
    fn settled(
        &mut self,
        args: &Args,
        (project, wanted, against): (&Project, &[&Gate], &Against),
        (sorted, failed): &(Vec<Unmeasured>, Option<String>),
        out: &mut String,
    ) {
        if let Some(why) = failed {
            self.problems.push(Problem::of_run(ErrorKind::Git, why));
            if !args.json() {
                let _ = writeln!(out, "  ERR: {why}");
            }
        }
        self.unmeasured(args, (project, wanted), sorted, out);
        self.not_read(wanted, &by_extension(project, against));
    }

    /// What binding the window found: the window and the tree the run judges, a `moved-pin`
    /// review item per moved pinned path, a `moved-skipped` one per file moved under a skipped
    /// directory, a note each for a rewritten push base or a base equal to HEAD, and the hole
    /// of a local base equal to HEAD that may hide unpushed commits. The Stop prints none of
    /// them. Spec 6.5, 7.3.
    fn windowed(
        &mut self,
        args: &Args,
        (project, wanted): (&Project, &[&Gate]),
        base: Option<&Window>,
        out: &mut String,
    ) {
        let says = args.text();
        self.window = base.map(Window::record);
        self.tree = (!args.at_stop()).then(|| base::tree_record(project.root()));
        for review in selected(project.moves(), wanted).filter_map(moved_review) {
            if says {
                let _ = writeln!(out, "  REVIEW: {}", review.text);
            }
            self.reviews.push(review);
        }
        let Some(base) = base else {
            return;
        };
        for note in &base.notes {
            if says {
                let _ = writeln!(out, "  NOTE: {note}");
            }
            self.notes.push(Note {
                check: None,
                kind: contract::check::WINDOW,
                message: note.clone(),
                coverage: None,
                file: Slot::Null,
                line: None,
                values: None,
            });
        }
        if let Some(text) = &base.unproven {
            self.holes.push(Incomplete {
                reason: Reason::ComparisonUnproven,
                detail: None,
                text: text.clone(),
            });
        }
    }

    /// One gate's result in the document: its row, its measurement record, its errors and what
    /// it found, where `failed` names every gate of the run that failed. Spec 10.6, 11.7.
    fn took(
        &mut self,
        args: &Args,
        (gate, project, against): (&Gate, &Project, &Against),
        ran: Ran,
        failed: &[&str],
        out: &mut String,
    ) {
        let Ran {
            code,
            told,
            records,
            ms,
        } = ran;
        let ((told, shown), recorded) = clock::timed(|| shown(args, (gate, project), told, failed));
        let ms = ms + recorded;
        let axes = axes_of(code, &shown);
        self.axes = self.axes.and(axes);
        let told = self.held_back(args, &gate.name, axes, told);
        let asks = render::asks_only(&told);
        self.asked += usize::from(axes.only_asks(args.view, asks));
        if shows(args, code, &shown) {
            let (provenance, text) = match args.view {
                View::Stop => (render::pinned(&told), render::stop(&told, code == 0)),
                View::Text | View::Json => (render::provenance(&told), render::text(&told)),
            };
            printed(
                &gate.name,
                axes.row(args.view, asks),
                (&provenance, &text),
                out,
            );
        }
        self.told += shown.notes.iter().filter(|note| note.told()).count();
        let erred = usize::from(code == 2 && !records.errors.is_empty());
        let counts = diagnostics::Counts {
            findings: shown.findings.len() + erred,
            notes: shown.notes.len(),
        };
        self.gates.push(diagnostics::row(
            &gate.name,
            axes.status(),
            counts,
            &records,
            ms,
        ));
        self.measured.add(gate.check, &records);
        self.capabilities
            .push(Capability::active(gate, axes, &records));
        let basis = basis(gate, (project, against), &records, &shown);
        self.measurements
            .push(measurement(&gate.name, axes, &shown.holes, basis));
        self.problems.extend(gate_errors(&gate.name, code, &told));
        self.seen.extend(unmeasured_by(&gate.name, &told));
        self.reviews.extend(shown.reviews);
        self.findings.extend(shown.findings);
        self.notes.extend(shown.notes);
    }

    /// A Stop's gate whose only failures are of ride-along rows: those failures go to the text
    /// the Stop prints only when another finding blocks it, and the gate prints what it tells
    /// on its own. Every other gate keeps its result. Spec 9.6.
    fn held_back(&mut self, args: &Args, gate: &str, axes: Axes, told: Vec<Told>) -> Vec<Told> {
        let failed = told.iter().any(|item| {
            matches!(
                item,
                Told::Ratchet(Ratchet::New { .. } | Ratchet::Worse { .. } | Ratchet::Asked { .. })
            )
        });
        if !args.at_stop() || !failed || axes.judgement == Judgement::Fail {
            return told;
        }
        let (told, riders) = riding(told);
        let text = render::stop(&riders, false);
        let mut block = String::new();
        printed(gate, "FAIL", (&[], &text), &mut block);
        self.riding.push(block);
        told
    }

    /// The files the run could not measure, sorted once: the `measurement-lost` row and its
    /// findings, a review item per opened gap, a coverage note per limit, and the gap and limit
    /// counts on the row of each gate that reported one. Spec 7.2, 11.7.
    fn unmeasured(
        &mut self,
        args: &Args,
        (project, wanted): (&Project, &[&Gate]),
        sorted: &[Unmeasured],
        out: &mut String,
    ) {
        let held = holes::held_files(&project.config);
        let row = render::lost_row(sorted, &held);
        let failing = match row {
            Some(_) => self.lost(sorted, &held),
            None => false,
        };
        self.reviewed(sorted);
        let unmatched: Vec<String> = unmatched_lost(&held, sorted, wanted).collect();
        self.reviews.extend(
            unmatched
                .iter()
                .map(|file| render::unmatched_lost_review(file)),
        );
        let unread = sorted.iter().filter(|item| item.class != Class::Lost);
        self.told += unmatched.len() + unread.count();
        if args.json() {
            return;
        }
        if args.at_stop() {
            out.push_str(&row.filter(|_| failing).unwrap_or_default());
            for file in &unmatched {
                let _ = writeln!(out, "  NOTE: {}", render::unmatched_lost_text(file));
            }
        } else {
            out.push_str(&row.unwrap_or_default());
        }
        out.push_str(&render::unmeasured_lines(sorted, args.at_stop()));
    }

    /// A review item per opened gap and a coverage note per limit, every sorted file counted as
    /// not measured but one a gate still measured under a form an attribute gives it, and each
    /// gate's gap and limit counts. Spec 7.2, 11.7.
    fn reviewed(&mut self, sorted: &[Unmeasured]) {
        self.not_measured.extend(
            sorted
                .iter()
                .filter(|item| !item.measured)
                .map(|item| item.file.clone()),
        );
        for item in sorted {
            match item.class {
                Class::Lost => (),
                Class::Opened if item.reason == Cause::LeftScope && self.fails_at(&item.file) => (),
                Class::Opened => self.reviews.push(render::opened_review(item)),
                Class::Limit => self.notes.push(render::limit_note(item)),
            }
        }
        for row in &mut self.capabilities {
            row.count_unmeasured(sorted);
        }
    }

    /// Whether a gate failed a finding at this file, which a file that left a scope keeps under
    /// the base's scope, so the file is that FAIL and not also a review item. Spec 7.2.
    fn fails_at(&self, file: &str) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.file == file && finding.failing())
    }

    /// The files in the run's scope in a language each gate that reads code does not read, on
    /// that gate's row. Spec 7.2, 11.7.
    fn not_read(&mut self, wanted: &[&Gate], by_extension: &BTreeMap<String, usize>) {
        for gate in wanted.iter().filter(|gate| gate.check.reads_code()) {
            let not_read: usize = by_extension
                .iter()
                .filter(|(extension, _)| !reads(gate.check, extension))
                .map(|(_, count)| count)
                .sum();
            let row = self
                .capabilities
                .iter_mut()
                .find(|row| row.name == gate.name);
            if let Some(counts) = row.and_then(|row| row.coverage.as_mut()) {
                counts.not_read = not_read;
            }
        }
    }

    /// The built-in row of the lost files and their findings, held where an accepted entry names
    /// the file, and whether any lost file fails. Spec 7.2, 11.7.
    fn lost(&mut self, sorted: &[Unmeasured], held: &[String]) -> bool {
        let lost: Vec<&Unmeasured> = sorted
            .iter()
            .filter(|item| item.class == Class::Lost)
            .collect();
        let failing = |item: &&&Unmeasured| !held.contains(&item.file);
        let failed = lost.iter().filter(failing).count();
        let axes = Axes {
            judgement: match failed {
                0 => Judgement::Pass,
                _ => Judgement::Fail,
            },
            ..Axes::default()
        };
        self.axes = self.axes.and(axes);
        self.grammar_lag = lost
            .iter()
            .filter(failing)
            .any(|item| item.reason == Cause::Parse);
        self.findings.extend(
            lost.iter()
                .map(|item| render::lost_finding(item, held.contains(&item.file))),
        );
        self.capabilities
            .push(Capability::lost(axes, (lost.len() - failed) as u64));
        failed > 0
    }

    /// The row of a gate that reads the base tree klin could not lay out. The other gates still
    /// run. Spec 7.3.
    fn unlaid(&mut self, args: &Args, gate: &Gate, why: &str, out: &mut String) {
        let message = format!("the base tree could not be laid out for this gate: {why}");
        let told = [Told::Plain(Plain::Error(message.clone()))];
        let axes = Axes {
            error: true,
            ..Axes::default()
        };
        self.axes = self.axes.and(axes);
        if !args.json() {
            printed(
                &gate.name,
                axes.state(args.view),
                (&[], &render::text(&told)),
                out,
            );
        }
        self.capabilities
            .push(Capability::active(gate, axes, &Records::default()));
        self.measurements
            .push(measurement(&gate.name, axes, &[], produced(gate.check)));
        self.problems
            .push(Problem::new(ErrorKind::Git, Some(&gate.name), &message));
    }

    /// The row of a capability a selector named that does not apply to this tree. Spec 7.2.
    fn unsupported(&mut self, args: &Args, check: &'static catalogue::Row, out: &mut String) {
        let hole = Incomplete {
            reason: Reason::Unsupported,
            detail: None,
            text: format!(
                "{} does not apply to this tree, or needs a \"{}\" section klin.json does not \
                 hold, so it measured nothing",
                check.name, check.section
            ),
        };
        let told = [Told::Incomplete(hole.clone())];
        let axes = Axes {
            incomplete: true,
            ..Axes::default()
        };
        self.axes = self.axes.and(axes);
        if !args.json() {
            printed(
                check.name,
                axes.state(args.view),
                (&[], &render::text(&told)),
                out,
            );
        }
        self.capabilities
            .push(Capability::inapplicable(check, Some(axes)));
        self.measurements
            .push(measurement(check.name, axes, &[hole], produced(check)));
    }

    /// The rows of the capabilities that do not apply, which a whole run lists. Spec 11.7.
    fn not_applicable(&mut self, plan: &Plan) {
        self.excluded = plan
            .excluded
            .iter()
            .filter_map(|name| {
                let check = catalogue::CATALOGUE.iter().find(|row| row.name == name)?;
                Some(Unrun {
                    gate: name.clone(),
                    check: check.name,
                })
            })
            .collect();
        self.capabilities.extend(
            plan.needs_a_section
                .iter()
                .map(|check| Capability::inapplicable(check, None)),
        );
    }

    /// The run's axes once every record is in: an error row or a run-scope error makes it an
    /// error, a hole makes it incomplete, and a review item makes a pass a review. Spec 7.5.
    fn totals(&self) -> Axes {
        let mut axes = self.axes;
        axes.error |= self.errored() > 0;
        axes.incomplete |= !self.holes.is_empty();
        if !self.reviews.is_empty() {
            axes.judgement = axes.judgement.max(Judgement::Review);
        }
        axes
    }

    /// The text of a `klin check` that did not print JSON, or the JSON document, and the exit
    /// code of spec 7.4.
    pub fn finish(self, args: &Args, out: &mut String) -> u8 {
        let totals = self.totals();
        let exit = totals.exit();
        let (judgement, measurement) = match self.stopped {
            true => ("none", "none"),
            false => (totals.judgement.name(), totals.measurement()),
        };
        if !args.json() {
            for hole in &self.holes {
                let mut said = String::new();
                render::incomplete(hole, &mut said);
                let _ = write!(out, "  {said}");
            }
            let _ = writeln!(
                out,
                "judgement: {judgement}, measurement: {measurement}, execution: {}, exit {exit}",
                totals.execution()
            );
            return exit;
        }
        out.clear();
        let _ = writeln!(out, "{}", self.into_json());
        exit
    }

    /// The check document of spec 11.7, which the journal line of a Stop holds under `result`.
    /// Spec 13.1.
    pub fn into_json(self) -> Value {
        let totals = self.totals();
        let ran = !self.stopped;
        let run = ran.then(|| self.measured_run());
        let list = Value::Array;
        let mut out = Map::new();
        let mut put = |key: &str, value: Value| out.insert(key.to_string(), value);
        put("schema_version", 1.into());
        put("command", "check".into());
        put("klin", json!({ "version": env!("CARGO_PKG_VERSION") }));
        put("config", self.config.into());
        put("window", self.window.into());
        put("tree", self.tree.into());
        put("judgement", ran.then(|| totals.judgement.name()).into());
        put("measurement", ran.then(|| totals.measurement()).into());
        put("execution", totals.execution().into());
        put("exit", totals.exit().into());
        put(
            "capabilities",
            list(self.capabilities.iter().map(Capability::json).collect()),
        );
        put(
            "findings",
            list(self.findings.iter().map(Finding::json).collect()),
        );
        put(
            "reviews",
            list(self.reviews.iter().map(Review::json).collect()),
        );
        put("notes", list(self.notes.iter().map(Note::json).collect()));
        put(
            "measurements",
            list(run.into_iter().chain(self.measurements).collect()),
        );
        put("not_measured", self.not_measured.len().into());
        put(
            "errors",
            list(self.problems.iter().map(Problem::json).collect()),
        );
        let mut diagnostics = Map::new();
        diagnostics.insert("gates".to_string(), list(self.gates));
        put("diagnostics", Value::Object(diagnostics));
        Value::Object(out)
    }

    /// The measurement record of the run itself, which holds the run-level holes.
    fn measured_run(&self) -> Value {
        let holes: Vec<Value> = self.holes.iter().map(hole_record).collect();
        json!({
            "check": null,
            "basis": {
                "producer": null,
                "klin": env!("CARGO_PKG_VERSION"),
                "policy": self.built,
            },
            "state": if holes.is_empty() { "complete" } else { "incomplete" },
            "holes": holes,
        })
    }
}

/// Whether the text report prints this gate. The hook prints a gate that did not pass, and a
/// passing gate only where it left a note the hook tells, so the agent reads what it must act
/// on. Every other run prints every gate. The records keep every gate either way. Spec 9.5.
fn shows(args: &Args, code: u8, shown: &GateRecords) -> bool {
    match args.view {
        View::Json => false,
        View::Text => true,
        View::Stop => code != 0 || shown.notes.iter().any(Note::told),
    }
}

/// One gate's block of the text report: its provenance, its status row, and its rendered result.
fn printed(gate: &str, state: &str, (provenance, text): (&[String], &str), out: &mut String) {
    for line in provenance {
        let _ = writeln!(out, "  {line}");
    }
    let _ = writeln!(out, "  {state}  {gate}");
    for line in text.lines() {
        let _ = writeln!(out, "        {line}");
    }
}

fn counted(sorted: &[Unmeasured], class: Class, gate: &str) -> usize {
    sorted
        .iter()
        .filter(|item| item.class == class && item.gates.iter().any(|named| named == gate))
        .count()
}

/// The moves of the sections whose gates this run selected, and every move no section decides,
/// such as a file moved under a skipped directory, whichever gates run.
fn selected<'a>(moves: &'a Moves, wanted: &'a [&Gate]) -> impl Iterator<Item = &'a Moved> {
    moves.iter().filter(|moved| {
        moved
            .section()
            .is_none_or(|section| wanted.iter().any(|gate| gate.check.section == section))
    })
}

/// The review item of a moved pinned path or of a file moved under a skipped directory, and
/// nothing for a file moved out of a scope, whose findings carry `moved_out_of_scope`.
/// Spec 7.3, 11.7.
fn moved_review(moved: &Moved) -> Option<Review> {
    let (kind, path) = match moved {
        Moved::Pin { path, .. } => (contract::check::MOVED_PIN, path),
        Moved::Skipped { path, .. } => (contract::check::MOVED_SKIPPED, path),
        Moved::Out { .. } => return None,
    };
    let check = moved
        .section()
        .and_then(|section| {
            catalogue::CATALOGUE
                .iter()
                .find(|row| row.section == section)
        })
        .map(|row| row.name.to_string());
    Some(Review {
        check,
        kind,
        file: path.clone(),
        line: Slot::Null,
        text: moved.said().unwrap_or_default(),
        reason: moved.reason().map_or(Slot::Null, Slot::Is),
        values: None,
    })
}

/// Whether the run leaves a check that reads code to find its own roots: one it selected, or one
/// a whole run would select once the tree holds code.
pub fn leaves_code(args: &Args, plan: &Plan, wanted: &[&Gate], project: &Project) -> bool {
    let derives = |check: &catalogue::Row| check.reads_code() && !pins_in(project, check);
    let unselected =
        args.gates.is_empty() && plan.needs_a_section.iter().any(|check| derives(check));
    unselected || wanted.iter().any(|gate| derives(gate.check))
}

fn pins_in(project: &Project, check: &catalogue::Row) -> bool {
    project
        .config
        .pinned(check.section)
        .is_some_and(|section| section.get("in").is_some())
}

pub fn no_source_root_said(project: &Project) -> String {
    format!(
        "the survey of {} found no source root — a source root is a directory that holds \
         nothing but source files, so no gate that reads code ran here at all; run klin from \
         the tree you mean to gate, or set those gates to false to exclude them",
        project.root().display()
    )
}

/// The hole of a whole run that a check reading code was left to measure and that measured no
/// code: where the survey found no source root and nothing at all was measured, or where the
/// derivation commit held a source root that no source file of the working tree sits under. A documentation-only tree whose documents were measured is no
/// hole, and a tree that lost its source does not pass on its documents. #500 owns the finer
/// classification of the lost source. Spec 7.2.
fn unmeasured_run(
    args: &Args,
    (plan, wanted): (&Plan, &[&Gate]),
    project: &Project,
    measured: Measured,
) -> Option<Incomplete> {
    if !leaves_code(args, plan, wanted, project) || measured.code > 0 {
        return None;
    }
    let lost = held_roots(project);
    let nothing = measured.files == 0 && project.found_no_source_root();
    let text = match (nothing, lost.is_empty()) {
        (_, false) => format!(
            "the derivation commit held source under {} and the working tree holds no source \
             file there, so no check that reads code measured anything — restore the source, or \
             set those gates to false to exclude them",
            lost.join(", ")
        ),
        (true, true) => format!(
            "the repository holds no language or document klin measures: {} — write klin.json \
             naming one of: {}",
            no_source_root_said(project),
            every_check()
        ),
        (false, true) => return None,
    };
    Some(Incomplete {
        reason: Reason::NothingMeasured,
        detail: None,
        text,
    })
}

/// The source roots the derivation commit's survey held that no source file of the working
/// tree sits under any more. A file of no language klin reads, left where the source was, keeps
/// no root.
fn held_roots(project: &Project) -> Vec<String> {
    let files = project.tree().files().unwrap_or_default();
    let holds = |root: &str| {
        files.iter().any(|file| {
            survey::surveyed(file)
                && survey::source(file)
                && crate::config::scope::under_or_at(file, root)
        })
    };
    project
        .source_derivation()
        .map(|(held, _, _)| {
            held.roots
                .iter()
                .filter(|root| !holds(root))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// How many files of the run's scope each extension names, of the files in a language the
/// survey knows, counted once for every gate. A whole run's scope is every surveyed file, and a
/// changed run's is its changed files. Spec 7.2.
fn by_extension(project: &Project, against: &Against) -> BTreeMap<String, usize> {
    let files: Vec<&String> = match &against.scope {
        Some(scope) => scope.iter().collect(),
        None => project.tree().files().unwrap_or_default().iter().collect(),
    };
    let mut counted = BTreeMap::new();
    for file in files {
        if !survey::surveyed(file) || !survey::source(file) {
            continue;
        }
        if let Some((_, extension)) = file.rsplit_once('.') {
            *counted.entry(format!(".{extension}")).or_insert(0) += 1;
        }
    }
    counted
}

/// Whether a gate reads files of this extension, by the extensions its languages name.
fn reads(check: &catalogue::Row, extension: &str) -> bool {
    let quoted = format!("`{extension}`");
    check.languages.is_some_and(|languages| {
        languages()
            .iter()
            .any(|(_, listed)| listed.contains(&quoted))
    })
}

/// The files the accepted list holds as lost that the run did not find unmeasurable for any
/// reason, so their entries match nothing. An entry stays matched while its file is lost, a gap
/// or a limit, so it never goes stale while the grammar still lags. Spec 7.2, 7.6.
fn unmatched_lost<'a>(
    held: &'a [String],
    sorted: &'a [Unmeasured],
    wanted: &'a [&Gate],
) -> impl Iterator<Item = String> + 'a {
    held.iter()
        .filter(move |file| read_by(wanted, file))
        .filter(|file| !sorted.iter().any(|item| &item.file == *file))
        .cloned()
}

/// Whether a selected gate reads this file's language, so a run that measured it can say an
/// accepted entry for it matches nothing.
fn read_by(wanted: &[&Gate], file: &str) -> bool {
    let Some((_, extension)) = file.rsplit_once('.') else {
        return false;
    };
    let extension = format!(".{extension}");
    wanted.iter().any(|gate| reads(gate.check, &extension))
}

/// The files one gate could not measure, as it saw them, for the run to sort once. Spec 7.2.
fn unmeasured_by(gate: &str, told: &[Told]) -> Vec<(String, String, Seen)> {
    let seen = |file: &str, seen| (gate.to_string(), file.to_string(), seen);
    told.iter()
        .flat_map(|item| match item {
            Told::Hole(Hole::Unparsed(files)) => files
                .iter()
                .map(|file| seen(&file.file, Seen::Unread))
                .collect(),
            Told::Hole(Hole::Lost { file, why }) => vec![seen(file, Seen::Left(*why))],
            Told::Hole(Hole::Formed {
                file,
                form,
                measured,
            }) => vec![seen(file, Seen::Form(*form, *measured))],
            Told::Hole(Hole::Manifest { site, class }) => vec![seen(
                &site.file,
                Seen::Manifest {
                    class: *class,
                    why: site.text.clone(),
                },
            )],
            _ => Vec::new(),
        })
        .collect()
}

fn hole_record(hole: &Incomplete) -> Value {
    json!({ "reason": hole.reason.name(), "detail": hole.detail, "text": hole.text })
}

/// An integration claims only the results its report states, never that a file it did not
/// report on is clean. Spec 9.4.
fn coverage_claim(check: &catalogue::Row) -> &'static str {
    match check.activation {
        Activation::Integration => "unverified",
        Activation::Automatic | Activation::Policy => "verified",
    }
}

/// One measurement record: its basis, its state and its holes. Spec 8.1, 11.7.
fn measurement(name: &str, axes: Axes, holes: &[Incomplete], basis: Value) -> Value {
    json!({
        "check": name,
        "basis": basis,
        "state": axes.measurement(),
        "holes": holes.iter().map(hole_record).collect::<Vec<_>>(),
    })
}

/// The basis of a capability that measured nothing: what would have produced it, and the klin
/// that ran. Spec 8.1.
fn produced(check: &catalogue::Row) -> Value {
    json!({
        "producer": { "capability": check.name, "semantics_version": check.semantics },
        "klin": env!("CARGO_PKG_VERSION"),
    })
}

/// What one gate's measurement rests on: the producer and its semantics version, the klin that
/// ran, the policy by provenance (the derived values under `policy`, what a person pinned under
/// `pinned`, built-in for the rest) with the derivation commit, the integration entry, the scope
/// with its coverage counts, the window and the reason of each hole. It holds no machine path,
/// time, process id or duration, and it reads only what the run already holds, so the Stop
/// starts no git process and reads no file for it. Spec 8.1.
fn basis(
    gate: &Gate,
    (project, against): (&Project, &Against),
    records: &Records,
    shown: &GateRecords,
) -> Value {
    let pinned = pinned_entry(gate, project);
    let integration = (gate.check.activation == Activation::Integration).then(|| {
        json!({
            "entry": gate.name,
            "run": pinned.and_then(|entry| entry.get("run")),
            "report": pinned.and_then(|entry| entry.get("report")),
        })
    });
    let mut basis = produced(gate.check);
    basis["policy"] = shown.derived.clone().into();
    basis["pinned"] = pinned.cloned().into();
    basis["derivation"] = project.surveyed_commit().into();
    basis["integration"] = integration.into();
    basis["scope"] = json!({
        "changed": against.scope.is_some(),
        "coverage": records.coverage.as_ref().map(diagnostics::coverage_json),
    });
    basis["window"] = against.base.as_ref().map_or(Value::Null, |base| {
        json!({ "kind": base.kind.name(), "before": base.before, "after": "the working tree" })
    });
    basis["holes"] = shown
        .holes
        .iter()
        .map(|hole| json!({ "reason": hole.reason.name() }))
        .collect();
    basis
}

/// What a person pinned for this gate: its own entry where each entry is a gate, and the whole
/// section otherwise.
fn pinned_entry<'a>(gate: &Gate, project: &'a Project) -> Option<&'a Value> {
    let section = project.config.pinned(gate.check.section)?;
    match gate.check.gate_per_entry {
        true => section.as_array()?.iter().find(|entry| {
            entry
                .get(contract::check::NAMED.name)
                .and_then(Value::as_str)
                == Some(gate.name.as_str())
        }),
        false => Some(section),
    }
}

/// The capability-scope errors of a gate that is exit 2, each under its kind. A gate that
/// could not read its own configuration is a configuration error. A file or form klin could not
/// measure is klin's own limit until spec 7.2 sorts it into its class. Spec 7.3.
fn gate_errors(gate: &str, code: u8, told: &[Told]) -> Vec<Problem> {
    if code != 2 {
        return Vec::new();
    }
    let mut errors: Vec<Problem> = told.iter().flat_map(|item| erred(gate, item)).collect();
    if errors.is_empty() {
        errors.push(Problem::new(
            ErrorKind::Internal,
            Some(gate),
            "the gate stopped before it finished its measurement",
        ));
    }
    errors
}

/// The errors one item of a gate's result names: one per site for a file or form klin could
/// not measure, with its site and its 0.x outcome as the reason, so no site is lost.
fn erred(gate: &str, item: &Told) -> Vec<Problem> {
    let one = |message: &str| vec![Problem::new(ErrorKind::Configuration, Some(gate), message)];
    match item {
        Told::Plain(Plain::Error(problem)) => one(problem),
        Told::Plain(Plain::PathMissing(named)) => one(&format!(
            "{named} — correct the path, or take it out of \"in\"."
        )),
        _ => Vec::new(),
    }
}

/// The aggregation of spec 7.5 over in-memory results, for the holes no shipped capability can
/// reach yet. `work-limit` is one: no shipped capability has a work bound. A ride-along finding
/// beside an error, or beside a failure its gate found apart, is another: no shipped gate with a
/// ride-along row errs or fails apart from its findings. AGENTS.md.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::file::Config;
    use crate::contract::check::{Counted, Line};
    use crate::contract::ratchet::{self, Evaluator, Remedy};
    use crate::sys::record::Values;

    const RIDER: &str = "rider";

    const EVALUATOR: Evaluator<'static> = Evaluator {
        metrics: &["count"],
        unit: "site(s)",
        condition: "where the code opts out",
        fix_advice: Remedy::Fixed("Fix it."),
        ceiling: None,
        format_metrics: |_| "count 1".to_string(),
        nested: None,
        rides_along: |values| values.get("kind").and_then(Value::as_str) == Some(RIDER),
        worsened: ratchet::Worsened::Fails,
        fails: None,
    };

    fn site(kind: &str) -> ratchet::Finding {
        let mut values = Values::new();
        values.insert("kind".into(), kind.into());
        values.insert("count".into(), 1.into());
        ratchet::Finding {
            file: format!("src/{kind}.ts"),
            line: 3,
            text: format!("site of a {kind}"),
            values,
            body: None,
        }
    }

    fn found(at: &Context, kinds: &[&str], out: &mut Sink) -> u8 {
        let line = Line::new(
            Counted::Markers {
                sites: kinds.len(),
                unit: "site(s)",
                skipped: 0,
            },
            None,
        );
        let now = kinds.iter().map(|kind| site(kind)).collect();
        EVALUATOR.evaluate(now, Vec::new(), Vec::new(), at, line, out)
    }

    fn gate(name: &'static str, run: contract::check::Run) -> Gate {
        let shipped = catalogue::CATALOGUE
            .iter()
            .find(|row| row.name == "escapes")
            .expect("the escapes row");
        let row = catalogue::Row {
            name,
            run,
            labels: catalogue::Labels {
                one: "site",
                many: "sites",
            },
            ..*shipped
        };
        Gate {
            name: name.to_string(),
            check: Box::leak(Box::new(row)),
        }
    }

    fn ran(view: View, gates: &[Gate]) -> (CheckDocument, String) {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let root = dir.path();
        let project = Project::of(Config::empty(&root.join("klin.json")), root);
        let args = Args {
            changed: true,
            view,
            ..Args::default()
        };
        let wanted: Vec<&Gate> = gates.iter().collect();
        let mut doc = CheckDocument::default();
        let mut out = String::new();
        let plan = Plan::default();
        let against = Against::default();
        doc.ran(
            &args,
            &project,
            (&plan, &wanted, Vec::new()),
            &against,
            &mut out,
        );
        (doc, out)
    }

    #[test]
    fn a_stop_whose_only_findings_ride_along_on_a_gate_that_erred_does_not_print_them() {
        let erred = gate("erred", |at, out| {
            found(at, &[RIDER], out);
            Err(Error("the tool stopped".to_string()))
        });
        let (doc, out) = ran(View::Stop, &[erred]);

        assert_eq!(doc.failed(), 0, "{out}");
        assert!(out.contains("the tool stopped"), "{out}");
        assert!(!out.contains("src/rider.ts"), "{out}");
    }

    #[test]
    fn a_ride_along_finding_does_not_mask_a_failure_its_gate_found_apart() {
        let independent = gate("independent", |at, out| {
            found(at, &[RIDER], out);
            Ok(1)
        });
        let (doc, out) = ran(View::Stop, &[independent]);
        let (check, _) = ran(
            View::Json,
            &[gate("independent", |at, out| {
                found(at, &[RIDER], out);
                Ok(1)
            })],
        );

        assert_eq!(doc.failed(), 1, "{out}");
        assert!(out.contains("src/rider.ts:3"), "{out}");
        assert_eq!(check.into_json()["judgement"], "fail");
    }

    fn work_limited() -> GateRecords {
        render::records(
            "gate",
            &[Told::Incomplete(Incomplete {
                reason: Reason::WorkLimit,
                detail: None,
                text: "stopped at its bound".to_string(),
            })],
        )
    }

    #[test]
    fn a_work_limit_hole_makes_the_gate_incomplete_and_the_run_exit_3() {
        let gate = axes_of(0, &work_limited());

        assert_eq!(gate.state(View::Text), "INCOMPLETE");
        assert_eq!(gate.measurement(), "incomplete");
        assert_eq!(Axes::default().and(gate).exit(), 3);
        assert_eq!(work_limited().holes[0].reason.name(), "work-limit");
    }

    #[test]
    fn a_failing_gate_beside_a_work_limit_hole_exits_1_and_an_error_exits_2() {
        let hole = axes_of(0, &work_limited());
        let failed = Axes {
            judgement: Judgement::Fail,
            ..Axes::default()
        };
        let erred = Axes {
            error: true,
            ..Axes::default()
        };

        assert_eq!(hole.and(failed).exit(), 1);
        assert_eq!(hole.and(failed).and(erred).exit(), 2);
    }
}
