# Public API is derived from library and package entry points

> ADR 0054 amends "Judgement, holes and the failure model": a changed measured
> contract fails unless every difference is a change ADR 0054 classifies as
> compatible. "Widening" there means a widened visibility, never a widened
> type. ADR 0050 follows this record for publication metadata.

Issue #46 names one agent failure: a local refactor changes a contract that
another crate or package consumes, while the repository still compiles and
its own tests still pass. Spec 8.4 listed `public-api` as a tier-2 check over
the reference extractor. #49 shipped the extractor, #50 the module graph, and
#195 and #196 made the facts readable without a name index, so the gate is
built now, over those layers and nothing else.

## The decision

### Klin derives public API, and a person configures nothing

The section is absent, or `false` to exclude the gate. Any object under it is
a config error. Roots, languages, package lists, entry points, ignore lists
and a `no_widening` flag were considered and are not V1 configuration: each
of them is a fact of the tree, or a policy that already has a home. An
intentional break is an accepted entry, as for every other gate. There is no
public-api baseline and no ignore list of its own.

### One new layer, above the two that exist

`surface` converts structural facts and module facts into consumer-facing
surfaces and items. It sits above `syntax::structural` and `modules`, because
it consumes both, and below `public_api`, which owns before/after judgement,
accepted debt, reporting and the command line. The dependency direction is:

```text
syntax::structural -> modules -> surface -> public_api
```

Tree-sitter node kinds stay inside `syntax::structural`. Module and target
resolution stays inside `modules`. `public_api` holds neither.

### The structural adapters extract what a surface needs, once

`FileFacts` grew the smallest facts that express exposure without another
parse: a declaration's own `Visibility`, the inline modules that hold it, the
name it is exported under where that differs from its own, the type an
inherent `impl` adds a method to, and its canonical declared `signature`. A
module declaration carries its visibility. An `Export` fact lists what a
`pub use` or a TypeScript `export` clause exposes, leaf by leaf, with the
external name of each. The structural cache writes them all, and its epoch
rose to 3.

Canonicalization belongs to the language adapter, not to the check: it drops
bodies, initializers, comments and attributes, keeps one space between
tokens, drops a private field or member, and writes a binding name that is
not contract as `_`. A type the compiler would infer is written as `?`, so a
partial contract is visibly partial and never fabricated from a body.

### The module graph exposes what the surface must not infer again

`modules` now names each target's package, crate name, kind, root file and
manifest, and each module's parent, children, nesting and target. One
`resolve` answers where a path from a module ends up: at a module with the
segments left after it, outside the crate, or nowhere. `reached_at` names the
modules the dependencies of one line resolve to, so a TypeScript re-export
follows the graph's own edge. Nothing in `surface` resolves a path or a
specifier for itself.

### Identity is what a consumer writes

A Rust surface is a Cargo library target, named by its crate name. A binary
target and a Rust file no target reaches are not surfaces. A TypeScript
surface is one subpath of a package's explicit entry points: an `exports`
target that reduces to one checked-in source file, or the first of `types`,
`typings`, `main` and `module` that names one. Generated JavaScript is never
mapped back to source and `src/index.ts` is never guessed. A package none of
whose entries names a supported source is not applicable and is said so.

An item is the surface, the exported path or name and the item's kind. Its
declaring file and line are explanation only, so a move behind an unchanged
identity passes and a package move stays a removal and an addition. An item is
measured where its contract is canonical and opaque where klin proves only
that it exists; an opaque item's normalized clause is compared where that
clause is the contract.

### Judgement, holes and the failure model

Base and working tree are derived independently, each under its own topology
and metadata. A missing base surface fails once at the surface. A missing
item, a changed measured contract, a contract no longer declared and a
changed opaque clause fail. Additions, widening and an opaque item that
became measured pass. Each break carries `break` at 1; the base holds no
break, so the accepted list is the only way through.

The gate refuses to guess, so its holes are visible: a glob or star over
another crate or package, a name two globs or two stars provide, an export
form klin recognizes and cannot list, and a module or specifier the graph
could not resolve inside a surface. A hole is a NOTE in the hook and exit 2
elsewhere, while other findings still print, as ADR 0021 has it for every
structural gate.

## Consequences

- `public-api` is an Automatic check with `Needs::TheCommit` and no scope. A
  changed run judges every file of both trees, because a manifest or a
  re-export can change what an unchanged file means, and it takes the base's
  facts for unchanged files as `layering` does, so it reads and parses no
  unchanged source in a warm hook.
- `klin public-api --report` prints the derived contract of the working tree,
  so automatic derivation is inspectable.
- A gate row carries `surface`: surfaces, items, measured, opaque, holes and
  milliseconds (11.2), beside `graph`.
- Known limits: a module bound by `use` and then re-exported by its bare
  name is opaque, a renamed generic parameter is a changed contract, trait
  implementations, macros, `cfg` evaluation, `typesVersions`, conditional
  exports that do not reduce to one file, `tsconfig` paths and package
  aliases are outside V1.
