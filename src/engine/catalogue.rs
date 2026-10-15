//! The `CATALOGUE` is the one table of the checks klin has. The runner takes its gates from it,
//! `config` takes the section names it accepts from it, and `reference` prints it. A new check
//! is one row here, which `klin check` selects by its name. The catalogue names every check, and
//! no check names the catalogue. ADR 0036.

use crate::config::key::{Key, Languages, Section, SectionShape};
use crate::contract::check::{
    Activation, Count, Debt, Derivation, Judges, Needs, Placement, Run, Unit,
};
use crate::contract::project::Project;
use crate::contract::ratchet::Accepts;
use crate::{
    checks::{
        complexity, conventions, doc_citations, doc_size, escapes, inventory, lockfile, sarif,
        secrets,
        structural::{dead_symbols, layering, public_api, reachability},
        stubs,
    },
    syntax,
};

/// The name of the `public-api` gate, for a reader of the journal that may not depend on the
/// gate itself.
pub const PUBLIC_API: &str = public_api::NAME;

/// The words a report gives one check's findings, in the person's language. `klin report` reads
/// these so the report keeps no second gate-name vocabulary of its own, and a new row does not
/// compile until it supplies them. They are presentation only: they change no gate identity, no
/// section, no journal record, no accepted entry and no judgement. Spec 11.5.
pub struct Labels {
    pub one: &'static str,
    pub many: &'static str,
}

impl Labels {
    pub fn count(&self, many: usize) -> &'static str {
        match many {
            1 => self.one,
            _ => self.many,
        }
    }
}

/// The words one gate's findings print under, and `None` for a gate this binary holds no row
/// for: a journal line naming a gate klin no longer has, or a `sarif` entry under the name a
/// person gave it, is read under its recorded name. Spec 11.5.
pub fn labels(gate: &str) -> Option<&'static Labels> {
    CATALOGUE
        .iter()
        .find(|row| row.name == gate)
        .map(|row| &row.labels)
}

/// One row of the catalogue: one check, as the runner, the configuration, the reference and
/// the plan all read it.
pub struct Row {
    /// What `klin check` calls this check, which for two checks is not the name of the section
    /// they read.
    pub name: &'static str,
    pub section: &'static str,
    /// What the section's absence means: derive it, or run nothing. Spec 4.6.
    pub activation: Activation,
    /// The paths the engine runs this check at. A check placed at `klin check` alone adds no
    /// work to the Stop. Spec 6.2.
    pub placement: Placement,
    /// Rises when the same inputs can produce a different measurement or judgement. Spec 8.2.
    pub semantics: u32,
    /// The configuration keys the section reads, declared in the check's own module and printed
    /// by `klin policy --reference`. Spec 5.8.
    pub keys: &'static [Key],
    pub reference_text: Option<&'static str>,
    /// The built-in language coverage this check reports, and none for a check that reads no
    /// programming language. This is capability, never configurable source topology. Spec 5.8.
    pub languages: Option<Languages>,
    /// Whether the tree holds what an Automatic check applies to when its section is absent,
    /// answered from facts alone and never from a derived number. Spec 4.6, ADR 0040.
    pub available: fn(&Project) -> bool,
    pub run: Run,
    /// What `klin policy` prints for this check without a run. Spec 11.6.
    pub derivation: Derivation,
    /// What the check judges in this tree, which `klin policy` counts without a run. Spec 11.6.
    pub judges: Judges,
    pub needs: Needs,
    pub takes_scope: bool,
    /// The values an accepted entry for this check gives a number for, which `klin policy`
    /// judges each entry by without a run. Spec B.4.8.
    pub accepts: Accepts,
    /// Whether the section is a list of entries a person writes, each its own gate under its
    /// own `name`, rather than one section the whole check runs under. Spec 8.3.
    pub gate_per_entry: bool,
    pub shape: SectionShape,
    /// What a report calls this check's findings when it writes for a person. Spec 11.5.
    pub labels: Labels,
}

pub const CATALOGUE: &[Row] = &[
    Row {
        name: "doc-size",
        section: doc_size::SECTION,
        activation: Activation::Automatic,
        placement: Placement::StopAndCheck,
        semantics: 1,
        keys: doc_size::KEYS,
        reference_text: None,
        languages: None,
        available: doc_size::applies,
        run: doc_size::gate,
        derivation: Derivation::Values(doc_size::derive),
        judges: Judges {
            unit: Unit::Documents,
            count: Some(|project, _| doc_size::judges(project).map(Count::Selected)),
            absent: "the tree holds no agent instruction file",
            debt: Debt::Held,
        },
        needs: Needs::Nothing,
        takes_scope: false,
        labels: Labels {
            one: "long document",
            many: "long documents",
        },
        accepts: Accepts::all(&[]),
        gate_per_entry: false,
        shape: SectionShape::DocumentMap(&doc_size::DOCUMENT),
    },
    Row {
        name: "doc-citations",
        section: doc_citations::SECTION,
        activation: Activation::Automatic,
        placement: Placement::StopAndCheck,
        semantics: 1,
        keys: doc_citations::KEYS,
        reference_text: None,
        languages: None,
        available: |project| !project.facts().found.documents.is_empty(),
        run: doc_citations::gate,
        derivation: Derivation::Values(doc_citations::derive),
        judges: Judges {
            unit: Unit::Documents,
            count: Some(|project, _| doc_citations::judges(project).map(Count::Selected)),
            absent: "the tree holds no document",
            debt: Debt::Held,
        },
        needs: Needs::TheTree,
        takes_scope: false,
        labels: Labels {
            one: "broken citation",
            many: "broken citations",
        },
        accepts: doc_citations::ACCEPTS,
        gate_per_entry: false,
        shape: SectionShape::FalseOnly(doc_citations::POLICY),
    },
    Row {
        name: "lockfile",
        section: lockfile::SECTION,
        activation: Activation::Automatic,
        placement: Placement::StopAndCheck,
        semantics: 1,
        keys: lockfile::KEYS,
        reference_text: None,
        languages: None,
        available: lockfile::applies,
        run: lockfile::gate,
        derivation: Derivation::Values(lockfile::derive),
        judges: Judges {
            unit: Unit::Manifests,
            count: Some(|project, _| lockfile::judges(project).map(Count::Selected)),
            absent: "the tree holds no manifest klin reads a lockfile for",
            debt: Debt::Held,
        },
        needs: Needs::TheTree,
        takes_scope: false,
        labels: Labels {
            one: "unlocked dependency",
            many: "unlocked dependencies",
        },
        accepts: lockfile::ACCEPTS,
        gate_per_entry: false,
        shape: SectionShape::Object,
    },
    Row {
        name: "escapes",
        section: escapes::SECTION,
        activation: Activation::Automatic,
        placement: Placement::StopAndCheck,
        semantics: 1,
        keys: escapes::KIND.keys,
        reference_text: None,
        languages: Some(escapes::language_extensions),
        available: |project| !project.found_no_source_root(),
        run: escapes::gate,
        derivation: Derivation::Nothing,
        judges: Judges {
            unit: Unit::Files,
            count: Some(|project, _| escapes::judges(project).map(Count::Selected)),
            absent: "the tree holds no source root",
            debt: Debt::Held,
        },
        needs: Needs::TheTree,
        takes_scope: true,
        labels: Labels {
            one: "escape hatch",
            many: "escape hatches",
        },
        accepts: escapes::KIND.accepts(),
        gate_per_entry: false,
        shape: SectionShape::Object,
    },
    Row {
        name: "stubs",
        section: stubs::SECTION,
        activation: Activation::Automatic,
        placement: Placement::StopAndCheck,
        semantics: 1,
        keys: stubs::KIND.keys,
        reference_text: None,
        languages: Some(stubs::language_extensions),
        available: |project| !project.found_no_source_root(),
        run: stubs::gate,
        derivation: Derivation::Nothing,
        judges: Judges {
            unit: Unit::Files,
            count: Some(|project, _| stubs::judges(project).map(Count::Selected)),
            absent: "the tree holds no source root",
            debt: Debt::Held,
        },
        needs: Needs::TheTree,
        takes_scope: true,
        labels: Labels {
            one: "stub",
            many: "stubs",
        },
        accepts: stubs::KIND.accepts(),
        gate_per_entry: false,
        shape: SectionShape::Object,
    },
    Row {
        name: "secrets",
        section: secrets::SECTION,
        activation: Activation::Automatic,
        placement: Placement::StopAndCheck,
        semantics: 1,
        keys: secrets::KEYS,
        reference_text: None,
        languages: Some(secrets::language_extensions),
        available: secrets::applies,
        run: secrets::gate,
        derivation: Derivation::Nothing,
        judges: Judges {
            unit: Unit::Files,
            count: Some(|project, _| secrets::judges(project).map(Count::Selected)),
            absent: "the tree holds no source root and no .env file",
            debt: Debt::Held,
        },
        needs: Needs::TheTree,
        takes_scope: true,
        labels: Labels {
            one: "hardcoded secret",
            many: "hardcoded secrets",
        },
        accepts: secrets::ACCEPTS,
        gate_per_entry: false,
        shape: SectionShape::Object,
    },
    Row {
        name: "inventory",
        section: inventory::SECTION,
        activation: Activation::Automatic,
        placement: Placement::StopAndCheck,
        semantics: 1,
        keys: inventory::KEYS,
        reference_text: None,
        languages: None,
        available: inventory::applies,
        run: inventory::gate,
        derivation: Derivation::Values(inventory::derive),
        judges: Judges {
            unit: Unit::TestFiles,
            count: Some(|project, _| inventory::judges(project).map(Count::Selected)),
            absent: "the tree holds no test root and no file its test convention marks",
            debt: Debt::Accepted,
        },
        needs: Needs::TheTree,
        takes_scope: true,
        labels: Labels {
            one: "missing test",
            many: "missing tests",
        },
        accepts: inventory::ACCEPTS,
        gate_per_entry: false,
        shape: SectionShape::Object,
    },
    Row {
        name: "complexity",
        section: complexity::SECTION,
        activation: Activation::Automatic,
        placement: Placement::StopAndCheck,
        semantics: 1,
        keys: complexity::KEYS,
        reference_text: None,
        languages: Some(syntax::language_extensions),
        available: |project| !project.found_no_source_root(),
        run: complexity::gate,
        derivation: Derivation::Values(complexity::derive),
        judges: Judges {
            unit: Unit::Files,
            count: Some(|project, _| complexity::judges(project).map(Count::Selected)),
            absent: "the tree holds no source root",
            debt: Debt::Held,
        },
        needs: Needs::TheTree,
        takes_scope: true,
        labels: Labels {
            one: "tangled function",
            many: "tangled functions",
        },
        accepts: complexity::ACCEPTS,
        gate_per_entry: false,
        shape: SectionShape::Object,
    },
    Row {
        name: "dead-symbols",
        section: dead_symbols::SECTION,
        activation: Activation::Automatic,
        placement: Placement::StopAndCheck,
        semantics: 1,
        keys: dead_symbols::KEYS,
        reference_text: None,
        languages: Some(dead_symbols::language_extensions),
        available: |project| !project.found_no_source_root(),
        run: dead_symbols::gate,
        derivation: Derivation::Nothing,
        judges: Judges {
            unit: Unit::Files,
            count: Some(|project, _| dead_symbols::judges(project).map(Count::Selected)),
            absent: "the tree holds no source root",
            debt: Debt::Held,
        },
        needs: Needs::TheTree,
        takes_scope: true,
        labels: Labels {
            one: "dead symbol",
            many: "dead symbols",
        },
        accepts: dead_symbols::ACCEPTS,
        gate_per_entry: false,
        shape: SectionShape::Object,
    },
    Row {
        name: "reachability",
        section: reachability::SECTION,
        activation: Activation::Automatic,
        placement: Placement::StopAndCheck,
        semantics: 1,
        keys: reachability::KEYS,
        reference_text: None,
        languages: Some(reachability::language_extensions),
        available: |project| !project.found_no_source_root(),
        run: reachability::gate,
        derivation: Derivation::Values(reachability::derive),
        judges: Judges {
            unit: Unit::Files,
            count: Some(|project, _| reachability::judges(project).map(Count::Selected)),
            absent: "the tree holds no source root",
            debt: Debt::Held,
        },
        needs: Needs::TheTree,
        takes_scope: false,
        labels: Labels {
            one: "unreferenced file",
            many: "unreferenced files",
        },
        accepts: reachability::ACCEPTS,
        gate_per_entry: false,
        shape: SectionShape::Object,
    },
    Row {
        name: "layering",
        section: layering::SECTION,
        activation: Activation::Policy,
        placement: Placement::StopAndCheck,
        semantics: 1,
        keys: layering::KEYS,
        reference_text: None,
        languages: Some(layering::language_extensions),
        available: |_| false,
        run: layering::gate,
        derivation: Derivation::Nothing,
        judges: Judges {
            unit: Unit::Files,
            count: Some(|project, _| layering::judges(project).map(Count::Selected)),
            absent: "it runs only under a section a person writes",
            debt: Debt::Held,
        },
        needs: Needs::TheCommit,
        takes_scope: false,
        labels: Labels {
            one: "layering breach",
            many: "layering breaches",
        },
        accepts: layering::ACCEPTS,
        gate_per_entry: false,
        shape: SectionShape::Asks(layering::ASKS),
    },
    Row {
        name: public_api::NAME,
        section: public_api::SECTION,
        activation: Activation::Automatic,
        placement: Placement::StopAndCheck,
        semantics: 1,
        keys: public_api::KEYS,
        reference_text: Some(public_api::REFERENCE_TEXT),
        languages: Some(public_api::language_extensions),
        available: |project| !project.found_no_source_root(),
        run: public_api::gate,
        derivation: Derivation::ExplainedWhenNamed(public_api::explain),
        judges: Judges {
            unit: Unit::Files,
            count: Some(|project, _| public_api::judges(project).map(Count::Selected)),
            absent: "the tree holds no source root",
            debt: Debt::Held,
        },
        needs: Needs::TheCommit,
        takes_scope: false,
        labels: Labels {
            one: "broken public contract",
            many: "broken public contracts",
        },
        accepts: public_api::ACCEPTS,
        gate_per_entry: false,
        shape: SectionShape::FalseOnly(public_api::POLICY),
    },
    Row {
        name: "conventions",
        section: conventions::rules::SECTION,
        activation: Activation::Policy,
        placement: Placement::StopAndCheck,
        semantics: 1,
        keys: conventions::rules::KEYS,
        reference_text: None,
        languages: Some(syntax::pattern::language_extensions),
        available: |_| false,
        run: conventions::gate,
        derivation: Derivation::Explained(conventions::report::explain),
        judges: Judges {
            unit: Unit::Files,
            count: Some(conventions::judges),
            absent: "it runs only under a section a person writes",
            debt: Debt::Held,
        },
        needs: Needs::TheTree,
        takes_scope: true,
        labels: Labels {
            one: "convention breach",
            many: "convention breaches",
        },
        accepts: conventions::ACCEPTS,
        gate_per_entry: false,
        shape: SectionShape::Conventions(conventions::rules::read),
    },
    Row {
        name: "sarif",
        section: sarif::SECTION,
        activation: Activation::Integration,
        placement: Placement::Check,
        semantics: 1,
        keys: sarif::KEYS,
        reference_text: None,
        languages: None,
        available: |_| false,
        run: sarif::gate,
        derivation: Derivation::Nothing,
        judges: Judges {
            unit: Unit::Files,
            count: None,
            absent: "it runs only under a section a person writes",
            debt: Debt::Held,
        },
        needs: Needs::TheCommit,
        takes_scope: false,
        labels: Labels {
            one: "scanner finding",
            many: "scanner findings",
        },
        accepts: sarif::ACCEPTS,
        gate_per_entry: true,
        shape: SectionShape::Sarif,
    },
];

/// The section each check reads, as the data `config` judges a configuration's shape by.
pub fn sections() -> Vec<Section> {
    CATALOGUE
        .iter()
        .map(|check| Section {
            command: check.name,
            name: check.section,
            automatic: check.activation == Activation::Automatic,
            keys: check.keys,
            shape: check.shape,
        })
        .collect()
}

/// Every check by name, for the errors that list what a person may write.
pub fn names() -> impl Iterator<Item = &'static str> {
    CATALOGUE.iter().map(|check| check.name)
}

impl Row {
    /// What the check document calls this row's capability. Spec 4.2, 11.7.
    pub fn kind(&self) -> &'static str {
        match self.activation {
            Activation::Integration => "integration",
            Activation::Automatic | Activation::Policy => "check",
        }
    }

    /// Whether this check measures code, which is what a tree with no source root leaves it
    /// nothing to measure. Spec 10, 14.
    pub fn reads_code(&self) -> bool {
        self.activation == Activation::Automatic && self.languages.is_some()
    }
}
