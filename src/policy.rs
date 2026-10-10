use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::base;
use crate::check::contract::{self, Activation, Derivation, Told};
use crate::check::{catalogue, render};
use crate::config;
use crate::error::{Error, ErrorKind, Fault, fault};
use crate::plan::{Gate, Plan, State};
use crate::project::Project;
use crate::{build, ceiling, reference, state};

/// What `policy` indents a capability's own lines by, under the row that names it.
const UNDER: &str = "      ";

#[derive(clap::Args)]
pub struct Policy {
    /// Explain only this check, by the name a gate takes
    section: Option<String>,
    /// Explain only this entry of the check, such as one convention
    #[arg(requires = "section")]
    entry: Option<String>,
    /// Print the configuration reference, as Markdown, from the keys the checks declare
    #[arg(long, conflicts_with_all = ["section", "schema", "json", "config"])]
    reference: bool,
    /// Print the JSON Schema of klin.json
    #[arg(long, conflicts_with_all = ["section", "json", "config"])]
    schema: bool,
    /// Print one JSON object instead of the text
    #[arg(long)]
    json: bool,
    /// The klin.json to read (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
}

/// One `klin policy`: the effective policy as text or as the policy document, and a refusal
/// as the `FAIL:` line or as that document with its error. Spec 11.6, 11.7.
pub fn run(args: &Policy, start: &Path, out: &mut String) -> Result<u8, Error> {
    if args.reference || args.schema {
        return reference::run(args.schema, out);
    }
    if !args.json {
        for note in config::notes(args.config.as_deref(), start) {
            let _ = writeln!(out, "{note}");
        }
    }
    let outcome = loaded(args, start).and_then(|mut project| {
        if let Ok(window) = base::choose(project.root()) {
            project.bind(&window);
        }
        listed(args, &project, out)
    });
    let Err(refused) = outcome else {
        return Ok(0);
    };
    if !args.json {
        return Err(refused.error);
    }
    let located = config::located(args.config.as_deref(), start);
    let document = json!({
        "schema_version": 1,
        "command": "policy",
        "config": {
            "path": located.as_ref().map(|file| file.display().to_string()),
            "present": located.as_ref().is_some_and(|file| file.is_file()),
        },
        "errors": [{ "kind": refused.kind.name(), "check": null, "message": refused.error.to_string() }],
    });
    out.clear();
    let _ = writeln!(out, "{document}");
    Ok(2)
}

/// The project `policy` reads, where a `--config` that names no file is an invocation error,
/// as under `klin check`.
fn loaded(args: &Policy, start: &Path) -> Result<Project, Fault> {
    let named_nothing = args.config.is_some()
        && !config::located(args.config.as_deref(), start).is_some_and(|file| file.exists());
    Project::load(args.config.as_deref(), start, &catalogue::sections())
        .map(Project::read_only)
        .map_err(fault(match named_nothing {
            true => ErrorKind::Invocation,
            false => ErrorKind::Configuration,
        }))
}

/// The effective policy of every gate, or of the one `policy` names: its state, and each value
/// it uses with where the value came from. Each check's own derivation step gives the values, so
/// no gate runs, no base is laid out and no file is measured. Spec 11.6.
fn listed(args: &Policy, project: &Project, out: &mut String) -> Result<(), Fault> {
    let plan = Plan::of(project).map_err(fault(ErrorKind::Configuration))?;
    let wanted = asked(args, project, &plan).map_err(fault(ErrorKind::Invocation))?;
    let mut capabilities = wanted
        .into_iter()
        .map(|gate| capability(args, project, gate))
        .collect::<Result<Vec<_>, Fault>>()?;
    capabilities.extend(inactive(args, &plan));
    let shared = Shared::of(project).map_err(fault(ErrorKind::Configuration))?;
    match args.json {
        true => policy_json(project, &capabilities, &shared, out),
        false => policy_text(&capabilities, args.section.is_none(), &shared, out),
    }
    Ok(())
}

/// One capability as `policy` prints it: the lines a person reads under its name, and the
/// values and limitations its JSON row carries. Spec 11.6, 11.7.
struct Capability<'a> {
    name: &'a str,
    check: &'static catalogue::Row,
    state: State,
    lines: Vec<String>,
    values: Vec<Value>,
    limitations: Vec<&'static str>,
}

impl<'a> Capability<'a> {
    /// A capability that does not run, which says only what limits it.
    fn inactive(name: &'a str, check: &'static catalogue::Row, state: State) -> Capability<'a> {
        let limitations = limitations(check, &Map::new());
        Capability {
            name,
            check,
            state,
            lines: limitation_lines(&limitations),
            values: Vec::new(),
            limitations,
        }
    }

    fn json(&self) -> Value {
        json!({
            "name": self.name,
            "section": self.check.section,
            "kind": self.check.kind(),
            "activation": self.check.activation.name(),
            "placement": self.check.placement.names(),
            "state": self.state.name(),
            "values": self.values,
            "limitations": self.limitations,
        })
    }

    fn text(&self, out: &mut String) {
        let _ = writeln!(out, "{} — {}", self.name, self.state.said());
        let _ = writeln!(
            out,
            "{UNDER}placement: {}",
            self.check.placement.names().join(", ")
        );
        let _ = writeln!(out, "{UNDER}activation: {}", self.check.activation.name());
        for line in &self.lines {
            let _ = writeln!(out, "{UNDER}{line}");
        }
    }
}

/// The policy no one gate owns: the build, the accepted list and where klin keeps its state.
struct Shared<'a> {
    build: Build,
    accepted: &'a [Value],
    state: Option<PathBuf>,
}

impl<'a> Shared<'a> {
    fn of(project: &'a Project) -> Result<Shared<'a>, Error> {
        Ok(Shared {
            build: Build::of(project)?,
            accepted: project
                .config
                .pinned(config::ACCEPTED.name)
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            state: state::dir(project.root()),
        })
    }
}

/// The build a run makes before it measures: the one a person pinned, or the one the manifests
/// derive. Spec 5.4, 11.6.
struct Build {
    /// The word after `build —`: where the build came from, or that there is none.
    said: &'static str,
    lines: Vec<String>,
    json: Value,
}

impl Build {
    fn of(project: &Project) -> Result<Build, Error> {
        let plan = build::plan(project)?;
        if let Some(value) = project.config.pinned(config::BUILD.name) {
            return Ok(Build {
                said: "pinned",
                lines: vec![format!("pinned: {} {}", config::BUILD.name, shown(value))],
                json: json!({ "value": value, "provenance": "pinned" }),
            });
        }
        let (lines, entries): (Vec<String>, Vec<Option<Value>>) = plan.said.into_iter().unzip();
        let entry = entries.into_iter().flatten().next();
        let said = match entry {
            Some(_) => "derived",
            None => "none, no manifest the survey found names a command",
        };
        Ok(Build {
            said,
            lines,
            json: json!({
                "value": entry.as_ref().and_then(|entry| entry.get("value")),
                "provenance": "derived",
                "rule": entry.as_ref().and_then(|entry| entry.get("rule")),
            }),
        })
    }

    fn text(&self, out: &mut String) {
        let _ = writeln!(out, "build — {}", self.said);
        for line in &self.lines {
            let _ = writeln!(out, "{UNDER}{line}");
        }
    }
}

/// The names `policy` takes. A plan with no gate is still listed, because each capability that
/// does not run says why. Spec 11.6.
fn listable(args: &Policy, project: &Project, plan: &Plan) -> Result<(), Error> {
    args.section
        .iter()
        .try_for_each(|name| stated(name, plan, project))
}

/// A name `policy` takes: a gate that runs, one a person excluded, or one that needs a section.
fn stated(name: &str, plan: &Plan, project: &Project) -> Result<(), Error> {
    let inactive = plan.excluded.iter().any(|excluded| excluded == name)
        || plan.needs_a_section.iter().any(|check| check.name == name);
    match inactive {
        true => Ok(()),
        false => plan.known(name, project),
    }
}

/// The gates that do not run: the ones a person excluded, an Automatic check whose facts the
/// tree does not hold, which does not apply, and a check that needs a section a person writes.
fn inactive<'a>(args: &Policy, plan: &'a Plan) -> Vec<Capability<'a>> {
    let excluded = plan
        .excluded
        .iter()
        .filter(|name| named(args, name))
        .filter_map(|name| {
            let check = catalogue::CATALOGUE
                .iter()
                .find(|check| check.name == name)?;
            Some(Capability::inactive(name, check, State::Excluded))
        });
    let needed = plan
        .needs_a_section
        .iter()
        .filter(|check| named(args, check.name))
        .map(|check| {
            let state = match check.activation {
                Activation::Automatic => State::NotApplicable,
                Activation::Policy | Activation::Integration => State::NeedsPolicy,
            };
            Capability::inactive(check.name, check, state)
        });
    excluded.chain(needed).collect()
}

fn named(args: &Policy, name: &str) -> bool {
    args.section.as_deref().is_none_or(|one| one == name)
}

/// Each gate that runs and that `policy` names, once the names it took are known.
fn asked<'a>(args: &Policy, project: &Project, plan: &'a Plan) -> Result<Vec<&'a Gate>, Error> {
    listable(args, project, plan)?;
    let wanted: Vec<&Gate> = plan
        .gates
        .iter()
        .filter(|gate| named(args, &gate.name))
        .collect();
    if let (Some(entry), [gate]) = (&args.entry, wanted.as_slice())
        && !matches!(
            gate.check.derivation,
            Derivation::Explained(_) | Derivation::ExplainedWhenNamed(_)
        )
    {
        return Err(Error(format!(
            "{} has no entries to explain one by one, so drop {entry}",
            gate.name
        )));
    }
    Ok(wanted)
}

/// One gate's policy: what its derivation step derives, or what its explanation says, then the
/// values a person pinned that the step did not name, then the values neither gave, as built in.
fn capability<'a>(
    args: &Policy,
    project: &Project,
    gate: &'a Gate,
) -> Result<Capability<'a>, Fault> {
    let check = gate.check;
    let fields = section_of(project, gate, args.entry.as_deref());
    let (mut lines, mut values) = said_by(gate, project, args, &fields)?;
    values.extend(
        fields
            .iter()
            .map(|(key, value)| pinned_value(project, check.section, key, value)),
    );
    let limitations = limitations(check, &fields);
    lines.extend(limitation_lines(&limitations));
    Ok(Capability {
        name: &gate.name,
        check,
        state: State::Active,
        lines,
        values,
        limitations,
    })
}

/// What one gate's derivation says under `policy`: its explanation, its derived values, or a
/// pointer to the named form when its explanation parses the working tree.
fn said_by(
    gate: &Gate,
    project: &Project,
    args: &Policy,
    fields: &Map<String, Value>,
) -> Result<(Vec<String>, Vec<Value>), Fault> {
    let check = gate.check;
    match check.derivation {
        Derivation::Explained(explain) => explained(explain, project, args),
        Derivation::ExplainedWhenNamed(explain) if args.section.is_some() => {
            explained(explain, project, args)
        }
        Derivation::ExplainedWhenNamed(_) => Ok((
            vec![format!(
                "derived from the source of the working tree, which `klin policy {}` lists",
                gate.name
            )],
            Vec::new(),
        )),
        Derivation::Values(derive) => {
            let said = derive(project).map_err(fault(ErrorKind::Configuration))?;
            Ok(said_values(check, as_told(said), fields))
        }
        Derivation::Nothing => Ok(said_values(check, Vec::new(), fields)),
    }
}

/// What a check's explanation says, where an entry it does not have is an invocation error,
/// and anything else that stops the explanation is a configuration error. Spec 7.3, 11.7.
fn explained(
    explain: contract::Explain,
    project: &Project,
    args: &Policy,
) -> Result<(Vec<String>, Vec<Value>), Fault> {
    match explain(project, args.entry.as_deref()) {
        Ok(explained) => Ok((explained.lines, explained.values)),
        Err(error) if args.entry.is_some() && explain(project, None).is_ok() => {
            Err(fault(ErrorKind::Invocation)(error))
        }
        Err(error) => Err(fault(ErrorKind::Configuration)(error)),
    }
}

/// A value a person pinned, and for a dated schedule the step in force as the value, with the
/// date of that step and the whole schedule beside it. Spec 5.4, 11.6.
fn pinned_value(project: &Project, section: &str, key: &str, value: &Value) -> Value {
    let step = value
        .as_object()
        .filter(|steps| ceiling::is_schedule(steps))
        .and_then(|_| ceiling::read(&project.config.file, section, key, value, "a number").ok());
    match step {
        Some(ceiling) => json!({
            "key": key,
            "value": ceiling.value,
            "step": ceiling.step,
            "schedule": value,
            "provenance": "pinned",
        }),
        None => json!({ "key": key, "value": value, "provenance": "pinned" }),
    }
}

/// The lines and values of a check with no explanation of its own: what its derivation step
/// said, as a run prints it, then each value a person pinned that the step did not name, then
/// each value neither gave, as built in.
fn said_values(
    check: &catalogue::Row,
    told: Vec<Told>,
    fields: &Map<String, Value>,
) -> (Vec<String>, Vec<Value>) {
    let keys = said_keys(&told);
    let mut lines = render::provenance(&told);
    let mut values: Vec<Value> = render::json(&told)
        .derived
        .iter()
        .map(|entry| {
            json!({
                "key": entry.get("key"),
                "value": entry.get("value"),
                "provenance": "derived",
                "rule": entry.get("rule"),
            })
        })
        .collect();
    let unsaid = fields
        .iter()
        .filter(|(key, _)| !keys.contains(&key.as_str()));
    lines.extend(
        unsaid.map(|(key, value)| format!("pinned: {} {key} {}", check.section, shown(value))),
    );
    let built_in = check.keys.iter().filter(|key| {
        !key.default.is_empty() && !fields.contains_key(key.name) && !keys.contains(&key.name)
    });
    for key in built_in {
        values.push(key.built_in());
        lines.push(format!(
            "built-in: {} {} {}",
            check.section, key.name, key.default
        ));
    }
    (lines, values)
}

/// The fields a person wrote for one gate: its section, or its own entry of a section that is a
/// list of named entries, less the name. With an entry named, such as one convention, only that
/// entry's field.
fn section_of(project: &Project, gate: &Gate, entry: Option<&str>) -> Map<String, Value> {
    let section = project.config.pinned(gate.check.section);
    let fields = match section {
        Some(Value::Array(entries)) => entries
            .iter()
            .find(|entry| entry.get(contract::NAMED.name) == Some(&json!(gate.name))),
        other => other,
    };
    let mut fields = fields
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if gate.check.gate_per_entry {
        fields.remove(contract::NAMED.name);
    }
    fields.retain(|key, _| entry.is_none_or(|entry| entry == key));
    fields
}

/// The keys a check's derivation step already named, pinned or derived, so their lines are the
/// ones a run prints.
fn said_keys(told: &[Told]) -> Vec<&str> {
    told.iter()
        .filter_map(|item| match item {
            Told::Provenance(contract::Provenance::Pinned { key, .. }) => Some(key.as_str()),
            Told::Provenance(contract::Provenance::Derived(derived)) => derived.key.as_deref(),
            _ => None,
        })
        .collect()
}

fn as_told(said: Vec<contract::Provenance>) -> Vec<Told> {
    said.into_iter().map(Told::Provenance).collect()
}

/// What an integration does not claim, which `policy` lists beside its values. Spec 9.4.
fn limitations(check: &catalogue::Row, fields: &Map<String, Value>) -> Vec<&'static str> {
    if check.activation != Activation::Integration {
        return Vec::new();
    }
    let mut said = vec![
        "runs at klin check only, never at the Stop",
        "coverage unverified: klin judges the report's results on the changed lines, and a \
         file the report does not name is not proven clean",
    ];
    if fields.contains_key("run") {
        said.push(
            "the command is the project's own: klin bounds its run but does not claim it is \
             deterministic, and any network or ambient state it reads is the project's trust \
             choice",
        );
    }
    said
}

fn limitation_lines(limitations: &[&str]) -> Vec<String> {
    limitations
        .iter()
        .map(|said| format!("limitation: {said}"))
        .collect()
}

fn accepted_text(accepted: &[Value], out: &mut String) {
    let _ = match accepted.len() {
        0 => writeln!(out, "accepted — nothing is accepted"),
        1 => writeln!(out, "accepted — 1 entry"),
        many => writeln!(out, "accepted — {many} entries"),
    };
    for entry in accepted {
        let word = |key: &str| entry.get(key).and_then(Value::as_str).unwrap_or_default();
        let _ = writeln!(
            out,
            "{UNDER}{} {}: {}{}",
            word("gate"),
            word("file"),
            word("text"),
            accepted_values(entry)
        );
    }
}

/// The values an accepted entry allows, such as `(count 1)`, which tell two entries at one site
/// apart.
fn accepted_values(entry: &Value) -> String {
    let said: Vec<String> = entry
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(key, _)| {
            !["gate", "file", "text", config::ACCEPTED_REASON].contains(&key.as_str())
        })
        .map(|(key, value)| format!("{key} {}", shown(value)))
        .collect();
    match said.is_empty() {
        true => String::new(),
        false => format!(" ({})", said.join(", ")),
    }
}

fn policy_text(capabilities: &[Capability], whole: bool, shared: &Shared, out: &mut String) {
    for capability in capabilities {
        capability.text(out);
    }
    if whole {
        out.push_str(&render::lost_policy());
        shared.build.text(out);
        accepted_text(shared.accepted, out);
    }
    if let Some(at) = &shared.state {
        let _ = writeln!(out, "state: {}", at.display());
    }
}

fn policy_json(project: &Project, capabilities: &[Capability], shared: &Shared, out: &mut String) {
    let document = json!({
        "schema_version": 1,
        "command": "policy",
        "config": {
            "path": project.config.file.display().to_string(),
            "present": project.config.written(),
            "ignored": config::ignored(project.start()),
        },
        "derivation": { "commit": project.facts().commit },
        "capabilities": capabilities.iter().map(Capability::json).collect::<Vec<_>>(),
        "build": shared.build.json,
        "accepted": shared.accepted,
        "state_dir": shared.state.as_ref().map(|at| at.display().to_string()),
    });
    let _ = writeln!(out, "{document}");
}

/// A pinned value as a person reads it: a path or a list of paths as text, anything else as JSON.
fn shown(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) if items.iter().all(Value::is_string) => items
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<&str>>()
            .join(", "),
        other => other.to_string(),
    }
}
