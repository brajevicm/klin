<p align="center">
  <img src="assets/klin-logo.svg" alt="klin" width="280">
</p>

<p align="center">
  <strong>Quality ratchets for AI-driven development, in one binary.</strong>
</p>

> [!WARNING]
> **klin is early-stage and under active development.**
> The specification and CLI are still evolving.

Coding agents optimize for green. Sometimes the easiest route there is the
wrong one: skip a test, silence a check, leave a stub, grow an already-complex
function, or change the guardrails themselves.

**klin makes existing debt the floor, not the blocker.**

```text
existing debt     improvement       regression
     8                 6                 9
     ✓                 ✓                 ✗
```

It compares the tree before a change with the tree after it and rejects only
**new or worsened debt**. Existing problems do not have to be fixed before
adopting stricter quality gates.

## Install

### Claude Code

```text
/plugin marketplace add brajevicm/klin
/plugin install klin@klin
```

That is the whole install. The plugin carries the hooks, and its
[wrapper](plugins/claude-code/bin/klin) fetches the pinned release on first
use and caches it. No `init` step, and no binary to install by hand. Two
commands come with it: `/klin:gate` runs the gates over the changed files,
and `/klin:gates` lists every gate and what it measures.

### Every other host

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/brajevicm/klin/releases/latest/download/klin-installer.sh | sh
```

It verifies a checksum and puts `klin` in `~/.local/bin`.
[19.1](docs/SPEC.md) pins a version or moves that directory.

## What it checks

klin is not a linter or test runner. Its checks target agent-driven
development:

- **complexity** — functions that become more complex or longer
- **escapes** — new skipped tests, silenced checks, or swallowed errors
- **stubs** — placeholder work left behind
- **doc-size** — documents that keep growing
- **doc-citations** — references to files that no longer resolve
- **inventory** — important declarations that disappear
- **lockfile** and **SARIF** integration

## Local feedback, CI authority

Agent hooks give fast feedback during a turn, but an agent controls its own
working tree and can work around them. The authoritative verdict belongs in CI,
on a checkout the agent never touched.

klin targets **Claude Code first**. Cursor and Codex CLI adapters are planned.

## Development

```sh
cargo build --release
./target/release/klin gate --strict
```

## Design

The behaviour is defined by the
[specification](docs/SPEC.md), [ADRs](docs/adr/), and CLI tests.

Apache-2.0.
