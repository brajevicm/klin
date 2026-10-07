//! The renderers over a gate's typed result. The checks and the ratchet return `Told` items, and
//! this writes them in their order: as the text `klin check` prints, as the text the Stop hook
//! prints, as the `derived:` and `pinned:` lines above a gate's row, and as the findings, notes
//! and derived entries of the JSON. Spec 3.1, 6.1, 11.1, 11.2.

use std::fmt::Write;

use serde_json::{Map, Value};

use crate::ceiling::Ceiling;
use crate::check::contract::{
    self, Complexity, Counted, DELETED, Derived, Entry, Failed, Held, HeldAtBase, Hole, Incomplete,
    Judged, LOST, Layering, Line, Listed, Located, Matched, Measured, NOT_MEASURED, Plain,
    Provenance, PublicApi, Ratchet, Site, Standing, Told, UNPARSED, UNRESOLVED, Unmatched,
    Unresolvable, Wording,
};
use crate::coverage::Coverage;

/// How many rows a listed block prints before it says how many more there are.
const SHOWN: usize = 20;

const LOST_REMEDY: &str = "Drop the exclusion or restore the rule that reached it, or exclude it \
                           on purpose and accept that nothing measures it.";

const NOT_MEASURED_REMEDY: &str = "Add a structural adapter for the language, or exclude the file \
                                   and accept that nothing measures it.";

const UNPARSED_REMEDY: &str = "A file klin cannot read is a hole in the ratchet. Update the \
                               grammar, or exclude the file and accept that nothing measures it.";

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

fn holed(hole: &Hole, out: &mut String) {
    match hole {
        Hole::Lost(site) => {
            let _ = writeln!(
                out,
                "NOTE: {} was measured at the base and is not measured now — {}",
                site.file, site.text
            );
        }
        Hole::LeftScrutiny(count) => {
            let _ = writeln!(
                out,
                "FAIL: {count} file(s) left scrutiny — a file klin measured at the base and \
                 does not measure now, though it is still in the tree, is a failure. {LOST_REMEDY}"
            );
        }
        Hole::NotMeasured { fail, files } => block(
            out,
            &format!(
                "{}: {} file(s) in unsupported structural languages were not measured:",
                word(*fail),
                files.len()
            ),
            files
                .iter()
                .map(|file| format!("{}  {}", file.file, file.text)),
            NOT_MEASURED_REMEDY,
        ),
        Hole::Unresolved { fail, kind, forms } => block(
            out,
            &format!("{}: {} {}:", word(*fail), forms.len(), unresolved(*kind).0),
            forms
                .iter()
                .map(|(form, why)| format!("{}  {}  — {why}", at(form), form.text)),
            unresolved(*kind).1,
        ),
        Hole::Unparsed { fail, files } => block(
            out,
            &format!(
                "{}: {} file(s) the grammar could not parse, so nothing in them was measured:",
                word(*fail),
                files.len()
            ),
            files
                .iter()
                .map(|file| format!("{}  {}", file.file, file.text)),
            UNPARSED_REMEDY,
        ),
    }
}

/// What a block of unresolved forms of one kind says they are, and how to close them.
fn unresolved(kind: Unresolvable) -> (&'static str, &'static str) {
    match kind {
        Unresolvable::Dependency => (
            "dependency form(s) klin resolves could not be resolved, so what they reach was not judged",
            "Make each one name exactly one module file the tree holds, or take its file out of the section's scope.",
        ),
        Unresolvable::PublicSurface => (
            "form(s) inside a supported public surface could not be resolved, so the surface is not completely measured",
            "Write the export or re-export in a form klin lists, or make each path name exactly one module file the tree holds.",
        ),
    }
}

/// A heading, its rows indented under it, and the line that closes the block.
fn block(out: &mut String, heading: &str, rows: impl Iterator<Item = String>, close: &str) {
    let _ = writeln!(out, "{heading}");
    for row in rows {
        let _ = writeln!(out, "  {row}");
    }
    let _ = writeln!(out, "{close}");
}

fn sites(listed: &Listed, out: &mut String) {
    let (heading, rows): (String, Vec<String>) = match listed {
        Listed::DeadSymbols(dead) => (
            format!("REPORT: {} dead symbol(s):", dead.len()),
            dead.iter()
                .map(|(site, name)| format!("{}  {name}  {}", at(site), site.text))
                .collect(),
        ),
        Listed::TestsDeleted(went) => (
            format!(
                "NOTE: {} test site(s) the base holds went in this window:",
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
        Ratchet::AcceptedUnmatched(unmatched) => listed(
            out,
            &format!(
                "NOTE: {} accepted entr{} matched nothing this run:",
                unmatched.len(),
                entries(unmatched.len())
            ),
            &unmatched.iter().map(unmatched_row).collect::<Vec<_>>(),
        ),
        Ratchet::AcceptedStale { count, rows } => listed(
            out,
            &format!(
                "FAIL: the accepted list holds {count} entr{} that matched nothing — an entry that no longer describes the code is a failure. Delete the line.",
                entries(*count)
            ),
            &rows
                .iter()
                .map(|row| format!("{}: {}", row.file, row.text))
                .collect::<Vec<_>>(),
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

fn word(fail: bool) -> &'static str {
    if fail { "FAIL" } else { "NOTE" }
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

/// What the JSON of spec 11.2 carries of one gate's result.
#[derive(Default)]
pub struct Json {
    pub findings: Vec<Value>,
    pub notes: Vec<Value>,
    pub derived: Vec<Value>,
    /// The `{reason, detail, text}` of each hole the gate told. Spec 7.2, 11.7.
    pub holes: Vec<Value>,
}

/// The findings, notes and derived entries of one gate's result, in the order the result says
/// them. Spec 11.2.
pub fn json(told: &[Told]) -> Json {
    let mut out = Json::default();
    for item in told {
        json_one(item, &mut out);
    }
    out
}

fn json_one(told: &Told, out: &mut Json) {
    match told {
        Told::Document {
            name,
            words,
            ceiling,
            standing,
        } => document_json((name, *words, ceiling.value), standing, out),
        Told::Provenance(said) => out.derived.extend(derived_json(said)),
        Told::Plain(said) => out.notes.extend(note_json(said)),
        other => found_json(other, out),
    }
}

/// What a gate found beyond its own documents, policy and plain lines.
fn found_json(told: &Told, out: &mut Json) {
    match told {
        Told::Hole(hole) => hole_json(hole, out),
        Told::Listed(listed) => listed_json(listed, out),
        Told::Ratchet(said) => ratchet_json(said, out),
        Told::Incomplete(hole) => out.holes.push(serde_json::json!({
            "reason": hole.reason.name(),
            "detail": hole.detail,
            "text": hole.text,
        })),
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

/// The record of a NOTE, and nothing for a line the JSON records elsewhere or not at all.
fn note_json(said: &Plain) -> Option<Value> {
    let Plain::Note(note) = said else {
        return None;
    };
    Some(record(note.outcome, Some(&note.file), None, &note.text))
}

fn record(outcome: &str, file: Option<&str>, line: Option<u64>, text: &str) -> Value {
    Value::Object(fields(outcome, file, line, text))
}

/// One record of a site: what it is, the file and line it names where it names them, its text.
fn fields(outcome: &str, file: Option<&str>, line: Option<u64>, text: &str) -> Map<String, Value> {
    let mut out = Map::new();
    out.insert("outcome".into(), outcome.into());
    if let Some(file) = file {
        out.insert("file".into(), file.into());
    }
    if let Some(line) = line {
        out.insert("line".into(), line.into());
    }
    out.insert("text".into(), text.into());
    out
}

fn document_json((name, words, ceiling): (&str, u64, u64), standing: &Standing, out: &mut Json) {
    let site = |outcome: &str| {
        let mut site = Map::new();
        site.insert("outcome".into(), outcome.into());
        site.insert("file".into(), name.into());
        site.insert(
            "values".into(),
            serde_json::json!({ "words": words, "ceiling": ceiling }),
        );
        site
    };
    match standing {
        Standing::Under | Standing::Held(_) => (),
        Standing::Near(_) => out.notes.push(Value::Object(site("near-ceiling"))),
        Standing::Over {
            id,
            condition,
            fix_advice,
        } => {
            let mut over = site("new");
            over.insert("id".into(), id.clone().into());
            over.insert("line".into(), Value::Null);
            over.insert("text".into(), Value::Null);
            over.insert("ceiling".into(), serde_json::json!({ "words": ceiling }));
            over.insert("matched".into(), Value::Null);
            over.insert("condition".into(), (*condition).into());
            over.insert("fix_advice".into(), (*fix_advice).into());
            out.findings.push(Value::Object(over));
        }
    }
}

fn hole_json(hole: &Hole, out: &mut Json) {
    match hole {
        Hole::Lost(site) => out.notes.push(site_json(LOST, site)),
        Hole::LeftScrutiny(_) => (),
        Hole::NotMeasured { fail, files } => sited(out, *fail).extend(files.iter().map(|file| {
            record(
                NOT_MEASURED,
                Some(&file.file),
                None,
                &format!("{} has no structural adapter", file.text),
            )
        })),
        Hole::Unresolved { fail, forms, .. } => {
            sited(out, *fail).extend(forms.iter().map(|(form, why)| {
                record(
                    UNRESOLVED,
                    Some(&form.file),
                    Some(form.line),
                    &format!("{} — {why}", form.text),
                )
            }));
        }
        Hole::Unparsed { fail, files } => {
            sited(out, *fail).extend(files.iter().map(|file| site_json(UNPARSED, file)));
        }
    }
}

/// Where a hole's records go: its findings where it fails, and its notes where it does not.
fn sited(out: &mut Json, fail: bool) -> &mut Vec<Value> {
    match fail {
        true => &mut out.findings,
        false => &mut out.notes,
    }
}

fn site_json(outcome: &str, site: &Site) -> Value {
    record(outcome, Some(&site.file), None, &site.text)
}

fn listed_json(listed: &Listed, out: &mut Json) {
    let notes: Vec<Value> = match listed {
        Listed::DeadSymbols(_) => Vec::new(),
        Listed::TestsDeleted(went) => went
            .iter()
            .map(|site| {
                record(
                    DELETED,
                    Some(&site.file),
                    Some(site.line),
                    &format!(
                        "the test site {} in {} went in this window",
                        site.text, site.file
                    ),
                )
            })
            .collect(),
        Listed::TestFunctionsOrphaned(went) => went
            .iter()
            .map(|site| {
                record(
                    "note",
                    Some(&site.file),
                    Some(site.line),
                    &format!(
                        "the test function {} went with the file {} that held it",
                        site.text, site.file
                    ),
                )
            })
            .collect(),
        Listed::TestFilesPaired { files, rule } => files
            .iter()
            .map(|site| {
                record(
                    "note",
                    Some(&site.file),
                    None,
                    &format!(
                        "the test file {} went with its subject {}, matched by {rule}",
                        site.file, site.text
                    ),
                )
            })
            .collect(),
        Listed::Held(_) | Listed::NoSurface(_) => noted(listed)
            .map(|text| record("note", Some(""), None, &text))
            .into_iter()
            .collect(),
    };
    out.notes.extend(notes);
}

fn ratchet_json(said: &Ratchet, out: &mut Json) {
    match said {
        Ratchet::New {
            condition, failed, ..
        } => out.findings.extend(
            failed
                .iter()
                .map(|finding| failed_json("new", condition, finding)),
        ),
        Ratchet::Worse {
            condition, failed, ..
        } => out.findings.extend(
            failed
                .iter()
                .map(|finding| failed_json("worsened", condition, finding)),
        ),
        Ratchet::AcceptedUnmatched(unmatched) => {
            out.notes.extend(unmatched.iter().map(|entry| {
                let site = &entry.entry;
                let mut record = fields("unmatched", Some(&site.file), site.line, &site.text);
                record.insert("values".into(), Value::Object(entry.entry.values.clone()));
                Value::Object(record)
            }));
        }
        Ratchet::AcceptedStale { .. } => (),
    }
}

/// One failure with what the ratchet held against it, so a harness sees both sides of the
/// comparison. Spec 11.2.
fn failed_json(outcome: &str, condition: &str, finding: &Failed) -> Value {
    let mut out = fields(
        outcome,
        Some(&finding.file),
        Some(finding.line),
        &finding.text,
    );
    out.insert("id".into(), finding.id.clone().into());
    out.insert("values".into(), Value::Object(finding.values.clone()));
    out.insert("condition".into(), condition.into());
    out.insert("fix_advice".into(), finding.fix_advice.clone().into());
    out.insert(
        "ceiling".into(),
        finding.ceiling.clone().map_or(Value::Null, Into::into),
    );
    let matched = match &finding.matched {
        Matched::Nothing => Value::Null,
        Matched::Accepted(entry) => matched_json(entry, true),
        Matched::Base(entry) => matched_json(entry, false),
    };
    out.insert("matched".into(), matched);
    Value::Object(out)
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
