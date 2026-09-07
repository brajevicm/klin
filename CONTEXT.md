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
A construct that opts out of a safety the language or its tools provide, such
as `any`, `unwrap()`, or `.skip`.
_Avoid_: suppression, override, bypass

**Provenance**:
The tool and version that produced a baseline, stored beside it.
_Avoid_: metadata, fingerprint

**Guard**:
The hook mode that refuses an agent's edits to the config, the baselines, or the
hooks themselves.
_Avoid_: lock, protection, shield
