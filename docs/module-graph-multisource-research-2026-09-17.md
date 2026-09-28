# Multi-source ModuleGraph research — 2026-09-17

Issue #219 asks one narrow question before structural language #3:

> What is the smallest shared graph contract that stops equating one semantic
> dependency node with one physical source file, without replacing Klin's
> Rust/TypeScript architecture with a universal build/language framework?

This note records the answer against current `main`. It changes no runtime
behavior. ADR 0058 records the resulting decision; #220 owns implementation
and #221 owns admission on the existing Rust/TypeScript performance fixture.

## Current-main constraints

The existing architecture has the right layers:

```text
physical source
    -> syntax::structural::FileFacts
    -> modules::ModuleGraph
    -> layering / cycles
    -> surface / public-api
```

`FileFacts` is already the right physical unit. One fact object owns one
repository-relative source path and that file's declarations, imports,
references, module declarations, paths and exports. Measurements/cache reuse
those physical facts and higher layers parse no source again.

The accidental one-to-one assumption is above that boundary:

- `Module.file` is both semantic-node identity input and physical evidence;
- `Dependency` records `(from, to, line)` but not the physical source file;
- `ModuleGraph::reached_at(from, line)` assumes one line number is enough to
  identify an edge site inside a source module;
- `ModuleGraph::identity` reconstructs a module name from its file;
- `layering` selects cycle nodes by `module.file`, classifies both ends from
  the two module files and keys findings by the writing file;
- Rust and TypeScript surface derivation retrieve facts through
  `module.file`;
- cycle construction inserts every resolved dependency site into petgraph,
  even when several sites have the same semantic `(from, to)` endpoints.

Those are valid simplifications while every semantic node has one source.
They do not survive a Go package or Swift target whose semantic dependency
node owns several source files.

Two existing contracts must remain unchanged:

1. Rust may create several module identities from one physical source where
   different targets prove different contexts. Multi-source support must not
   merge them merely because their source sets overlap.
2. TypeScript V1 remains one physical TypeScript/TSX file per semantic module.

## Frozen shared contract

### 1. `FileFacts` remains physical

One `FileFacts` is exactly one physical source file.

Do not merge the facts of a Go package, Swift target or future build target
into a package-sized cached object. A resolver groups existing physical facts
into semantic graph nodes; the syntax/cache layer does not learn package or
workspace topology.

This preserves the completed structural cache/delta architecture and keeps a
language adapter's extraction independently reusable by `SourceIndex`-based
consumers such as `dead-symbols` and `reachability`.

### 2. A `Module` is resolver-proven semantic identity with one or more sources

A graph node means one dependency identity proven by that language's
resolver. It owns one or more physical repository-relative sources.

Required invariants:

```text
sources is non-empty
sources is duplicate-free
sources is deterministic
source order has no semantic meaning
```

The resolver owns the semantic module identity. It must not be reconstructed
from `sources[0]`.

The same physical source may participate in several semantic nodes where the
resolver proves distinct contexts. Existing Rust target-scoped attachment is
the compatibility example.

Do not create zero-source synthetic modules merely to make a package tree
look complete. If the repository cannot attach a semantic node to at least
one tracked physical source, that relationship remains external, unsupported
or a coverage-visible hole. Python namespace packages are the adversarial
case: an importable namespace may span several locations or have no single
concrete source representation, which is evidence to under-resolve, not to
make the shared graph virtual.

The exact representation (`Vec<String>`, one/many, or equivalent) remains an
implementation choice until #221 measures the current one-source case.

### 3. A dependency has semantic endpoints and a physical site

Every resolved dependency has two independent kinds of information:

```text
semantic relationship:  from module -> to module
physical evidence:       source path + line that wrote it
```

The physical source must be one of the source module's physical sources.

The semantic contract is the exact repository-relative source path plus line.
The in-memory representation need not clone an owned `String` into every
edge. A local ordinal/reference into `modules[from].sources` is equivalent if
it deterministically resolves to the exact path.

Any lookup that identifies a dependency site only as `(module, line)` is no
longer sufficient. Two physical files in one package may both write an import
on line 10. Consumers that resolve a source statement to graph targets need a
source-aware site, conceptually:

```text
(from module, physical source, line)
```

### 4. Keep dependency sites distinct from semantic topology edges

A dependency site is physical evidence. A semantic graph edge is a module
relationship.

Several files of one semantic module may all depend on the same destination:

```text
billing/invoice.go  --\
billing/refund.go    ---> billing -> orders
billing/tax.go      --/
```

Klin keeps all three sites for reporting, path policy and provenance. SCC
construction gains no topology information from three copies of
`(billing, orders)`, so eligible endpoint pairs should be deduplicated before
Tarjan/SCC work.

This separation is also the performance boundary: provenance multiplicity
must not become graph-topology multiplicity.

### 5. Path policy is folded conservatively over a destination's sources

The source side of a dependency is unambiguous: apply path/scope/layer policy
to the physical source that wrote the dependency.

A destination module no longer has one path. Classify its source set once and
fold it conservatively.

For top-level `layering.in` / `except` scope:

```text
Inside  = every source selected
Outside = no source selected
Mixed   = some selected and some not selected
```

For configured layers:

```text
Layer(L)   = every source belongs to the same layer L
Unlayered  = every source belongs to no configured layer
Mixed      = sources span layers, or mix layered and unlayered sources
```

A physical file still belongs to at most one configured layer, as today. The
new ambiguity is across several files backing one semantic destination.

For a forbidden-edge judgement:

- the physical dependency source must be selected by top-level scope;
- source-layer membership comes from that physical file;
- an `Inside` destination with one `Layer(L)` is judgeable;
- an `Outside` destination is outside this policy judgement;
- a `Mixed` destination is not assigned by first/last/sorted source; surface
  it as coverage/measurement ambiguity and do not emit a guessed blocking
  verdict;
- an `Unlayered` destination retains today's non-violation behavior where
  either endpoint has no layer.

Classification is computed once per relevant source/module for the run and
consumed by dependency judgement. The shared design must not require:

```text
O(dependency sites * destination sources * layers)
```

work.

### 6. `acyclic` selects dependency evidence, while SCCs remain semantic

Today's `cycles(keep: Fn(&Module))` works only because a node has one path.
Neither `any source selected` nor `all sources selected` gives correct
multi-source behavior:

- `any` can leak dependency sites written by out-of-scope sources into the
  cycle graph;
- `all` can drop legitimate in-scope dependency sites merely because another
  file of the same package is outside scope.

The selection boundary therefore moves to dependency evidence:

1. decide whether the physical source site is selected;
2. require the destination module's scope classification to be unambiguous
   `Inside` for a scoped dependency to participate;
3. turn eligible sites into unique semantic `(from, to)` endpoint pairs;
4. run SCCs over semantic module nodes and those endpoint pairs.

A `Mixed` destination is coverage-visible rather than guessed into or out of
the SCC graph.

Cycle findings still point at physical current evidence sites. The graph does
not become a file graph.

### 7. Semantic continuity is paired before physical findings

A physical evidence move and a semantic relationship change are different.
The adversarial case is:

```text
before: billing/invoice.go imports orders
after:  billing/refund.go  imports orders
```

If both files belong to the same semantic `billing` node and the source-side
policy meaning did not change, this is the same semantic dependency relocated
inside its node. It must not become new architecture debt solely because the
reporting path changed.

Conversely, these remain observable changes:

- the dependency retargets to another semantic node;
- the package/module semantic identity changes (for example a Go package
  directory/import path moves);
- a node splits or nodes merge;
- relocation changes the dependency source's effective layer/scope policy;
- a formerly unambiguous destination becomes mixed, or vice versa.

The generic ratchet currently matches ordinary findings primarily by physical
`(file, text)`. Do not broaden that generic identity pre-emptively. Layering
should first pair/normalize semantic base/current dependency judgements, then
materialize the current physical `Finding` used for reporting and ordinary
ratchet evaluation.

The implementation may choose the narrowest representation that preserves all
existing Rust/TypeScript characterization. A generic ratchet change requires
a concrete fixture proving consumer-local pairing cannot represent the frozen
semantics.

### 8. Capability dispatch comes from the topology pass, not only successful facts

Future language count must not multiply repository-sized resolver/surface
work. A static registry remains the right design, but invoking every resolver
unconditionally is not.

A subtle constraint from current code is that presence cannot be defined only
as "there is a successful `FileFacts` for this language". A tree may contain
Rust or TypeScript files whose grammar rejected them. Current resolvers can
still establish target/module topology around those paths, and surface/coverage
logic must not change merely because every source of one language failed
structural extraction.

The capability signal therefore is **language path presence in the topology's
already-owned file list**, collected while `Topology::new` already normalizes
that list. This introduces no second repository walk.

Conceptually:

```text
Topology construction (existing file-list pass)
    -> present logical LanguageIds

static resolver registry
    Rust present       -> run Rust resolver
    TypeScript present -> run TypeScript resolver
    Go absent          -> zero Go resolver repository work
```

A resolver remains responsible for deciding whether a present path has enough
facts/metadata to produce a node, edge or hole. Successful `FileFacts` remain
the semantic evidence; path presence only answers whether a capability could
apply.

Surface dispatch follows the same static capability rule. No DI container,
plugin registry, second project language catalogue or per-resolver filesystem
walk is introduced.

### 9. Performance invariants

The shared semantics intentionally allow cheap implementation:

```text
source grouping/membership        O(source memberships), near-linear
path/layer classification         O(relevant source memberships + policy lookup)
module classification             once per module
judging dependency sites          O(dependency sites) after classification
SCC input                         O(unique eligible semantic edges)
absent capability decision        O(registered capabilities), zero repository-sized work
```

The current one-source Rust/TypeScript path must add zero structural reads,
parses or fact extractions. #220 adds deterministic work counters and a
multi-source stress characterization; #221 owns controlled 300k/1M A/B timing
and RSS admission.

Do not select `SmallVec`, repository-wide `FileId`, path interning, a persistent
ModuleGraph/SCC cache, parallel resolution or a resident service from theory.
Measure the simple representation first. If the one-source allocation or
provenance path copies are measurably responsible for a regression, revise
only that local representation.

## Adversarial identity cases

The implementation characterization should pin these outcomes:

| Change | Expected semantic result |
| --- | --- |
| Move one source within the same resolver identity | held unless resolver semantics say the identity changed |
| Move one dependency between sources of the same module | held when target + effective policy meaning are unchanged |
| Move dependency to a source in another layer | new/removed judgement as policy meaning changes |
| Retarget import/use to another module | observable new relationship |
| Move a Go package directory/import path | semantic node identity changes |
| Split one package/node into two | topology changes, observable |
| Merge two nodes | topology changes, observable |
| Rename current Rust/TS one-source file inside same layer | preserve ADR 0043/SPEC behavior |
| Rename current Rust/TS file across layers | preserve current new-debt behavior |

Existing Rust/TypeScript tests are the compatibility oracle; the new pairing
must not split or merge existing findings merely because the internal graph
representation changed.

## Eleven-language feasibility pass

The probe is deliberately about shared architecture, not about committing to
support every language.

| Ecosystem | Classification | Evidence / implication |
| --- | --- | --- |
| Rust | fits existing one-source subset | A crate is a module tree; file-backed and inline modules already fit current resolver semantics. One source may appear in multiple target contexts, which multi-source support must preserve rather than merge. |
| TypeScript | fits existing one-source subset | ES/TS modules are naturally file-oriented in current V1. Relative resolution remains resolver-local. |
| Go | adapter/resolver-local; **proves shared multi-source change** | A package is built from one or more source files while each import declaration belongs to a physical source file. Exactly one semantic node + many source facts + physical dependency provenance. Build constraints remain adapter-local uncertainty. |
| Python | adapter/resolver-local, conservative | Regular physical modules fit. Namespace packages may span locations or be virtual; this validates holes/under-resolution rather than zero-source shared nodes. |
| Java | not safely useful as a general blocker without build/host context | The JLS lets the host determine observable compilation units and their module association. Shared node/source/provenance shape is sufficient once that context is proven; no new generic layer follows. |
| C# | build-context gated | Compilation units are processed as a program; namespaces and partial types can span units, while project/assembly/module membership determines the real compilation boundary. The shared graph can represent a proven target but cannot infer it safely from namespaces alone. |
| Swift | adapter/resolver-local; **proves shared multi-source change** | A SwiftPM target contains a set of sources compiled into one module and declares target/product dependencies. This independently validates multi-source semantic nodes. |
| Kotlin | build-context gated | `internal` is scoped to files compiled together (for example a Gradle source set/Maven module), not a package directory. Build metadata is required before blocking module semantics are safe. |
| PHP | adapter/resolver-local with explicit metadata | Composer autoload metadata can conservatively map namespace/class identities to one or several directories. This is resolver work; it does not prove a universal package/workspace graph. |
| C | not safely useful without build/compiler context | Preprocessing, include search and conditional compilation depend on implementation/build settings. A source-only blocking dependency graph would overclaim. |
| C++ | named-module subset fits; general graph build-context gated | A named module is a collection of module units and fits the multi-source node. General translation-unit/header/include semantics still depend heavily on preprocessing/build configuration. |

No probe requires a universal `Target`, `SemanticUnit`, `PackageGraph`,
`WorkspaceGraph` or build model between `FileFacts` and `ModuleGraph`.

The recurring lesson from Java/C#/Kotlin/C/C++ is not "add another shared
layer". It is "do not invent build semantics from filenames when the language
does not make them source-provable."

## Direct consumer review

### `layering` / cycles — shared contract changes

This is the principal consumer affected by #220. It needs physical dependency
provenance, folded destination classification, provenance-aware cycle scope,
semantic endpoint deduplication and consumer-local base/current edge pairing.

### `surface` / `public-api` — API adaptation, not identity redesign

Rust/TypeScript surface code currently retrieves facts through `module.file`
and TypeScript resolves re-exports with `reached_at(module, line)`. Those APIs
must become source-aware/iterate source membership as appropriate.

The public surface identity model itself is already correct: a surface item is
identified by what a consumer addresses, not the file that declares it.
Physical origin is explanation. That is the same semantic/evidence separation
now being introduced one layer lower.

### `dead-symbols` / `reachability` — no graph leakage

Both gates consume structural `Measurement`/`SourceIndex`, not `ModuleGraph`.
Multi-source graph implementation must not modify their judgement. Future
language support reaches them through new per-file structural facts and the
existing index boundary.

## Future-ticket review

### #48 duplication — fits unchanged

#48 is deliberately per-function/per-file over the existing parsed structural
source and a shared fingerprint/multiplicity engine. Future languages add
candidate/canonicalization adapters. It has no module-graph dependency and
needs no #219 adjustment.

### #53 coverage reader — fits unchanged

#53 streams external LCOV/Cobertura into repository-relative selected line
facts and is explicitly language-agnostic. It has no graph dependency.

### #54 changed coverage + #70 postflight — fit unchanged

#54 consumes `Hunks` plus #53 evidence and intentionally uses cheap path/source
classification rather than structural module resolution. #70 owns execution
phase/report lifecycle. Neither acquires a graph assumption from this work.

No ticket-spec changes are required for #48, #53, #54 or #70.

## Rejected shared abstractions

This research does not justify:

- package/workspace dependency graphs;
- a universal build target model;
- a `SemanticUnit` layer between structural facts and modules;
- zero-source virtual ModuleGraph nodes;
- repository-wide `FileId` conversion;
- persistent ModuleGraph/SCC state;
- per-language parser/resolver services;
- dynamic plugin/DI registration;
- compiler/type-checker integration;
- a generic ratchet identity redesign.

Each can be revisited only when a concrete language fixture and a real Klin
consumer cannot be represented safely by the frozen contract.

## Adapter-local questions deliberately left open

The shared contract does not decide:

- Go `go.mod`/nested-module/build-tag resolution details (#222);
- Python package-root/namespace/dynamic import limits (#223);
- Java/C#/Kotlin/Swift project-file formats;
- PHP Composer mapping details;
- C/C++ compilation-database/preprocessor strategy;
- public API derivation for any new language.

Those are resolver/surface-adapter concerns. They should create holes or stay
unsupported when required evidence is absent rather than expanding the shared
architecture speculatively.

## Implementation handoff

#220 should implement only this generalization:

```text
FileFacts (physical, unchanged)
       |
       v
resolver-owned Module identity
  + one-or-many physical sources
       |
       v
Dependency sites
  semantic endpoints + physical provenance
       |
       +--> layering: precomputed path/module classification
       |
       +--> cycles: unique eligible semantic endpoint pairs
       |
       +--> surfaces: source-aware fact/edge access
```

#221 then admits that implementation on the current Rust/TypeScript 300k/1M
fixture. Go (#222) remains the first real multi-source proving language; Python
(#223) remains the conservative-resolution proving language.

## External references used by the language probes

- Rust Reference, modules: https://doc.rust-lang.org/reference/items/modules.html
- Rust Reference, crates/source files: https://doc.rust-lang.org/reference/crates-and-source-files.html
- TypeScript Handbook, modules: https://www.typescriptlang.org/docs/handbook/2/modules.html
- Go language specification: https://go.dev/ref/spec
- Go module reference: https://go.dev/doc/modules/gomod-ref
- Go build package / constraints: https://pkg.go.dev/go/build
- Python import system: https://docs.python.org/3/reference/import.html
- Java Language Specification, packages/modules: https://docs.oracle.com/javase/specs/jls/se26/html/jls-7.html
- C# language specification, namespaces: https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/language-specification/namespaces
- Swift PackageDescription `Target`: https://developer.apple.com/documentation/PackageDescription/Target
- Kotlin visibility / module definition: https://kotlinlang.org/docs/visibility-modifiers.html
- Composer schema, autoload: https://getcomposer.org/doc/04-schema.md#autoload
- C working draft: https://www.open-std.org/jtc1/sc22/wg14/www/docs/n3220.pdf
- C++ draft, module units: https://eel.is/c++draft/module.unit
- C++ draft, translation phases: https://eel.is/c++draft/lex.phases
