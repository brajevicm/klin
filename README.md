# klin

Quality ratchets for AI-driven development, in one binary. It records today's
debt, then fails the build on anything new. Supports Claude Code, Cursor and
Codex CLI.

## Only CI is authoritative

The guard refuses the agent's edits to the config, the hooks and CODEOWNERS.
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

## Origin and license

A Rust rewrite of [cleat](https://github.com/svetdev/cleat) by Andrey
Kasatkin. MIT, with both copyrights in [LICENSE](LICENSE). `docs/adr/` records
what changed.
