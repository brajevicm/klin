# klin

A quality ratchet for AI-driven development. It measures a codebase, records
what is already wrong, and refuses anything newly wrong.

## Language

**Gate**:
One configured instance of a check, which passes or fails the build.
_Avoid_: rule, lint, policy

**Check**:
The measurement and judgment behind a gate, shared by every gate that uses it.
_Avoid_: gate, checker

**Base**:
The commit a run measures the working tree against. A finding the base holds is
debt the branch inherited, not debt it added.
_Avoid_: baseline, snapshot, reference

**Accepted**:
The list in the config where a person records debt they allow, in a reviewed
commit. Only a person writes it.
_Avoid_: allowlist, exemption, suppression

**Finding**:
One violation at one site, as a gate reports it.
_Avoid_: error, issue, violation

**Site**:
The identity a finding is keyed by, chosen so that moving code does not read as
new debt.
_Avoid_: location, position

**Ceiling**:
The value a measure may reach before its gate fails.
_Avoid_: limit, threshold, budget

**Escape**:
A construct that opts out of a safety the language or its tools provide, such
as `any`, `unwrap()`, or `.skip`.
_Avoid_: suppression, override, bypass

**Guard**:
The hook mode that refuses an agent's edits to the config, the hooks
themselves, and the code owners.
_Avoid_: lock, protection, shield

**Turn**:
The window between one message from the person and the next. Everything inside
it is work the agent chose.
_Avoid_: session, iteration, round

**Radius**:
How far one turn's changes spread: the lines, the share of them that is
formatting or moved code, and the directories touched.
_Avoid_: churn, diff size, blast radius

**Note**:
Something klin reports and never fails on. A note carries no ceiling, because
nothing about it can be exceeded.
_Avoid_: warning, advisory, info

**Window**:
The pair of trees a run compares, and where each came from. Three kinds: turn,
branch, push.
_Avoid_: scope, range, diff

**Derived**:
A value klin computed because the config did not pin it, and printed as such.
A number comes from one commit. A set of paths adds what the working tree
holds.


_Avoid_: default, inferred, guessed
