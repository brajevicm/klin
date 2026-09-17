<p align="center">
  <img src="assets/klin-logo.svg" alt="klin" width="280">
</p>

<h1 align="center">Quality control for coding agents.</h1>

<p align="center">
  <strong>Keep agents from getting green the wrong way.</strong>
</p>

<p align="center">
  Deterministic quality gates and feedback loops for Claude Code, Codex, and other coding agents.
</p>

Coding agents are very good at getting to green.

Sometimes they get there by taking a shortcut you would never accept in review: deleting a failing test, skipping a check, swallowing an error, leaving a stub, making an internal API public, or changing code without wiring it into the product.

**klin catches those regressions while the agent is still working and sends them back for repair.**

```text
             ┌──────────────────────────────┐
             │                              │
             ▼                              │
      coding agent works                    │
             │                              │
             ▼                              │
      klin checks the change                │
             │                              │
             ▼                              │
     new regression found?                  │
          │          │                      │
         no         yes                     │
          │          │                      │
          ▼          └── failure returned ──┘
        done
          │
          ▼
 independent CI verifies
   the same quality policy
```

klin does not require a clean codebase first. It compares what existed before the change with what exists after it and rejects only **new or worsened debt**.

```text
quality debt           before      after      result
unchanged                  8          8         ✓ pass
improved                   8          6         ✓ pass
worsened                   8          9         ✗ fail
```

**Existing problems don't block adoption. klin only stops the change when the measured quality gets worse.**

## What klin catches

klin focuses on changes that can look green while still making the codebase worse.

- **Tests disappear** — a failing test is deleted instead of the behavior being fixed.
- **Checks get silenced** — tests are skipped, warnings are suppressed, or errors are swallowed.
- **Work is left unfinished** — TODOs, placeholders, empty implementations, and other stubs remain behind.
- **Complexity grows** — an already-complex function gets another branch instead of being simplified.
- **Dependencies drift** — code starts relying on a dependency without the corresponding lock state.
- **Code is added but never wired in** — finished-looking files or symbols exist, but nothing can reach them.
- **Internal APIs leak outward** — an existing private/internal symbol becomes public just to make a change work.
- **Project conventions are ignored** — exact repository-specific rules are violated.
- **Architecture drifts** — dependencies cross boundaries or introduce cycles they did not before.
- **Documentation falls behind** — code moves while references to it remain stale.
- **Changed code loses test coverage** — the implementation changes without equivalent exercised behavior.

These are deterministic checks. klin does not ask an LLM whether code is "good"; it measures specific regressions and gives the agent a concrete failure to repair.

For everything else, keep using the tools that already do it well: linters, type checkers, test runners, security scanners, and human review.

## Quick start

Install the klin plugin for your coding agent.

### Claude Code

```text
/plugin marketplace add brajevicm/klin
/plugin install klin@klin
```

### Codex

```sh
codex plugin marketplace add brajevicm/klin
codex plugin add klin@klin
```

Codex does not trust a plugin's hooks when it installs them. Run `/hooks`, review the klin hook sources, trust them, and then start a fresh session so the hooks run.

### Cursor

Teams: Dashboard → Plugins → Team Marketplaces → import
`https://github.com/brajevicm/klin`, then install klin. Otherwise copy the
plugin and reload:

```sh
git clone https://github.com/brajevicm/klin /tmp/klin
mkdir -p ~/.cursor/plugins/local
cp -R /tmp/klin/plugins/klin ~/.cursor/plugins/local/klin
```

Then opt the repository into klin. The marker is a `klin.json` at the
repository root, and `{}` is a complete one:

```sh
echo '{}' > klin.json
```

That's enough for local feedback. When the agent finishes a turn, klin checks what changed and returns new regressions for repair.

The first run downloads the `klin` release the plugin pins, verifies its checksum, and caches it. Every later run executes the cached binary. If that download fails, the plugin runs a `klin` on your PATH instead, and where there is none it says so and lets the turn end rather than blocking it.

For independent enforcement, add klin to CI:

```yaml
- uses: brajevicm/klin@v1
```

```text
while the agent works     → fast feedback and repair
before the change merges  → independent verification in CI
```

The plugin is the preferred install. A standalone binary plus committed hooks is the fallback, for teams that want to manage the integration themselves.

## Conformance levels

**Feedback** — hooks only. klin puts every new regression in front of the agent during the turn and refuses its edits to `klin.json` and to klin's own state. This is local assistance. An agent that controls the worktree can still work around it.

**Enforced** — Feedback plus a required `klin gate --strict` run on an independent CI checkout, against a protected branch, with `klin.json`, the workflow and CODEOWNERS under review. Only at this level does a gate hold against an agent, and loosening it takes a reviewed commit.

[What each level guarantees, and where the boundaries are](docs/THREAT_MODEL.md).

## Built for the agent loop

**Turn-aware.** klin judges the change against the state of the repository when the agent's turn began, keeping feedback focused on what this agent just changed.

**Ratcheted.** Existing debt is held. Improvements pass. Only new or worsened debt fails, so real codebases can adopt strict checks without a cleanup project first.

**Verified again in CI.** Local hooks are fast feedback, not a security boundary. The same policy runs again in independent CI before merge. See the [trust model](docs/THREAT_MODEL.md).

**The agent gets the mechanical failures. Humans keep the judgment calls.**

## Works with your existing tools

klin is not trying to replace your linters, tests, type checkers, coverage tools, or security scanners. Those tools are good at **finding problems**. klin is concerned with what happens next:

```text
linter / test / scanner / policy
              ↓
      deterministic finding
              ↓
             klin
              ↓
      coding agent gets it
              ↓
           repairs
              ↓
          checked again
```

Use the best tool for each kind of analysis. klin adds the turn-aware ratchet, feedback loop, and CI verification around those signals.

## Does it actually help?

klin is designed to improve the final changes produced by coding agents, not just to add more checks.

Before 1.0, we compare the same agent, task, and starting repository in two modes:

- **Shadow** — klin observes the work but does not send feedback to the agent.
- **Active** — klin sends failures back during the turn so the agent can repair them.

Each task is judged by an external oracle rather than by klin itself.

The comparison looks at correctness, shortcuts left in the final tree, useful repairs, unwanted interventions, repair work, and runtime overhead.

The full benchmark methodology and results are published separately so positive, negative, and mixed outcomes remain visible.

## What klin is not

klin is one layer of an agentic engineering system, not the whole stack. It is **not**:

- an AI code reviewer;
- a replacement for tests, linters, or type checkers;
- a general-purpose SAST or vulnerability scanner;
- a sandbox or security boundary for coding agents;
- a hosted dashboard or agent orchestration platform.

klin handles the deterministic quality-control loop around code changes. Humans still own requirements, architecture, judgment, and review.

## Documentation

- [Configuration reference](docs/REFERENCE.md)
- [Trust model](docs/THREAT_MODEL.md)

Apache-2.0.
