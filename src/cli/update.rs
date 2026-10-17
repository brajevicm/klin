//! `klin update` installs the newest release over this binary, after its signature checks out.
//! Spec 11.8.

use std::process::Command;

use crate::cli::setup;
use crate::sys::out;
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

/// Install the newest release over this binary. Spec 11.8.
fn install() -> u8 {
    match release::update() {
        Ok(Some(tag)) => {
            out::println(&format!(
                "klin updated to {tag}. Release notes: {}",
                tag.notes()
            ));
            shadowed_hint();
            reconcile_hint();
            0
        }
        Ok(None) => {
            out::println(&format!(
                "klin {} is the newest release.",
                env!("CARGO_PKG_VERSION")
            ));
            shadowed_hint();
            0
        }
        Err(why) => {
            out::eprintln(&format!("FAIL: {why}"));
            2
        }
    }
}

/// Another klin that PATH finds first, which a shell and klin's hooks run instead of this one,
/// whether or not this one changed. Spec 11.8.
fn shadowed_hint() {
    if let Some(other) = release::shadowed() {
        let other = other.display();
        out::println(&format!(
            "klin: NOTE: PATH finds another klin first, at {other}, and a shell and klin's hooks \
             run that one. Run {other} update, or remove it."
        ));
    }
}

/// Write the person's auto-update setting. Spec 11.8, 11.9.
fn auto(switch: Switch) -> u8 {
    let on = matches!(switch, Switch::On);
    match release::set_auto(on) {
        Ok(()) if on => {
            out::println(
                "klin: auto-update is on. A session start installs a newer signed release on \
                 its own.",
            );
            0
        }
        Ok(()) => {
            out::println("klin: auto-update is off. klin tells you when a newer release is out.");
            0
        }
        Err(why) => {
            out::eprintln(&format!("FAIL: {why}"));
            2
        }
    }
}

/// The installed version and the newest release, which GitHub is asked for now. Spec 11.8.
fn check() -> u8 {
    let version = env!("CARGO_PKG_VERSION");
    let Some((tag, saved)) = release::fetch() else {
        out::eprintln(&format!(
            "FAIL: klin {version} could not ask GitHub for the newest release."
        ));
        return 2;
    };
    if !saved {
        out::eprintln(
            "klin: NOTE: klin could not save this answer in its cache, so klin status still \
             shows the earlier one.",
        );
    }
    match tag.is_newer() {
        true => out::println(
            &release::Audience::Person
                .note(
                    release::route(),
                    &release::News::Available(tag.clone()),
                    None,
                )
                .to_string(),
        ),
        false => out::println(&format!("klin {version} is the newest release.")),
    }
    out::println(&format!("Release notes: {}", tag.notes()));
    0
}

/// After an update, the new binary reads this repository's integrations, and any that is not
/// `current` is named with the command that reconciles it. Spec 11.8.
fn reconcile_hint() {
    let Some(read) = release::installed()
        .ok()
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
        out::println(&format!(
            "klin: run klin setup to reconcile this repository's integration: {}.",
            described(&project)
        ));
    }
    if !user.is_empty() {
        let hosts: String = user
            .iter()
            .map(|row| format!(" --host {}", row["host"].as_str().unwrap_or("")))
            .collect();
        out::println(&format!(
            "klin: run klin setup --user{hosts} to reconcile your own integration: {}.",
            described(&user)
        ));
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
