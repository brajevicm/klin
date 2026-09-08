# Working on klin

klin is a Rust rewrite of cleat, a quality-ratchet tool. Read `CONTEXT.md`
for the vocabulary and `docs/adr/` for decisions that are already made.

## The reference implementation

cleat is the behavioural specification. Find it at `$CLEAT_SRC`, defaulting to
`../cleat` beside this repository. It is not vendored here and must not be:
klin gates itself, and a vendored Python tree would be scanned as its own debt.

That checkout is indexed with CodeGraph, so use it instead of grep:

    codegraph explore "<symbols or question>" --project $CLEAT_SRC

**Read cleat's tests and docstrings freely.** They are the specification.
Cleat's `$CLEAT_SRC/quality/bin/` ratchet.py docstring states the five
outcomes outright, and the suites under `quality/tests/` pin every edge case.

**Read cleat's implementation only when behaviour is ambiguous**, and never
port it line by line. It is idiomatic Python and the idioms do not carry. For
example ratchet.py dispatches to a field by string name:

    getattr(verdict, compare(finding, entry, metrics)).append((finding, entry))

That is good Python. Transliterated it is bad Rust, where the same thing is an
enum and a `match`. Take the behaviour from the test that covers it, then write
Rust.

klin's numbers are not required to match cleat's. A ratchet compares today's
measurement against yesterday's measurement from the same tool, so being
self-consistent is the whole requirement. See ADR 0001.

## Tests

One seam: the binary's command line. Build a throwaway tree, write a
`klin.json`, commit a base, run the real binary, assert on the exit code and
the printed text. Do not reach inside. The matching logic is the most likely
thing to be rewritten, so nothing should be coupled to its shape.

The harness gives every tree a repository whose base holds nothing, so every
finding is new. `tree.base()` makes the tree as it stands the base.

## The rules klin enforces on itself

Do not edit `klin.json` or the hooks to make a gate pass, and do not add an
entry to the `accepted` list. That list records debt a person accepted. Only a
person writes it, in a reviewed commit. A gate that fails names code to fix.
