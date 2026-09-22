<p align="center">
  <img src="assets/klin-logo.svg" alt="klin" width="280">
</p>

<h1 align="center">Quality control for coding agents.</h1>

<p align="center">
  <strong>Keep agents from getting green the wrong way.</strong>
</p>

<p align="center">
  Deterministic quality gates and feedback loops for Claude Code, Codex, Cursor, and other coding agents.
</p>

Coding agents are very good at getting to green.

Sometimes they get there by a shortcut you would never accept in review: deleting a failing test, skipping a check, swallowing an error, or leaving a stub.

**klin catches those regressions while the agent is still working and sends them back for repair.**

```text
agent works → klin checks the change → new regression?
                                 yes → failure returned → agent repairs
                                 no  → done → independent CI verifies
                                              the same quality policy
```

klin needs no clean codebase first. It compares what existed before the change with what exists after it and rejects only **new or worsened debt**.

```text
quality debt           before      after      result
unchanged                  8          8         ✓ pass
improved                   8          6         ✓ pass
worsened                   8          9         ✗ fail
```

**Existing problems don't block adoption. klin only stops the change when the measured quality gets worse.**

## What klin catches

klin looks for a change that passes while making the codebase worse.

- **Tests disappear** — a failing test is deleted instead of fixed.
- **Checks get silenced** — tests skipped, warnings suppressed, errors swallowed.
- **Work is left unfinished** — TODOs, placeholders, and other stubs remain.
- **Complexity grows** — an already-complex function gets another branch.
- **Dependencies drift** — code relies on a dependency without the lock state.
- **Code is added but never wired in** — it looks finished, but nothing reaches it.
- **Internal APIs leak outward** — a private symbol goes public to make a change work.
- **Project conventions are ignored** — exact repository-specific rules are violated.
- **Architecture drifts** — dependencies cross boundaries or introduce new cycles.
- **Documentation falls behind** — code moves while references to it stay stale.
- **Changed code loses test coverage** — behavior changes, tests do not.

These are deterministic checks. klin does not ask an LLM whether code is "good"; it measures specific regressions and gives the agent a concrete failure to repair.

## Quick start

**First-class integrations:** Claude Code, Codex and Cursor. klin owns the native plugin, the host adapter, the tests and the compatibility notes for each one.

**Other coding agents:** integrate through klin's [generic harness contract](docs/HARNESS_INTEGRATION.md).

On a first-class host the native plugin is the install: it carries the hooks, klin's agent skill and a pinned runtime, so it needs no second binary. Its first run downloads that runtime, verifies its checksum, and caches it; where the download fails it runs a `klin` on your PATH, and with neither it says so and lets the turn end.

### Claude Code

```text
/plugin marketplace add brajevicm/klin
/plugin install klin@klin
```

The plugin, and the settings that enable it, live on your machine. A remote or cloud session does not see them; commit the project integration there.

### Codex

```sh
codex plugin marketplace add brajevicm/klin
codex plugin add klin@klin
```

Codex does not trust a plugin's hooks when it installs them. Run `/hooks`, review the klin hook sources, trust them, and start a fresh session so the hooks run. The Codex IDE extension loads no plugins; use the standalone route there.

### Cursor

Copy the plugin and reload the window. Run it twice, one copy remains:

```sh
d=$(mktemp -d) && git clone --depth 1 https://github.com/brajevicm/klin "$d"
rm -rf ~/.cursor/plugins/local/klin
mkdir -p ~/.cursor/plugins/local
cp -R "$d/plugins/klin" ~/.cursor/plugins/local/klin
rm -rf "$d"
```

Cursor Teams can import `https://github.com/brajevicm/klin` under Dashboard → Plugins → Team Marketplaces. klin has recorded no verification of that route.

A local plugin, hook or skill stays on that machine. Cursor Cloud Agents load a repository's committed hooks and skills, not a person's own, so use the project integration there ([notes](docs/cursor-compatibility.md)).

## Activate this repository

Installing a plugin makes klin available to every repository the host opens, and opts none of them in. The marker is a `klin.json` at the repository root, and `{}` is a complete marker:

```sh
echo '{}' > klin.json
```

Without it, hook-mode klin stays deliberately silent. Writing it needs no standalone binary; on the standalone route `klin install` writes it.

That's enough for local feedback: when the agent finishes a turn, klin checks what changed and returns a new regression for repair.

## Standalone / managed installation

The standalone binary is klin's portability layer, not an equal second default. Take it where no plugin loads, where the team wants committed hook files under review, where a managed environment needs explicit files, or for a custom harness.

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/brajevicm/klin/releases/latest/download/klin-installer.sh | sh
```

Releases ship for macOS and Linux on x86_64 and arm64. Native Windows is unsupported, and WSL is not a documented route.

`klin install` then opts the repository in and reconciles the explicit hook files and klin's skill for every host it can prove. `--host claude|codex|cursor` names one host. Project scope is the default, and those files are committed. `--user` writes one person's files on one machine: local user scope, and no promise of cloud, team or organization coverage. Where a native plugin already owns a host, klin writes no duplicate.

Both routes install the same canonical skill; the standalone route copies it alone, not the plugin's slash commands, and a skill file you edited is reported, never overwritten.

## Other coding agents

Another harness integrates through klin's versioned generic lifecycle contract. It may be promoted to first-class later, once its native semantics are proven and maintained. Speaking that contract is not first-class support. See [porting klin to another harness](docs/HARNESS_INTEGRATION.md).

## CI enforcement

For independent enforcement, add klin to CI:

```yaml
- uses: brajevicm/klin@v0.2.1
```

```text
while the agent works     → fast feedback and repair
before the change merges  → independent verification in CI
```

## Updating klin

**Native plugin.** Update through the host's plugin mechanism, such as `/plugin marketplace update`. The plugin version owns the runtime it pins, so updating a PATH binary does not change it.

**Standalone binary.** `klin update` installs the newest release over the current one. Rerun `klin install` afterwards to reconcile the hook files and the skill.

## Conformance levels

**Feedback** — hooks only. klin puts every new regression in front of the agent during the turn and refuses its edits to `klin.json` and to klin's own state. This is local assistance. An agent that controls the worktree can still work around it.

**Enforced** — Feedback plus a required `klin gate --strict` run on an independent CI checkout, against a protected branch, with `klin.json`, the workflow and CODEOWNERS under review. Only at this level does a gate hold against an agent, and loosening it takes a reviewed commit.

[What each level guarantees, and where the boundaries are](docs/THREAT_MODEL.md).

## Built for the agent loop

**Turn-aware.** klin judges the change against the repository as the agent's turn began, so the feedback is about this change alone.

**Ratcheted.** Existing debt is held and improvements pass, so a real codebase adopts strict checks without a cleanup project first.

**Verified again in CI.** Local hooks are feedback, not a security boundary. The same policy runs again in independent CI before merge.

**The agent gets the mechanical failures. Humans keep the judgment calls.**

## Works with your existing tools

Your linters, tests, type checkers, coverage tools, and security scanners are good at **finding problems**. klin takes a deterministic finding, puts it in front of the coding agent, and checks the repair. Use the best tool for each kind of analysis; klin adds the turn-aware ratchet, the feedback loop, and CI verification around those signals.

## Does it actually help?

klin is designed to improve the final changes produced by coding agents, not just to add more checks.

Before 1.0, we compare the same agent, task, and starting repository in two modes:

- **Shadow** — klin observes the work but does not send feedback to the agent.
- **Active** — klin sends failures back during the turn so the agent can repair them.

Each task is judged by an external oracle rather than by klin itself.

The full methodology and results are published separately, so positive, negative, and mixed outcomes remain visible.

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
- [Porting klin to another harness](docs/HARNESS_INTEGRATION.md)
- [Host compatibility](docs/HOST_COMPATIBILITY.md)
- [Shadow/Active benchmark result, 2026-09-22](docs/benchmark-result-2026-09-22.md)

Found a problem or have a question? [Open an issue](https://github.com/brajevicm/klin/issues/new/choose).  
Security issue? [Follow the private reporting instructions](SECURITY.md).

Apache-2.0.
