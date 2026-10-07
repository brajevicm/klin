//! The text renderer over a gate's typed result: every word, status and indent a person reads of
//! what a check found. The checks and the ratchet return `Told` items, and this writes them in
//! their order. The JSON renders the `Records` beside them. Spec 3.1, 6.1, 11.1.

use std::fmt::Write;

use crate::check::contract::{
    Failed, Hole, Line, Listed, Matched, Plain, Ratchet, Site, Standing, Told, Unmatched,
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
        Hole::Unresolved {
            fail,
            what,
            remedy,
            forms,
        } => block(
            out,
            &format!("{}: {} {what}:", word(*fail), forms.len()),
            forms
                .iter()
                .map(|(form, why)| format!("{}  {}  — {why}", at(form), form.text)),
            remedy,
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
        let (was, from) = finding.was.clone().unwrap_or_default();
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
fn judged(line: &Line, (accepted, base): (usize, usize), out: &mut String) {
    let qualifier = match (accepted, base) {
        (0, 0) => String::new(),
        (0, _) => ", all held at the base".to_string(),
        (_, 0) => ", all on the accepted list".to_string(),
        (accepted, base) => {
            format!(", {base} held at the base and {accepted} on the accepted list")
        }
    };
    let coverage = line.coverage.as_ref().map(covered).unwrap_or_default();
    let _ = writeln!(
        out,
        "OK: {}{qualifier}{}{coverage}{}",
        line.state, line.aside, line.after
    );
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
