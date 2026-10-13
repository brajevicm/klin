use std::fmt::Write;
use std::path::Path;

use crate::cli::check;
use crate::engine::catalogue;
use crate::engine::document::Floor;

/// What `setup` ends with: the debt today's tree holds, which the Stop holds as its floor, and
/// each check that judges nothing in this tree. Spec 11.2.
pub fn said(root: &Path, out: &mut String) {
    let floor = match check::document(root).floor() {
        Ok(floor) => floor,
        Err(error) => {
            let _ = writeln!(out, "klin holds no floor yet: {error}");
            return;
        }
    };
    let _ = writeln!(
        out,
        "klin now holds today's debt as the floor: {}.",
        held(&floor)
    );
    let _ = writeln!(
        out,
        "Only new problems block. Blind spots here: {}.",
        blind(&floor)
    );
}

/// Each check's findings in the tree, held at the base or new against it, in a person's words,
/// with the checks that share words counted together.
fn held(floor: &[Floor]) -> String {
    let mut counted: Vec<(String, usize)> = Vec::new();
    for row in floor.iter().filter(|row| row.findings > 0) {
        let label = labelled(&row.gate, row.findings);
        match counted.iter_mut().find(|(said, _)| *said == label) {
            Some((_, total)) => *total += row.findings,
            None => counted.push((label, row.findings)),
        }
    }
    match counted.is_empty() {
        true => "nothing, no check found a problem".to_string(),
        false => counted
            .iter()
            .map(|(label, count)| format!("{count} {label}"))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// What a report calls one gate's findings, and the gate's name for one this binary holds no
/// words for, such as a `sarif` entry.
fn labelled(gate: &str, count: usize) -> String {
    let check = gate.split('/').next().unwrap_or(gate);
    match catalogue::labels(check) {
        Some(labels) => labels.count(count).to_string(),
        None => format!("{gate} findings"),
    }
}

/// Each check that judges nothing here: one that runs and judged none of the files it found,
/// and one that does not run.
fn blind(floor: &[Floor]) -> String {
    let said: Vec<String> = floor
        .iter()
        .filter_map(|row| match (row.runs, row.coverage) {
            (false, _) => Some(format!("{} (off)", row.gate)),
            (true, Some((0, found))) => Some(format!("{} (0 of {found} files judged)", row.gate)),
            (true, _) => None,
        })
        .collect();
    match said.is_empty() {
        true => "none, every check judges this tree".to_string(),
        false => said.join(", "),
    }
}
