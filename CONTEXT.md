# klin

## Language

**Gate**:
One configured instance of a check, which passes or fails the build.
_Avoid_: rule, lint, policy

**Check**:
The measurement and judgment behind a gate, which gates can share.
_Avoid_: gate, checker

**Base**:
The commit a run measures the working tree against. A finding the base holds
is inherited debt.
_Avoid_: baseline, snapshot, reference

**Accepted**:
The config list where only a person records allowed debt, in a reviewed
commit.
_Avoid_: allowlist, exemption, suppression

**Finding**:
One violation at one site, as a gate reports it.
_Avoid_: error, issue, violation

**Shortcut**:
A new finding the agent introduced in a turn, as `klin stats` counts it.
_Avoid_: regression, slip

**Site**:
The identity a finding is keyed by, so moved code is not new debt.
_Avoid_: location, position

**Ceiling**:
The value a measure may reach before its gate fails.
_Avoid_: limit, threshold, budget

**Escape**:
A construct that opts out of a language's safety: `any`, `unwrap()`, `.skip`.
_Avoid_: suppression, override, bypass

**Guard**:
The hook mode that refuses agent edits to klin's config, hooks and code
owners.
_Avoid_: lock, protection, shield

**Turn**:
The window between two messages from the person, holding work the agent
chose.
_Avoid_: session, iteration, round

**Radius**:
How far one turn's changes spread: lines, formatting or moved share, and
directories.
_Avoid_: churn, diff size

**Note**:
Something klin reports and never fails on.
_Avoid_: warning, advisory

**Window**:
The pair of trees a run compares, and their source: turn, branch, push.
_Avoid_: scope, range, diff

**Derived**:
A value klin computed because the config did not pin the value. A number
comes from one commit; a set of paths adds what the tree holds.
_Avoid_: default, inferred

**Session**:
The host's grouping of turns under one id, which klin records and never
judges. A host with no id leaves the session empty.
_Avoid_: conversation, run

**Intervention**:
One gate failure on a stop that spent the prompt's gate block. A later failure,
once that block is spent, is an observation.
_Avoid_: catch, prevention

**Journal**:
The per-worktree record of each stop, guard refusal and reset, written
best-effort and read by `klin stats`.
_Avoid_: log, telemetry
