//! `klin update` runs the `klin-update` program the installer puts beside this binary, which
//! installs the newest release over it. Spec 19.5.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Command;

use crate::cli::setup;

const INSTALL: &str = "curl --proto '=https' --tlsv1.2 -LsSf \
                       https://github.com/brajevicm/klin/releases/latest/download/klin-installer.sh | sh";

pub fn run() -> u8 {
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("klin-update")))
        .filter(|path| path.is_file());
    let program = beside.unwrap_or_else(|| PathBuf::from("klin-update"));
    match Command::new(&program).status() {
        Ok(status) if status.success() => {
            reconcile_hint();
            0
        }
        Ok(status) => u8::try_from(status.code().unwrap_or(2)).unwrap_or(2),
        Err(_) => {
            eprintln!(
                "FAIL: klin-update was not found beside klin or on PATH. The installer puts it \
                 there: {INSTALL}. A plugin install updates with its plugin instead."
            );
            2
        }
    }
}

/// After an update, the new binary reads this repository's integrations, and any that is not
/// `current` is named with the command that reconciles it. Spec 11.8.
fn reconcile_hint() {
    let Some(read) = std::env::current_exe()
        .ok()
        .map(replaced)
        .and_then(|exe| Command::new(exe).args(["status", "--json"]).output().ok())
        .filter(|read| read.status.success())
    else {
        return;
    };
    let Ok(document) = serde_json::from_slice::<serde_json::Value>(&read.stdout) else {
        return;
    };
    let stale: Vec<&serde_json::Value> = document["integrations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| row["state"] != setup::State::Current.as_str())
        .collect();
    let (user, project): (Vec<_>, Vec<_>) = stale
        .into_iter()
        .partition(|row| row["scope"] == setup::Owner::User.as_str());
    if !project.is_empty() {
        println!(
            "klin: run klin setup to reconcile this repository's integration: {}.",
            described(&project)
        );
    }
    if !user.is_empty() {
        let hosts: String = user
            .iter()
            .map(|row| format!(" --host {}", row["host"].as_str().unwrap_or("")))
            .collect();
        println!(
            "klin: run klin setup --user{hosts} to reconcile your own integration: {}.",
            described(&user)
        );
    }
}

fn described(rows: &[&serde_json::Value]) -> String {
    rows.iter()
        .map(|row| {
            format!(
                "{} ({}) is {}",
                row["host"].as_str().unwrap_or(""),
                row["scope"].as_str().unwrap_or(""),
                row["state"].as_str().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The path the updated binary sits at. Linux names a running binary that was replaced on disk
/// with a ` (deleted)` suffix, and the new binary is at the path without it.
fn replaced(exe: PathBuf) -> PathBuf {
    match exe.as_os_str().as_bytes().strip_suffix(b" (deleted)") {
        Some(path) => PathBuf::from(OsStr::from_bytes(path)),
        None => exe,
    }
}
