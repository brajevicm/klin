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

The plugin carries the hooks. Its [wrapper](plugins/claude-code/bin/klin)
fetches and caches the pinned release. `/klin:gate` runs changed-file gates;
`/klin:gates` lists them.

### Codex CLI

The same plugin installs from the repository's marketplace:

```sh
codex plugin marketplace add brajevicm/klin
codex plugin add klin@klin
```

Codex asks you to review the hooks once. The IDE extension loads no plugins.

A team that wants the hooks committed, where CODEOWNERS covers them, installs
the binary as below and writes the project hook file:

```sh
klin init --hooks --host codex
```

`--global` writes the user-level file instead. Neither form duplicates an
enabled plugin. Both hook-file forms reach the Feedback level below.

### Standalone binary

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/brajevicm/klin/releases/latest/download/klin-installer.sh | sh
```

The installer verifies its checksum and writes to `~/.local/bin`; [§19.1](docs/SPEC.md)
describes pinning a version or another directory.

### Update

Update the plugin with `/plugin marketplace update` or Claude's auto-update;
its wrapper fetches the new version. Use `klin update` for standalone installs.
Move CI's `uses` tag manually or with Renovate or Dependabot. One tag names
every route ([ADR 0029](docs/adr/0029-the-release-tag-is-the-one-version.md)).

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

## Conformance levels

klin supports **Claude Code and Codex CLI**.

### Feedback

Hooks put failures in front of the agent during a turn. A gate failure keeps
the turn window open until it is fixed, accepted, or reset by a person. A
deleted test is asked about once, then reported to the person while the next
stop ends green.

The guard protects `klin.json` and klin's own state directory. Hook files,
CODEOWNERS, verification files and the stamp refs are ordinary files. An agent
controls its working tree and can remove or bypass local hooks, so this level
is feedback, not enforcement. Installing the plugin reaches this level.

### Enforced

Enforcement adds `klin gate --strict` in CI, on a checkout the agent never
touched and against a protected branch. CODEOWNERS must cover `klin.json`, the
workflow, the hook settings, and CODEOWNERS itself. Loosening a gate then takes
a reviewed commit by a person.

A deleted test remains a note in CI. It is held only by the hook's one question
and a review of the diff.

### CI setup

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

The `cut-release` workflow accepts `patch`, `minor`, `major`, or an exact
version. It updates every version reference, commits, tags, pushes, and lets
the tag publish the release. The equivalent local command is:

```sh
cargo release minor --execute
```

## Design

The behaviour is defined by the [specification](docs/SPEC.md),
[ADRs](docs/adr/), and CLI tests. `klin reference` prints the
[configuration reference](docs/REFERENCE.md), every key klin reads.

Apache-2.0.
