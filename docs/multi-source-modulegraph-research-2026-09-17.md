# Multi-source ModuleGraph research

Date: 2026-09-17  
Issue: #219  
Baseline reviewed: `main` at `af76ad967876295757c02f94209b62014991d375`

## Decision summary

The current architecture needs one narrow generalization before a structural
language whose semantic dependency node may contain several source files:

```text
FileFacts
  = facts for exactly one physical source file

Module
  = one resolver-proven semantic graph node
  = one or more physical source files

Dependency site
  = one resolved semantic from/to relationship
  + the exact physical source file and line that wrote it

Semantic graph edge
  = one unique (from module, to module) relationship used for topology/SCCs
```

No language probe justifies a new universal `SemanticUnit`, package graph,
workspace graph, build graph, repository-wide `FileId`, or persistent semantic
graph.

The current Rust/TypeScript behavior remains the compatibility oracle. The
shared generalization is intended to admit a Go package or Swift target without
making those languages define the architecture for Rust and TypeScript.

## Current-code findings

The existing graph is more deeply one-file-shaped than changing one struct
field suggests.

`src/modules/mod.rs` currently has:

```rust
pub struct Module {
    pub name: String,
    pub file: String,
    // ...
}

pub struct Dependency {
    pub from: usize,
    pub to: usize,
    pub line: u64,
}
```

The same one-file assumption is used by:

- `ModuleGraph::identity`, which derives identity from `Module.file` plus inline
  suffixes;
- `ModuleGraph::reached_at(from, line)`, which assumes `(module, line)` is a
  sufficient dependency-site lookup;
- `ModuleGraph::cycles`, whose selection predicate receives a whole `Module`;
- `layering`, which uses `module.file` for source scope, destination scope,
  source layer and destination layer;
- TypeScript surface derivation, which builds a file-to-module map from
  `module.file`, loads facts from that file and calls `reached_at(module, line)`;
- Rust surface derivation, which likewise reads the one physical file behind a
  module.

That is implementation pressure, not evidence for another architectural layer.
The boundary underneath it is already correct: `FileFacts` is one physical-file
fact object and `Topology`/`ModuleGraph` derives semantic topology above those
facts without reparsing source.

`surface` is also already conceptually ahead of `ModuleGraph`: public API item
identity is consumer-facing path/name + kind, while physical declaration origin
is explanatory evidence. That same separation should be preserved in the graph:
semantic identity and physical evidence are different things.

## Frozen shared semantics

### 1. Module source membership

A `Module` is a resolver-proven semantic node backed by one or more tracked
physical repository-relative source paths.

The semantic invariants are:

```text
sources is non-empty
sources is duplicate-free
sources has deterministic presentation order
source order is not semantic identity
```

One physical file may still participate in more than one module identity where
a resolver proves that topology. Rust already requires this when one file is
reached from multiple targets.

The shared graph does **not** gain zero-source modules. If a language contains a
namespace/package concept that has no tracked physical source, Klin either:

- resolves through physical descendants without materializing the virtual node;
- treats the relationship as external/unsupported; or
- exposes a measurement hole.

Which of those applies is resolver-local. The shared graph must not fabricate a
source-less node merely to mirror a language's terminology.

### 2. Module semantic identity

Module identity is resolver-owned and independent of source collection order.
It must never be reconstructed by choosing `sources[0]`.

This does **not** require #220 to redesign the existing Rust/TypeScript public
finding identities. Their resolver-owned identity may preserve current V1
behavior where compatibility requires it. The shared rule is only that a
future multi-source resolver can provide a semantic identity that remains valid
when its member source set changes without changing the semantic node.

Examples:

- TypeScript V1 can retain one file as one semantic module.
- Rust V1 can retain its current target/module identity behavior while no
  generic API assumes that identity is the first physical source.
- A Go resolver can identify a package by its proven import/package identity,
  not by whichever `.go` file sorts first.
- A SwiftPM resolver can identify a target/module by the target identity, not a
  representative `.swift` file.

Petgraph node/edge indices, collection positions and physical-source ordering
are never semantic identity.

### 3. Dependency physical provenance

Every resolved dependency site carries:

```text
from semantic module
to semantic module
exact physical source path that wrote the relationship
source line
```

The exact in-memory representation is not frozen. In particular, the semantic
requirement does not require an owned path `String` on every dependency. A
local source ordinal/reference that deterministically resolves through
`modules[from].sources` is valid and may avoid duplicating path allocations.

The following lookup is no longer sufficient once one module has two sources:

```text
(from module, line)
```

Two source files can both contain an import on line 10. Any API equivalent to
`reached_at(from, line)` must therefore become source-aware, conceptually:

```text
(from module, physical source, line)
```

or consume an equivalent dependency-site value.

### 4. Dependency sites and semantic topology edges are separate

Several physical sites may prove the same semantic relationship:

```text
billing/invoice.go:10  billing -> orders
billing/refund.go:22   billing -> orders
billing/tax.go:17      billing -> orders
```

All three sites remain available for layering, findings and explanation. For
SCC/topology work they contribute one semantic endpoint pair:

```text
billing -> orders
```

Cycle construction should deduplicate eligible `(from, to)` pairs before SCC
computation. The SCC still contains semantic module nodes, never physical-file
nodes.

This keeps provenance complete without making graph-algorithm cost scale with
irrelevant duplicate evidence.

## Layering semantics

`layering.in`, `layering.except` and each layer's `in` remain repository-relative
**path policy**. Multi-source modules do not turn those selectors into semantic
module-name policy.

### Source side

The source side of a dependency is unambiguous:

```text
source scope/layer = classification of dependency.source
```

The semantic source module's other files do not decide where this particular
site lives.

### Destination layer classification

A destination module is folded from all of its physical sources once per run.
The classification is conservative:

```text
all sources in the same configured layer L
    -> Layer(L)

all sources in no configured layer
    -> Unlayered

sources in more than one configured layer
    -> Ambiguous

some sources in L and some unlayered
    -> Ambiguous
```

An `Ambiguous` destination cannot produce a guessed forbidden-edge verdict.
Klin should expose that uncertainty through the existing visible-hole/coverage
philosophy. The precise note plumbing is an implementation concern for #220.

This is intentionally different from `find_map`: filesystem/sort order must
never decide the destination's layer.

### Top-level scope classification

Destination scope is likewise tri-state:

```text
all sources selected by top-level in/except
    -> Inside

no sources selected
    -> Outside

some selected and some not selected
    -> Mixed
```

For ordinary edge judgement:

- the physical dependency source site must itself be selected;
- an `Inside` destination may be judged;
- an `Outside` destination is out of scope;
- a `Mixed` destination is a conservative measurement hole, not a guessed
  verdict.

The source semantic module may itself contain selected and unselected sources;
only the physical dependency sites that satisfy scope participate.

### Cycle scope

Current V1 selects whole nodes from `module.file` and then feeds every edge
between selected nodes to Tarjan. That does not survive a multi-source node.

The replacement rule is provenance-first:

1. classify source paths/modules once;
2. select physical dependency sites through the same scope rule as edge
   judgement;
3. drop sites whose destination is Outside or Mixed;
4. deduplicate the resulting semantic `(from, to)` endpoint pairs;
5. run SCCs over the semantic module nodes reached by those eligible pairs.

This avoids both failure modes of whole-node selection:

- selecting a module because any source matches and leaking dependencies written
  by its out-of-scope sources;
- requiring all source files to match and losing legitimate dependencies written
  by the selected subset.

Mixed destination membership is deliberately a false-negative/hole rather than
an arbitrary inclusion rule.

## Stable edge identity and ratcheting

The current `layering` implementation keys a finding under the physical file
that writes it plus the layer/target text. The generic ratchet also starts from
physical `file + text` sites, with a special body-hash path for moved source
declarations.

That is insufficient for this case:

```text
before: billing/invoice.go -> orders
after:  billing/refund.go  -> orders
```

If both files belong to the same semantic `billing` module and remain in the
same path policy/layer, the semantic architecture relationship did not change.
The physical evidence moved.

Freeze a gate-local semantic edge identity before ordinary findings are built.
Conceptually:

```text
resolved dependency sites
        |
        v
semantic EdgeKey/base-after pairing
        |
        v
current physical site for reporting
        |
        v
existing ratchet Evaluator
```

For forbidden edges the semantic key includes enough policy meaning to make a
real architecture change observable, conceptually:

```text
kind
source semantic module identity
destination semantic module identity
source layer -> destination layer
```

For cyclic edges the semantic relationship is the source and destination module
identity plus `cycle` kind.

Consequences:

- moving an equivalent dependency between two sources of the same semantic
  module and same layer is held;
- changing the target module is new;
- moving the dependency site across a configured layer changes the policy key
  and is new when the resulting edge is forbidden;
- splitting or merging semantic nodes is observable because resolver-owned
  identities change;
- a package/module move is held only when the resolver itself proves it remains
  the same semantic node. The shared graph does not guess this from similar
  paths.

Prefer implementing this pairing inside `layering` before converting to generic
`Finding`s. Do not redesign generic `ratchet.rs` unless a concrete #220
characterization proves the gate-local bridge cannot preserve the required
behavior.

Accepted-entry portability across arbitrary physical-site moves is not expanded
by #219; preserve existing accepted-entry semantics unless separate evidence
requires a product change.

## Capability-aware dispatch

The current graph registry is a static list of resolver functions and runs every
entry. `surface::derive` similarly calls the Rust and TypeScript derivations
unconditionally.

Keep static Rust-native registries, but bind each row to its structural
`LanguageId` and decide applicability from already-owned `FileFacts`.

Conceptually:

```rust
struct Resolver {
    language: LanguageId,
    run: fn(&mut Builder),
}
```

`Topology`/the graph builder can derive the set of present structural languages
once from its `FileFacts`:

```text
Rust present       -> run Rust resolver
TypeScript present -> run TypeScript resolver
Go absent          -> zero Go resolver repository work
```

The same principle applies to surface derivation.

Requirements:

- no new filesystem walk;
- no second project-wide language catalogue;
- no dynamic plugin/DI framework;
- dispatch itself may be O(number of registered capabilities);
- an absent capability performs zero repository-sized file/module scan.

The fact that `syntax::LanguageId` already contains logical IDs for more
languages is useful but does not mean those languages have structural adapters.
Presence is derived from actual `FileFacts`, not merely from grammars registered
in `syntax::LANGUAGES`.

## Complexity and representation constraints

The research freezes asymptotic shape, not one Rust container type.

Expected work:

```text
source membership/grouping       O(source memberships) or near-linear
path/layer classification        once per relevant physical source
module policy fold               O(source memberships)
dependency judgement             O(dependency sites) after classification
SCC input                         O(unique eligible semantic edges)
SCC                               O(modules + unique eligible semantic edges)
absent-language dispatch          O(registered capabilities), no repository scan
```

An implementation that repeatedly scans every destination source while judging
every dependency is not admissible:

```text
O(dependency sites x destination sources x layer selectors)
```

The safe implementation precomputes path classification and folds module state
once, then uses lookups while judging dependencies.

For source storage, start with the simplest owned representation that satisfies
the semantics, such as `Vec<String>`. #221 owns evidence for whether the common
one-source Rust/TypeScript case warrants a one/many optimization. Do not choose
`SmallVec`, interning or repository-wide IDs from theory.

Dependency provenance deserves separate measurement because cloning one path
`String` per dependency can cost more than the module-source collection on a
dependency-dense repository. A local source ordinal/reference is allowed if
#221 shows it is useful.

## Eleven-language feasibility pass

The categories below mean:

- **fits unchanged** — current one-source subset already represents the
  ecosystem's relevant node shape;
- **adapter/resolver-local** — the shared contract is sufficient; language
  semantics live in its resolver/structural adapter;
- **build-context gated** — useful conservative support for the general
  ecosystem requires build/compiler/host evidence that Klin does not currently
  own; this is not a shared-graph gap;
- **shared gap** — a concrete fixture cannot be represented by the frozen shared
  contract. No probe produced this result.

| Ecosystem | Classification | Adversarial result |
| --- | --- | --- |
| Rust | fits unchanged | External file modules and inline modules fit the current one-source node subset; one physical file may still occur in multiple target-scoped identities. |
| TypeScript | fits unchanged | Current V1 one-file module model remains valid; relative import/re-export provenance is naturally a physical file site. |
| Go | adapter/resolver-local | A package is constructed from one or more files while each import is written by one source file. This is the strongest direct proof for multi-source nodes + site provenance. Build constraints should under-resolve/produce holes rather than make the shared graph build-aware. |
| Python | adapter/resolver-local | Ordinary `.py` modules can stay one-source. Namespace packages may span locations or have no physical representation, which validates the no-zero-source-node rule rather than requiring virtual shared nodes. |
| Java | build-context gated | Packages span compilation units, but the host system decides observable compilation units and their association with Java modules. Full safe JPMS/build topology cannot be guessed from paths alone. |
| C# | build-context gated | Compilation units are processed together and contribute to common namespaces, while assemblies/modules are physical containers chosen by the compilation/project. Namespace text alone is not a safe dependency-node boundary. |
| Swift | adapter/resolver-local | SwiftPM explicitly defines a target as a set of sources compiled into a module/test suite and gives target dependencies. This directly fits multi-source semantic nodes. |
| Kotlin | build-context gated | Kotlin defines a module as files compiled together (compiler invocation/Maven/Gradle source set/project); `internal` semantics depend on that compilation boundary. |
| PHP | adapter/resolver-local | Composer provides explicit PSR-4/classmap/files evidence; one namespace prefix can map to multiple directories. Resolver-local Composer evidence is enough without a universal package graph. |
| C | build-context gated | Translation units are formed after preprocessing/includes and include lookup/macros/conditionals depend on compiler invocation. A generic repository-path module graph would guess. |
| C++ | build-context gated | Named modules themselves fit the shared multi-source model because a named module is a collection of module units, but general C++ dependency topology still depends on translation units, preprocessing and build context. |

### External evidence used for the probes

- Rust Reference, modules and source-file rules:
  <https://doc.rust-lang.org/reference/items/modules.html>
- Rust Reference, crates/source files:
  <https://doc.rust-lang.org/reference/crates-and-source-files.html>
- TypeScript Handbook, modules:
  <https://www.typescriptlang.org/docs/handbook/2/modules.html>
- Go language specification, packages/source files/imports:
  <https://go.dev/ref/spec>
- Go module layout guide:
  <https://go.dev/doc/modules/layout>
- Python glossary, namespace packages:
  <https://docs.python.org/3/glossary.html#term-namespace-package>
- Java Language Specification chapter 7, packages/modules/host support:
  <https://docs.oracle.com/javase/specs/jls/se25/html/jls-7.html>
- C# language specification, namespaces/compilation units:
  <https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/language-specification/namespaces>
- Swift PackageDescription `Target`:
  <https://developer.apple.com/documentation/packagedescription/target>
- Kotlin visibility/modules:
  <https://kotlinlang.org/docs/visibility-modifiers.html#modules>
- Composer schema, PSR-4 mappings:
  <https://getcomposer.org/doc/04-schema.md#psr-4>
- WG14 C material describing preprocessing translation units/translation units:
  <https://www.open-std.org/jtc1/sc22/wg14/www/docs/n843.htm>
- C++ working draft, module units/named modules:
  <https://eel.is/c++draft/module>

## Existing consumer review

### #49 / structural facts and cache

**Fits unchanged.**

`FileFacts` remains exactly one physical-file unit. The structural cache remains
a cache of those facts. No package-sized cached object, second parse path,
persistent ModuleGraph or AST retention is needed.

### #50 / ModuleGraph and layering

**Needs the narrow #220 implementation already specified.**

The one-file assumptions are localized but real: module storage/identity,
dependency provenance/site lookup, layering destination classification, cycle
scope and SCC duplicate edges. No broader graph layer is required.

### #46 / surface/public-api

**Fits the shared contract with API adaptation.**

Surface derivation must iterate/choose facts through explicit module source
membership and make dependency-site lookup source-aware. It does not need a new
identity model: public API identity is already consumer-facing and physical
origin is evidence only.

### dead-symbols / reachability

**Fits unchanged; verify non-leakage only.**

Those checks consume `Measurement`/`SourceIndex`, not `ModuleGraph`. Adding
multi-source graph semantics must not route them through modules. Future
languages extend them through conservative structural facts/name evidence, not
through #220.

## Planned-ticket review

### #48 duplication

**Fits the contract unchanged.**

Its unit is a per-file parsed function/method candidate and a canonical
language-qualified fingerprint. Its shared multiplicity arithmetic and changed
set logic do not depend on `ModuleGraph`. Future Go/Python work remains a
normalization/candidate adapter concern.

No #48 spec change is required by #219.

### #53 external coverage reader

**Fits the contract unchanged.**

It consumes external path/line coverage evidence and intentionally knows
nothing about source-language topology. Multi-source semantic modules do not
change its streaming selected-line contract.

No #53 spec change is required.

### #54 changed coverage + #70 postflight lifecycle

**Fit the contract unchanged.**

#54 judges changed repository-relative path/range evidence from `Hunks` and an
external coverage report. #70 owns phase/report lifecycle. Neither should gain
module resolution merely because #220 exists.

No #54/#70 spec change is required.

## Rejected shared abstractions

The research found no evidence for introducing any of these before #220:

- `SemanticUnit` between `FileFacts` and `Module`;
- universal Package/Target/Workspace graph;
- build-system model shared across languages;
- zero-source virtual modules;
- repository-wide `FileId` conversion;
- persistent ModuleGraph or SCC cache;
- dynamic resolver/plugin/DI framework;
- compiler/type-checker integration;
- path heuristics that guess Java/C#/Kotlin/C/C++ build topology.

Java, C#, Kotlin, C and general C++ are evidence to **stop at a conservative
hole when build context is missing**, not evidence to make every language use a
speculative universal build abstraction.

## Handoff to #220

#220 should implement only the frozen contract:

1. one-or-many non-empty deterministic module sources;
2. resolver-owned semantic identity not derived from the first source;
3. exact dependency source provenance with source-aware site lookup;
4. precomputed source/module policy classification;
5. provenance-aware edge/cycle scope;
6. unique semantic SCC edges while retaining every physical dependency site;
7. gate-local semantic edge pairing before physical findings;
8. capability-aware resolver/surface dispatch from already-owned structural
   facts;
9. deterministic work counters and a synthetic multi-source work-shape test.

#221 remains the admission gate for representation/runtime/RSS cost on the
existing 300k/1M Rust/TypeScript fixture.

## Research conclusion

The working hypothesis is accepted.

The smallest shared architecture change is not a new language framework. It is
only the removal of the accidental invariant:

> one semantic module == one physical file

The frozen replacement is:

> one semantic module has one or more physical sources; dependencies retain the
> exact physical site that proves them; topology, policy identity and reporting
> do not confuse those two layers.

All eleven language probes can either use that shared contract or conservatively
stop where additional build/compiler context would be required. No concrete
probe demonstrates a further shared architecture gap.