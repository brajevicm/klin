# Working on detent

detent is a Rust rewrite of cleat, a quality-ratchet tool. Read `CONTEXT.md`
for the vocabulary and `docs/adr/` for decisions that are already made.

## The reference implementation

cleat is the behavioural specification. Find it at `$CLEAT_SRC`, defaulting to
`../cleat` beside this repository. It is not vendored here and must not be:
detent gates itself, and a vendored Python tree would be scanned and baselined.

That checkout is indexed with CodeGraph, so use it instead of grep:

    codegraph explore "<symbols or question>" --project $CLEAT_SRC

**Read cleat's tests and docstrings freely.** They are the specification.
`quality/bin/ratchet.py`'s module docstring states the five outcomes outright,
and the suites under `quality/tests/` pin every edge case.

**Read cleat's implementation only when behaviour is ambiguous**, and never
port it line by line. It is idiomatic Python and the idioms do not carry. For
example `ratchet.py` dispatches to a field by string name:

    getattr(verdict, compare(finding, entry, metrics)).append((finding, entry))

That is good Python. Transliterated it is bad Rust, where the same thing is an
enum and a `match`. Take the behaviour from the test that covers it, then write
Rust.

detent's numbers are not required to match cleat's. A ratchet compares today's
measurement against yesterday's measurement from the same tool, so being
self-consistent is the whole requirement. See ADR 0001.

## Tests

One seam: the binary's command line. Build a throwaway tree, write a
`klin.json` and any baseline, run the real binary, assert on the exit code
and the printed text. Do not reach inside. The matching logic is the most
likely thing to be rewritten, so nothing should be coupled to its shape.

## The rules detent enforces on itself

Do not edit `klin.json`, a baseline, or the hooks to make a gate pass, and
do not run `--write-baseline`. A baseline records debt a person accepted. Only
a person loosens it, in a reviewed commit.
