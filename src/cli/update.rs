//! `klin update` installs the newest release over the binary the installer placed, after its
//! signature checks out. Spec 11.8.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Command;

use crate::cli::setup;
use crate::sys::release;

#[derive(Clone, Copy, clap::ValueEnum)]
enum Switch {
    On,
    Off,
}

#[derive(clap::Args)]
pub struct Args {
    /// Install each newer release on its own at a session start, or stop doing so
    #[arg(long, value_enum, conflicts_with = "check")]
    auto: Option<Switch>,
    /// Ask GitHub for the newest release and name it, without installing anything
    #[arg(long)]
    check: bool,
}

pub fn run(args: &Args) -> u8 {
    if args.check {
        return check();
    }
    if let Some(switch) = args.auto {
        return auto(switch);
    }
    install()
}

/// Install the newest release over the binary the installer placed. Spec 11.8.
fn install() -> u8 {
    match release::update() {
        Ok(Some(tag)) => {
            println!("klin updated to {tag}. Release notes: {}", tag.notes());
            reconcile_hint();
            0
        }
        Ok(None) => {
            println!("klin {} is the newest release.", env!("CARGO_PKG_VERSION"));
            0
        }
        Err(why) => {
            eprintln!("FAIL: {why}");
            2
        }
    }
}

/// Write the person's auto-update setting. Spec 11.8, 11.9.
fn auto(switch: Switch) -> u8 {
    let on = matches!(switch, Switch::On);
    match release::set_auto(on) {
        Ok(()) if on => {
            println!(
                "klin: auto-update is on. A session start installs a newer signed release on \
                 its own."
            );
            0
        }
        Ok(()) => {
            println!("klin: auto-update is off. klin tells you when a newer release is out.");
            0
        }
        Err(why) => {
            eprintln!("FAIL: {why}");
            2
        }
    }
}

/// The installed version and the newest release, which GitHub is asked for now. Spec 11.8.
fn check() -> u8 {
    let version = env!("CARGO_PKG_VERSION");
    let Some((tag, saved)) = release::fetch() else {
        eprintln!("FAIL: klin {version} could not ask GitHub for the newest release.");
        return 2;
    };
    if !saved {
        eprintln!(
            "klin: NOTE: klin could not save this answer in its cache, so klin status still \
             shows the earlier one."
        );
    }
    match tag.is_newer() {
        true => println!(
            "{}",
            release::Audience::Person.note(
                release::route().unwrap_or(release::Route::Installer),
                &release::News::Available(tag.clone()),
                None
            )
        ),
        false => println!("klin {version} is the newest release."),
    }
    println!("Release notes: {}", tag.notes());
    0
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
