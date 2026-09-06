# detent

A quality ratchet for AI-driven development. It measures a codebase, records
what is already wrong, and refuses anything newly wrong.

## Language

**Gate**:
A check that inspects the codebase and either passes or fails the build.
_Avoid_: rule, lint, policy

**Ratchet**:
A gate that records the violations already present and fails only on new ones.
_Avoid_: threshold, budget

**Baseline**:
The recorded set of findings a person has accepted. Only a person changes it.
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
A construct that switches a check off, such as `any`, `unwrap()`, or `.skip`.
_Avoid_: suppression, override, bypass

**Provenance**:
The tool and version that produced a baseline, stored beside it.
_Avoid_: metadata, fingerprint

**Drift**:
Disagreement between a stored baseline and what the current code produces.
_Avoid_: staleness, rot

**Guard**:
The hook mode that refuses an agent's edits to the config, the baselines, or the
hooks themselves.
_Avoid_: lock, protection, shield
