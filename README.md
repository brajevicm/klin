<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/klin-logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/klin-logo-light.svg">
  <img src="assets/klin-logo-light.svg" alt="klin" width="190">
</picture>

# Catch regressions while the agent can still fix them

**Deterministic quality control for coding agents.**

klin catches new or worsened deterministic problems during coding-agent work and returns concrete feedback while the working context is still available.

**Claude Code · Codex · Cursor**

<!-- PRE-RELEASE: Replace with one authentic klin regression/output from #259/#262. -->

## Install

Use the native plugin for Claude Code, Codex, or Cursor.

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
<summary>Install the local plugin</summary>

```sh
d=$(mktemp -d) && git clone --depth 1 https://github.com/brajevicm/klin "$d"
rm -rf ~/.cursor/plugins/local/klin
mkdir -p ~/.cursor/plugins/local
cp -R "$d/plugins/klin" ~/.cursor/plugins/local/klin
rm -rf "$d"
```

Reload Cursor after installation.

</details>

### Activate this repository

Native plugins stay silent until the repository opts in:

```sh
echo '{}' > klin.json
```

`{}` is a complete configuration. klin discovers repository facts automatically; `klin.json` contains only the policy you choose to configure.

**Configure policy, not your repository.**

<!-- PRE-RELEASE: #268 is exploring a more intentional user-owned native activation flow. Until something better ships and is verified, keep the explicit klin.json marker. -->

### Standalone

Need committed project hooks or an environment where the native plugin isn't available? Install the binary:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/brajevicm/klin/releases/latest/download/klin-installer.sh | sh
klin install
```

`klin install` configures the supported Claude Code, Codex, and Cursor integrations it can prove are present.

**Updating:** update native plugins through the host's plugin mechanism. Standalone users can run `klin update` followed by `klin install`.

### Other coding agents

Other coding-agent harnesses can integrate through klin's versioned [generic harness contract](docs/HARNESS_INTEGRATION.md).

These are not first-class integrations: installing the klin binary alone does not connect another harness. The harness must translate its lifecycle events into klin's generic protocol.

See [Integrating another coding-agent harness](docs/HARNESS_INTEGRATION.md).

<!-- PRE-RELEASE: #271 is exploring clearer naming/taxonomy for the generic harness contract and its repository path. Update this subsection after that decision lands. -->

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
- **A public API breaks.** An exported contract changes or disappears even though the repo still builds.
- **Dependencies fall out of sync.** A manifest changes without the corresponding lockfile update.
- **References in docs go stale.** Code moves, but documentation still points to the old location.

Already use a static analysis tool? If it emits SARIF, its findings can go through the same ratchet.

See the [configuration reference](docs/REFERENCE.md) for exact check and language coverage.

<!-- PRE-RELEASE: Revisit the examples and their ordering after #259/#261/#262. Prefer high-value regressions with strong validation evidence and low feedback friction. -->

## Keep your existing tools

Keep your linters, type checkers, tests, security scanners, AI review, and human review. klin does not replace them; it adds deterministic quality control around the coding agent's change.

## Feedback locally. Verify independently in CI.

Local hooks are for repair. CI is for verification.

While the coding agent works, klin returns regressions while the change is still in context. Before merge, CI checks the committed change again from an independent checkout the agent never touched.

GitHub Actions, after checkout:

```yaml
- uses: brajevicm/klin@v0.2.1
```

Other CI, after installing klin:

```sh
klin gate --strict
```

Hooks alone are klin's **Feedback** level. Make the independent CI check required and protect changes to the enforcement setup through review to reach **Enforced**.

[See the trust model and enforcement boundaries](docs/THREAT_MODEL.md).

<!-- PRE-RELEASE:
Add the evidence section after #259/#262.

It should:
- state only conclusions supported by the repaired evidence;
- link the dated methodology/results;
- include mixed/negative findings where material;
- avoid turning the README into a benchmark report;
- remain ~2–4 sentences unless one number genuinely earns prominence.
-->

## Documentation

- [Configuration reference](docs/REFERENCE.md)
- [Host compatibility](docs/HOST_COMPATIBILITY.md)
- [Trust and enforcement](docs/THREAT_MODEL.md)
- [Integrating another coding-agent harness](docs/HARNESS_INTEGRATION.md)

Found a problem or have a question? [Open an issue](https://github.com/brajevicm/klin/issues/new/choose).  
Security issue? [Follow the private reporting instructions](SECURITY.md).

[Apache-2.0](LICENSE)
