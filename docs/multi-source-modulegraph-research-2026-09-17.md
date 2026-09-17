# Multi-source ModuleGraph research — 2026-09-17

Issue: #219

This note freezes the smallest shared graph contract needed before a third structural language is implemented. It is research/design only: no runtime behavior changes here.

## Executive decision

Keep the existing architecture:

```text
physical source file
    -> FileFacts
    -> language resolver
    -> semantic ModuleGraph
    -> layering / cycles
    -> surface / public-api
```

Generalize only the accidental one-to-one assumption between `Module` and a physical file.

The frozen shared model is:

```text
FileFacts
  = one physical source file

Module
  = one semantic resolver node
  = one or more physical repository-relative sources

Dependency site
  = semantic from/to modules
  + exact physical source provenance
  + source line

Semantic graph edge
  = one eligible (from, to) relationship for topology/SCC work
```

The shared graph does **not** gain a universal package/workspace/build-target abstraction. Where useful semantics depend on build/compiler context that Klin cannot prove, the relationship stays external, unsupported or a visible hole.

## Current-code facts that force the change

Current `src/modules/mod.rs` makes `Module.file` structural to more than storage:

- `ModuleGraph::identity()` derives identity from the one file;
- `reached_at(from, line)` assumes `(module, line)` uniquely identifies dependency evidence;
- `cycles(keep)` selects whole semantic nodes through a predicate over the one file;
- dependencies carry `{ from, to, line }` and therefore lose the physical source site once one module can have several sources.

Current `src/layering.rs` then relies on that shape:

- cycle scope selects `module.file`;
- source and destination layer membership use the two modules' files;
- forbidden/cyclic findings key/report the file that writes the dependency;
- selected dependency accounting is file-shaped.

Current surface derivation also assumes one file:

- TypeScript builds a file -> module map from `module.file`;
- TypeScript re-export lookup uses `reached_at(module, line)`;
- Rust/TypeScript surface facts are retrieved through the module's one file.

These are implementation seams that #220 must adapt. They do not justify a second graph layer.

## Frozen shared semantics

### 1. Module source membership

One semantic module owns one or more physical repository-relative source paths.

Required invariants:

```text
sources is non-empty
sources is deterministic
sources is duplicate-free
source order has no semantic meaning
```

A physical source may participate in more than one semantic module identity when a resolver proves that topology. Current Rust target-scoped attachment remains the characterization case.

Do not create source-less graph nodes merely to model a language concept that may be virtual. If Klin cannot attach at least one tracked physical source to a node, that relationship remains external, unsupported or a coverage-visible hole.

The physical representation is not frozen. `Vec<String>` is the simplest starting shape; a one/many representation is allowed only if #221 measures a material common-case benefit.

### 2. Semantic module identity is resolver-owned

A semantic module's identity must not be reconstructed from `sources[0]`.

Each resolver owns the stable identity implied by the language/topology it proves. The generic graph stores/serves that identity without interpreting language semantics.

Compatibility constraints:

- Rust preserves current target/module identity and current report behavior;
- TypeScript preserves today's one-file module identity;
- a future Go resolver may identify a package by its proved module/package boundary and package path rather than by an arbitrary file;
- a future Swift resolver may identify a target/module independently of which source file is first.

The exact Rust type/API remains an implementation choice for #220 so long as these semantics are preserved.

### 3. Dependency source provenance

A resolved dependency site needs:

```text
from semantic module
to semantic module
exact physical source file that wrote the dependency
line
```

The physical source is part of evidence, not necessarily another owned `String`. A compact local source ordinal/reference into the source module is acceptable if it deterministically resolves to the exact repository-relative path.

Any API that identifies a dependency site only as `(module, line)` becomes invalid once a module has several sources. A source-aware lookup must include the physical source identity because two source files in one module may contain dependency evidence on the same line.

### 4. Dependency sites and semantic graph edges are different

Several physical sites may express the same semantic dependency:

```text
billing/invoice.go -> orders
billing/refund.go  -> orders
billing/tax.go     -> orders
```

All three sites remain available for provenance, findings and source-layer policy.

For topology/SCC work they contribute one semantic endpoint relationship:

```text
billing -> orders
```

#220 should deduplicate eligible `(from, to)` endpoint pairs before SCC construction while retaining every physical dependency site. `Cycles::closes(site)` may still answer for each site from the resulting component map.

This avoids provenance multiplying Tarjan input without losing evidence.

### 5. Layering source classification

The source side of a dependency is classified by the dependency's exact physical source path.

This is required because files in one multi-source module may sit under different repository path policy.

A source file that belongs to no configured layer behaves as it does today: no forbidden-layer verdict is proven for that site.

### 6. Destination module classification

A destination module is classified conservatively across all of its physical sources.

For top-level `layering.in` / `except`, fold source paths to:

```text
Inside   every source is selected
Outside  no source is selected
Mixed    some selected, some not
```

For configured layers, fold source paths to:

```text
Layer(L)     every source belongs to the same layer L
Unlayered    every source belongs to no layer
Mixed        sources span layers, or some are layered and some are not
```

`Mixed` is not silently assigned to the first/last source. Where a dependency judgement needs that destination classification, `Mixed` is a measurement hole/ambiguity, not a guessed PASS or FAIL.

This keeps `in`, `except` and `layers.*.in` as repository-relative path policy rather than converting them into module-name policy.

### 7. Layering work shape

Path/layer classification must be precomputed rather than repeated per dependency.

Required asymptotic shape:

```text
classify physical paths           O(source memberships + policy lookup)
fold module classification        O(source memberships)
judge dependency sites            O(dependency sites) after classification
```

An implementation shaped as:

```text
dependency sites x destination sources x layers
```

is rejected even if current one-source fixtures remain fast.

### 8. Cycle scope is provenance-aware

Cycle detection continues to run over semantic module nodes.

Scope eligibility, however, is determined from physical evidence:

- the dependency source site must be selected by policy;
- the destination module must classify `Inside` for the scope;
- an `Outside` destination excludes the relationship;
- a `Mixed` destination creates a visible ambiguity/hole rather than guessing.

The SCC input is then the unique eligible semantic `(from, to)` relationships.

This avoids both failure modes of selecting whole multi-source nodes by `any source` or `all sources` before considering which dependency sites actually participate.

### 9. Stable ratchet identity and physical reporting

Physical file/line remains reporting evidence. It is not sufficient semantic identity for a dependency that moves between files inside one semantic module.

For a future multi-source module:

```text
before: billing/invoice.go -> orders
after:  billing/refund.go  -> orders
```

is held when all verdict-defining semantics are unchanged: same semantic relationship and same policy classification. Moving the dependency to a source with a different layer/scope classification is a real semantic policy change and may become new debt.

Do **not** redesign generic `ratchet.rs` by default. The preferred bridge is gate-local:

```text
resolved dependency sites
    -> semantic layering/cycle edge identity
    -> base/after pairing
    -> current physical Finding for output
    -> existing Evaluator
```

Compatibility rule: the current one-source Rust/TypeScript finding identity and rename behavior remain unchanged. The new semantic pairing is needed only where several physical sources legitimately belong to one semantic node.

If #220 cannot preserve both properties with a gate-local bridge, it must record the concrete counterexample before proposing a generic ratchet change.

### 10. Capability-aware resolver/surface dispatch

Do not create a project-wide language service or per-resolver filesystem walk.

`Topology` already receives the measured `FileFacts`. Derive the present structural `LanguageId` set once from those facts and attach a language/capability to each static resolver/surface registry row.

Conceptually:

```text
measured FileFacts
    -> present structural LanguageIds

Rust present        -> run Rust resolver
TypeScript present  -> run TypeScript resolver
Go absent           -> skip Go resolver
```

The same rule applies to surface derivation.

Required absent-language cost:

```text
O(number of registered capabilities)
zero repository-sized resolver/surface work
zero extra source reads/parses/extractions
```

This remains a static Rust-native registry, not DI or a plugin framework.

## Representation/performance constraints

Semantics are frozen independently of representation.

Start with the simplest implementation that satisfies them. #221 owns A/B admission.

Allowed evidence-driven local optimizations include:

- one/many storage instead of `Vec<String>` if the one-source Rust/TypeScript case shows a material allocation/RSS tax;
- a dependency-local source ordinal/reference instead of cloning an owned path per edge.

Do not infer a need for repository-wide `FileId`, path interning, persistent ModuleGraph/SCC caches, parallel resolution or a resident process.

Complexity requirements:

```text
source grouping                     O(source memberships) or near-linear
module policy classification        O(source memberships + policy lookup)
dependency judgement                O(dependency sites)
SCC input                            O(unique eligible semantic edges)
absent capability dispatch           O(registered capabilities)
```

#220 should add deterministic work-count characterization for genuinely multi-source synthetic shapes. #221 should then prove that the generalized representation does not tax the shipped one-source Rust/TypeScript product on the existing 300k/1M fixtures.

## Eleven-language adversarial feasibility pass

The purpose of these probes is to falsify the **shared** contract, not to design eleven adapters.

| Ecosystem | Classification | Evidence / implication |
| --- | --- | --- |
| Rust | fits existing one-source subset | Rust modules may be inline or loaded from external files; current target-scoped graph remains the oracle. No shared gap. |
| TypeScript | fits existing one-source subset | TypeScript's ordinary module model is file-oriented; today's one-file resolver remains valid. |
| Go | adapter/resolver-local; strongly validates shared change | A Go package is constructed from one or more source files; imports are written in individual files. This directly requires multi-source nodes + physical dependency provenance. |
| Python | adapter/resolver-local, conservative | Regular modules/packages can map to physical sources. Namespace packages may span locations or be virtual with no concrete filesystem representation; do not add zero-source shared nodes. |
| Java | not safely complete without host/build context | Compilation units declare packages, while host systems determine observability and association with Java modules. Do not infer a universal package/module/build node. |
| C# | not safely complete without project/build context | Programs consist of compilation units and namespaces aggregate names; the compilation/project/assembly set is external context. No new shared abstraction is proven. |
| Swift | adapter/resolver-local; strongly validates shared change | A SwiftPM target has a set of sources and dependencies, providing another natural multi-source semantic node. |
| Kotlin | not safely complete without build context | `internal` means same module, and Kotlin defines module as files compiled together (for example a Gradle source set or Maven project). Package paths alone are insufficient. |
| PHP | adapter/resolver-local with Composer evidence | Composer PSR-4/classmap/files metadata provides explicit resolver evidence; one namespace prefix may map to multiple directories. This needs resolver-local logic, not a new shared graph layer. |
| C | not safely useful generically without compiler/build context | Preprocessing forms translation units and include search is implementation-defined. Conservative syntax-only include edges cannot stand in for build-resolved architecture. |
| C++ | named-module subset fits; general C++ remains build-context gated | A named module is a collection of module units, validating multi-source semantics. Header/TU/preprocessing relationships still depend on compiler/build context. |

### Official language evidence

- Rust modules: https://doc.rust-lang.org/reference/items/modules.html
- TypeScript modules: https://www.typescriptlang.org/docs/handbook/2/modules.html
- Go specification — packages/source files/imports: https://go.dev/ref/spec
- Python import system / namespace packages: https://docs.python.org/3/reference/import.html
- Java Language Specification chapter 7: https://docs.oracle.com/javase/specs/jls/se25/html/jls-7.html
- C# language specification — programs/compilation units/namespaces: https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/language-specification/lexical-structure and https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/language-specification/namespaces
- Swift PackageDescription Target source/dependency model: https://developer.apple.com/documentation/packagedescription/target
- Kotlin module visibility: https://kotlinlang.org/docs/visibility-modifiers.html
- Composer autoload schema: https://getcomposer.org/doc/04-schema.md
- C23 working draft source inclusion: https://www.open-std.org/jtc1/sc22/wg14/www/docs/n3220.pdf
- C++ modules: https://eel.is/c++draft/module.unit

## Shared-foundation review

### `syntax::structural` / #49

**Fits with no shared redesign.**

`FileFacts` remains one physical source file. Multi-source semantics belong above it in language resolvers. Do not merge package-sized facts or keep ASTs.

The language capability used for dispatch should come from actual measured `FileFacts`, not from every grammar listed in `syntax::LANGUAGES`, because a grammar existing does not mean a structural adapter exists.

### `ModuleGraph` / #50

**Requires the narrow #220 generalization.**

This is the only shared foundation with a real gap: one semantic node needs one-or-many sources and dependencies need physical provenance.

### `surface` / #46

**Fits after API adaptation.**

Surface identity is already consumer-facing rather than declaration-file identity. Rust/TypeScript derivation must iterate/select the correct physical facts through the generalized module API and make re-export lookup source-aware; it must not re-resolve modules or gain language-specific judgement in the shared layer.

### `dead-symbols` / `reachability`

**Fits unchanged.**

They operate on structural `Measurement` / `SourceIndex`, not ModuleGraph topology. Multi-source graph work must not leak into their identity or query model. Future language support extends their structural adapter/index evidence separately.

### Structural extraction/cache

**Fits unchanged.**

Physical file facts remain the persistent/cached unit. No package-sized cache object or persisted ModuleGraph is justified.

## Future-ticket contract review

### #48 — duplication

**Fits the contract unchanged.**

Its shared engine consumes per-file parsed structural evidence and canonical function fingerprints. Future languages add candidate/canonicalization adapters; multi-source module topology is not part of occurrence arithmetic, changed-set delta logic or reporting identity.

No #48 specification change is required.

### #53 — coverage report reader

**Fits the contract unchanged.**

It is deliberately language-agnostic external path/range evidence. It neither consumes ModuleGraph nor needs structural package identity.

No #53 specification change is required.

### #54 + #70 — changed coverage + postflight lifecycle

**Fits the contract unchanged.**

#54 uses Hunks and report-declared executable lines plus cheap source-path classification; #70 owns execution/report lifecycle. Neither should depend on structural ModuleGraph semantics.

No #54/#70 specification change is required.

## Rejected shared abstractions

The probes do **not** justify any of these before a concrete future consumer proves them necessary:

- universal `SemanticUnit`;
- universal `PackageGraph` / `WorkspaceGraph`;
- universal build target/project/assembly model;
- zero-source/virtual `Module` nodes;
- repository-wide `FileId` conversion;
- persisted ModuleGraph or SCC cache;
- dynamic resolver plugin/DI framework;
- compiler/type-checker integration;
- guessing build-dependent C/C++/Java/C#/Kotlin relationships from directory names.

## Handoff to #220

#220 should implement exactly this delta:

1. one-or-many non-empty deterministic module sources;
2. resolver-owned semantic identity not derived from first source;
3. physical dependency source provenance and source-aware lookup;
4. precomputed conservative destination scope/layer classification;
5. provenance-aware cycle selection over unique semantic edges;
6. a gate-local stable-edge/physical-Finding bridge that preserves current Rust/TypeScript behavior;
7. capability-aware resolver/surface dispatch from already-owned `FileFacts`;
8. deterministic multi-source work counters/characterization for #221.

Do not add Go/Python or a universal build/package model in #220.

## Decision

The candidate architecture survives the eleven-language adversarial pass.

The concrete shared pressure is narrow and evidence-backed:

> `Module.file` conflates semantic dependency-node identity with one physical source file.

Generalizing that relation to non-empty multi-source membership plus dependency-site provenance is sufficient for the strongest near-term probes (Go and Swift), remains compatible with Rust/TypeScript, and lets build-context-heavy ecosystems stay conservative without forcing speculative abstractions into the core.
