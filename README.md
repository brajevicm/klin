<p align="center">
  <img src="assets/klin-logo.svg" alt="klin" width="280">
</p>

<h1 align="center">Quality control for coding agents.</h1>

<p align="center">
  <strong>Deterministic quality gates and feedback loops for Claude Code, Codex, Cursor, and other coding agents.</strong>
</p>

Coding agents are very good at getting to green — sometimes by a shortcut you would never accept in review: deleting a failing test, skipping a check, swallowing an error, or leaving a stub.

**klin catches those regressions while the agent works, and sends them back for repair.**

```text
agent works → klin checks the change → new regression?
                                 yes → failure returned → agent repairs
                                 no  → done → CI verifies
```

klin needs no clean codebase first. It compares what existed before the change with what exists after, and rejects only **new or worsened debt**.

```text
quality debt           before      after      result
unchanged                  8          8         ✓ pass
improved                   8          6         ✓ pass
worsened                   8          9         ✗ fail
```

**Existing debt doesn't block adoption.** klin measures against the repository as the agent's turn began, so the feedback is about this change alone.

**The agent gets the mechanical failures. Humans keep the judgment calls.**

## What klin catches

klin looks for a change that passes while making the codebase worse.

- **Tests disappear** — a failing test is deleted, not fixed.
- **Checks get silenced** — tests skipped, warnings suppressed, errors swallowed.
- **Work is left unfinished** — TODOs and stubs.
- **Complexity grows** — a complex function gets another branch.
- **Dependencies drift** — a dependency is used without the lock state.
- **Code is added but never wired in** — nothing reaches it.
- **Internal APIs leak outward** — a private symbol goes public.
- **Project conventions are ignored** — repository-specific rules are violated.
- **Architecture drifts** — dependencies cross boundaries or add cycles.
- **Documentation falls behind** — code moves, references stay stale.
- **Changed code loses test coverage** — behavior changes, tests do not.

These are deterministic checks. klin does not ask an LLM whether code is "good"; it measures a specific regression and hands the agent a concrete failure.

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
- uses: brajevicm/klin@v1
```

Local hooks are feedback, not a security boundary: the same policy runs again in an independent checkout before merge.

## Updating klin

**Native plugin.** Update through the host's plugin mechanism, such as `/plugin marketplace update`. The plugin version owns the runtime it pins, so updating a PATH binary does not change it.

**Standalone binary.** `klin update` installs the newest release over the current one. Rerun `klin install` afterwards to reconcile the hook files and the skill.

## Conformance levels

**Feedback** — hooks only. klin puts every new regression in front of the agent during the turn, and refuses its edits to `klin.json` and to klin's own state. This is local assistance: an agent that controls the worktree can still work around it.

**Enforced** — Feedback plus a required `klin gate --strict` on an independent CI checkout, against a protected branch, with `klin.json`, the workflow and CODEOWNERS under review. Only here does a gate hold against an agent, and loosening it takes a reviewed commit.

[What each level guarantees, and where the boundaries are](docs/THREAT_MODEL.md).

## Works with your existing tools

Your linters, tests, type checkers, and scanners are good at **finding problems**. klin takes a deterministic finding, puts it in front of the agent, and checks the repair.

## Does it actually help?

Before 1.0, we compare one agent, task, and repository two ways: **shadow**, where klin observes but sends no feedback, and **active**, where klin sends failures back during the turn. An external oracle judges each task. Results are published separately, so negative and mixed outcomes stay visible.

## What klin is not

klin is one layer of an agentic system. It is **not**:

- an AI code reviewer;
- a replacement for tests, linters, or type checkers;
- a general-purpose SAST or vulnerability scanner;
- a sandbox or security boundary for coding agents;
- a hosted dashboard or agent orchestration platform.

klin handles the deterministic quality-control loop around a change.

## Documentation

- [Configuration reference](docs/REFERENCE.md)
- [Trust model](docs/THREAT_MODEL.md)
- [Porting klin to another harness](docs/HARNESS_INTEGRATION.md)

Apache-2.0.
