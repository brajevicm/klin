//! The renderers over a gate's typed result. The checks and the ratchet return `Told` items, and
//! this writes them in their order: as the text `klin check` prints, as the text the Stop hook
//! prints, as the `derived:` and `pinned:` lines above a gate's row, and as the findings, notes
//! and derived entries of the JSON. Spec 3.1, 6.1, 11.1, 11.2.

use std::fmt::Write;

use serde_json::{Map, Value};

use crate::ceiling::Ceiling;
use crate::check::contract::{
    self, AMBIGUOUS, Caller, Cause, Class, Complexity, Counted, DELETED, DELETED_TEST, DERIVATION,
    Derived, Entry, Failed, Held, HeldAtBase, Hole, Incomplete, Judged, Layering, Line, Listed,
    Located, Matched, Measured, Plain, Provenance, PublicApi, Ratchet, Standing, Told, UNBUILT,
    UNMATCHED, UNMATCHED_ACCEPTED, UNRESOLVED, Unmatched, Unresolvable, Wording,
};
use crate::check::holes::{self, Unmeasured};
use crate::config::MEASUREMENT_LOST;
use crate::coverage::Coverage;

/// How many rows a listed block prints before it says how many more there are.
const SHOWN: usize = 20;

/// The report text `klin check` prints of one gate's result.
pub fn text(told: &[Told]) -> String {
    rendered(told, true)
}

/// The report text the Stop hook prints of one gate's result. A gate that passed prints only what
/// it tells beside its success, so the agent reads what it must act on. Spec 9.5.
pub fn stop(told: &[Told], passed: bool) -> String {
    rendered(told, !passed)
}

fn rendered(told: &[Told], with_ok: bool) -> String {
    let mut out = String::new();
    for item in told {
        one(item, with_ok, &mut out);
    }
    out
}

/// One item, where `with_ok` says whether its `OK:` lines print.
fn one(told: &Told, with_ok: bool, out: &mut String) {
    match told {
        Told::Judged { line, held } if with_ok => judged(line, *held, out),
        Told::Document {
            name,
            words,
            ceiling,
            standing,
        } => document((name, *words, ceiling), standing, with_ok, out),
        other => said(other, out),
    }
}

/// One item that says the same in every report.
fn said(told: &Told, out: &mut String) {
    match told {
        Told::Plain(said) => plain(said, out),
        Told::Hole(hole) => holed(hole, out),
        Told::Listed(listed) => sites(listed, out),
        Told::Ratchet(said) => ratchet(said, out),
        Told::Incomplete(hole) => incomplete(hole, out),
        Told::Judged { .. } | Told::Document { .. } | Told::Provenance(_) => (),
    }
}

fn plain(said: &Plain, out: &mut String) {
    let _ = match said {
        Plain::Note(note) => writeln!(out, "NOTE: {}", note.text),
        Plain::Remedy(text) => writeln!(out, "{text}"),
        Plain::PathMissing(named) => writeln!(
            out,
            "FAIL: {named} — correct the path, or take it out of \"in\"."
        ),
        Plain::Error(problem) => writeln!(out, "ERR: {problem}"),
    };
}

/// The `HOLE:` line of a measurement that is not complete: its reason, its detail, its words.
pub fn incomplete(hole: &Incomplete, out: &mut String) {
    let detail = hole
        .detail
        .map(|detail| format!(" ({detail})"))
        .unwrap_or_default();
    let _ = writeln!(out, "HOLE: {}{detail} — {}", hole.reason.name(), hole.text);
}

/// The forms a gate could not resolve, under its row. A file a gate could not measure prints
/// once for the run, where the runner sorts it. Spec 7.2.
fn holed(hole: &Hole, out: &mut String) {
    let Hole::Unresolved {
        opened,
        kind,
        forms,
    } = hole
    else {
        return;
    };
    let word = match opened {
        true => "REVIEW",
        false => "NOTE",
    };
    block(
        out,
        &format!("{word}: {} {}:", forms.len(), unresolved(*kind)),
        forms
            .iter()
            .map(|(form, why)| format!("{}  {}  — {why}", at(form), form.text)),
    );
}

/// What a block of unresolved forms of one kind says they are.
fn unresolved(kind: Unresolvable) -> &'static str {
    match kind {
        Unresolvable::Dependency => {
            "dependency form(s) klin resolves could not be resolved, so what they reach was not judged"
        }
        Unresolvable::PublicSurface => {
            "form(s) inside a supported public surface could not be resolved, so the surface is not completely measured"
        }
    }
}

/// A heading and its rows indented under it.
fn block(out: &mut String, heading: &str, rows: impl Iterator<Item = String>) {
    let _ = writeln!(out, "{heading}");
    for row in rows {
        let _ = writeln!(out, "  {row}");
    }
}

fn sites(listed: &Listed, out: &mut String) {
    let (heading, rows): (String, Vec<String>) = match listed {
        Listed::DeadSymbols(dead) => (
            format!("REPORT: {} dead symbol(s):", dead.len()),
            dead.iter()
                .map(|(site, name)| format!("{}  {name}  {}", at(site), site.text))
                .collect(),
        ),
        Listed::TestsDeleted { went, caller } => (
            format!(
                "{}: {} test site(s) the base holds went in this window:",
                said_as(*caller),
                went.len()
            ),
            went.iter()
                .map(|site| format!("{}  {}", at(site), site.text))
                .collect(),
        ),
        Listed::TestFunctionsOrphaned(went) => (
            format!(
                "NOTE: {} deleted test function(s) whose file went in the same window:",
                went.len()
            ),
            went.iter()
                .map(|site| format!("{}  {}  its file went too", at(site), site.text))
                .collect(),
        ),
        Listed::TestFilesPaired { files, rule } => (
            format!(
                "NOTE: {} deleted test file(s) whose subject went in the same window:",
                files.len()
            ),
            files
                .iter()
                .map(|site| format!("{}  its subject {} went too", site.file, site.text))
                .chain([format!("each subject matched by {rule}")])
                .collect(),
        ),
        Listed::Held(_) | Listed::NoSurface(_) => {
            let _ = writeln!(out, "NOTE: {}", noted(listed).unwrap_or_default());
            return;
        }
    };
    let _ = writeln!(out, "{heading}");
    for row in rows {
        let _ = writeln!(out, "  {row}");
    }
}

fn ratchet(said: &Ratchet, out: &mut String) {
    match said {
        Ratchet::New {
            unit,
            condition,
            held,
            failed,
        } => {
            let _ = writeln!(
                out,
                "FAIL: {} new {unit} {condition}, beyond the {held} the base holds:",
                failed.len()
            );
            for (lead, inside) in grouped(failed) {
                for (indent, at) in
                    std::iter::once(("", lead)).chain(inside.into_iter().map(|at| ("  ", at)))
                {
                    let finding = &failed[at];
                    let _ = writeln!(
                        out,
                        "  {indent}{}:{}  {}  {}{}",
                        finding.file,
                        finding.line,
                        finding.shown,
                        clip(&finding.text),
                        against(finding)
                    );
                }
            }
        }
        Ratchet::Worse { unit, failed, .. } => worse(unit, failed, out),
        Ratchet::AcceptedUnmatched {
            entries: unmatched,
            caller,
        } => listed(
            out,
            &format!(
                "{}: {} accepted entr{} matched nothing this run:",
                said_as(*caller),
                unmatched.len(),
                entries(unmatched.len())
            ),
            &unmatched.iter().map(unmatched_row).collect::<Vec<_>>(),
        ),
    }
}

/// Every new finding once, as the leads in their order, each with the findings that print inside
/// its group. A finding whose named lead is itself inside a group, or out of range, leads.
fn grouped(found: &[Failed]) -> Vec<(usize, Vec<usize>)> {
    let lead = |at: usize| {
        found[at]
            .lead
            .filter(|held| *held != at && found.get(*held).is_some_and(|lead| lead.lead.is_none()))
    };
    let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut slot: Vec<Option<usize>> = vec![None; found.len()];
    for at in (0..found.len()).filter(|at| lead(*at).is_none()) {
        slot[at] = Some(groups.len());
        groups.push((at, Vec::new()));
    }
    for at in 0..found.len() {
        if let Some(group) = lead(at).and_then(|held| slot[held]) {
            groups[group].1.push(at);
        }
    }
    groups
}

fn worse(unit: &str, failed: &[Failed], out: &mut String) {
    let _ = writeln!(
        out,
        "FAIL: {} {unit} got worse — the ratchet only tightens:",
        failed.len()
    );
    for finding in failed {
        let (was, from) = finding
            .was
            .as_ref()
            .map(|was| (was.shown.as_str(), was.at.as_deref()))
            .unwrap_or_default();
        let from = from.map(|file| format!(" at {file}")).unwrap_or_default();
        let _ = writeln!(
            out,
            "  {}:{}  {}, was {was}{from}  {}{}",
            finding.file,
            finding.line,
            finding.shown,
            clip(&finding.text),
            against(finding)
        );
    }
}

fn at(site: &Located) -> String {
    format!("{}:{}", site.file, site.line)
}

fn entries(count: usize) -> &'static str {
    if count == 1 { "y" } else { "ies" }
}

/// The `OK:` line: the check's phrase, why every finding passed in the words the comparison
/// proved, and the coverage. A run that judged no finding claims nothing. Spec 8.6.
fn judged(line: &Line, held: Held, out: &mut String) {
    let qualifier = match (held.accepted, held.base) {
        (0, 0) => String::new(),
        (0, _) => ", all held at the base".to_string(),
        (_, 0) => ", all on the accepted list".to_string(),
        (accepted, base) => {
            format!(", {base} held at the base and {accepted} on the accepted list")
        }
    };
    let coverage = line.coverage.as_ref().map(covered).unwrap_or_default();
    let (state, aside, after) = match &line.judged {
        Judged::Counted(counted) => (
            counted_state(counted),
            counted_aside(counted),
            String::new(),
        ),
        Judged::Measured(measured) => measured_phrases(measured),
    };
    let _ = writeln!(out, "OK: {state}{qualifier}{aside}{coverage}{after}");
}

fn counted_state(counted: &Counted) -> String {
    match counted {
        Counted::Documents(count) => format!("{count} document(s) judged"),
        Counted::Conventions(count) => format!("{count} convention(s) judged"),
        Counted::Convention { name, sites } => format!("{name}: {sites} site(s)"),
        Counted::Citations(sites) => format!("{sites} citation(s) resolve nowhere"),
        Counted::Dependencies { judged, manifests } => format!(
            "{judged} dependenc{} in {manifests} manifest(s), each locked and pinned",
            entries(*judged)
        ),
        Counted::Tests { held, gone } => held_tests(*held, *gone),
        Counted::Markers { sites, unit, .. } => format!("{sites} {unit} in the tree"),
    }
}

fn held_tests(held: usize, gone: usize) -> String {
    match gone {
        0 => format!("{held} test site(s) the base holds"),
        gone => format!("{held} test site(s) the base holds, {gone} of them gone"),
    }
}

fn counted_aside(counted: &Counted) -> String {
    match counted {
        Counted::Markers { skipped, .. } if *skipped > 0 => {
            format!(" ({skipped} in tests skipped)")
        }
        _ => String::new(),
    }
}

/// The state, the aside before the coverage, and what closes the line, of a gate that measures
/// structure.
fn measured_phrases(measured: &Measured) -> (String, String, String) {
    let state = |state: String| (state, String::new(), String::new());
    match measured {
        Measured::DeadSymbols { judged, dead } => state(format!(
            "{judged} declaration(s) judged, {dead} dead symbol(s)"
        )),
        Measured::Reachability {
            judged,
            unreached,
            unjudged,
        } => state(format!(
            "{judged} file(s) judged, {unreached} unreached, {unjudged} measured with no \
             eligible declaration"
        )),
        Measured::Complexity(complexity) => (
            format!(
                "{} function(s) judged, {} over the gate{}",
                complexity.judged,
                complexity.over,
                in_force(&complexity.steps)
            ),
            unjudged_tests(complexity),
            String::new(),
        ),
        Measured::Layering(layering) => layering_phrases(layering),
        Measured::PublicApi(api) => public_api_phrases(api),
        Measured::Sarif {
            judged,
            differential: true,
            ..
        } => state(format!(
            "{judged} result(s) judged, which is every result the scanner reported"
        )),
        Measured::Sarif { judged, held, .. } => state(format!(
            "{judged} result(s) on lines this window changed, {held} held on lines it did not"
        )),
    }
}

/// The dated steps among the ceilings in force, and nothing where a person pinned each one.
fn in_force(steps: &[(&str, Ceiling)]) -> String {
    let steps: Vec<String> = steps
        .iter()
        .filter(|(_, ceiling)| ceiling.step.is_some())
        .map(|(key, ceiling)| format!("{key} {ceiling}"))
        .collect();
    match steps.is_empty() {
        true => String::new(),
        false => format!(" under {}", steps.join(" and ")),
    }
}

fn unjudged_tests(complexity: &Complexity) -> String {
    if complexity.unjudged == 0 {
        return String::new();
    }
    let named = match complexity.arrived.is_empty() {
        true => String::new(),
        false => format!("; added or renamed: {}", complexity.arrived.join(", ")),
    };
    format!(
        "; {} test function(s) not judged on length, with no test_lines pinned{named}",
        complexity.unjudged
    )
}

fn layering_phrases(layering: &Layering) -> (String, String, String) {
    (
        format!(
            "{} dependency site(s) judged, {} forbidden, {} cyclic",
            layering.sites, layering.forbidden, layering.cyclic
        ),
        String::new(),
        format!(
            "; {} file(s) attached, {} by a Cargo manifest and {} by a conventional root, {} not attached, {} external or unsupported dependenc(ies)",
            layering.attached,
            layering.by_manifest,
            layering.by_convention,
            layering.unattached,
            layering.external
        ),
    )
}

fn public_api_phrases(api: &PublicApi) -> (String, String, String) {
    (
        format!(
            "{} external item(s) on {} surface(s) judged against the base, {} measured, {} opaque, no removal or contract change",
            api.items, api.surfaces, api.measured, api.opaque
        ),
        String::new(),
        format!(
            "; {} Rust library target(s), {} TypeScript entry point(s), {} package(s) or target(s) with no supported surface",
            api.rust, api.typescript, api.inapplicable
        ),
    )
}

fn covered(coverage: &Coverage) -> String {
    match coverage.not_measured {
        0 => format!(
            " ({} file(s) found, {} measured, {} excluded, {} unreadable)",
            coverage.found, coverage.measured, coverage.excluded, coverage.unreadable
        ),
        not_measured => format!(
            " ({} file(s) found, {} measured, {} not measured, {} excluded, {} unreadable)",
            coverage.found, coverage.measured, not_measured, coverage.excluded, coverage.unreadable
        ),
    }
}

fn document(
    (name, words, ceiling): (&str, u64, &Ceiling),
    standing: &Standing,
    with_ok: bool,
    out: &mut String,
) {
    match standing {
        Standing::Under | Standing::Near(_) if with_ok => {
            let _ = writeln!(out, "OK: {name} is {words} words, ceiling {ceiling}");
        }
        _ => (),
    }
    match standing {
        Standing::Under => (),
        Standing::Near(remaining) => {
            let _ = writeln!(
                out,
                "WARN: {name} is {words} words, {remaining} from its ceiling of {ceiling}."
            );
        }
        Standing::Held(_) if !with_ok => (),
        Standing::Held(before) => {
            let _ = writeln!(
                out,
                "OK: {name} is {words} words, over its ceiling of {ceiling}, held at the base \
                 at {before} words"
            );
        }
        Standing::Over { fix_advice, .. } => {
            let _ = writeln!(
                out,
                "FAIL: {name} is {words} words, over its ceiling of {ceiling}."
            );
            let _ = writeln!(out, "{fix_advice}");
        }
    }
}

/// What one failure was judged against, and the ceiling in force beside it. Spec 8.6.
fn against(finding: &Failed) -> String {
    let matched = match &finding.matched {
        Matched::Nothing => "nothing matched".to_string(),
        Matched::Accepted(entry) => format!("matched the accepted entry for {}", entry.file),
        Matched::Base(entry) => format!(
            "matched the base site at {}:{}",
            entry.file,
            entry.line.unwrap_or(0)
        ),
    };
    match &finding.ceiling {
        Some(ceiling) => format!("  — {matched}, ceiling {ceiling}"),
        None => format!("  — {matched}"),
    }
}

/// The word a line opens with for what fails nothing: `REVIEW` at `klin check`, `NOTE` at the
/// Stop. Spec 7.6, 9.2.
fn said_as(caller: Caller) -> &'static str {
    match caller {
        Caller::Gate => "REVIEW",
        Caller::Hook => "NOTE",
    }
}

fn unmatched_row(entry: &Unmatched) -> String {
    let retired = entry
        .retired
        .as_ref()
        .map(|went| format!("  — {went}"))
        .unwrap_or_default();
    format!(
        "{}  {}  {}{retired}",
        entry.entry.file,
        entry.shown,
        clip(&entry.entry.text)
    )
}

fn listed(out: &mut String, heading: &str, rows: &[String]) {
    let _ = writeln!(out, "{}", capped(heading, rows));
}

/// A heading and its first rows indented under it, with how many more there are.
fn capped(heading: &str, rows: &[String]) -> String {
    let mut out = heading.to_string();
    for row in rows.iter().take(SHOWN) {
        let _ = write!(out, "\n  {row}");
    }
    if rows.len() > SHOWN {
        let _ = write!(out, "\n  … and {} more", rows.len() - SHOWN);
    }
    out
}

/// The text of a listed NOTE that the JSON records as one note, and nothing for any other list.
fn noted(listed: &Listed) -> Option<String> {
    let (heading, rows): (String, Vec<String>) = match listed {
        Listed::Held(HeldAtBase::DeadSymbols(dead)) => (
            format!("{} dead symbol(s) the base already held:", dead.len()),
            dead.iter()
                .map(|site| format!("{}  {}", at(site), site.text))
                .collect(),
        ),
        Listed::Held(HeldAtBase::Edges(edges)) => (
            format!(
                "{} forbidden or cyclic edge(s) the base already held:",
                edges.len()
            ),
            edges
                .iter()
                .map(|edge| format!("{}  {}", edge.file, edge.text))
                .collect(),
        ),
        Listed::Held(HeldAtBase::Unreached(files)) => (
            format!("{} unreached file(s) the base already held:", files.len()),
            files.clone(),
        ),
        Listed::NoSurface(held) => (
            format!(
                "{} package(s) or target(s) with no supported public surface:",
                held.len()
            ),
            held.iter()
                .map(|held| format!("{}: {}", held.file, held.text))
                .collect(),
        ),
        _ => return None,
    };
    Some(capped(&heading, &rows))
}

/// The `derived:` and `pinned:` lines of the values a gate used, which the report prints above
/// its row. Spec 4.3.
pub fn provenance(told: &[Told]) -> Vec<String> {
    told.iter()
        .filter_map(|item| match item {
            Told::Provenance(said) => Some(provenance_line(said)),
            _ => None,
        })
        .collect()
}

fn provenance_line(said: &Provenance) -> String {
    let derived = match said {
        Provenance::Pinned {
            section,
            key,
            shown,
        } => return format!("pinned: {section} {key} {shown}"),
        Provenance::Derived(derived) => derived,
    };
    let Derived {
        section,
        key,
        shown,
        rule,
        wording,
        ..
    } = derived;
    let key = key
        .as_deref()
        .map(|key| format!(" {key}"))
        .unwrap_or_default();
    match wording {
        Wording::Keyed => format!("derived: {section}{key} {shown}, {rule}"),
        Wording::Bare => format!("derived:{key} {shown}, {rule}"),
        Wording::Sampled(recorded) => format!("derived: {section}{key} {shown} ({rule}){recorded}"),
    }
}

/// A key of a record that the check document leaves out, writes as `null`, or writes with a
/// value, because the three say different things. Spec 11.7.
#[derive(Clone, Default)]
pub enum Slot<T> {
    #[default]
    Absent,
    Null,
    Is(T),
}

impl<T: Clone + Into<Value>> Slot<T> {
    fn entry(&self, key: &str) -> Option<(String, Value)> {
        match self {
            Slot::Absent => None,
            Slot::Null => Some((key.to_string(), Value::Null)),
            Slot::Is(value) => Some((key.to_string(), value.clone().into())),
        }
    }
}

/// A key a record writes only when it holds a value.
fn entry<T: Into<Value>>(key: &str, value: Option<T>) -> Option<(String, Value)> {
    value.map(|value| (key.to_string(), value.into()))
}

impl Slot<String> {
    /// The file of a record, left out where the site names none.
    fn file(file: &str) -> Slot<String> {
        match file.is_empty() {
            true => Slot::Absent,
            false => Slot::Is(file.to_string()),
        }
    }
}

/// How a finding stands against what the ratchet holds. Only a held finding passes. Spec 11.7.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    New,
    Worsened,
    Held,
}

impl Outcome {
    fn name(self) -> &'static str {
        match self {
            Outcome::New => "new",
            Outcome::Worsened => "worsened",
            Outcome::Held => "held",
        }
    }
}

/// A finding of the check document. Spec 11.7.
#[derive(Clone)]
pub struct Finding {
    pub id: String,
    pub check: Option<String>,
    pub kind: &'static str,
    pub outcome: Outcome,
    pub file: String,
    pub line: Option<u64>,
    pub text: Option<String>,
    pub values: Map<String, Value>,
    pub ceiling: Value,
    pub matched: Value,
    pub condition: String,
    pub remedy: String,
}

impl Finding {
    pub fn failing(&self) -> bool {
        self.outcome != Outcome::Held
    }

    /// The site as `file:line  text`, which a Stop records for a deleted test it has not asked
    /// about. Spec 9.2.
    pub fn site(&self) -> String {
        let line = self.line.unwrap_or_default();
        let text = self.text.as_deref().unwrap_or_default();
        format!("{}:{line}  {text}", self.file)
    }

    pub fn json(&self) -> Value {
        serde_json::json!({
            "id": self.id,
            "check": self.check,
            "kind": self.kind,
            "outcome": self.outcome.name(),
            "file": self.file,
            "line": self.line,
            "text": self.text,
            "values": self.values,
            "ceiling": self.ceiling,
            "matched": self.matched,
            "condition": self.condition,
            "remedy": self.remedy,
        })
    }
}

/// A note of the check document: the words a person reads, and what the note is about. Spec 11.7.
#[derive(Clone, Default)]
pub struct Note {
    pub check: Option<String>,
    pub kind: &'static str,
    pub message: String,
    pub coverage: Option<bool>,
    pub file: Slot<String>,
    pub line: Option<u64>,
    pub values: Option<Map<String, Value>>,
}

impl Note {
    /// A note about the run, with no check and no site.
    pub fn of_run(kind: &'static str, message: &str) -> Note {
        Note {
            kind,
            message: message.to_string(),
            coverage: Some(false),
            ..Note::default()
        }
    }

    /// Whether the hook tells a person this note even when nothing blocks the stop: a file the
    /// run could not read or stopped measuring, and a deleted test the run let through. Spec 8.2,
    /// 8.6, 14.
    pub fn told(&self) -> bool {
        matches!(
            self.kind,
            DELETED | DERIVATION | UNRESOLVED | AMBIGUOUS | UNBUILT | UNMATCHED
        )
    }

    pub fn json(&self) -> Value {
        let mut out = Map::new();
        out.insert("check".into(), self.check.clone().into());
        out.insert("kind".into(), self.kind.into());
        out.insert("message".into(), self.message.clone().into());
        out.extend(entry("coverage", self.coverage));
        out.extend(self.file.entry("file"));
        out.extend(entry("line", self.line));
        out.extend(entry("values", self.values.clone()));
        Value::Object(out)
    }
}

/// A review item of the check document, which never changes an exit code. Spec 11.7.
#[derive(Clone, Default)]
pub struct Review {
    pub check: Option<String>,
    pub kind: &'static str,
    pub file: String,
    pub line: Slot<u64>,
    pub text: String,
    pub reason: Slot<String>,
    pub values: Option<Map<String, Value>>,
}

impl Review {
    pub fn json(&self) -> Value {
        let mut out = Map::new();
        out.insert("check".into(), self.check.clone().into());
        out.insert("kind".into(), self.kind.into());
        out.insert("file".into(), self.file.clone().into());
        out.extend(self.line.entry("line"));
        out.insert("text".into(), self.text.clone().into());
        out.extend(self.reason.entry("reason"));
        out.extend(entry("values", self.values.clone()));
        Value::Object(out)
    }
}

/// What one gate's result adds to the check document of spec 11.7: its findings, notes and
/// review items under the gate's name, the derived entries of its policy, and its holes.
#[derive(Default)]
pub struct GateRecords {
    gate: String,
    pub findings: Vec<Finding>,
    pub notes: Vec<Note>,
    pub reviews: Vec<Review>,
    pub derived: Vec<Value>,
    pub holes: Vec<Incomplete>,
}

/// The records of one gate's result, in the order the result says them. Spec 11.7.
pub fn records(gate: &str, told: &[Told]) -> GateRecords {
    let mut out = GateRecords {
        gate: gate.to_string(),
        ..GateRecords::default()
    };
    for item in told {
        record_one(item, &mut out);
    }
    out
}

/// The derived entries of a result, which `klin policy` lists beside the lines it prints.
pub fn derived(told: &[Told]) -> Vec<Value> {
    told.iter()
        .filter_map(|item| match item {
            Told::Provenance(said) => derived_json(said),
            _ => None,
        })
        .collect()
}

fn record_one(told: &Told, out: &mut GateRecords) {
    match told {
        Told::Document {
            name,
            words,
            ceiling,
            standing,
        } => document_records((name, *words, ceiling.value), standing, out),
        Told::Provenance(said) => out.derived.extend(derived_json(said)),
        Told::Plain(said) => out.notes.extend(note_record(said, &out.gate)),
        other => found_records(other, out),
    }
}

/// What a gate found beyond its own documents, policy and plain lines.
fn found_records(told: &Told, out: &mut GateRecords) {
    match told {
        Told::Hole(hole) => hole_records(hole, out),
        Told::Listed(listed) => listed_records(listed, out),
        Told::Ratchet(said) => ratchet_records(said, out),
        Told::Incomplete(hole) => out.holes.push(hole.clone()),
        Told::Judged { .. } | Told::Document { .. } | Told::Provenance(_) | Told::Plain(_) => (),
    }
}

/// The entry of a derived value, and nothing for a pinned one, which the config already holds.
fn derived_json(said: &Provenance) -> Option<Value> {
    let Provenance::Derived(derived) = said else {
        return None;
    };
    let rule = match &derived.wording {
        Wording::Sampled(recorded) => format!("{}{recorded}", derived.rule),
        Wording::Keyed | Wording::Bare => derived.rule.clone(),
    };
    Some(contract::derived_entry(
        derived.section,
        derived.key.as_deref(),
        derived.value.clone(),
        &rule,
    ))
}

/// The note of a NOTE line, and nothing for a line the JSON records elsewhere or not at all.
fn note_record(said: &Plain, gate: &str) -> Option<Note> {
    let Plain::Note(note) = said else {
        return None;
    };
    Some(site_note(
        gate,
        note.outcome,
        (&note.file, None),
        &note.text,
    ))
}

/// The note of one site: what it is, the file and line it names where it names them, its text.
fn site_note(
    gate: &str,
    kind: &'static str,
    (file, line): (&str, Option<u64>),
    text: &str,
) -> Note {
    Note {
        check: Some(gate.to_string()),
        kind,
        message: text.to_string(),
        coverage: Some(false),
        file: Slot::file(file),
        line,
        ..Note::default()
    }
}

fn document_records(
    (name, words, ceiling): (&str, u64, u64),
    standing: &Standing,
    out: &mut GateRecords,
) {
    let mut values = Map::new();
    values.insert("words".into(), words.into());
    values.insert("ceiling".into(), ceiling.into());
    match standing {
        Standing::Under | Standing::Held(_) => (),
        Standing::Near(_) => out.notes.push(Note {
            values: Some(values),
            ..site_note(&out.gate, "near-ceiling", (name, None), "near-ceiling")
        }),
        Standing::Over {
            id,
            condition,
            fix_advice,
        } => out.findings.push(Finding {
            id: id.clone(),
            check: Some(out.gate.clone()),
            kind: METRIC,
            outcome: Outcome::New,
            file: name.to_string(),
            line: None,
            text: None,
            values,
            ceiling: serde_json::json!({ "words": ceiling }),
            matched: Value::Null,
            condition: condition.to_string(),
            remedy: fix_advice.to_string(),
        }),
    }
}

/// The kind of a finding a ratchet or a ceiling judged. Spec 11.7.
const METRIC: &str = "metric";

/// The forms a gate could not resolve: a review item each where the change opened them, and a
/// coverage note each where the base held them too. Spec 7.2, 11.7.
fn hole_records(hole: &Hole, out: &mut GateRecords) {
    let Hole::Unresolved { opened, forms, .. } = hole else {
        return;
    };
    for (form, why) in forms {
        let reason = holes::form_reason(why);
        let text = format!("{} — {why}", form.text);
        match opened {
            true => out.reviews.push(Review {
                reason: Slot::Is(reason.to_string()),
                ..located_review(&out.gate, holes::UNMEASURED, form, text)
            }),
            false => out.notes.push(Note {
                coverage: Some(true),
                ..site_note(&out.gate, reason, (&form.file, Some(form.line)), &text)
            }),
        }
    }
}

fn located_review(gate: &str, kind: &'static str, site: &Located, text: String) -> Review {
    Review {
        check: Some(gate.to_string()),
        kind,
        file: site.file.clone(),
        line: Slot::Is(site.line),
        text,
        ..Review::default()
    }
}

fn listed_records(listed: &Listed, out: &mut GateRecords) {
    let gate = out.gate.clone();
    let notes: Vec<Note> = match listed {
        Listed::DeadSymbols(_) => Vec::new(),
        Listed::TestsDeleted { went, caller } => {
            let said = |site: &Located| {
                format!(
                    "the test site {} in {} went in this window",
                    site.text, site.file
                )
            };
            if *caller == Caller::Gate {
                out.reviews.extend(
                    went.iter()
                        .map(|site| located_review(&gate, DELETED_TEST, site, said(site))),
                );
                return;
            }
            went.iter()
                .map(|site| site_note(&gate, DELETED, (&site.file, Some(site.line)), &said(site)))
                .collect()
        }
        Listed::TestFunctionsOrphaned(went) => went
            .iter()
            .map(|site| {
                let text = format!(
                    "the test function {} went with the file {} that held it",
                    site.text, site.file
                );
                site_note(&gate, "note", (&site.file, Some(site.line)), &text)
            })
            .collect(),
        Listed::TestFilesPaired { files, rule } => files
            .iter()
            .map(|site| {
                let text = format!(
                    "the test file {} went with its subject {}, matched by {rule}",
                    site.file, site.text
                );
                site_note(&gate, "note", (&site.file, None), &text)
            })
            .collect(),
        Listed::Held(_) | Listed::NoSurface(_) => noted(listed)
            .map(|text| site_note(&gate, "note", ("", None), &text))
            .into_iter()
            .collect(),
    };
    out.notes.extend(notes);
}

fn ratchet_records(said: &Ratchet, out: &mut GateRecords) {
    match said {
        Ratchet::New {
            condition, failed, ..
        } => failed_findings(Outcome::New, condition, failed, out),
        Ratchet::Worse {
            condition, failed, ..
        } => failed_findings(Outcome::Worsened, condition, failed, out),
        Ratchet::AcceptedUnmatched { entries, caller } => {
            for unmatched in entries {
                unmatched_record(&unmatched.entry, *caller, out);
            }
        }
    }
}

/// An accepted entry that matched nothing: a review item at `klin check`, and a note at the Stop.
/// Spec 7.6.
fn unmatched_record(site: &Entry, caller: Caller, out: &mut GateRecords) {
    let values = Some(site.values.clone());
    match caller {
        Caller::Gate => out.reviews.push(Review {
            check: Some(out.gate.clone()),
            kind: UNMATCHED_ACCEPTED,
            file: site.file.clone(),
            line: site.line.map_or(Slot::Absent, Slot::Is),
            text: site.text.clone(),
            values,
            ..Review::default()
        }),
        Caller::Hook => out.notes.push(Note {
            values,
            ..site_note(&out.gate, UNMATCHED, (&site.file, site.line), &site.text)
        }),
    }
}

/// One failure with what the ratchet held against it, so a harness sees both sides of the
/// comparison. Spec 11.2.
fn failed_findings(outcome: Outcome, condition: &str, failed: &[Failed], out: &mut GateRecords) {
    for finding in failed {
        let matched = match &finding.matched {
            Matched::Nothing => Value::Null,
            Matched::Accepted(entry) => matched_json(entry, true),
            Matched::Base(entry) => matched_json(entry, false),
        };
        out.findings.push(Finding {
            id: finding.id.clone(),
            check: Some(out.gate.clone()),
            kind: METRIC,
            outcome,
            file: finding.file.clone(),
            line: Some(finding.line),
            text: Some(finding.text.clone()),
            values: finding.values.clone(),
            ceiling: finding.ceiling.clone().map_or(Value::Null, Into::into),
            matched,
            condition: condition.to_string(),
            remedy: finding.fix_advice.clone(),
        });
    }
}

fn matched_json(entry: &Entry, accepted: bool) -> Value {
    let mut out = Map::new();
    out.insert("file".into(), entry.file.clone().into());
    out.insert("text".into(), entry.text.clone().into());
    if let Some(line) = entry.line {
        out.insert("line".into(), line.into());
    }
    out.insert("accepted".into(), accepted.into());
    out.insert("values".into(), Value::Object(entry.values.clone()));
    Value::Object(out)
}

fn clip(text: &str) -> String {
    text.chars().take(70).collect()
}

/// The `measurement-lost` row of one run and the line under it per lost file, `None` for a run
/// that lost none. A file an accepted entry holds prints as held. Spec 7.2.
pub fn lost_row(sorted: &[Unmeasured], held: &[String]) -> Option<String> {
    let lost: Vec<&Unmeasured> = sorted
        .iter()
        .filter(|item| item.class == Class::Lost)
        .collect();
    if lost.is_empty() {
        return None;
    }
    let failing = lost.iter().any(|item| !held.contains(&item.file));
    let mut out = format!(
        "  {}  {MEASUREMENT_LOST}\n",
        if failing { "FAIL" } else { "ok  " }
    );
    for item in lost {
        let line = match held.contains(&item.file) {
            true => format!(
                "held: {} — {}, matched the accepted entry for it",
                item.file, item.text
            ),
            false => format!("FAIL: {} {}", lost_condition(item), lost_remedy(item)),
        };
        let _ = writeln!(out, "        {line}");
    }
    Some(out)
}

/// The line of each opened gap and coverage note `klin check` prints, or at the Stop, of each
/// opened gap alone as a note the agent sees. Spec 7.2.
pub fn unmeasured_lines(sorted: &[Unmeasured], at_stop: bool) -> String {
    let mut out = String::new();
    for item in sorted {
        let word = match (item.class, at_stop) {
            (Class::Lost, _) => continue,
            (Class::Opened, false) => "REVIEW",
            (Class::Opened, true) | (Class::Limit, _) => "NOTE",
        };
        let _ = writeln!(out, "  {word}: {}", unmeasured_said(item));
    }
    out
}

fn unmeasured_said(item: &Unmeasured) -> String {
    format!(
        "{} is not measured ({}) — {}",
        item.file,
        item.reason.name(),
        item.text
    )
}

fn lost_condition(item: &Unmeasured) -> String {
    format!(
        "{} was measured at the base and klin cannot measure it now: {}, so nothing in it is judged.",
        item.file, item.text
    )
}

/// What to do about a lost file: for a parse, make it valid in its language from its first
/// error node, and for the other reasons, the reason. Spec 7.2.
fn lost_remedy(item: &Unmeasured) -> String {
    match (item.reason, item.at) {
        (Cause::Parse, Some((line, column))) => format!(
            "Make the file valid {} again from line {line}, column {column}.",
            item.language.unwrap_or("source")
        ),
        (Cause::LineCeiling, _) => {
            "Keep every line under the source-line ceiling of 65536 bytes.".to_string()
        }
        (Cause::Manifest, _) => "Make the manifest parse again.".to_string(),
        _ => "Keep the file a regular text file.".to_string(),
    }
}

/// The finding of one lost file: keyed by the file, with no check, no line and no ratcheted
/// value, held where an accepted entry names the file. Spec 7.2, 11.7.
pub fn lost_finding(item: &Unmeasured, held: bool) -> Finding {
    let mut values = Map::new();
    values.insert("reason".into(), item.reason.name().into());
    if let Some((line, column)) = item.at {
        values.insert("line".into(), line.into());
        values.insert("column".into(), column.into());
    }
    let matched = held.then(|| {
        serde_json::json!({
            "file": item.file, "line": null, "text": "", "accepted": true, "values": {},
        })
    });
    Finding {
        id: holes::lost_id(&item.file),
        check: None,
        kind: MEASUREMENT_LOST,
        outcome: if held { Outcome::Held } else { Outcome::New },
        file: item.file.clone(),
        line: None,
        text: Some(item.file.clone()),
        values,
        ceiling: serde_json::json!({}),
        matched: matched.unwrap_or(Value::Null),
        condition: lost_condition(item),
        remedy: lost_remedy(item),
    }
}

/// The review item of one opened gap. Spec 7.2, 11.7.
pub fn opened_review(item: &Unmeasured) -> Review {
    Review {
        kind: holes::UNMEASURED,
        file: item.file.clone(),
        line: Slot::Null,
        text: item.text.clone(),
        reason: Slot::Is(item.reason.name().to_string()),
        ..Review::default()
    }
}

/// The coverage note of one file klin's own limit leaves unmeasured. Spec 7.2, 11.7.
pub fn limit_note(item: &Unmeasured) -> Note {
    Note {
        kind: item.reason.name(),
        message: unmeasured_said(item),
        coverage: Some(true),
        file: Slot::Is(item.file.clone()),
        ..Note::default()
    }
}

/// The review item of an accepted entry of gate `measurement-lost` whose file klin measures
/// again. Spec 7.2, 7.6, 11.7.
pub fn unmatched_lost_review(file: &str) -> Review {
    Review {
        check: Some(MEASUREMENT_LOST.to_string()),
        kind: UNMATCHED_ACCEPTED,
        file: file.to_string(),
        line: Slot::Null,
        text: unmatched_lost_text(file),
        reason: Slot::Null,
        ..Review::default()
    }
}

/// How `klin policy` tells a person to hold a file klin's grammar does not read yet. The Stop
/// points a person here and never tells the agent. Spec 7.2.
pub fn lost_policy() -> String {
    format!(
        "{MEASUREMENT_LOST} — built-in\n{UNDER}a file the base measured that klin cannot \
         measure now fails here, for every capability that reads it.\n{UNDER}where klin's \
         grammar does not read a valid construct yet, a person holds the file in a reviewed \
         commit with the accepted entry {{\"gate\": \"{MEASUREMENT_LOST}\", \"file\": PATH}}, \
         which stays matched while klin cannot measure the file.\n"
    )
}

/// What the Stop tells the person, never the agent, when a file it did not block on is lost
/// to a grammar that may lag. Spec 7.2.
pub const LOST_TO_A_PERSON: &str = "klin: a file klin's grammar cannot read is a \
    measurement-lost failure. If the file is valid and klin's grammar lags, a person can hold it \
    in the accepted list — `klin policy` shows how.";

const UNDER: &str = "      ";

/// What an accepted entry of gate `measurement-lost` that matches nothing says.
pub fn unmatched_lost_text(file: &str) -> String {
    format!(
        "the accepted entry of {MEASUREMENT_LOST} for {file} matches nothing: klin measures the file now"
    )
}
