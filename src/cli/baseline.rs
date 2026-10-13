use std::fmt::Write;
use std::path::Path;

use crate::cli::check;
use crate::engine::catalogue;
use crate::engine::document::{Floor, Standing};

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
    let held = counted(&floor, |row| row.held);
    let failing = counted(&floor, |row| row.failing);
    let floor_said = match (&held, &failing) {
        (Some(held), _) => held.as_str(),
        (None, Some(_)) => "nothing yet",
        (None, None) => "nothing, no check found a problem",
    };
    let _ = writeln!(
        out,
        "klin now holds today's debt as the floor: {floor_said}."
    );
    if let Some(failing) = failing {
        let _ = writeln!(
            out,
            "Not in the floor, so these still fail klin check: {failing}."
        );
    }
    let _ = writeln!(
        out,
        "Only new problems block. Blind spots here: {}.",
        blind(&floor)
    );
}

/// One count of each check's findings in a person's words, summed over the gates of one check,
/// such as every `sarif` entry, and `None` where every count is 0.
fn counted(floor: &[Floor], count: fn(&Floor) -> usize) -> Option<String> {
    let mut totals: Vec<(&str, usize)> = Vec::new();
    for row in floor.iter().filter(|row| count(row) > 0) {
        match totals.iter_mut().find(|(check, _)| *check == row.check) {
            Some((_, total)) => *total += count(row),
            None => totals.push((row.check, count(row))),
        }
    }
    let said: Vec<String> = totals
        .iter()
        .map(|(check, total)| format!("{total} {}", labelled(check, *total)))
        .collect();
    (!said.is_empty()).then(|| said.join(", "))
}

/// What a report calls one check's findings, and its name with "finding" for one this binary
/// holds no words for, such as the built-in row of the lost files.
fn labelled(check: &str, count: usize) -> String {
    match (catalogue::labels(check), count) {
        (Some(labels), _) => labels.count(count).to_string(),
        (None, 1) => format!("{check} finding"),
        (None, _) => format!("{check} findings"),
    }
}

/// Each check that judges nothing here: one that runs and judged none of the files it found,
/// one that does not run, and an integration setup did not run.
fn blind(floor: &[Floor]) -> String {
    let said: Vec<String> = floor
        .iter()
        .filter_map(|row| {
            let why = match (row.standing, row.coverage) {
                (Standing::Off, _) => "off".to_string(),
                (Standing::Skipped, _) => "not run by setup".to_string(),
                (Standing::Measured, Some((0, found))) => format!("0 of {found} files judged"),
                (Standing::Measured, _) => return None,
            };
            Some(format!("{} ({why})", row.gate))
        })
        .collect();
    match said.is_empty() {
        true => "none, every check judges this tree".to_string(),
        false => said.join(", "),
    }
}
