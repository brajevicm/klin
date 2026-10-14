# An agent edits `klin.json` when the person asks

> Amends ADR 0027 and ADR 0032. Recorded for #576, 2026-10-14, from the
> owner's decision of the same day. The guarded set shrinks to klin's own
> state directory.

The guard denied every edit tool call on `klin.json`, so a person changed the
file by hand. The owner wants every agent on every host to be able to change
it, but only when the person asks for that change.

## The decision

`klin.json` leaves the guarded set. The guard answers an edit, a redirect or a
shell write of it with `allow`, on every host. klin's own state directory
stays guarded as before.

The rule moves into the klin skill, which every host's plugin carries. The
agent changes `klin.json` only when the person explicitly asks for that
change. It shows the person the exact change and writes it only after the
person says yes. It never adds an `accepted` entry or raises a ceiling to make
a gate pass.

## Rejected

- **A code the person types to approve one edit.** The first draft of #576
  built this: the guard computed the file each edit leaves, showed the person
  a diff of the settings, and stored a hash of a random code. It cost about 700
  lines, including a copy of Codex's patch rules, and the agent could still
  find the 4-letter code by brute force from the state directory. Its real
  defense was the same as the skill's: the person decides.
- **Allow the edit in a turn whose prompt names `klin.json`.** A host also
  fires the prompt hook for text the person did not type, such as a subagent
  report, and a substring cannot tell "change klin.json" from "do not touch
  klin.json".
- **Claude Code's `ask` decision.** Only Claude Code has it on this event, and
  the owner wants one rule for every agent.

## Consequences

klin does not enforce the rule. An agent that ignores the skill can weaken
`klin.json`, and the diff a person reviews and a required `klin check` in CI
are what catch it (`docs/THREAT_MODEL.md`). CODEOWNERS over `klin.json` stays
the way to make that review required.

A proven shell write of `klin.json`, which #426 put to the person as `ask`, is
now `allow` like an edit. The journal writes no `config-write` or
`config-mention` line, and `klin report` still reads them from an older
journal.
