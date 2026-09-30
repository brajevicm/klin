# Trust model

This document answers one question for every guarantee klin makes.

> Is this behavior feedback inside an environment the agent controls, or is it
> enforced outside that environment?

Local hooks and the guard are feedback. They put a finding in front of an agent
and refuse a small, named set of edits. They are not a tamper-proof security
boundary, and klin does not claim they are. The enforcement boundary sits
outside the agent's environment: an independent CI checkout, plus repository
policy a person owns.

The two conformance levels of `docs/SPEC.md` section 15 name the same split:
Feedback is hooks only, Enforced is hooks plus a required `klin gate --strict`
run on a checkout the agent never touched.

## Zone 1: the agent-controlled worktree

Source, tests, project-local hook files, `klin.json`, CODEOWNERS and every
other file the coding agent can edit through its host.

- klin reads this tree and reports what changed in it.
- The guard can answer `deny` or `ask` on a tool call the host routes to it.
- The guarded set is exactly two things: this tree's `klin.json`, and this
  tree's own state directory, which is `.git/klin` unless `KLIN_STATE_DIR`
  names another place. A host's hook file, CODEOWNERS, a check's own
  configuration and another worktree's state directory are ordinary files, and
  an edit to one of them is `allow` (SPEC 9.4, ADR 0027, ADR 0032, ADR 0033).
- The guard answers only where it can prove that a call writes a guarded path.
  A shell wildcard, an unbalanced quote, `$(`, a backtick, `${`, `<<`, a
  backslash, a `cd` command word, or a path it cannot resolve all end in
  `allow`.
- The agent may hold tools the host never routes to the guard. The plugin's
  `PreToolUse` matcher names `Write`, `Edit`, `MultiEdit`, `NotebookEdit`,
  `Bash` and `apply_patch`. A write through any other path is unseen.

## Zone 2: the local host and the developer machine

Coding-agent host configuration, plugin installation, the `klin` binary in
`~/.cache/klin/bin` or on `PATH`, user-level hook configuration, and klin's
per-worktree state and journal.

These components make the feedback useful and reproducible. A person owns their
own installation and its permissions. Nothing here is cryptographically or
administratively protected against the person or against a process that runs as
them. A `PATH` shim, a `chmod -x`, an edit to user-level settings, or an
uninstall of the plugin removes the feedback, and klin cannot detect that from
inside the same machine.

A path that is hard to reach through the ordinary agent path is not a protected
path. klin gives the first and no more. Where this document says something is
protected, another system supplies that protection.

## Zone 3: the independent CI checkout

A workflow that checks out the proposed commit on its own, installs a pinned
klin release, and runs `klin gate --strict`. It reads the committed tree. It
does not trust any hook output the agent's worktree recorded.

`action.yml` is that workflow step: it resolves a version from its input, then
`klin.json`, then the tag it is pinned at, then the latest release, and
installs through the release's `klin-installer.sh` (SPEC 19.1). The install
step runs under `set -euo pipefail` with `curl ... -LsSf`, so a download that
fails, or an installer that rejects a checksum, fails the job instead of
skipping the gate.

This is the first enforcement boundary for klin's measurements.

## Zone 4: repository policy and human approval

Protected branches, required checks, and review rules over the enforcement
surface: `klin.json`, the workflow, the hook configuration where it is
committed, and CODEOWNERS itself. This repository's own `.github/CODEOWNERS`
covers those paths.

Policy is what makes a red independent run consequential instead of advisory.
Without a required check, CI reports and merges anyway. Without review over the
enforcement surface, the same change that introduces debt can also weaken the
gate that would have named it.

## Guarantee matrix

| Behavior | Local feedback | Independent CI | Needs repository policy |
| --- | --- | --- | --- |
| A new complexity, escape, stub or other finding | The stop hook blocks the turn and names it | `klin gate --strict` measures the committed tree again | Yes, to make the check required |
| An agent edit to `klin.json` through a routed edit tool | `deny`, naming the file | The diff is visible, and `--strict` reads the committed config | Yes, review over `klin.json` |
| An agent edit to `klin.json` through a path the guard cannot prove | May pass, by design (SPEC 9.4) | Same as the row above | Yes |
| An agent removes or disables the local hook | May remove the feedback, and klin does not detect it | Still measures the final tree | Yes |
| An agent changes the CI workflow | The hook file and the workflow are not guarded | The same change may alter the run that judges it | Yes, CODEOWNERS and protected settings |
| An agent adds an entry to `accepted` | The guard denies routed edits to `klin.json`, and the skill says not to | `--strict` reads the committed config, so an accepted entry is silent in CI too | Yes, a reviewed commit by a person |
| `klin init` or `klin turn reset` from an agent | `deny`, naming the command a person runs | Not applicable | No |
| A deleted test | The hook blocks the first stop once, then reports it | A NOTE, including under `--strict` (SPEC 8.2) | Yes, human review remains part of this guarantee |
| No binary, or no network | Fails open, see below | The install must fail, see below | Yes |

A deleted test is a NOTE outside the hook, `--strict` included, so no CI run
fails on it. At the Enforced level it holds through the hook's one question,
the guard in front of the record that question leaves, and the reviewer who
reads the diff. An agent that runs with no hook meets none of the three. ADR
0031 and ADR 0032 record the cost.

## A missing binary or a missing network

The two zones answer this differently, on purpose.

Locally, klin fails open. The plugin wrapper downloads the pinned release once,
verifies its `sha256` before it moves the binary into the cache, and
executes the cached binary on every later run with no network call. When the
download or the checksum fails, the wrapper runs a `klin` that `PATH` resolves
if there is one, and otherwise prints one line and exits 0. The hook lines do
the same when neither the wrapper nor a `klin` on `PATH` resolves. A turn is
never blocked because klin was absent, so a missing binary or a broken network
means no measurement, not a failed turn.

In CI, the install fails closed. The install step aborts the job when the
pinned release cannot be downloaded. A workflow author who wraps that step in
`continue-on-error`, or who makes the gate step non-required, turns the
enforcement boundary back into feedback.

## klin's own repository

`.github/workflows/quality.yml` builds klin from the commit under review and
runs `./target/debug/klin gate --strict`. The checkout is independent of the
agent's worktree, but the binary is not independent of the change: a commit
that alters a check also alters the binary that judges it. `.github/CODEOWNERS`
over `/.github/workflows/`, `/klin.json`, `/.claude/settings.json` and
`/.github/CODEOWNERS` is what covers that gap, together with review of the
diff.

Only a push to `main` saves the Rust cache that `quality` restores, so a pull
request cannot put build artifacts into the runs of `main` or of other pull
requests. A release tag builds and checks with no cache.

## Out of scope

klin does not sandbox the coding agent, harden the developer machine, manage
secrets, or automate organization policy. It reads no secrets and sends nothing
anywhere (SPEC 15.3). The `build` and `run` commands execute shell lines the
config owner wrote.
