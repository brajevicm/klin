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

**Regression**:
One finding site a blocked stop put in front of the agent, counted once per
finding identity in a window, as `klin stats` counts it.
_Avoid_: shortcut, slip

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

**Tree**:
One set of files at one point: the working tree, or the base laid out beside
it. A run reads a tree's file list once, and every gate selects from it. A run
extracts a source file's structural facts once, and each gate still selects
its own files.
_Avoid_: snapshot, index, catalogue

**Project**:
One run's composition of the config a person wrote and the facts of its
trees, read once and borrowed by every check the run selects.
_Avoid_: context, container, environment

**Scope**:
The `in` and `except` paths a section applies to, each a selector naming
itself and everything below it, never a glob.
_Avoid_: filter, include, exclude, glob

**Layer**:
A named set of paths whose modules may depend only on themselves and the
layers the policy lets them use.
_Avoid_: tier, package

**Derived**:
A value klin computed because the config did not pin the value. A number
comes from one commit; a set of paths adds what the tree holds.
_Avoid_: default, inferred

**Pin**:
A value a person wrote into `klin.json` in place of the one klin would
derive: a ceiling, a scope, a build command. A pin is policy, never a fact
about the tree.
_Avoid_: snapshot, default

**Session**:
The host's grouping of turns under one id, which klin records and never
judges. A host with no id leaves the session empty.
_Avoid_: conversation, run

**External**:
In `public-api`, outside the crate or package that declares an item,
including a sibling in the same repository. The gate judges what is
external. Registry publication does not decide it: `publish = false` and
`"private": true` change nothing.
_Avoid_: published, outside the repository

**Intervention**:
One gate failure on a stop that spent the prompt's gate block. A later failure,
once that block is spent, is an observation. It is the hook's unit, not the
person's: `klin stats` counts regressions.
_Avoid_: catch, prevention

**Journal**:
The per-worktree record of each stop, guard refusal and reset, written
best-effort and read by `klin stats`.
_Avoid_: log, telemetry
