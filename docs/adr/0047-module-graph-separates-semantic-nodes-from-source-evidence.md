# ModuleGraph separates semantic nodes from source evidence

> Extends ADR 0043. ADR 0043 remains the Rust/TypeScript V1 history; this
> decision removes only its accidental one-semantic-module/one-physical-file
> assumption before structural language #3.

## Context

ADR 0043 established one Klin-owned `ModuleGraph` above per-file structural
facts and below `layering`, cycles and public-surface derivation. That boundary
is still correct.

Its first resolvers could represent every semantic node with one `file`:

- a Rust module is file-backed or inline inside one file, with one source file
  potentially participating in several target-scoped module identities;
- a TypeScript module is one TypeScript/TSX file.

A Go package and a SwiftPM target falsify the generality of that storage shape.
One semantic dependency node legitimately owns several physical files, while a
dependency is still written at one concrete source location.

The shared graph therefore needs to separate semantic identity from physical
evidence without moving package/build semantics into `syntax::structural` or
creating a universal build model.

## Decision

### Structural facts stay per physical file

`FileFacts` remains one physical repository source file. Structural extraction,
its cache and `SourceIndex` keep that unit.

Resolvers compose those facts into semantic dependency nodes. They do not
merge several files into a package-sized fact/cache object.

### A Module is resolver-owned semantic identity with non-empty sources

One `Module` is one dependency identity proved by a language resolver and owns
one or more physical repository-relative sources.

Its source membership is:

- non-empty;
- deterministic;
- duplicate-free;
- unordered semantically.

The resolver owns module identity. No consumer reconstructs it from the first
source path.

A physical source may participate in more than one semantic module where the
resolver proves different contexts, preserving Rust target-scoped behavior.

The shared graph has no zero-source synthetic modules. A namespace/package or
runtime concept with no provable tracked physical source remains external,
unsupported or a hole.

### A dependency records semantic endpoints and exact physical provenance

A resolved dependency carries:

```text
from semantic module
to semantic module
physical source path that wrote it
source line
```

The source belongs to the `from` module's source membership.

The physical representation may use a local ordinal/reference rather than an
owned path copy per edge, provided it resolves deterministically to the exact
repository-relative path.

A dependency-site lookup is source-aware. `(module, line)` alone is not an
identity once a semantic module owns several files.

### Physical dependency sites and semantic topology edges are distinct

Every physical site remains available for layering, findings and explanation.
For topology/SCC work, equivalent eligible `(from, to)` endpoint pairs are one
semantic edge.

Cycle construction deduplicates those endpoint pairs before SCC computation;
it does not turn multiple physical import sites into duplicate topology work.

### Layering classifies sources once and folds destinations conservatively

The source side of a dependency uses the physical source file that wrote it.

A destination module's path policy is folded across all its sources:

- scope is `Inside` when every source is selected, `Outside` when none is, and
  `Mixed` otherwise;
- layer is `Layer(L)` when every source belongs to the same configured layer,
  `Unlayered` when none belongs to any configured layer, and `Mixed` otherwise.

`Mixed` is an ambiguity in the measurement. It is never resolved by choosing
one source according to iteration/sort order. A blocking forbidden-edge
verdict is emitted only from unambiguous policy evidence.

The implementation classifies relevant physical paths once, folds each module
once and judges dependency sites by lookup. It does not rescan every
destination source for every dependency.

### Acyclic scope applies to dependency evidence, SCCs remain semantic

`acyclic` no longer selects whole nodes through one module path.

A physical dependency site first has to satisfy source scope; its destination
must be unambiguously inside the scope. Eligible sites then become unique
semantic endpoint pairs and SCCs run over semantic module nodes.

Thus an out-of-scope file of a multi-source package cannot leak its import into
the cycle graph, and one unrelated out-of-scope package file does not erase an
otherwise valid in-scope dependency site.

A mixed destination remains coverage-visible rather than guessed into or out
of the graph.

### Semantic continuity is established before physical Findings

Moving dependency evidence between physical files of the same semantic module
is not automatically new architecture debt.

If the resolver identity, target module and effective path/layer meaning stay
the same, the dependency is semantically held and only its reporting site
moved. If the move changes target identity, semantic node identity, layer,
scope, split/merge topology or ambiguity, the change remains observable.

`layering` performs the narrow semantic base/current pairing it needs before
materializing ordinary physical `Finding`s. The generic ratchet is not given a
new universal semantic-identity mechanism unless a concrete implementation
fixture proves this consumer-local bridge insufficient.

Existing Rust/TypeScript finding characterization is a compatibility
constraint.

### Capability dispatch uses topology path-language presence

The resolver/surface registries stay static Rust tables, but an adapter is run
only when its logical language is present.

Presence is collected during the existing `Topology` file-list construction,
not by making each adapter walk the repository and not only from successful
`FileFacts`.

Path presence matters because source can be present while structural parsing
failed. Current target/module/surface/hole behavior must not change merely
because every source of one language is unparsed.

Successful `FileFacts` remain the semantic evidence used by resolvers; path
presence only answers whether an adapter could apply.

An absent registered language costs only the fixed capability-table check and
zero repository-sized resolver/surface work.

## Performance contract

The semantic design admits this work shape:

```text
source membership/grouping       O(source memberships), near-linear
path/layer classification        O(relevant source memberships + policy lookup)
module classification            once per module
dependency judgement             O(dependency sites)
SCC input                        O(unique eligible semantic edges)
absent language dispatch         O(registered capabilities), no repository scan
```

No multi-source change may introduce another structural source read, parse or
fact extraction on the current Rust/TypeScript path.

Representation remains evidence-driven. Start with the smallest clear owned
shape; #221 may replace it with a local one/many source representation or
local provenance reference if controlled measurements isolate a real cost.
This decision does not justify repository-wide `FileId`, path interning,
persistent graph/SCC state, parallel resolution or a resident service.

## Language probes

The design was challenged against Rust, TypeScript, Go, Python, Java, C#,
Swift, Kotlin, PHP, C and C++.

- Rust and TypeScript remain the existing one-source subset.
- Go and Swift independently require/prove useful multi-source semantic nodes.
- Python validates conservative holes rather than zero-source virtual graph
  nodes.
- PHP can remain resolver-local where Composer metadata proves topology.
- Java, C#, Kotlin, C and general C++ demonstrate that build/compiler context
  may be required before a blocking graph is safe; they do not justify a
  speculative shared build/package layer.
- C++ named modules themselves fit the multi-source contract, while the wider
  translation-unit/include graph remains build-context dependent.

No probe requires another shared abstraction between `FileFacts` and
`ModuleGraph`.

## Downstream consequences

- #220 implements this contract and adapts existing Rust/TypeScript graph,
  layering/cycle and surface consumers without adding a language.
- #221 measures/admit the representation on the existing Rust/TypeScript
  300k/1M fixture.
- #222 uses Go as the first real multi-source proof.
- #223 uses Python as the conservative-resolution proof.
- `dead-symbols` and `reachability` remain `Measurement`/`SourceIndex`
  consumers and gain no ModuleGraph dependency.
- #48 duplication, #53 coverage reader and #54/#70 changed-coverage/postflight
  fit this decision unchanged.

## Rejected

This decision does not introduce:

- `SemanticUnit`;
- package/workspace graphs;
- a universal build target model;
- zero-source `Module` nodes;
- repository-wide `FileId` conversion;
- a persistent ModuleGraph/SCC cache;
- dynamic resolver plugins/DI;
- compiler/type-checker integration;
- a generic ratchet identity redesign.

A later ticket may add one only when a concrete language fixture and a Klin
consumer cannot be represented conservatively by this graph.
