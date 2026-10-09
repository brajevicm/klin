# Working on klin

klin is a quality ratchet for AI-driven development. Read `CONTEXT.md`
for the vocabulary and `docs/adr/` for decisions that are already made.

## Behavioural specification

Before changing a check, read its contract in `docs/SPEC.md`, the
applicable decisions in `docs/adr/`, and its CLI tests under `tests/`. These
are klin's authority; ADR 0025 records the scope of that authority.
`docs/SPEC.md` is the one spec. Its Appendix B holds the 0.x rules that vNext
carries forward, and ADR 0066 records how to read them.

When behaviour is unspecified or a test conflicts with the contract, resolve
it against klin's requirements and record the intended result in the spec and
a CLI test. Use idiomatic Rust to implement that result.

Measurements must be deterministic and self-consistent across the two trees
measured by one binary. See ADR 0001.

## Tests

One seam: the binary's command line. Build a throwaway tree, write a
`klin.json`, commit a base, run the real binary, assert on the exit code and
the printed text. Do not reach inside. The matching logic is the most likely
thing to be rewritten, so nothing should be coupled to its shape.

The harness gives every tree a repository whose base holds nothing, so every
finding is new. `tree.base()` makes the tree as it stands the base.

One exception: an architecture invariant that no shipped resolver or adapter
can reach yet may be pinned by a `#[cfg(test)]` test over a structure built in
memory, such as a module of several files before a language groups files
(#220). Name such tests in `docs/SPEC.md` as the pins, and move the invariant
to a CLI test once a shipped language reaches it.

## The rules klin enforces on itself

Do not edit `klin.json` or the hooks to make a gate pass, and do not add an
entry to the `accepted` list. That list records debt a person accepted. Only a
person writes it, in a reviewed commit. A gate that fails names code to fix.

## Agent skills

### Issue tracker

Issues live as GitHub issues in `brajevicm/klin`, through the `gh` CLI. See
`docs/agents/issue-tracker.md`.

### Triage labels

The five canonical roles, each label string equal to its name. See
`docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` and `docs/adr/` at the repository root. See
`docs/agents/domain.md`.
