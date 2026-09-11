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

### Codex CLI

The same plugin installs from the repository's marketplace:

```sh
codex plugin marketplace add brajevicm/klin
codex plugin add klin@klin
```

Codex asks you to review the plugin's hooks once. After that the install is
Claude Code's: the plugin carries the hooks, and its wrapper fetches the
pinned release on first use. The Codex IDE extension loads no plugins.

Codex rejects an `ask` on `PreToolUse`, so an ambiguous guard decision is a
block (exit 2) with the reason on stderr. Allowed calls exit 0.

A team that wants the hooks committed, where CODEOWNERS covers them, installs
the binary as below and writes the project hook file:

```sh
klin init --hooks --host codex
```

`--global` writes the user-level file instead. Both write nothing while the
plugin is enabled, so klin never runs twice on an event.

### Every other host

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/brajevicm/klin/releases/latest/download/klin-installer.sh | sh
```

It verifies a checksum and puts `klin` in `~/.local/bin`.
[19.1](docs/SPEC.md) pins a version or moves that directory.

### Update

Each route updates with the tool it came from:

- The plugin, with `/plugin marketplace update`, or on its own when Claude
  Code's plugin auto-update is on. The wrapper fetches the new version on the
  next run.
- The installer or a downloaded binary, with `klin update`.
- CI, by moving the tag in `uses: brajevicm/klin@vX.Y.Z`, which Renovate and
  Dependabot do.

One tag names every route, so they never disagree. [ADR 0029](docs/adr/0029-the-release-tag-is-the-one-version.md).

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

klin targets **Claude Code and Codex CLI**. A Cursor adapter is planned.

### CI

```yaml
steps:
  - uses: actions/checkout@v5
    with:
      fetch-depth: 0
  - uses: brajevicm/klin@v0.1.0
```

The action installs the release its tag names, or the `version` that
`klin.json` pins, and runs `klin gate --strict`. An `args` input appends
flags such as `--sarif PATH`.

## Development

```sh
cargo build --release
./target/release/klin gate --strict
```

A release is the `cut-release` workflow under Actions. Press "Run workflow",
pick `patch`, `minor` or `major`, or type an exact version, and it runs
`cargo release` on `main`. That moves the version in `Cargo.toml`, the
plugin, its wrapper and this file, commits, tags `vX.Y.Z` and pushes. The tag
builds and publishes the release. The same command runs locally with
[cargo-release](https://github.com/crate-ci/cargo-release) installed:

```sh
cargo release minor --execute
```

## Design

The behaviour is defined by the
[specification](docs/SPEC.md), [ADRs](docs/adr/), and CLI tests.

Apache-2.0.
