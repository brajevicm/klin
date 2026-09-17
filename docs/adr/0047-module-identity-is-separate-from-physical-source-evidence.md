# Module identity is separate from physical source evidence

Issue #219 revisits one assumption in ADR 0043 before Klin adds a structural
language whose dependency node can be backed by several files.

ADR 0043 remains the decision that `modules::ModuleGraph` is the shared layer
above per-file structural facts, that containment is not a dependency edge,
that resolution must prove relationships rather than guess them, and that
petgraph types/indices are not product identity.

This ADR amends only ADR 0043's accidental one-file representation.

## Context

The shipped Rust/TypeScript graph stores:

```text
Module.file
Dependency { from, to, line }
```

That shape works for V1 because a TypeScript module is one file and the Rust
resolver can represent each target/module identity with one backing file, even
when one physical file participates in several target-scoped module identities.

It does not generalize to ecosystems such as Go, where one package is
constructed from one or more source files and an import is written by one
particular source file, or SwiftPM, where one target contains a set of source
files compiled into one module.

The research also challenged the graph against Python, Java, C#, Kotlin, PHP,
C and C++. Those probes do not justify a broader shared build/package model:
where build/compiler/host context is required, Klin should conservatively leave
a relationship unresolved rather than invent topology.

## Decision

### `FileFacts` stays physical

`syntax::structural::FileFacts` remains the facts of exactly one physical
repository source file. Structural extraction/cache ownership does not move up
to package/module scope.

### A `Module` has one or more physical sources

One semantic module may be backed by one or many repository-relative source
paths.

Its source membership is:

- non-empty;
- duplicate-free;
- deterministic for presentation/testing;
- not semantic identity.

A physical file may still participate in several semantic module identities
where a resolver proves that topology.

The shared graph does not materialize zero-source/virtual modules. A language
concept with no tracked physical source remains resolver-local external,
unsupported or a visible measurement hole.

### Module identity is resolver-owned

A semantic module's identity is supplied/proven by its resolver and is never
derived generically by choosing the first physical source.

Existing Rust/TypeScript behavior is the compatibility constraint: #220 need
not redesign their public identities merely because the shared representation
becomes one-or-many. Future multi-source resolvers can provide identities that
survive harmless membership changes when their language/build evidence proves
the semantic node is unchanged.

Petgraph allocation indices, source collection positions and source ordering
remain non-semantic.

### A dependency carries physical provenance

A resolved dependency site contains:

```text
from semantic module
to semantic module
exact physical source that wrote it
line
```

The physical representation may use a local source ordinal/reference instead
of cloning an owned path string per dependency, provided it resolves
unambiguously to the exact repository path.

Consequently, `(from module, line)` is no longer a sufficient site lookup.
Dependency-site APIs must include physical source identity or an equivalent
site value.

### Physical dependency sites are not SCC edges

Several physical sites may prove the same semantic `(from, to)` relationship.
All sites remain available for evidence, layering and findings, while SCC
construction uses unique eligible semantic endpoint pairs.

Cycle algorithms continue to operate on semantic module nodes, never physical
source-file nodes.

### Layering keeps path policy and classifies multi-source destinations conservatively

A dependency's source path decides its source scope/layer.

A destination module is classified once from all of its sources:

- all sources in the same configured layer -> that layer;
- all sources unlayered -> unlayered;
- more than one layer, or layered plus unlayered -> ambiguous.

Top-level `in`/`except` scope is likewise folded over all destination sources:
all selected, none selected, or mixed.

Ambiguous/mixed destinations do not receive a guessed verdict. They are visible
measurement holes under the existing conservative philosophy.

Path/layer classification is precomputed per source/module. Dependency
judgement must not repeatedly rescan every destination source.

Cycle scope is provenance-aware: eligible physical dependency sites are chosen
first, their semantic endpoint pairs are deduplicated, and SCCs run over those
semantic edges. Whole-node selection by an arbitrary representative source is
not valid for a multi-source module.

### Layering pairs semantic edges before physical findings

Moving equivalent dependency evidence between physical files of the same
semantic module and the same layer does not create new architecture debt.
Retargeting to another semantic module, changing the policy layer transition,
or splitting/merging semantic nodes remains observable.

`layering` should therefore pair base/current relationships under a semantic
edge key before emitting the ordinary physical-site `Finding` used for current
reporting.

This is a gate-local bridge. Generic ratchet identity is not redesigned unless
a concrete implementation characterization proves that necessary.

### Resolver and surface dispatch is capability-aware

Resolver and surface registration stays static Rust code.

Applicability is decided from structural languages present in already-owned
`FileFacts`, so adding future adapters does not make each absent adapter scan
the repository.

An absent registered capability may cost O(number of registered capabilities)
to skip, but performs no repository-sized file/module work and no new source
read/parse/extraction.

## Complexity contract

The architecture should admit implementations with this work shape:

```text
source grouping/classification   O(source memberships) or near-linear
module policy fold               O(source memberships)
dependency judgement             O(dependency sites)
SCC input                         O(unique eligible semantic edges)
SCC                               O(modules + unique eligible edges)
absent-language dispatch          O(registered capabilities)
```

An implementation shaped as dependency-sites × destination-sources is rejected.

Representation remains measurement-driven. Begin with the simplest owned
source collection; #221 decides from A/B evidence whether one-source
Rust/TypeScript needs a one/many optimization or compact dependency provenance.
This decision does not authorize repository-wide `FileId`, interning, persistent
graph state or another performance architecture.

## Language-probe result

The eleven probes produced no additional shared gap:

- Rust, TypeScript: current one-source subset remains valid.
- Go, Swift: directly fit the multi-source semantic-node contract.
- Python, PHP: fit with resolver-local conservative behavior; Python namespace
  packages specifically argue against source-less synthetic graph nodes.
- Java, C#, Kotlin, C and general C++: useful complete topology depends on
  build/compiler/host evidence. Under-resolve rather than generalize the shared
  graph from guessed paths. C++ named modules themselves fit the multi-source
  shape but do not make general C++ build-independent.

The dated research note `docs/multi-source-modulegraph-research-2026-09-17.md`
records fixtures, source references and the downstream review.

## Consequences

- #220 can remain a narrow extension of the landed #49/#50/#46 boundaries.
- #221 measures representation/runtime cost without reopening SPEC performance
  budgets or the #197-#205 architecture round.
- `dead-symbols` and `reachability` remain `Measurement`/`SourceIndex`
  consumers; multi-source graph semantics do not leak into them.
- #48 duplication remains a per-file structural/fingerprint concern.
- #53 coverage reading and #54/#70 changed-coverage/postflight remain
  path/range/external-report concerns and gain no module dependency.
- New language resolvers own their build/package-specific evidence and expose
  holes when that evidence is insufficient.

## Rejected alternatives

- replace `ModuleGraph` with a universal package/workspace/build graph;
- add `SemanticUnit` between physical facts and modules;
- model virtual/namespace concepts as zero-source modules;
- make physical files the SCC graph to preserve provenance;
- derive semantic module identity from `sources[0]`;
- repeatedly classify destination sources per dependency;
- run every future resolver/surface adapter over every repository;
- add repository-wide file IDs, path interning or persistent graph/SCC caches
  without measured evidence.