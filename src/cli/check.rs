use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::config::file::{Config, FILENAME, ignored, located, notes};
use crate::contract::check::{Activation, CONFIG_LOOSENED};
use crate::contract::project::Project;
use crate::engine::against::against;
use crate::engine::document::{Args, CheckDocument, View};
use crate::engine::plan::{Gate, Plan};
use crate::engine::render::{Finding, Note, Review, Slot};
use crate::engine::{catalogue, diagnostics};
use crate::sys::error::{Error, ErrorKind, Fault, fault};
use crate::window::base::{self, Window};

#[derive(clap::Args)]
pub struct Check {
    /// Run only these checks, by the name a gate takes
    #[arg(value_name = "CHECK")]
    checks: Vec<String>,
    /// Judge only the files changed against the base
    #[arg(long)]
    changed: bool,
    /// Print one JSON object for the run instead of the human report
    #[arg(long)]
    json: bool,
    /// The klin.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
}

pub fn check(check: &Check, start: &Path, out: &mut String) -> Result<u8, Error> {
    let args = Args {
        config: check.config.clone(),
        gates: check.checks.clone(),
        changed: check.changed,
        view: match check.json {
            true => View::Json,
            false => View::Text,
        },
        footprint: diagnostics::footprint_requested(),
        without_integrations: false,
    };
    run(&args, start, out)
}

fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    if !args.json() {
        for note in notes(args.config.as_deref(), start) {
            let _ = writeln!(out, "{note}");
        }
    }
    let loaded = Project::load(args.config.as_deref(), start, &catalogue::sections());
    Ok(checked(args, start, loaded, out).finish(args, out))
}

/// The check document of a whole `klin check` from `start`, which `setup` reads its baseline
/// from. Spec 11.2, 11.7.
pub fn document(start: &Path) -> CheckDocument {
    let args = Args {
        view: View::Json,
        without_integrations: true,
        ..Args::default()
    };
    let loaded = Project::load(None, start, &catalogue::sections());
    checked(&args, start, loaded, &mut String::new())
}

/// One `klin check`: each run-scope step under the kind of error it can raise, and every
/// selected gate, as the check document. Spec 7, 11.3, 11.7.
fn checked(
    args: &Args,
    start: &Path,
    loaded: Result<Project, Error>,
    out: &mut String,
) -> CheckDocument {
    let located = located(args.config.as_deref(), start);
    let named_nothing =
        args.config.is_some() && !located.as_ref().is_some_and(|file| file.exists());
    let mut report = CheckDocument::default();
    let measured = loaded
        .map_err(fault(match named_nothing {
            true => ErrorKind::Invocation,
            false => ErrorKind::Configuration,
        }))
        .and_then(|mut project| {
            report.set_config(json!({
                "path": project.config.file.display().to_string(),
                "present": project.config.written(),
                "ignored": ignored(start),
            }));
            measured(args, &mut project, &mut report, out)
        });
    if let Err(fault) = measured {
        report.config_or(|| {
            json!({
                "path": located.as_ref().map(|file| file.display().to_string()),
                "present": located.as_ref().is_some_and(|file| file.is_file()),
            })
        });
        if !args.json() {
            let _ = writeln!(out, "ERR: {}", fault.error);
        }
        report.stop(fault);
    }
    report
}

fn measured(
    args: &Args,
    project: &mut Project,
    report: &mut CheckDocument,
    out: &mut String,
) -> Result<(), Fault> {
    let window = base::choose(project.root()).ok();
    if let Some(window) = &window {
        project.bind(window);
    }
    let project = &*project;
    if let Some(note) = deleted_config(args, project, window.as_ref()) {
        if !args.json() {
            let _ = writeln!(out, "  NOTE: {}", note.message);
        }
        report.note(note);
    }
    let plan = Plan::of(project).map_err(fault(ErrorKind::Configuration))?;
    let (mut wanted, unsupported) = chosen_gates(&args.gates, &plan, project)?;
    if args.without_integrations {
        let (integrations, kept): (Vec<&Gate>, Vec<&Gate>) =
            wanted.into_iter().partition(|gate| left_alone(args, gate));
        report.skip(&integrations);
        wanted = kept;
    }
    let against = against(args, &wanted, project, window.as_ref(), out)?;
    report.ran(args, project, (&plan, &wanted, unsupported), &against, out);
    loosened(args, project, window.as_ref(), report, out);
    Ok(())
}

/// A review item per finding a gate whose policy the change loosened fails under the base's
/// `klin.json` and not under the tree's. The tree's policy alone sets the exit, so a person
/// can accept debt in the same change, and a person's review decides. Spec 16.2.
fn loosened(
    args: &Args,
    project: &Project,
    window: Option<&Window>,
    report: &mut CheckDocument,
    out: &mut String,
) {
    let Some((judged, window)) =
        window.and_then(|window| Some((under_base(args, project, window)?, window)))
    else {
        return;
    };
    let Ok(plan) = Plan::of(&judged) else {
        return;
    };
    let changed = changed_gates(args, (&judged.config, &project.config), &plan);
    let hidden = hidden(args, (&judged, &changed), window, &report.reported());
    if hidden.is_empty() {
        return;
    }
    if args.text() {
        let _ = writeln!(out, "  REVIEW: {FILENAME} loosens the policy:");
    }
    for (gate, change) in &changed {
        let found: Vec<&Finding> = hidden
            .iter()
            .filter(|finding| finding.check.as_ref() == Some(&gate.name))
            .collect();
        told(args, (gate, change), &found, report, out);
    }
}

/// The run under the `klin.json` the base holds at the worktree root, read as the tree's file,
/// where the change altered that file. A missing file on either side reads as `{}`. A run under
/// a `--config` names no such file.
fn under_base(args: &Args, project: &Project, window: &Window) -> Option<Project> {
    if args.config.is_some() {
        return None;
    }
    let base = at_base(project, window);
    if base == std::fs::read(&project.config.file).ok() {
        return None;
    }
    let text = String::from_utf8(base.unwrap_or_else(|| b"{}".to_vec())).ok()?;
    let base = Config::read(&project.config.file, &text, &catalogue::sections()).ok()?;
    let mut judged = Project::of(base, project.start());
    judged.bind(window);
    Some(judged)
}

/// The gates of the base's plan the run selects whose policy the change altered, each with
/// what the change did to it.
fn changed_gates<'a>(
    args: &Args,
    (base, tree): (&Config, &Config),
    plan: &'a Plan,
) -> Vec<(&'a Gate, String)> {
    plan.gates
        .iter()
        .filter(|gate| named(&args.gates, gate) && !left_alone(args, gate))
        .filter_map(|gate| Some((gate, change(base, tree, gate)?)))
        .collect()
}

/// The findings the changed gates fail under the base's policy, over the run's window, that the
/// run itself does not fail.
fn hidden(
    args: &Args,
    (judged, changed): (&Project, &[(&Gate, String)]),
    window: &Window,
    failing: &[String],
) -> Vec<Finding> {
    let gates: Vec<&Gate> = changed.iter().map(|(gate, _)| *gate).collect();
    if gates.is_empty() {
        return Vec::new();
    }
    let Ok(against) = against(args, &gates, judged, Some(window), &mut String::new()) else {
        return Vec::new();
    };
    CheckDocument::failing_under(args, (judged, &gates), &against)
        .into_iter()
        .filter(|finding| !failing.contains(&finding.id))
        .collect()
}

/// One changed gate's line under the report's heading, and a review item per finding it hides.
fn told(
    args: &Args,
    (gate, change): (&Gate, &str),
    found: &[&Finding],
    report: &mut CheckDocument,
    out: &mut String,
) {
    if found.is_empty() {
        return;
    }
    if args.text() {
        let sites: Vec<String> = found.iter().map(|finding| finding.site()).collect();
        let noun = match found.len() {
            1 => "finding",
            _ => "findings",
        };
        let _ = writeln!(
            out,
            "    {change} hides {} new {noun}: {}",
            found.len(),
            sites.join(", ")
        );
    }
    for finding in found {
        report.review(Review {
            check: Some(gate.name.clone()),
            kind: CONFIG_LOOSENED,
            file: finding.file.clone(),
            line: finding.line.map_or(Slot::Null, Slot::Is),
            text: format!(
                "{FILENAME} loosens the policy: {change} hides {}",
                finding.site()
            ),
            reason: Slot::Null,
            values: None,
        });
    }
}

/// What the change did to one gate's policy, where it changed the gate's section or the
/// `accepted` entries that name the gate.
fn change(base: &Config, tree: &Config, gate: &Gate) -> Option<String> {
    let section = gate.check.section;
    let stated = tree.pinned(section);
    if base.pinned(section) != stated {
        return Some(match stated {
            Some(value) => format!("{section}: {value}"),
            None => format!("{section} removed"),
        });
    }
    (base.accepted_for(&gate.name) != tree.accepted_for(&gate.name))
        .then(|| format!("the accepted list of {}", gate.name))
}

/// The note of a run under `{}` whose base still holds the worktree root's `klin.json`, so the
/// change deleted the policy the base was judged under. Spec 5.1.
fn deleted_config(args: &Args, project: &Project, window: Option<&Window>) -> Option<Note> {
    if args.config.is_some() || project.config.written() {
        return None;
    }
    at_base(project, window?)?;
    Some(Note {
        check: None,
        kind: "config-deleted",
        message: format!(
            "{} is deleted: the base holds it and the working tree does not, so this run is \
             under {{}}",
            FILENAME
        ),
        coverage: None,
        file: Slot::Is(FILENAME.to_string()),
        line: None,
        values: None,
    })
}

/// The gates a run selects, and the capabilities a selector named that do not apply to this
/// tree or need a section the configuration does not hold. A name klin does not know, and a gate
/// a person set to `false`, is an invocation error. Spec 7.2, 7.3.
fn chosen_gates<'a>(
    named: &[String],
    plan: &'a Plan,
    project: &Project,
) -> Result<(Vec<&'a Gate>, Vec<&'static catalogue::Row>), Fault> {
    if named.is_empty() && plan.gates.is_empty() && !plan.excluded.is_empty() {
        return Err(Fault {
            kind: ErrorKind::Configuration,
            error: plan.no_gate(project),
        });
    }
    let mut unsupported = Vec::new();
    for name in named {
        let missing = plan.needs_a_section.iter().find(|check| check.name == name);
        match missing {
            Some(check) if !plan.gates.iter().any(|gate| &gate.name == name) => {
                unsupported.push(*check)
            }
            _ => plan
                .known(name, project)
                .map_err(fault(ErrorKind::Invocation))?,
        }
    }
    let wanted = plan
        .gates
        .iter()
        .filter(|gate| self::named(named, gate))
        .collect();
    Ok((wanted, unsupported))
}

/// Whether a run's selector takes this gate: it names no gate, or names this one.
fn named(names: &[String], gate: &Gate) -> bool {
    names.is_empty() || names.contains(&gate.name)
}

/// Whether the run leaves this gate alone, as the baseline of `setup` leaves every integration.
fn left_alone(args: &Args, gate: &Gate) -> bool {
    args.without_integrations && gate.check.activation == Activation::Integration
}

/// The worktree root's `klin.json` as the window's base holds it.
fn at_base(project: &Project, window: &Window) -> Option<Vec<u8>> {
    crate::sys::git::Repo::at(project.root()).blob(&window.before, FILENAME)
}
