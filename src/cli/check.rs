use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::config::file::{FILENAME, ignored, located, notes};
use crate::contract::project::Project;
use crate::engine::against::against;
use crate::engine::catalogue;
use crate::engine::document::{Args, CheckDocument, View};
use crate::engine::plan::{Gate, Plan};
use crate::engine::render::{Note, Slot};
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
    Ok(checked(args, start, loaded, out))
}

/// One `klin check`: each run-scope step under the kind of error it can raise, every selected
/// gate, and the check document or its text. Spec 7, 11.3, 11.7.
fn checked(args: &Args, start: &Path, loaded: Result<Project, Error>, out: &mut String) -> u8 {
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
    report.finish(args, out)
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
    let (wanted, unsupported) = chosen_gates(&args.gates, &plan, project)?;
    let against = against(args, &wanted, project, window.as_ref(), out)?;
    report.ran(args, project, (&plan, &wanted, unsupported), &against, out);
    Ok(())
}

/// The note of a run under `{}` whose base still holds the worktree root's `klin.json`, so the
/// change deleted the policy the base was judged under. Spec 5.1.
fn deleted_config(args: &Args, project: &Project, window: Option<&Window>) -> Option<Note> {
    if args.config.is_some() || project.config.written() {
        return None;
    }
    let window = window?;
    crate::sys::git::Repo::at(project.root()).blob(&window.before, FILENAME)?;
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
        .filter(|gate| named.is_empty() || named.contains(&gate.name))
        .collect();
    Ok((wanted, unsupported))
}
