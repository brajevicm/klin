//! The text renderer over a gate's typed result: every word, status and indent a person reads of
//! what a check found. The checks and the ratchet return `Told` items, and this writes them in
//! their order. The JSON renders the `Records` beside them. Spec 3.1, 6.1, 11.1.

use std::fmt::Write;

use crate::ceiling::Ceiling;
use crate::check::contract::{
    Complexity, Counted, Failed, Held, Hole, Judged, Layering, Line, Listed, Matched, Measured,
    Plain, PublicApi, Ratchet, Site, Standing, Told, Unmatched, Unresolvable,
};
use crate::coverage::Coverage;

const LOST_REMEDY: &str = "Drop the exclusion or restore the rule that reached it, or exclude it \
                           on purpose and accept that nothing measures it.";

const NOT_MEASURED_REMEDY: &str = "Add a structural adapter for the language, or exclude the file \
                                   and accept that nothing measures it.";

const UNPARSED_REMEDY: &str = "A file klin cannot read is a hole in the ratchet. Update the \
                               grammar, or exclude the file and accept that nothing measures it.";

/// The report text of one gate's result.
pub fn text(told: &[Told]) -> String {
    let mut out = String::new();
    for item in told {
        one(item, &mut out);
    }
    out
}

fn one(told: &Told, out: &mut String) {
    match told {
        Told::Judged { line, held } => judged(line, *held, out),
        Told::Document {
            name,
            words,
            ceiling,
            standing,
        } => document(name, *words, ceiling, standing, out),
        Told::Plain(said) => plain(said, out),
        Told::Hole(hole) => holed(hole, out),
        Told::Listed(listed) => sites(listed, out),
        Told::Ratchet(said) => ratchet(said, out),
    }
}

fn plain(said: &Plain, out: &mut String) {
    let _ = match said {
        Plain::Note(text) => writeln!(out, "NOTE: {text}"),
        Plain::Remedy(text) => writeln!(out, "{text}"),
        Plain::PathMissing(named) => writeln!(
            out,
            "FAIL: {named} — correct the path, or take it out of \"in\"."
        ),
        Plain::Error(problem) => writeln!(out, "FAIL: {problem}"),
    };
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
            for finding in failed {
                let indent = if finding.nested { "  " } else { "" };
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
        Ratchet::Worse { unit, failed } => worse(unit, failed, out),
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

fn at(site: &Site) -> String {
    format!("{}:{}", site.file, site.line.unwrap_or_default())
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

fn document(name: &str, words: u64, ceiling: &str, standing: &Standing, out: &mut String) {
    match standing {
        Standing::Under => {
            let _ = writeln!(out, "OK: {name} is {words} words, ceiling {ceiling}");
        }
        Standing::Near(remaining) => {
            let _ = writeln!(out, "OK: {name} is {words} words, ceiling {ceiling}");
            let _ = writeln!(
                out,
                "WARN: {name} is {words} words, {remaining} from its ceiling of {ceiling}."
            );
        }
        Standing::Held(before) => {
            let _ = writeln!(
                out,
                "OK: {name} is {words} words, over its ceiling of {ceiling}, held at the base \
                 at {before} words"
            );
        }
        Standing::Over(remedy) => {
            let _ = writeln!(
                out,
                "FAIL: {name} is {words} words, over its ceiling of {ceiling}."
            );
            let _ = writeln!(out, "{remedy}");
        }
    }
}

/// What one failure was judged against, and the ceiling in force beside it. Spec 8.6.
fn against(finding: &Failed) -> String {
    let matched = match &finding.matched {
        Matched::Nothing => "nothing matched".to_string(),
        Matched::Accepted(file) => format!("matched the accepted entry for {file}"),
        Matched::Base(file, line) => format!("matched the base site at {file}:{line}"),
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
        entry.file,
        entry.shown,
        clip(&entry.text)
    )
}

fn listed(out: &mut String, heading: &str, rows: &[String]) {
    let _ = writeln!(out, "{heading}");
    for row in rows.iter().take(20) {
        let _ = writeln!(out, "  {row}");
    }
    if rows.len() > 20 {
        let _ = writeln!(out, "  … and {} more", rows.len() - 20);
    }
}

fn clip(text: &str) -> String {
    text.chars().take(70).collect()
}
