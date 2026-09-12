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
`tree_sitter::Parser`:

- the language registry, and which grammar reads which path
- the parse, and the file the grammar rejected (ADR 0003)
- the function node kinds a language writes
- the test convention and the placeholder body shapes, which `inventory` and
  `stubs` share

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
rejects a file; it is never a structural language a configuration or a
consumer can name.

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
by #50. The raw specifier and the module name are kept so that work needs no
second parse.

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

The structural facts have no consumer until #51 or #52 lands, so they are
tested where they can be: at the module seam, against hand-written Rust and
TypeScript fixtures. The binary's command line stays the one seam for
everything a person can see, and the first structural gate brings these facts
to it. Until then the module carries a lint expectation, not a lint
allowance, so the compiler names the line the moment it has a caller.

`survey` still reads `escapes::suffixes` and `escapes::language_of` to find a
tree's sources and name its languages. That table covers shell, which no
grammar here reads, so moving it would shrink what a survey discovers. It
stays where it is.
