# Syntax owns the parser and a check reads facts

Every check that reads source through a grammar used to come to `complexity`
for it. `complexity` held the grammars, the parser, the function node kinds,
the unparsed file, the test convention and the stub body shapes; `inventory`
took `Test` and `Unparsed` from it, `markers` called its `stubs`, and the
survey took its extensions. Adding declarations, imports, references, exports
and public surfaces to that module would have made a cyclomatic-complexity
check the foundation of every syntax-aware feature klin plans.

## The decision

`syntax` owns the parser mechanics, and nothing above it builds a
`tree_sitter::Parser` or reads a node kind of a language's grammar:

- the language registry, and which grammar reads which path
- the parse, and the file the grammar rejected (ADR 0003)
- the function node kinds a language writes
- the test convention, the placeholder body shapes and the inline test module,
  which `inventory`, `stubs` and `escapes` share

`complexity` keeps complexity policy: the decision node kinds, the operators,
the cyclomatic calculation, the two ceilings and the findings. A check that
uses Tree-sitter does not thereby own syntax, and syntax does not thereby own
what a check counts.

`syntax::structural` turns a parse into semantic facts: declarations,
imports, module declarations and references. A consumer of those facts sees
no node, no node kind and no query. Adding a third language is an adapter
file and one registry arm, not a branch in a consumer.

### Rust and TypeScript come first, and TSX is TypeScript

The parser registry holds every grammar `complexity` already shipped, so the
extraction changed no measurement. Structural facts are Rust and TypeScript
only.

A logical language is not a grammar variant. `.ts`, `.mts` and `.cts` are read
by one grammar and `.tsx` by another, and both report the language
`TypeScript`. TSX is a name inside the registry, printed when that grammar
rejects a file, and `complexity` has long let a configuration select it as a
file set.

`structural::languages()` is where a structural check reads its configured
languages from, and `tsx` is not in what it returns. One list, so no consumer
holds a language name of its own and none of them has to refuse that one.

### Resolution is by name, and it is deliberately coarse

`SourceIndex` resolves a reference name to every declaration of that name
under the roots. No type inference, no import-aware lookup. A declaration
whose visibility is in doubt reads as exposed.

A name a binding site writes — a parameter, a `let`, a struct or class field —
reads as a reference. No adapter states its language's binding sites in V1,
and keeping them errs the same way: a declaration nothing uses stays alive.

So `dead-symbols` under-reports dead declarations and `reachability`
over-reports reached files. Ambiguity makes a gate fail less, never more,
which is the same direction spec 8.4 already fixed for the reference
extractor. A consumer that needs a sharper answer sharpens the adapter, not
the consumer.

Resolving an import or a `mod foo;` to a file is a separate capability, owned
by #50. The raw specifier, the module name and the file an attribute remapped
the module to are kept so that work needs no second parse. No consumer
resolves a specifier for itself in the meantime.

### Ambiguity judges a tree and never writes its policy

The coarse answer has two uses, and they are not the same. Judging a tree
against its base, a name several declarations share counts for every one of
them, because the failure that costs is the false one. Deriving a section
the tree will then be judged by, the same shared name is no evidence at all:
a family of files pinned as "always reached" on the strength of one
ambiguous reference would fail the first tree that drops the unrelated
declaration. So `reachability` derives a family only from members proven
reached through a name exactly one declaration holds, with a reference from
another file, and judges the tree afterwards by the coarse rule. A consumer
that derives policy from these facts holds to the same split.

A declaration carries the line it ends on as well as the line it starts on,
because a check that asks whether anything outside a declaration uses it
cannot ask without that. A consumer that already holds a parse reads the test
convention off it rather than parsing the file again.

### A file nothing measured says so

Structural analysis has its own capability boundary, above the one ADR 0003
draws. One file comes to exactly one of four outcomes, and three of them are
not a measurement: facts, a language no adapter reads, a text the grammar
rejected, and a path no grammar reads. A file the mode rule could not read is
an error, where every other check raises one.

A consumer cannot infer any of this from an empty list of declarations,
because an empty list is only ever handed back with the facts. A green
structural run over a tree klin cannot read is visibly a run over nothing.

## Consequences

A query belongs to the grammar it was compiled against, so the compiled query
lives in the language's own place in the registry: TypeScript and TSX hold one
each, built on first use and held for the process. No second table has to be
kept in step with the registry, so the one registry arm above is one arm.

Tree-sitter stays the only parser. No `syn`, no SWC, no Oxc, no
rust-analyzer.

An import keeps the specifier exactly as the source wrote it, so a Rust
`use a::{b, c}` keeps the list and a `use a::b as c` keeps the alias. Cutting
a module path out of either is resolution, which belongs to #50.

No new error type came with this. The one failure class a parse has beyond
the rejected file is a grammar that will not load, which already reported
through `config::Error`, and a single-variant enum would state nothing the
message does not. `thiserror` was available and is not used.

The structural facts are consumed by `dead-symbols` and `reachability`, and
will be shared by later structural checks. They are tested at the module
seam against hand-written Rust and TypeScript fixtures, and at the binary's
command line for everything a person can see. A new structural consumer reads
these facts rather than parsing again.

`survey` still reads `escapes::suffixes` and `escapes::language_of` to find a
tree's sources and name its languages. That table covers shell, which no
grammar here reads, so moving it would shrink what a survey discovers. It
stays where it is.

## Follow-up: the module graph resolves what syntax keeps (#50)

ADR 0043 now owns resolution. To give it enough to resolve without a second
parse, the Rust adapter keeps every leaf path of a use tree, the inline modules
that hold an import, a module declaration or a qualified path, inline module
declarations, and every path outside an import that starts at `crate`, `self`
or `super`. Cutting a path out of a use tree is therefore syntax, and deciding
which module that path names is the module graph's work.

## Amendment: a name a destructuring declaration binds is a write (#384)

The rule above reads a name a binding site writes as a reference. #384
narrows it for a TypeScript `const`, `let` or `var` destructuring
declaration: a name its pattern binds is no reference to that name, at the
top level and inside a function. The TypeScript adapter now states those
binding sites, which is what this ADR asks of a consumer that needs a
sharper answer.

A binding writes a name and reads none. Inside a function,
`const [first] = load()` writes a new local and never reads an outer
`first`, yet the V1 rule kept an unused top-level `const first` alive
through it. The same rule let two unused destructurings of one name keep
each other alive. Parameters, `for` heads, `catch` clauses, class and struct
fields, a Rust `let`, an assignment such as `[first] = load()`, and every
later use of a bound name still read as references.

`dead-symbols` judges a destructuring declaration by the names it binds, and
the index files the declaration under each of them. The derivation
`reachability` runs therefore has one exception to "a name exactly one
declaration holds": a destructuring binding is no second declaration of a
member's name. A lazy
`const { Login } = await import("./handlers/LoginHandler")` binds the
member's own export, and counting it would stop the family from being
derived.

`reachability` keeps reading those bindings as references. A destructuring
declaration in another file that binds a member's name reaches the member
and can prove it, because `const { default: Profile } = await import(…)`
loads the file whether `Profile` is used or not. Reading the binding as no
reference would report a loaded file as unreached, and a gate that fails
more on ambiguity goes against this ADR. The rule covers shorthand bindings
too, which V1 never read as references, so
`const { Profile } = await import(…)` now reaches a member that V1 left
unreached.

A destructuring inside a function body is no declaration, so the index holds
nothing for it, and a name only it binds reaches no member. For
`function boot() { const [Login] = list; }` that is right: the binding names
a local value, and V1's reach there was a false green. For an unused
`const { default: Profile } = await import(…)` inside a function it is a
false alarm, accepted as rare. Reading those bindings as well would take a
list of binding sites in every file's facts, carried through the structural
cache.
