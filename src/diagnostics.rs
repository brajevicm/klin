//! The diagnostics of the check document: one row per gate of what its run cost and covered.
//! The rows sit outside the stable schema, and the performance rows read them. Spec 11.2, 11.7.

use serde_json::{Map, Value, json};

use crate::base;
use crate::check::contract::Records;
use crate::clock;
use crate::coverage::Coverage;
use crate::syntax::{LanguageId, structural};

/// What one gate's structural work came to, with the declaration states of the gate that builds
/// them. A gate that reads no structural facts records none. Spec 11.2.
fn facts(records: &Records) -> Value {
    let Some(facts) = records.facts else {
        return Value::Null;
    };
    let mut out = json!({
        "reads": facts.reads,
        "parses": facts.parses,
        "extracted": facts.extracted,
        "shared": facts.shared,
        "cached": facts.cached,
        "ms": clock::millis(facts.time),
        "cache_read_ms": clock::millis(facts.cache_read),
        "cache_write_ms": clock::millis(facts.cache_write),
    });
    if let (Some(fields), Some(states)) = (out.as_object_mut(), records.states) {
        fields.insert("states".into(), states.into());
    }
    out
}

/// What one name-resolving gate's evidence cost, each tree apart. Spec 11.2.
fn name_evidence(
    cost: &crate::syntax::structural::NameCost,
    layout: Option<base::Layout>,
) -> Value {
    let mut out = Map::new();
    out.insert(
        "layout".into(),
        layout.map_or(Value::Null, |layout| {
            json!({
                "written": layout.written,
                "worktree_add_ms": clock::millis(layout.worktree_add),
                "changes_ms": clock::millis(layout.changes),
                "renames_ms": clock::millis(layout.renames),
                "cache_name_ms": clock::millis(layout.cache_name),
                "ignored_ms": clock::millis(layout.ignored),
                "walk_ms": clock::millis(layout.walk),
            })
        }),
    );
    out.insert("base_ms".into(), clock::millis(cost.base).into());
    if let Some(lost) = cost.lost {
        out.insert("lost_ms".into(), clock::millis(lost).into());
    }
    for (tree, part) in [("before", &cost.before), ("after", &cost.after)] {
        let part = json!({
            "measure_ms": clock::millis(part.measure),
            "index_ms": clock::millis(part.index),
            "query_ms": clock::millis(part.query),
            "files": part.files,
            "declarations": part.declarations,
            "references": part.references,
            "distinct_names": part.distinct_names,
        });
        out.insert(tree.into(), part);
    }
    Value::Object(out)
}

/// What the facts one gate held cost, and what one of each structural value costs. Spec 11.2.
fn footprint(held: &crate::syntax::structural::footprint::Footprint) -> Value {
    let mut out = Map::new();
    for (name, value) in held.rows() {
        out.insert(name.into(), value.into());
    }
    let mut sizes = Map::new();
    for (name, value) in crate::syntax::structural::footprint::sizes() {
        sizes.insert(name.into(), value.into());
    }
    out.insert("sizes".into(), Value::Object(sizes));
    out.insert(
        "reference_canonical_allocation_ratio".into(),
        json!(if held.reference_distinct_names == 0 {
            0.0
        } else {
            held.reference_canonical_allocations as f64 / held.reference_distinct_names as f64
        }),
    );
    Value::Object(out)
}

pub fn coverage_json(coverage: &Coverage) -> Value {
    json!({
        "found": coverage.found,
        "measured": coverage.measured,
        "not_measured": coverage.not_measured,
        "excluded": coverage.excluded,
        "unreadable": coverage.unreadable,
    })
}

/// How many findings and notes one gate's row says it left.
pub struct Counts {
    pub findings: usize,
    pub notes: usize,
}

/// One gate's row in the JSON: what it is called, what it came to, how many findings and notes
/// it left, the scope it measured, how long its own measure and judge took, the count its `OK:`
/// line prints as held at the base, and the structural facts it extracted or shared. Spec 11.2.
pub fn row(name: &str, status: &str, counts: Counts, records: &Records, ms: u64) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), name.into());
    out.insert("status".into(), status.into());
    out.insert("findings".into(), counts.findings.into());
    out.insert("notes".into(), counts.notes.into());
    out.insert(
        "coverage".into(),
        records.coverage.as_ref().map_or(Value::Null, coverage_json),
    );
    out.insert("ms".into(), ms.into());
    out.insert("held".into(), records.held.map_or(Value::Null, Value::from));
    out.insert(
        "accepted".into(),
        records.accepted.map_or(Value::Null, Value::from),
    );
    out.insert("facts".into(), facts(records));
    out.insert(
        "names".into(),
        records
            .names
            .as_ref()
            .map_or(Value::Null, |names| name_evidence(names, records.layout)),
    );
    out.insert(
        "footprint".into(),
        records.footprint.as_ref().map_or(Value::Null, footprint),
    );
    costs(&mut out, records);
    Value::Object(out)
}

/// The content, module-graph and public-surface work of one gate's row. Spec 11.2.
fn costs(out: &mut Map<String, Value>, records: &Records) {
    out.insert(
        "work".into(),
        records.work.map_or(Value::Null, |work| {
            json!({
                "reads": work.reads,
                "parses": work.parses,
            })
        }),
    );
    out.insert(
        "graph".into(),
        records.graph.map_or(Value::Null, |graph| {
            json!({
                "modules": graph.modules,
                "sources": graph.sources,
                "dependencies": graph.dependencies,
                "edges": graph.edges,
                "dispatches": by_language(graph.dispatched()),
                "ms": clock::millis(graph.time),
            })
        }),
    );
    out.insert(
        "surface".into(),
        records.surface.map_or(Value::Null, |surface| {
            json!({
                "surfaces": surface.surfaces,
                "items": surface.items,
                "measured": surface.measured,
                "opaque": surface.opaque,
                "holes": surface.holes,
                "dispatches": by_language(surface.dispatched()),
                "ms": clock::millis(surface.time),
            })
        }),
    );
}

/// One count per structural language, under the name a config names the language by.
fn by_language(counts: impl Iterator<Item = (LanguageId, usize)>) -> Value {
    let names = structural::languages();
    counts
        .filter_map(|(language, count)| {
            let (name, _) = names.iter().find(|(_, id)| *id == language)?;
            Some((name.to_string(), Value::from(count)))
        })
        .collect::<Map<String, Value>>()
        .into()
}
