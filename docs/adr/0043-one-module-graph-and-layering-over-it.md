# One module graph, and layering judged over it

Issue #50 asks for a gate that names one agent failure: the agent solved a local
problem by reaching across a declared architectural boundary, or by closing a
dependency cycle, instead of using the intended interface. Spec 8.5 deferred
the gate until the reference extractor existed. #49 and #190–#193 shipped that
extractor, its shared per-tree extraction and its base cache, so this decision
reverses the deferral.

An earlier comment on #50 closed it against the criteria of spec 8.1, and
`docs/design-gaps-2026-09-08.md` asked for a user who needs constraints beyond
the compiler's module graph. The owner reopened #50 with the design below, and
the gate meets each criterion. It is deterministic and offline. Its remedy
adds work: depend through the layer's interface, or move the code. A linter
cannot do it, because it needs knowledge across files and a second tree to
ratchet against. It names an agent failure. The compiler accepts any
dependency that is visible, so a module boundary the compiler checks is no
layer a person declared.

## The decision

### The module graph is a layer of its own

`modules::ModuleGraph` sits above `syntax::structural` and below every check
that asks what depends on what. It reads the facts a run already extracted and
the tree's own file list. It parses nothing and walks nothing. `layering` is its
first consumer, and #46 is the next one.

The graph keeps three relations apart:

- target ownership: which Cargo target reaches a Rust file
- containment: which module declares which, which only builds the module tree
- dependency: which module a `use` tree, a qualified path or an import names

Only dependencies are edges. A `mod child;` never is, so a parent that declares
a child in another layer is no violation, and the pair is no cycle.

One Rust file that two targets reach is a module of each. The resolver never
picks one target for it. A TypeScript file is one module.

`petgraph` computes the strongly connected components. Its types stay inside
`modules`: a check sees `Module`, `Dependency`, `Hole` and `Cycles`, and a
module index is never part of a finding's identity.

A new structural language is a resolver file and one entry in `RESOLVERS`.
`layering` does not branch on a language.

### Resolution proves a target or says it could not

Rust targets come from Cargo manifests through `cargo_toml`. Its
`AbstractFilesystem` is implemented over the tree's file list, so implicit
`src/lib.rs`, `src/main.rs` and `src/bin` targets come from the list the run
already holds, and workspace inheritance reads only manifests that list holds.
Where no usable manifest sits above a file, `src/lib.rs`, `src/main.rs` and a
file directly in `src/bin` are conventional roots.

From each root the resolver follows `mod` declarations, a literal `#[path]`
included, with rustc's directory rules. A path from `crate`, `self` or `super`
resolves to the deepest module it names. A path from any other name may be
another crate or a local item, so it is counted and never resolved. A module
that two files answer, a module no file answers and a path above the crate root
are holes.

A TypeScript relative specifier resolves when exactly one candidate file
exists. Package names, aliases, `paths`, `baseUrl` and export maps are counted
and never resolved. `oxc_resolver` is not added.

A hole is a NOTE in the hook and exit 2 elsewhere, as an unparsed file is
(ADR 0021). A file on disk that the file list leaves out, such as generated
source git ignores, is counted as outside V1 and is no hole.

### The structural facts grew, and nothing parses twice

Resolving `super::` inside `mod tests { }` needs to know that `tests` holds the
`use`. So `Import`, `ModuleDecl` and the new `QualifiedPath` carry the inline
modules that hold them. A `ModuleDecl` now also records inline modules, with
`inline` set, and an inline one claims no references. A Rust `Import` keeps
every leaf path of its use tree. The structural cache writes the new fields,
and its epoch rose to 2.

### Each tree is judged under its own topology

The base checkout moves a renamed file to its current path, so gates keyed by
path keep their sites. For layering that would hide a move across a boundary.
So the base graph names every renamed file by its base path from the change
set, and a finding names it by its current path. A file moved from `ui` to
`domain` is placed in `ui` at the base and in `domain` now.

A manifest is read from each tree's own list. The derivation survey never
stands in for the base.

`base::whole` used the judgement scope to decide whether the runner's base
tree was whole. A changed run lays out only the changed files, and a check
that takes no scope saw that partial tree as the whole base. The change set now
decides.

### A finding is an edge, not a site

A forbidden edge is keyed by the file that writes it, the two layers and the
module it reaches, named by its current file and any inline modules after it. A
cyclic edge is keyed by the file and the module. Each carries `edge` at 1.
Retargeting a line to another module, an inline module of the same file
included, changes the key, so it is new debt. A second line on an edge the base
holds is not. The explanatory cycle path is a value, never part of the key.

A Rust path that resolves to its own module is no edge. `use self::Kind::*`
names the module's own items and depends on nothing, and counting it as a
self-loop would fail ordinary code under `acyclic`. A TypeScript file that
imports itself is an edge, and that self-loop is a cycle.

`layering` needs the commit, not the runner's tree. It reads the whole base
through the run's shared checkout, so a changed run lays out no partial tree
for it.

Today's policy judges both trees. Adding or tightening a layer therefore holds
the edges the base already had.

## Consequences

- `layering` is a Policy check with `Needs::TheCommit`. It builds both graphs
  only when its section exists.
- A changed run that is not strict takes the base's facts for unchanged files,
  and a second run reads them from the structural cache.
- A gate row carries `graph`: modules, dependencies and milliseconds (11.2).
- Known limits: a path inside a macro's tokens, a bare Rust path, a dynamic
  `import()` and `require()` are not dependencies in V1.

## Final self-enforcement

Klin's `layering` policy now enables `acyclic` over the module graph and pins
the runner, checks, syntax, project, catalogue, core and edge boundaries. A
resolved forbidden or cyclic edge is ratcheted; external and unsupported V1
forms are not guessed, and no accepted entry is used to make the policy green.

## Amendment: a bare path through a declared child module (#421)

The decision above counts a path from any name but `crate`, `self` or `super`
as external and never resolves it, and lists a bare Rust path as a known
limit. Since the 2018 edition a path may start with the name of a child
module, so a cycle closed through `pub use inner::X;` beside `mod inner;` went
unseen. A path whose first segment names a module the same file declares at
that path's nesting, directly in a module rather than inside a block, now resolves as if it started
with `self::`, in a `use` tree and outside an import. In a `use` tree of an
edition 2015 target the first segment starts at the target root, as rustc
reads it. Any other first segment stays external. A bare path through a name
no module of its file declares, and a name a block binds over a declared
child, remain known limits. SPEC.md 8.2.1 states the rule.

## Amendment: recognized TypeScript local paths (#445)

Direct exact and single-star `compilerOptions.paths` aliases in one conventional
ancestor `tsconfig.json`, with one target and a directly known baseUrl when
needed, now resolve using the existing conservative TypeScript candidate rule.
JSONC comments and trailing commas are supported. Exact matches precede the
longest wildcard prefix; equal-priority patterns are unresolved. A recognized
local alias that cannot be proved is a located graph hole, never an ordinary
package dependency. Held local extends files supply recognized names only.
Multiple/nested or alternate configs, extends, references and fallback targets
are not used to prove edges. Standalone baseUrl lookup, package resolution and
bundler aliases remain outside V1. Other-kind and ignored targets retain their
outside-V1 classification. Each tree reads its own held configs once.

Layering consumes relevant holes under the existing held/new semantics.
Public surface derivation consumes local alias holes at re-export sites; an
unrelated implementation import does not itself make a public contract
incomplete. Proven aliased re-exports follow `reached_at` like relative ones.
See SPEC 8.2.1 and the bounded survey in
`docs/typescript-aliases-2026-10-03.md`.


### Proof-boundary correction after PR #461 review

An ancestor config is insufficient proof of TypeScript project ownership.
Aliases now require proven root membership through explicit files or the
conservative include/exclude subset in SPEC 8.2.1. Sources outside that root
set remain held modules, but their recognized aliases stay locally unresolved;
V1 does not infer program membership through imports. Exclude filters include
roots and does not ban a file explicitly named by files.

Inherited paths recognition follows replacement, not additive merge: the
nearest paths object in a single held local extends chain supplies names.
An empty child object removes all inherited names. Array-form extends
supplies no inherited names; direct child rules remain recognized. Inheritance
still never proves an edge. Scope selection uses indexed ancestor directories
once per source file, and aliases retain their once-tree compiled lookup.
