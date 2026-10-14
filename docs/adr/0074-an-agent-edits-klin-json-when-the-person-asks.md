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

One exception stays. `klin.json` is also the opt-in file, and without it klin
answers nothing. So a proven `rm`, `rmdir`, `unlink`, `shred` or
`truncate` of the `klin.json` at the tree root, an `mv` that moves it away,
and a Codex patch that deletes it or moves it away are still `ask`: a person
decides whether klin goes off. Moving another file onto it is `allow`.

The rule moves into the klin skill, which every host's plugin carries. The
agent changes `klin.json` only when the person explicitly asks for that
change: the request names `klin.json`, or the setting or entry to change, and
"make the gates pass" is not one. When the person gave the exact change, the
agent makes it and shows the diff. When the agent chooses any value, it shows
the change and writes it only after the person says yes. It never adds an
`accepted` entry or raises a ceiling to make a gate pass.

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

klin does not enforce the rule. The Stop and `klin check` both judge under the
`klin.json` in the tree, so an agent that ignores the skill can weaken it and
pass both, in the same turn and in CI. Only a person who reads the
`klin.json` diff catches that, and CODEOWNERS over `klin.json` makes that
review required (`docs/THREAT_MODEL.md`). #575 adds a review item that names
what a changed `klin.json` loosens.

A proven shell write of `klin.json` other than a delete or move, which #426
put to the person as `ask`, is now `allow` like an edit. The journal writes no
`config-write` line, and `klin report` still reads one from an older journal.
A delete or move of `klin.json` keeps the `config-mention` reason.
