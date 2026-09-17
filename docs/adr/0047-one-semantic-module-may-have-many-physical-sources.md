# One semantic module may have many physical sources

Issue #219 extends ADR 0043 without replacing its Rust/TypeScript V1 history.
ADR 0043 was correct for the shipped languages: Rust file/inline modules and
TypeScript file modules fit a graph where one semantic node names one physical
file. Research for a third structural language found one concrete shared-model
pressure: Go and Swift both have useful semantic dependency nodes backed by
several source files.

The extension stays narrow. It does not introduce a universal package,
workspace, project, assembly, target or build graph.

## The decision

### `FileFacts` stays physical

`syntax::structural::FileFacts` remains the semantic facts of exactly one
physical source file. Structural extraction, the base cache and `SourceIndex`
keep that unit. A resolver may group several already-extracted `FileFacts` into
one module, but no package-sized fact object or second syntax cache is added.

### `Module` is semantic resolver output with one or more sources

A `Module` may own one or many repository-relative physical sources.

Source membership is:

- non-empty;
- deterministic;
- duplicate-free;
- unordered semantically.

A physical file may participate in more than one module identity where a
resolver proves that topology, as Rust already does when one source is reached
through more than one target context.

A module's semantic identity belongs to its resolver and is never reconstructed
from whichever source sorts first. Current Rust and TypeScript identity and
report behavior remain compatibility constraints.

A language concept with no tracked physical source does not force a synthetic
zero-source module into the graph. Such a relationship remains external,
unsupported or a visible hole until concrete evidence proves a shared
source-less node is required.

### Dependencies keep their physical source site

A resolved dependency records:

- semantic source module;
- semantic destination module;
- exact physical source file that wrote it;
- source line.

The physical representation may use a compact local source reference rather
than another owned path string, but it must resolve unambiguously to the exact
repository-relative path.

A lookup by only `(module, line)` is therefore insufficient. A source-aware
lookup includes physical source identity because two files of one module may
write dependencies on the same line number.

### Physical dependency sites and semantic graph edges are separate

Several source sites may resolve to the same semantic `(from, to)` relation.
Every site remains available for policy and reporting. SCC/cycle construction
uses the unique eligible semantic endpoint pairs, because duplicate physical
sites add no topology information.

Cycles still run over semantic modules, never physical files.

### Layering keeps path policy physical and classifies destinations
conservatively

The source layer of a dependency is the layer of the exact physical source file
that wrote it.

A destination module is folded across all of its sources. For top-level scope
it is `Inside` only when every source is selected, `Outside` when none are, and
`Mixed` otherwise. For layer membership it is `Layer(L)` only when every
source belongs to the same layer, `Unlayered` when none belongs to a layer, and
`Mixed` otherwise.

`Mixed` is measurement ambiguity. Klin does not choose the first source and it
does not silently turn the ambiguity into PASS or FAIL.

Cycle scope is provenance-aware: an eligible dependency site must be selected
by path policy and its destination module must classify inside the scope. The
SCC input is then deduplicated to semantic endpoint pairs.

Classification is precomputed per physical source and folded once per module.
Dependency judgement must not rescan every destination source for every edge.

### Semantic edge identity and physical findings stay separate

Moving equivalent dependency evidence between files of the same multi-source
module is not new debt when the semantic relationship and verdict-defining
policy classification stay unchanged. Moving it to a source with a different
layer/scope classification is a semantic policy change and may be new debt.

`layering` should pair semantic edges before it turns the current evidence into
ordinary physical `Finding`s. Generic `ratchet.rs` is not redesigned unless a
concrete characterization case proves this gate-local bridge insufficient.

Current one-source Rust/TypeScript finding identity and rename behavior remain
unchanged.

### Resolver and surface dispatch become capability-aware

`Topology` already receives measured `FileFacts`. The graph derives the set of
present structural `LanguageId`s from those facts and a static resolver
registry dispatches only capabilities that are present. Surface derivation uses
the same rule.

Adding future adapters therefore costs only a cheap scan of registered
capabilities when their language is absent. It causes no extra repository walk,
source read, parse, extraction, resolver file scan or surface module scan.

This remains a static Rust registry. No plugin or dependency-injection system
is introduced.

## Performance contract

The semantic decision does not require a particular container representation.
#220 starts with the simplest implementation that satisfies the contract and
#221 measures it on the existing Rust/TypeScript x10 fixture.

Required work shape:

```text
source grouping                  O(source memberships) or near-linear
module policy classification     O(source memberships + policy lookup)
dependency judgement             O(dependency sites)
SCC input                        O(unique eligible semantic edges)
absent capability dispatch       O(registered capabilities)
```

A local one/many source representation or compact dependency source ordinal is
allowed if measurement proves it removes a real common-case allocation/RSS
cost. This decision does not reopen repository-wide `FileId`, path interning,
persistent graph/SCC caching, parallel resolution or a resident process.

## Language probes

The eleven-language adversarial pass found no need for a broader shared graph:

- Rust and TypeScript fit the existing one-source subset.
- Go and Swift validate one semantic node backed by many physical sources.
- Python namespace packages show why virtual/source-less relationships should
  remain holes rather than forcing zero-source shared nodes.
- Java, C#, Kotlin, C and general C++ show that useful module/project semantics
  can depend on host/build/compiler context; Klin should under-resolve rather
  than guess a universal build model.
- PHP/Composer can be handled by resolver-local explicit metadata.
- C++ named modules themselves fit the multi-source contract, while ordinary
  translation-unit/header relationships remain build-context-sensitive.

The detailed evidence and official-language references live in
`docs/multi-source-modulegraph-research-2026-09-17.md`.

## Downstream consequences

- #220 implements the graph/provenance/layering/dispatch delta only.
- #221 admits the generalized representation on the existing performance seam.
- #222 Go can group many files into one package node without changing shared
  graph semantics.
- #223 Python can remain conservative around namespace/virtual packages.
- #48 duplication, #53 coverage reader and #54/#70 changed-coverage/postflight
  require no specification changes from this decision.
- `dead-symbols` and `reachability` remain `Measurement`/`SourceIndex`
  consumers and do not acquire ModuleGraph semantics.

## Rejected alternatives

No evidence from #219 justifies:

- `SemanticUnit` between `FileFacts` and `Module`;
- `PackageGraph` or `WorkspaceGraph`;
- a universal build target/project/assembly abstraction;
- zero-source virtual graph nodes;
- repository-wide `FileId` conversion;
- persisted ModuleGraph/SCC state;
- dynamic resolver plugins/DI;
- compiler/type-checker integration;
- guessed build-dependent relationships from directory shape alone.

Those require a separate concrete fixture and consumer that the frozen contract
cannot represent safely.
