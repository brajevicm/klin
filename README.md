<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/klin-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/klin-logo-light.svg">
  <img src="assets/klin-logo-light.svg" alt="klin" width="190">
</picture>

# Catch regressions while the agent can still fix them

**Deterministic quality control for coding agents.**

klin catches new or worsened deterministic problems during coding-agent work and returns concrete feedback while the working context is still available.

**Claude Code · Codex · Cursor**

```
FAIL  complexity
      FAIL: 1 function(s) got worse — the ratchet only tightens:
        src/quote.ts:18  cc 11, 35 lines, was cc 10, 33 lines
      Reduce the function's responsibility or decision complexity.
```

## Install klin on macOS or Linux

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/brajevicm/klin/releases/latest/download/klin-installer.sh | sh
```

Then, in a new terminal, turn klin on in your repository:

```sh
cd your-repo
klin install --host claude    # or: codex, cursor
```

This writes `klin.json` and the hooks for that host (if the klin plugin already runs there, `klin install` says what to change). Commit them, and your teammates get the same checks.

Using Codex? Run `/hooks`, review and trust the klin hooks, then start a fresh session.

**Updating:** run `klin update`, then `klin install` again.

### Or use your host's plugin

A plugin runs the same checks and fetches klin by itself, but it doesn't add the `klin` command.

#### Claude Code

```text
/plugin marketplace add brajevicm/klin
/plugin install klin@klin
```

#### Codex

```sh
codex plugin marketplace add brajevicm/klin
codex plugin add klin@klin
```

Run `/hooks`, review and trust the klin hooks, then start a fresh session.

#### Cursor

<details>
<summary>Install the local plugin</summary>

```sh
d=$(mktemp -d) &&
  git clone --depth 1 --branch v0.3.0 https://github.com/brajevicm/klin "$d" &&
  mkdir -p ~/.cursor/plugins/local &&
  rm -rf ~/.cursor/plugins/local/klin &&
  cp -R "$d/plugins/klin" ~/.cursor/plugins/local/klin
s=$?; rm -rf "$d"; [ "$s" -eq 0 ] || { echo "klin: the Cursor plugin copy failed. Run it again once the fetch works." >&2; false; }
```

Reload Cursor after installation.

</details>

A plugin's checks stay quiet until the repository opts in. At the repository root, run:

```sh
echo '{}' > klin.json
```

`{}` is a complete configuration. klin discovers repository facts automatically; `klin.json` contains only the policy you choose to configure.

**Configure policy, not your repository.**

### Other coding agents

Claude Code, Codex, and Cursor are the first-class integrations. Any other coding-agent harness can use klin through its versioned [harness protocol](harness-protocol/), but somebody has to build that integration for the harness.

Installing the klin binary alone does not connect another harness. The harness has to send its lifecycle events to klin in the protocol's format and turn klin's decisions back into its own answers. Using the protocol does not make that harness first-class.

Start with [the porting guide](docs/HARNESS_INTEGRATION.md).

## Why klin

A coding agent can complete the requested task while making something else worse.

Most deterministic tools tell you what is wrong **now**. klin adds the change boundary: **did this problem appear or get worse during this work?**

```text
quality debt           before      after      result
unchanged                  8          8         ✓ pass
improved                   8          6         ✓ pass
worsened                   8          9         ✗ fail
```

**Adopt without a cleanup project.** Existing measured debt does not block work. Unchanged and improved debt passes; new or worsened debt fails.

**Deterministic, not another LLM.** klin measures specific properties and returns concrete evidence instead of asking another model whether the code is "good."

**No baseline to maintain.** The before-state comes from the repository and the work boundary, not a baseline file that has to stay in sync.

**Repair now, verify later.** Local hooks return regressions while the agent still has the working context. CI independently verifies the same repository policy before merge.

## What klin catches

An agent can finish the task and still make something else worse. klin catches specific regressions while the work is still in context.

- **Complexity creeps up.** A function becomes too complex or too large for the repo's current bar.
- **Architecture drifts.** A new dependency cycle appears, code crosses a configured layer, or a project convention is broken.
- **Guardrails get bypassed.** A test disappears or gets skipped, or the change adds `@ts-ignore`, `eslint-disable`, or another escape hatch.
- **The work isn't actually finished.** The change leaves behind a new TODO, placeholder, or not-implemented stub, or adds code that nothing can reach.
- **A public contract changes.** An exported surface disappears or its declared contract changes, even though the repository still builds.
- **Dependencies fall out of sync.** A dependency is missing from the lockfile, loses its exact pin, or the lockfile still records a different version.
- **References in docs go stale.** Code moves, but documentation still points to the old location.

Already use a static analysis tool? If it emits SARIF, its findings can go through the same ratchet.

See the [configuration reference](docs/REFERENCE.md) for exact check and language coverage.

## Keep your existing tools

Keep your linters, type checkers, tests, security scanners, AI review, and human review. klin does not replace them; it adds deterministic quality control around the coding agent's change.

## Feedback locally. Verify independently in CI.

Local hooks are for repair. CI is for verification.

While the coding agent works, klin returns regressions while the change is still in context. Before merge, CI checks the committed change again from an independent checkout the agent never touched.

GitHub Actions, after checkout:

```yaml
- uses: brajevicm/klin@v0.3.0
```

Other CI, after installing klin:

```sh
klin gate --strict
```

Hooks alone are klin's **Feedback** level. Make the independent CI check required and protect changes to the enforcement setup through review to reach **Enforced**.

[See the trust model and enforcement boundaries](docs/THREAT_MODEL.md).

## Tested on real coding-agent work

We tested klin across repeated controlled coding-agent runs and used what we found to harden the product before release.

In the latest validation, every case that exposed the target regression was repaired with klin active, with no undesired signals. A separate seeded confirmation found no repair-by-appeasement.

## Documentation

- [Configuration reference](docs/REFERENCE.md)
- [Host compatibility](docs/HOST_COMPATIBILITY.md)
- [Trust and enforcement](docs/THREAT_MODEL.md)
- [Integrating another coding-agent harness](docs/HARNESS_INTEGRATION.md)
- [Coding-agent validation](docs/benchmark-result-2026-09-25.md)

Found a problem or have a question? [Open an issue](https://github.com/brajevicm/klin/issues/new/choose).  
Security issue? [Follow the private reporting instructions](SECURITY.md).

[Apache-2.0](LICENSE)
