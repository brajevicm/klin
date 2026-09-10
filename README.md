# klin

Quality ratchets for AI-driven development, in one binary. It records today's
debt, then fails the build on anything new. Supports Claude Code, Cursor and
Codex CLI.

## Only CI is authoritative

The guard refuses the agent's edits to the config, the hooks and CODEOWNERS,
and asks a person about a command it cannot read as a write, about klin's own
state, and about a file that configures a check.
It is feedback
and cannot stop an agent that works around it. It sees none of these routes:

- the binary, which a PATH shim or `chmod -x` defeats
- a deleted test, the cheapest route to green
- a moved local ref, which moves the base a local run compares against
- a deleted config section, which runs no gate and reads green

The verdict is a CI run on a checkout the agent never touched, against a
protected branch. Name every gate on the CI command line, and put the workflow
and the config under CODEOWNERS. ADR 0008 and 0009 record why.

## Accepting debt

A person adds one line to the `accepted` list in the config, in a reviewed
commit. Nothing else records debt, and klin writes no files.

## Specification and license

[The specification](docs/SPEC.md) defines klin's behaviour, [ADRs](docs/adr/)
record its decisions, and CLI tests pin the expected results. See
[ADR 0025](docs/adr/0025-klin-defines-its-own-behaviour.md).

Apache License 2.0, with copyright notices in [LICENSE](LICENSE).
