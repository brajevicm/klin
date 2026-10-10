use std::borrow::Cow;
use std::fmt::Write;

use crate::contract::project::Project;
use crate::engine::document::{Against, Args};
use crate::engine::plan::Gate;
use crate::sys::changed::Change;
use crate::sys::error::{Error, ErrorKind, Fault, fault};
use crate::window::base::{self, Prior, Window};

pub fn against<'a>(
    args: &Args,
    wanted: &[&Gate],
    project: &'a Project,
    window: Option<&Window>,
    out: &mut String,
) -> Result<Against<'a>, Fault> {
    let base = base(args, wanted, project, window, out).map_err(fault(ErrorKind::Base))?;
    let changes = changes(args, project, base.as_ref(), out).map_err(fault(ErrorKind::Base))?;
    let scope = changes
        .as_ref()
        .map(|changed| changed.iter().map(|change| change.path.clone()).collect());
    let (prior, unlaid) = match prior(project, base.as_ref(), changes.as_deref(), wanted) {
        Ok(prior) => (prior, None),
        Err(why) => (None, Some(why.to_string())),
    };
    Ok(Against {
        scope,
        changes,
        prior,
        base,
        unlaid,
    })
}

fn base(
    args: &Args,
    wanted: &[&Gate],
    project: &Project,
    window: Option<&Window>,
    out: &mut String,
) -> Result<Option<Window>, Error> {
    if !args.changed && !wanted.iter().any(|gate| gate.check.needs.the_commit()) {
        return Ok(None);
    }
    let base = chosen(window, project)?;
    if !args.json() {
        let _ = writeln!(out, "  {}", base.line());
    }
    Ok(Some(base))
}

/// The window the run judges: the one the hook already read, or the base a run by hand and CI
/// choose for themselves. Spec 6.1, 6.3.
pub fn chosen(window: Option<&Window>, project: &Project) -> Result<Window, Error> {
    match window {
        Some(window) => Ok(window.clone()),
        None => base::choose(project.root()),
    }
}

fn prior(
    project: &Project,
    base: Option<&Window>,
    changes: Option<&[Change]>,
    wanted: &[&Gate],
) -> Result<Option<Prior>, Error> {
    let Some(base) = base.filter(|_| wanted.iter().any(|gate| gate.check.needs.the_tree())) else {
        return Ok(None);
    };
    base::materialize(project, &base.before, changes).map(Some)
}

/// The changed set the scoped gates judge, computed once for the run and reused by the base
/// laid out for them. Spec 4.5, ADR 0038.
fn changes<'a>(
    args: &Args,
    project: &'a Project,
    base: Option<&Window>,
    out: &mut String,
) -> Result<Option<Cow<'a, [Change]>>, Error> {
    let (true, Some(base)) = (args.changed, base) else {
        return Ok(None);
    };
    let changed = project.changes(&base.before)?;
    if !args.json() {
        let _ = writeln!(
            out,
            "  changed: {} file(s) against the base — the scoped gates judge those; \
             CI judges everything",
            changed.len()
        );
    }
    Ok(Some(changed))
}
