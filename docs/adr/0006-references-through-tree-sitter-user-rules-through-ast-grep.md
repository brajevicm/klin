# References read through tree-sitter, user-written rules through ast-grep

cleat reads references through ast-grep for its layering, reachability and
dead-symbols checks. ADR 0001 already refused to bundle a foreign runtime for
complexity, and the obvious move was to refuse it again here. That reasoning
does not survive contact with the crate.

`ast-grep-core` is a Rust library, not a subprocess, and it resolves to
tree-sitter 0.27.0, the version this tree already pins. Cargo unifies them, so
there is no second parser and no duplicated grammar. Measured against a probe
binary holding only tree-sitter and the Rust grammar, it adds 96 KB. Over a
20,922-byte file it parses in 1.71 ms against 2.40 ms for a tree-sitter parse
plus this tree's own node-kind walk, and matching a pattern against an
already-parsed tree costs 0.50 ms. The 50x parse cost cleat recorded compared
ast-grep against regex over stripped source, and does not apply where both
sides are tree-sitter.

So the question is not cost. It is who writes the matcher.

For layering, reachability and dead-symbols the matcher is ours. Those gates
need declarations, identifiers and imports, which is the same node-kind lookup
the complexity gate already does. A pattern language buys nothing there, and
those three stay on tree-sitter directly.

For the conventions gate the matcher is the user's. A rule like "import the
client, not the vendor SDK" is painful as a regex and one line as a pattern.
That gate gets `ast-grep-core`.

`ast-grep-language` supplies per-language pattern pre-processing and is not
taken. It drags 29 grammar crates against this tree's 10, including a second
Kotlin grammar beside the one already here. The adapter is written instead: one
implementation over the existing grammar table, because that table already
holds the `tree_sitter::Language` the trait asks for.

## A pattern that matches nothing looks like a rule that passes

With the default pattern pre-processing, two of five probe patterns returned
zero matches with no error: `if $COND { $$$BODY }` and
`fn $NAME($$$ARGS) -> $RET { $$$BODY }`. Expression and statement patterns
worked. Block-bearing constructs did not.

A convention rule that silently matches nothing is a green gate measuring
nothing, which is the failure this tool exists to prevent, and it is worse here
than anywhere else because the pattern came from a user rather than from us.

So a rule carries an `example`, and its own matcher must match it. A rule whose
pattern does not match its own example is a config error naming the rule. This
is optional for a regex rule, where it keeps cleat's configs valid, and
required for a structural one.

## Consequences

A conventions rule carries `pattern` or `structural`, never both. `pattern`
stays exactly cleat's regex key, so cleat's rules run unchanged and the
differential test still reaches them. `structural` is additive and ours alone.

`ast-grep-core` is pre-1.0 at 0.45.x. Expect breaking changes on upgrade.
