<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/klin-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/klin-logo-light.svg">
  <img src="assets/klin-logo-light.svg" alt="klin" width="190">
</picture>

# Catch regressions while the agent can still fix them

**Deterministic quality control for coding agents.**

klin catches new or worsened deterministic problems during coding-agent work and returns concrete feedback while the working context is still available.

**Claude Code · Codex · Cursor**

```text
FAIL  complexity
      FAIL: 1 function(s) got worse — the ratchet only tightens:
        src/quote.ts:18  cc 11, 35 lines, was cc 10, 33 lines
      Reduce the function's responsibility or decision complexity.
```

## Quick start

1. Install klin. From your repository root, on macOS, Ubuntu 22.04 or later, or Debian 12 or later, run:

   ```sh
   sh -c 'i=$(curl --proto "=https" --tlsv1.2 -LsSf https://github.com/brajevicm/klin/releases/latest/download/klin-installer.sh) && sh -c "$i" && ~/.local/bin/klin install'
   ```

   This installs klin, creates `klin.json`, and adds hooks for the hosts your repository already uses. If it uses none yet, you get hooks for all three. It also writes klin's agent instructions as a skill for each host.

   New shells find `klin` on your PATH. In the terminal where you ran the installer, run `. ~/.local/bin/env` first.

2. Commit the new files, so your team gets the same checks.

3. Work with your agent as usual. On Codex, first run `/hooks`, trust the klin hooks, and start a new session.

4. See what klin caught:

   ```sh
   klin stats --session
   ```

   ```text
   Nothing needs your attention.

   klin caught 1 regression this session.
   It was fixed after klin flagged it.
   ```

   If something is still open, the report shows it first and names the sites to fix.

- For one host only, add `--host claude`, `--host codex` or `--host cursor` to `klin install`.
- To update, run `klin update`, then `klin install`.

<details>
<summary>Install as a plugin instead</summary>

A plugin runs the same checks and fetches klin by itself. It does not add the `klin` command, so install the CLI too if you want `klin stats`. If both run on your machine, klin runs once per event.

**Claude Code**

```text
/plugin marketplace add brajevicm/klin
/plugin install klin@klin
```

**Codex**

```sh
codex plugin marketplace add brajevicm/klin
codex plugin add klin@klin
```

Then run `/hooks`, trust the klin hooks, and start a new session.

**Cursor**

```sh
d=$(mktemp -d) &&
  git clone --depth 1 --branch v0.4.1 https://github.com/brajevicm/klin "$d" &&
  mkdir -p ~/.cursor/plugins/local &&
  rm -rf ~/.cursor/plugins/local/klin &&
  cp -R "$d/plugins/klin" ~/.cursor/plugins/local/klin
s=$?; rm -rf "$d"; [ "$s" -eq 0 ] || { echo "klin: the Cursor plugin copy failed. Run it again once the fetch works." >&2; false; }
```

Then reload Cursor.

A plugin stays quiet until the repository opts in. At the repository root, run:

```sh
echo '{}' > klin.json
```

`{}` is a complete configuration. klin finds the facts about your repository by itself.

</details>

## How it works

klin measures the code before and after the agent's work. Existing debt never blocks you. Only new or worsened debt fails.

```text
quality debt    before    after    result
unchanged           8        8    ✓ pass
improved            8        6    ✓ pass
worsened            8        9    ✗ fail
```

The checks are deterministic measurements, with no model judging the code. The "before" comes from your repository, so you have no baseline file to maintain.

In a session:

1. You send a prompt, and klin marks the start of the turn.
2. While the agent works, klin refuses its edits to `klin.json`, the hooks and CODEOWNERS.
3. When the agent tries to finish, klin compares the tree with the start of the turn.
4. If a problem is new or worse, klin blocks the stop and shows the agent the findings. The agent fixes them in the same turn.
5. Before merge, CI checks the change again.

The agent follows [klin's instructions](plugins/klin/skills/klin/SKILL.md): fix the code klin names, and leave the policy alone. If your team decides to keep a finding, a person adds it to the `accepted` list in `klin.json`, in a reviewed commit.

## Run klin yourself

| Command               | What it does                                          |
| --------------------- | ----------------------------------------------------- |
| `klin gate --changed` | Checks only the changed files. This is the fast loop. |
| `klin gate`           | Runs every check over the whole repository.           |
| `klin gate --list`    | Shows which checks run and the values they use.       |
| `klin stats --all`    | Lists each regression klin caught.                    |

Add `--json` to `klin gate` or `klin stats` for machine-readable output.

## What klin catches

- **Complexity creeps up.** A function gets too complex or too long for the repository's current bar.
- **Architecture drifts.** A new dependency cycle appears, code crosses a configured layer, or a project convention breaks.
- **Guardrails get bypassed.** A test disappears or gets skipped, or the change adds `@ts-ignore`, `eslint-disable` or another escape hatch.
- **The work isn't finished.** The change leaves a new TODO, placeholder or not-implemented stub, or adds code that nothing can reach.
- **A public contract changes.** An exported surface disappears or its declared contract changes, even though the build still passes.
- **Dependencies fall out of sync.** A dependency is missing from the lockfile, loses its exact pin, or has a different version in the lockfile.
- **Docs go stale.** Code moves, but the documentation still points to the old location.

Complexity covers Go, Java, JavaScript, Kotlin, Python, Ruby, Rust, Swift and TypeScript. The architecture, reachability and public API checks cover Rust and TypeScript. See the [full language table](docs/REFERENCE.md#built-in-language-coverage).

klin does not replace your linters, tests or reviews. If one of your tools emits SARIF, its findings can go through the same ratchet.

## Enforce it in CI

Hooks give the agent feedback inside its own environment. They are not a security boundary. To enforce the policy, make klin a required CI check.

GitHub Actions, after checkout:

```yaml
- uses: brajevicm/klin@v0.4.1
```

Other CI, after you install klin:

```sh
klin gate --strict
```

Also require review for changes to `klin.json` and the hook files. The [trust model](docs/THREAT_MODEL.md) explains why.

## Learn more

| I want to…                                    | Read                                                           |
| --------------------------------------------- | -------------------------------------------------------------- |
| Configure checks and policy                   | [Configuration reference](docs/REFERENCE.md)                   |
| Check which host features klin supports       | [Host compatibility](docs/HOST_COMPATIBILITY.md)               |
| Understand what hooks and CI each protect     | [Trust model](docs/THREAT_MODEL.md)                            |
| Connect a coding agent other than these three | [Harness integration guide](docs/HARNESS_INTEGRATION.md)       |
| See how klin did on real coding-agent runs    | [Coding-agent validation](docs/benchmark-result-2026-09-25.md) |

Found a problem or have a question? [Open an issue](https://github.com/brajevicm/klin/issues/new/choose).  
Security issue? [Follow the private reporting instructions](SECURITY.md).

[Apache-2.0](LICENSE)
