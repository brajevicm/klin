use std::fmt::Write;
use std::path::Path;

use serde_json::Value;

use crate::cli::check;
use crate::engine::catalogue;

/// What `setup` ends with: the debt today's tree holds, which the Stop holds as its floor, and
/// each check that judges nothing in this tree. Spec 11.2.
pub fn said(root: &Path, out: &mut String) {
    let document = check::document(root);
    if let Some(error) = document["errors"][0]["message"].as_str() {
        let _ = writeln!(out, "klin holds no floor yet: {error}");
        return;
    }
    let capabilities = document["capabilities"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let _ = writeln!(
        out,
        "klin now holds today's debt as the floor: {}.",
        held(&document, &capabilities)
    );
    let _ = writeln!(
        out,
        "Only new problems block. Blind spots here: {}.",
        blind(&capabilities)
    );
}

/// Each check's findings in the tree, held at the base or new against it, in a person's words.
fn held(document: &Value, capabilities: &[Value]) -> String {
    let mut counted: Vec<(String, u64)> = Vec::new();
    for capability in capabilities {
        let name = capability["name"].as_str().unwrap_or_default();
        let count = capability["held"].as_u64().unwrap_or_default() + new(document, name);
        if count == 0 {
            continue;
        }
        let label = labelled(name, count);
        match counted.iter_mut().find(|(said, _)| *said == label) {
            Some((_, total)) => *total += count,
            None => counted.push((label, count)),
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

fn new(document: &Value, name: &str) -> u64 {
    let findings = document["findings"].as_array().map(Vec::as_slice);
    findings
        .unwrap_or_default()
        .iter()
        .filter(|finding| finding["check"] == name && finding["outcome"] == "new")
        .count() as u64
}

/// What a report calls one gate's findings, and the gate's name for one this binary holds no
/// words for, such as a `sarif` entry.
fn labelled(name: &str, count: u64) -> String {
    let check = name.split('/').next().unwrap_or(name);
    match catalogue::labels(check) {
        Some(labels) => labels.count(count as usize).to_string(),
        None => format!("{name} findings"),
    }
}

/// Each check that judges nothing here: one that runs and judged none of the files it found,
/// and one that does not run.
fn blind(capabilities: &[Value]) -> String {
    let said: Vec<String> = capabilities
        .iter()
        .filter_map(|capability| {
            let name = capability["name"].as_str()?;
            match &capability["coverage"] {
                Value::Null => Some(format!("{name} (off)")),
                coverage if coverage["measured"] == 0 => {
                    Some(format!("{name} (0 of {} files judged)", coverage["found"]))
                }
                _ => None,
            }
        })
        .collect();
    match said.is_empty() {
        true => "none, every check judges this tree".to_string(),
        false => said.join(", "),
    }
}
