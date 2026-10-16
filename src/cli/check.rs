use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::config::file::{Config, FILENAME, MEASUREMENT_LOST, ignored, located, notes};
use crate::contract::check::{Activation, CONFIG_LOOSENED};
use crate::contract::project::Project;
use crate::engine::against::against;
use crate::engine::document::{Args, CheckDocument, Partial, Sites, View};
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
    loosened(
        args,
        (project, &plan, &wanted),
        window.as_ref(),
        report,
        out,
    );
    Ok(())
}

/// A review item per finding a gate whose policy the change loosened fails under the base's
/// `klin.json` and not under the tree's, or a note that the base's policy could not be judged.
/// The tree's policy alone sets the exit, so a person can accept debt in the same change, and a
/// person's review decides. Spec 16.2.
fn loosened(
    args: &Args,
    (project, plan, wanted): (&Project, &Plan, &[&Gate]),
    window: Option<&Window>,
    report: &mut CheckDocument,
    out: &mut String,
) {
    let Some(window) = window else {
        return;
    };
    let judged = match under_base(args, project, window) {
        Ok(Some(judged)) => judged,
        Ok(None) => return,
        Err(why) => return unjudged(args, &why, report, out),
    };
    let failing = report.failing_sites();
    let tree = Tree {
        project,
        plan,
        wanted,
    };
    let Partial { found, erred } = hidden_by_gate(args, (&tree, &judged), window, &failing);
    told(args, found, report, out);
    if let Some(why) = erred {
        unjudged(args, &why, report, out);
    }
}

/// The run under the tree's own policy: its project, its plan and the gates it selected.
struct Tree<'a> {
    project: &'a Project,
    plan: &'a Plan,
    wanted: &'a [&'a Gate],
}

/// The findings one changed gate hides, and what the change did to its policy.
struct Hidden {
    gate: String,
    change: String,
    found: Vec<Finding>,
}

/// The run under the `klin.json` the base holds at the worktree root, read as the tree's file,
/// where the change altered that file. A missing file on either side reads as `{}`. A run under
/// a `--config` names no such file. A base git could not read is an error, so the run does not
/// take it for a missing file.
fn under_base(args: &Args, project: &Project, window: &Window) -> Result<Option<Project>, Error> {
    if args.config.is_some() {
        return Ok(None);
    }
    let bytes = at_base(project, window).ok_or_else(|| {
        Error(format!(
            "git could not read {FILENAME} at the base {}",
            window.before
        ))
    })?;
    if bytes == std::fs::read(&project.config.file).ok() {
        return Ok(None);
    }
    let text = match &bytes {
        Some(bytes) => std::str::from_utf8(bytes).ok(),
        None => Some("{}"),
    };
    let Some(base) =
        text.and_then(|text| Config::read(&project.config.file, text, &catalogue::sections()).ok())
    else {
        return Ok(None);
    };
    let mut judged = Project::of(base, project.start());
    judged.bind(window);
    Ok(Some(judged))
}

/// Each changed gate that hides a finding, in the base plan's order, then the lost files the
/// change hides, with what it hides, and the error that kept the base's policy from being judged
/// whole. A change to the accepted lost files runs every gate the run selects, since any of them
/// can lose a file.
fn hidden_by_gate(
    args: &Args,
    (tree, judged): (&Tree, &Project),
    window: &Window,
    failing: &Sites,
) -> Partial<Hidden> {
    let plan = match Plan::of(judged) {
        Ok(plan) => plan,
        Err(why) => return Partial::stopped(why),
    };
    let (base, policy) = (&judged.config, &tree.project.config);
    let changed: Vec<(&Gate, String)> = selected(args, &plan, tree)
        .filter_map(|gate| Some((gate, change(base, policy, gate)?)))
        .collect();
    let lost = accepted_change(base, policy, MEASUREMENT_LOST);
    let gates: Vec<&Gate> = match lost {
        Some(_) => selected(args, &plan, tree).collect(),
        None => changed.iter().map(|(gate, _)| *gate).collect(),
    };
    let Partial { found, erred } = hidden(args, (judged, &gates), window, failing);
    Partial {
        found: grouped(changed, lost, found),
        erred,
    }
}

/// The gates of the base's plan the run selects: each it names, and each entry the tree's plan
/// no longer holds in the section of a gate it selected, which a rename may have moved there.
fn selected<'a>(args: &Args, base: &'a Plan, tree: &Tree) -> impl Iterator<Item = &'a Gate> {
    base.gates
        .iter()
        .filter(|gate| !left_alone(args, gate) && (named(&args.gates, gate) || renamed(gate, tree)))
}

/// Whether the tree's plan lost this base gate while the run selects a gate of its section.
fn renamed(gate: &Gate, tree: &Tree) -> bool {
    let section = gate.check.section;
    !tree.plan.gates.iter().any(|kept| kept.name == gate.name)
        && tree
            .wanted
            .iter()
            .any(|wanted| wanted.check.section == section)
}

/// The hidden findings of each changed gate, then the hidden lost files, under the change to
/// the accepted lost files or else every change the run judged.
fn grouped(
    changed: Vec<(&Gate, String)>,
    lost: Option<String>,
    mut hidden: Vec<Finding>,
) -> Vec<Hidden> {
    let lost_files: Vec<Finding> = hidden
        .extract_if(.., |finding| finding.kind == MEASUREMENT_LOST)
        .collect();
    let lost_change = lost.unwrap_or_else(|| {
        let changes: Vec<&str> = changed.iter().map(|(_, change)| change.as_str()).collect();
        changes.join("; ")
    });
    let mut found: Vec<Hidden> = changed
        .into_iter()
        .filter_map(|(gate, change)| {
            let found: Vec<Finding> = hidden
                .extract_if(.., |finding| finding.check.as_ref() == Some(&gate.name))
                .collect();
            (!found.is_empty()).then(|| Hidden {
                gate: gate.name.clone(),
                change,
                found,
            })
        })
        .collect();
    if !lost_files.is_empty() {
        found.push(Hidden {
            gate: MEASUREMENT_LOST.to_string(),
            change: lost_change,
            found: lost_files,
        });
    }
    found
}

/// The findings `gates` fail under the base's policy, over the run's window, beyond the ones the
/// run itself fails at the same site, and the first error of a gate that could not run. Both
/// runs measure the one working tree, so a site names the same place in each.
fn hidden(
    args: &Args,
    (judged, gates): (&Project, &[&Gate]),
    window: &Window,
    failing: &Sites,
) -> Partial<Finding> {
    if gates.is_empty() {
        return Partial::default();
    }
    let against = match against(args, gates, judged, Some(window), &mut String::new()) {
        Ok(against) => against,
        Err(fault) => return Partial::stopped(fault.error),
    };
    CheckDocument::failing_under(args, (judged, gates), &against, failing)
}

/// The report's heading and one line per changed gate that hides a finding, and a review item
/// per finding it hides.
fn told(args: &Args, hidden: Vec<Hidden>, report: &mut CheckDocument, out: &mut String) {
    if args.text() && !hidden.is_empty() {
        let _ = writeln!(out, "  REVIEW: {FILENAME} loosens the policy:");
    }
    for Hidden {
        gate,
        change,
        found,
    } in hidden
    {
        if args.text() {
            let sites: Vec<String> = found.iter().map(site).collect();
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
                check: Some(gate.clone()),
                kind: CONFIG_LOOSENED,
                text: format!(
                    "{FILENAME} loosens the policy: {change} hides {}",
                    site(&finding)
                ),
                line: finding.line.map_or(Slot::Null, Slot::Is),
                file: finding.file,
                reason: Slot::Null,
                values: None,
            });
        }
    }
}

/// Where a hidden finding is: its file, line and text, or the file alone for a finding of a
/// whole file, such as a lost one.
fn site(finding: &Finding) -> String {
    match finding.line {
        Some(_) => finding.site(),
        None => finding.file.clone(),
    }
}

/// The note of a changed `klin.json` whose base policy could not be judged, so no review item
/// says what the change hides. Spec 16.2.
fn unjudged(args: &Args, why: &Error, report: &mut CheckDocument, out: &mut String) {
    let note = Note::of_run(
        CONFIG_LOOSENED,
        &format!("{FILENAME} changed, and the base's policy could not be judged: {why}"),
    );
    if args.text() {
        let _ = writeln!(out, "  NOTE: {}", note.message);
    }
    report.note(note);
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
    accepted_change(base, tree, &gate.name)
}

/// The change to the `accepted` entries that name `gate`, where the change altered them.
fn accepted_change(base: &Config, tree: &Config, gate: &str) -> Option<String> {
    (base.accepted_for(gate) != tree.accepted_for(gate))
        .then(|| format!("the accepted list of {gate}"))
}

/// The note of a run under `{}` whose base still holds the worktree root's `klin.json`, so the
/// change deleted the policy the base was judged under. Spec 5.1.
fn deleted_config(args: &Args, project: &Project, window: Option<&Window>) -> Option<Note> {
    if args.config.is_some() || project.config.written() {
        return None;
    }
    at_base(project, window?).flatten()?;
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

/// The worktree root's `klin.json` as the window's base holds it, `Some(None)` where the base
/// holds none, and `None` where git could not read the base.
fn at_base(project: &Project, window: &Window) -> Option<Option<Vec<u8>>> {
    let mut held = None;
    crate::sys::git::Repo::at(project.root()).blobs(&window.before, &[FILENAME], |_, bytes| {
        held = Some(bytes.map(<[u8]>::to_vec));
    })?;
    held
}
