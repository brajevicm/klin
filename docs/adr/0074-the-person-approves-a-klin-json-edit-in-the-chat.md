# The person approves a `klin.json` edit in the chat

> Amends ADR 0020 and ADR 0033 for `klin.json`, and spec B.9.4. Recorded for
> #576, 2026-10-14, from the owner's decision of 2026-10-10.

Until now the guard denied every edit tool call on `klin.json`, and a person
changed the file by hand in a reviewed commit. The owner decided on
2026-10-10 that an agent may edit `klin.json`, also in the klin repository,
but only with the person's explicit approval of that one edit. The agent
never edits it on its own. The person runs no command to approve.

## The decision

The guard denies an edit tool call on `klin.json` and records a request: the
file the call would leave, and a hash of a random code. klin tells the person
the code and what the edit changes, on a channel only the person reads. The
person replies with a prompt that is exactly `approve <code>`. The same edit
then passes once. Spec 10.8.2 holds the rules.

1. **The agent never sees the code.** In Claude Code, task notifications and
   subagent reports also fire `UserPromptSubmit`, and the session that wrote
   #576 saw this happen. An agent that knew the code could relay it into one
   of those messages. So the code goes only to `systemMessage`, and the
   agent's reason says only that klin asked the person.
2. **The whole prompt must be the phrase.** Such host messages arrive
   wrapped in tags, so a prompt that only holds the phrase approves nothing.
   This is the second layer, and the only one on a host with no person-only
   channel.
3. **The approval binds the exact file.** The request holds the resulting
   file itself, not its hash. klin carries no collision-resistant hash, and
   `klin.json` is small. Any other edit gets a new request.
4. **One use, one turn.** The first call that uses the approval spends it.
   Every copy of klin's hooks for that one call (ADR 0055) passes
   too. The approval expires at a Stop that does not block, or at the next
   prompt.
5. **A shell write of `klin.json` is a deny.** klin cannot compute the file a
   shell command leaves, so a proven shell write gets no approval path. Its
   reason names the edit tool. This reverses the `ask` that #426 kept for
   `cp` and `install` onto `klin.json`. `ask` stays for a shell write of the
   state directory.

The person reads each setting the edit adds, removes or changes. Whether a
change loosens or tightens the policy is #575's work.

## Rejected

- **Claude Code's `ask` decision.** Only Claude Code has it on this event, the
  dialog shows the raw edit and not the policy difference, and an approval
  there needs no code a relayed message could forge.
- **A command the person runs**, such as `klin setup --apply`. The owner
  wants no command.
- **A hash of the resulting file.** `state::hash` is FNV-1a, and an agent can
  craft a collision for it. Holding the file costs a few kilobytes.

## Consequences

The state directory is readable by the agent, so the request holds a hash of
the code. That hides the code from a reading and not from a search. A host
may also write the `systemMessage` into a transcript the agent can read. The
guard is feedback, not a boundary: CI and CODEOWNERS stay the authority
(`docs/THREAT_MODEL.md`), and #575 shows a loosening in review.

Cursor sends no edit payload klin reads, and the harness protocol carries
none, so an edit of `klin.json` there is denied with no approval, as before.

These host checks stay open until a person runs them:

- that a PreToolUse `systemMessage` reaches the person and not the model, on
  Claude Code and on Codex, and that Codex honors the JSON deny;
- which messages fire the prompt hook, and how each host wraps them;
- whether Cursor has any person-only channel.

If they fail, the fallback is #592's: the skill shows the change as a diff,
and the person applies it.
