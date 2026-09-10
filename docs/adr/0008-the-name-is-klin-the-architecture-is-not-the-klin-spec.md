# The name is Klin, and the architecture is deliberately not the Klin spec's

> Spec 4.4 and 16.5 record how identical sites in one file are paired: by
> equal ratcheted values first, then by line distance. The line stays the
> tie-breaker this decision made it.

A specification exists for a tool called Klin, version 2.0.0-draft, covering
this problem. This project takes that name and rejects that architecture. A
reader who finds the two together will assume one implements the other, so the
reasons are recorded here.

Klin is the Serbian word for a wedge or chock, the thing that stops a wheel
rolling back. That is the same idea the old name carried, and it is why the
name is worth keeping.

Four of the spec's decisions are refused.

## The metric must not be part of the key

The spec keys a complexity entry by file, symbol and `cyclomatic`, and an
escapes entry by file, line and `pattern`. Put a measurement inside the
identity and the ratchet stops being able to see the measurement move.

Three behaviours that exist here today would go. A function whose complexity
grew would read as one entry gone stale and one finding arrived new, rather
than as a value that rose, so both headline messages change meaning. Two
identical escape sites in one file would become two entries, and `count`, the
only value the escapes gate ratchets on, would stop existing as a concept. One
line carrying two escape kinds would become two entries.

A site is keyed by its file and the text of its declaration line, with the line
number as a tie-breaker only. That is what lets a function that moved down the
file keep its entry, and it is the whole reason a project with existing debt
can adopt this on day one.

## Scoping to the diff must not be the default

The spec scopes checks to changed lines by default. Then `git commit -am wip`
leaves an empty diff, every gate reports itself executed, nothing is measured,
and the run passes. A gate that reports green while measuring nothing is the
worst failure this tool has.

Scoping stays opt-in, it announces how many files it judged, and it anchors on
the merge base so committing does not shrink the window.

## The baseline stays outside the config

The spec puts baselines inside the config file. Then the guard cannot tell a
config edit from a baseline edit, review protection over the two becomes one
switch, and provenance has nowhere to live.

Baselines stay in their own files, named so the guard can recognise one on
sight.

## Signing and automatic writes cannot both hold

The spec signs the baselines with an HMAC, and in the same document has the
tool tighten a baseline automatically when a value improves and drop an entry
automatically when its code is deleted.

Those requirements destroy each other. Automatic writes mean the signing key
must sit unattended on the machine the agent already controls, where the agent
can read it and re-sign. A key held only by CI cannot be verified locally at
all. The signature ends up protecting nothing while suggesting it protects
something.

Improvement stays a note with a command a person runs, and `--strict` turns
that note into a failure in CI. What actually holds is the guard, code-owner
review, and a protected branch. Only a CI run on a checkout the agent never
touched is authoritative. A local hook is feedback, not a control.

## What is taken

The spec's gate catalogue is its strongest part. `diff-radius`,
`db-migration-safety`, `asset-path-verification` and `hallucinated-deps` name
real failures the existing checks do not catch, and structured JSON output with
a remediation per violation is the right shape for an agent's loop.

`hallucinated-deps` reads the lockfile rather than a registry, so the gate stays
offline and deterministic. A dependency named in a manifest with no lockfile
entry is the hallucination.

`flaky-test-runner` is not taken. Executing a project's suite five times makes
this a test runner, with timeouts and sandboxing to match, and a Stop hook
measured in minutes. A ledger that reads results the suite already writes is the
cheaper shape and stays on the list.

The MCP server is not taken. The Stop hook and the guard already put failures in
front of the agent, and a second integration surface is a second thing to keep
working.
