# A versioned generic contract for a custom harness

> Renamed on 2026-09-22 by #271, with the wire shape unchanged. The contract is
> now the harness protocol, in `harness-protocol/`, where `adapter.sh` became
> `reference-adapter.sh` and the schema `$id` values moved with the path.
> `--host harness` is the override, and `--host generic` names no host. The
> text below records the decision as it was made.

klin maintains a native integration for Claude Code, Codex CLI and Cursor.
ADR 0023, ADR 0030, ADR 0045 and ADR 0046 decided how those three are built and
installed, and nothing here changes any of it.

Every other harness had one route: spawn `klin --host claude` and fabricate
Claude Code's payload around your own event. That route works, and it is wrong
in three ways.

It makes a harness lie about what it is. The journal then records Claude Code
for a turn Claude Code never ran, and a compatibility fix made for Claude Code's
real payload lands on a shim that only resembles it.

It leaks a host's fields into what is really klin's public seam. `tool_input`,
`stop_hook_active` and `hookSpecificOutput.permissionDecision` are Claude Code's
words. A harness that has none of those concepts still has to spell them.

It has no version. A shim written against klin 0.4 and a klin that has moved on
have no way to say so to each other, so a mismatch shows up as a decision read
wrongly rather than as an error.

## The decision

klin exposes one versioned generic contract, and a porting kit beside it.

A generic event is klin's own shape, not a host's. `klin_protocol` names the
version and places the event: no host klin maintains sends that field, so the
generic adapter is tried before all of them, and `--host generic` is the
override. `event` names one of four lifecycle kinds, and the rest of the object
is the evidence the harness proves. The decision is one small JSON object with
four actions. Both shapes are backed by a checked-in JSON Schema under
`integrations/generic/`, and section 9.7 of the specification states them.

The contract is a portability seam and not a second engine. The generic adapter
is one more implementation of the same `Adapter` trait the built-in hosts
implement, producing the same internal event, running the same turn, guard and
gate loop. A custom harness gains no code path of its own inside klin.

Evidence stays evidence. `file_paths` and `command` carry what the harness can
prove; a tool name is diagnostic and klin reads no write out of it. A call that
proves neither a path nor a command is allowed. The alternative — guessing
writes out of opaque tool arguments — refuses work that was never a threat and
teaches an agent to route around the guard.

There is no `ask` in the contract. klin's third decision needs a question a
host enforces, and nothing a custom harness sends can prove it has one. The
ambiguous class of 9.4 is refused instead, as it already is on Codex CLI and on
Cursor, and the porting guide has the integrator record that difference rather
than let it pass as an allow.

An unknown version fails closed. klin names the version the event sent and the
version it speaks, then refuses every call and blocks every stop while the
mismatch stands. It never falls back to reading the event as another host's,
because a payload klin cannot place is exactly the payload it must not guess at.

The three first-class hosts stay first-class and never route through this
contract in production. A generic integration is custom until a separate ticket
promotes the harness, on the evidence 19.4 lists.

## Consequences

A harness integrates as itself, and its journal lines say so.

klin's compatibility promise stays the size it was: three hosts, with real
payload fixtures behind them. The generic contract adds a documented seam, not
a fourth host to maintain.

The porting kit is now a public surface with its own upkeep. A change to the
internal event that reaches the generic shape is a version 2 of the contract,
not a silent edit of version 1.

A custom integration cannot reach the `ask` decision, so it is measurably
weaker than Claude Code on one class of call. That is stated rather than
smoothed over; closing it needs a native adapter, which is what promotion to
first-class is for.
