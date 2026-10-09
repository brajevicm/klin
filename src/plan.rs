use serde_json::Value;

use crate::check::catalogue;
use crate::check::contract::{self, Activation};
use crate::config::MEASUREMENT_LOST;
use crate::error::Error;
use crate::project::Project;

pub struct Gate {
    pub name: String,
    pub check: &'static catalogue::Row,
}

#[derive(Default)]
pub struct Plan {
    pub gates: Vec<Gate>,
    pub excluded: Vec<String>,
    /// The checks klin offers that neither the config nor the survey supplies a section for.
    /// Each needs a section a person writes, and none of them runs.
    pub needs_a_section: Vec<&'static catalogue::Row>,
}

impl Plan {
    /// The one gate a check runs as when its section is not a list of named entries.
    fn one(&mut self, check: &'static catalogue::Row) {
        self.gates.push(Gate {
            name: check.name.to_string(),
            check,
        });
    }
}

/// What one capability is in this tree, which `klin check` and `klin policy` both name.
/// Spec 11.6, 11.7.
#[derive(Clone, Copy)]
pub enum State {
    Active,
    Excluded,
    NotApplicable,
    NeedsPolicy,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Active => "active",
            State::Excluded => "excluded",
            State::NotApplicable => "not-applicable",
            State::NeedsPolicy => "needs-policy",
        }
    }

    /// The word the text of `policy` gives the state after the capability's name.
    pub fn said(self) -> &'static str {
        match self {
            State::Active => "runs",
            State::NeedsPolicy => "needs a section a person writes",
            State::Excluded | State::NotApplicable => self.name(),
        }
    }
}

fn names<'a>(named: impl Iterator<Item = &'a str>) -> String {
    named.collect::<Vec<&str>>().join(", ")
}

pub fn every_check() -> String {
    names(catalogue::names())
}

pub fn no_gate(project: &Project, plan: &Plan) -> Error {
    let config = &project.config;
    if !plan.excluded.is_empty() {
        return Error(format!(
            "{} excludes every gate it names: {} — a run that measures nothing cannot pass, \
             so lift one exclusion",
            config.file.display(),
            names(plan.excluded.iter().map(String::as_str))
        ));
    }
    if !config.written() {
        return Error(format!(
            "{} does not exist and the survey of {} found no source root, no document and no \
             manifest, so there is nothing to gate — run klin from the tree you mean to measure, \
             or write the file naming one of: {}",
            config.file.display(),
            config.root().display(),
            every_check()
        ));
    }
    Error(format!(
        "{} configures no gate — name at least one of: {}",
        config.file.display(),
        every_check()
    ))
}

pub fn plan(project: &Project) -> Result<Plan, Error> {
    let mut plan = Plan::default();
    for check in catalogue::CATALOGUE {
        add(project, check, &mut plan)?;
    }
    distinct(project, &plan)?;
    Ok(plan)
}

/// A check's gates, from what the config states for its section and, where it states nothing,
/// from what the section's absence means for this check: an Automatic check runs when its facts
/// are available, and a Policy or Integration check runs nothing until a person writes the
/// section. Planning derives no expensive number or topology. Spec 4.6, 5.2, ADR 0038.
fn add(project: &Project, check: &'static catalogue::Row, plan: &mut Plan) -> Result<(), Error> {
    let Some(stated) = project.config.pinned(check.section) else {
        absent(project, check, plan);
        return Ok(());
    };
    match stated {
        Value::Bool(false) => plan.excluded.push(check.name.to_string()),
        _ if check.gate_per_entry => {
            for (name, _) in contract::named_entries(&project.config, check.section)? {
                plan.gates.push(Gate { name, check });
            }
        }
        _ => plan.one(check),
    }
    Ok(())
}

/// The gate a section's absence plans: the check itself for an Automatic check whose facts are
/// available, and otherwise a check that needs a section a person writes.
fn absent(project: &Project, check: &'static catalogue::Row, plan: &mut Plan) {
    match check.activation == Activation::Automatic && (check.available)(project) {
        true => plan.one(check),
        false => plan.needs_a_section.push(check),
    }
}

fn distinct(project: &Project, plan: &Plan) -> Result<(), Error> {
    let mut seen: Vec<&str> = Vec::new();
    let named = plan
        .gates
        .iter()
        .map(|gate| gate.name.as_str())
        .chain(plan.excluded.iter().map(String::as_str));
    for name in named {
        if name == MEASUREMENT_LOST {
            return Err(Error(format!(
                "{}: {MEASUREMENT_LOST} is the name of klin's own row of files it can no longer \
                 measure, so no gate may take it — name the gate otherwise",
                project.config.file.display()
            )));
        }
        if seen.contains(&name) {
            return Err(Error(format!(
                "{}: two gates are named {name} — a name selects one gate, so each must differ",
                project.config.file.display()
            )));
        }
        seen.push(name);
    }
    Ok(())
}

pub fn select<'a>(
    named: &[String],
    plan: &'a Plan,
    project: &Project,
) -> Result<Vec<&'a Gate>, Error> {
    for name in named {
        known(name, plan, project)?;
    }
    let wanted: Vec<&Gate> = plan
        .gates
        .iter()
        .filter(|gate| named.is_empty() || named.iter().any(|wanted| wanted == &gate.name))
        .collect();
    if wanted.is_empty() {
        return Err(no_gate(project, plan));
    }
    Ok(wanted)
}

pub fn known(name: &str, plan: &Plan, project: &Project) -> Result<(), Error> {
    let config = &project.config;
    if plan.gates.iter().any(|gate| gate.name == name) {
        return Ok(());
    }
    if plan.excluded.iter().any(|excluded| excluded == name) {
        return Err(Error(format!(
            "the gate named {name} is excluded in {} — naming a gate is a claim that it runs, \
             so lift the exclusion or drop {name}",
            config.file.display()
        )));
    }
    Err(Error(format!(
        "no gate named {name} — {} configures: {}",
        config.file.display(),
        match plan.gates.is_empty() {
            true => "nothing".to_string(),
            false => names(plan.gates.iter().map(|gate| gate.name.as_str())),
        }
    )))
}
