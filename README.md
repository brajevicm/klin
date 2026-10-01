<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/klin-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/klin-logo-light.svg">
  <img src="assets/klin-logo-light.svg" alt="klin" width="190">
</picture>

# Catch regressions while the agent can still fix them

**Deterministic quality control for coding agents.**

klin catches new or worsened problems during coding-agent work and returns concrete feedback while the working context is still available.

**Works natively with:** Claude Code · Codex · Cursor

```text
FAIL  complexity
      FAIL: 1 function(s) got worse — the ratchet only tightens:
        src/quote.ts:18  cc 11, 35 lines, was cc 10, 33 lines
      Reduce the function's responsibility or decision complexity. …
```

## How klin works

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/klin-how-it-works-dark.svg">
  <img src="assets/klin-how-it-works-light.svg" width="880" alt="One agent turn: you prompt and klin marks the start. The agent edits and tries to finish, and klin compares the start of the turn with now. A new or worsened problem fails, and the agent repairs the named site in the same turn. A pass goes to CI, which checks the change again from its own checkout before merge.">
</picture>

## Quick start

### 1. Install

From your repository root on macOS, Ubuntu 22.04+, or Debian 12+:

```sh
sh -c 'i=$(curl --proto "=https" --tlsv1.2 -LsSf https://github.com/brajevicm/klin/releases/latest/download/klin-installer.sh) && sh -c "$i" && ~/.local/bin/klin install'
```

klin detects the supported hosts already used by the repository. If none are found, it configures Claude Code, Codex, and Cursor.

Commit `klin.json` and the generated integration files.

Only want one host? Append `--host claude`, `--host codex`, or `--host cursor` to the final `klin install` command.

If your shell can't find `klin` after the install, open a new terminal. To update later, run `klin update`, then `klin install`.

> [!NOTE]
> **Codex:** run `/hooks`, review and trust the klin hooks, then start a fresh session.

### 2. Work normally

Use your coding agent as usual. There is nothing to run manually - klin checks the turn when the agent tries to finish.

klin marks the start of the turn and checks what changed when the agent tries to finish. New or worsened deterministic problems go back to the agent while the change is still in context.

### 3. See what happened

```sh
klin stats --session
```

```text
Nothing needs your attention.

klin caught 1 regression this session. It was fixed after klin flagged it.
```

Use `klin stats --all` for individual findings or `klin stats --json` for machine-readable output.

## Native plugins

**Alternative to the CLI setup above.**

Claude Code, Codex, and Cursor can also run klin through their native plugin systems.

A plugin gives you no `klin` command in your shell. Install the CLI too if you want `klin stats`. If plugin and repository hooks are both present, one copy handles each event and the other stays quiet.

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

Run `/hooks`, review and trust the klin hooks, then start a fresh session.

### Cursor

<details>
<summary><strong>Install the local plugin</strong></summary>

```sh
d=$(mktemp -d) &&
  git clone --depth 1 --branch v0.4.1 https://github.com/brajevicm/klin "$d" &&
  mkdir -p ~/.cursor/plugins/local &&
  rm -rf ~/.cursor/plugins/local/klin &&
  cp -R "$d/plugins/klin" ~/.cursor/plugins/local/klin
s=$?; rm -rf "$d"; [ "$s" -eq 0 ] || {
  echo "klin: the Cursor plugin copy failed. Run it again once the fetch works." >&2
  false
}
```

Reload Cursor.

</details>

### Opt the repository in

A plugin stays quiet until the repository opts in. At the repository root, run:

```sh
echo '{}' > klin.json
```

`{}` is a complete configuration. klin derives repository facts automatically.

## Why klin

A coding agent can complete the requested task while making something else worse.

Most deterministic tools tell you what is wrong **now**. klin adds the change boundary:

**Did this problem appear or get worse during this work?**

```text
quality debt        before    after    result
unchanged               8        8       ✓ pass
improved                8        6       ✓ pass
worsened                8        9       ✗ fail
```

Existing debt does not block adoption. Only new or worsened debt fails. One exception: a build that fails blocks the agent until the code builds again.

**Deterministic, not another LLM.** klin measures specific properties and returns concrete evidence instead of asking another model whether the code is "good."

**No baseline to maintain.** klin reads the before-state from Git, so no baseline file has to stay in sync.

**Repair now, verify later.** Local hooks return findings while the agent still has context. CI independently checks the committed result.

**The agent fixes code, not the bar.** Intentional exceptions are human-reviewed policy in `accepted`.

## What klin catches

- **Complexity creeps up.** A function becomes too complex or too large for the repository's current bar.
- **Architecture drifts.** Code crosses a layer you defined, closes a new dependency cycle, or breaks a convention you wrote down. These checks need a `layering` or `conventions` section in `klin.json`.
- **Guardrails get bypassed.** A test gets skipped, or the change adds `@ts-ignore`, `eslint-disable`, or another escape hatch. If a test disappears, klin asks the agent about it once.
- **Work is left unfinished.** The change leaves a new TODO, placeholder, or stub, or adds code that nothing references.
- **A public contract changes.** An exported Rust or TypeScript surface disappears or its declared contract changes.
- **Dependencies fall out of sync.** A dependency is missing from the lockfile, loses its exact pin, or has a version the lockfile does not record.
- **Documentation goes stale.** Code moves, but a Markdown file at the repository root still points to the old path.

Keep your linters, type checkers, tests, security scanners, static analysis, AI review, and human review. klin adds a deterministic ratchet around the agent's change.

A scanner that writes SARIF can report through a `sarif` section in `klin.json`. klin then fails on any of its results that sit on a line the change touched.

[Configuration and language coverage →](docs/REFERENCE.md)

## CI

klin works at two levels:

- **Feedback:** hooks only. klin returns findings to the agent and refuses its edits to `klin.json`, but nothing outside the agent's environment checks the result.
- **Enforced:** hooks plus a required CI check on a protected branch. `CODEOWNERS` covers `klin.json`, the workflow, the hook files, and `CODEOWNERS` itself.

The Quick start setup reaches Feedback. Add the CI check to reach Enforced.

GitHub Actions:

```yaml
- uses: actions/checkout@v5
  with:
    fetch-depth: 0
- uses: brajevicm/klin@v0.4.1
```

Other CI: install klin, fetch the full Git history, then run:

```sh
klin gate --strict
```

[Trust model and enforcement boundaries →](docs/THREAT_MODEL.md)

## Learn more

- [Host compatibility](docs/HOST_COMPATIBILITY.md)
- [Agent instructions](plugins/klin/skills/klin/SKILL.md)
- [Integrating another coding-agent harness](docs/HARNESS_INTEGRATION.md)
- [Benchmark and validation evidence](docs/benchmark-result-2026-09-25.md)

Found a problem or have a question? [Open an issue](https://github.com/brajevicm/klin/issues/new/choose).  
Security issue? [Follow the private reporting instructions](SECURITY.md).

[Apache-2.0](LICENSE)
