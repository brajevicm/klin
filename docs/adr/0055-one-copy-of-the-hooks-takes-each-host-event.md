# One copy of the hooks takes each host event

> Amends ADR 0046, on plugin ownership, and the consequence in ADR 0053 that
> `klin install` writes no hooks a plugin already supplies. Spec 9.8 and 19.3.

A host can run several copies of klin's hooks for one event on one machine. A
native plugin can run beside committed project hooks or user-scope hooks.
Cursor runs the hooks in Claude Code's settings files beside its own by
default. Each copy ran klin in full, so the prompt counter moved twice, two
gates raced at the stop, and a refusal was journaled twice.

`klin install` avoided the first case: it wrote no project hooks when a plugin
or user hooks already served the host. A teammate without the plugin then had
no hooks at all. It could not avoid Cursor's case, where a repository with
klin's hooks for Claude Code and for Cursor ran klin twice in Cursor.

## The decision

**One copy takes each event, and `klin install` always writes the hooks and
the skill.**

- Each copy claims the event's identity in the state directory before it acts.
  The identity is the values of the payload fields that name one event. The
  probes of #317 recorded them for Claude Code, Codex CLI and Cursor
  (`docs/HOST_COMPATIBILITY.md`).
- A copy yields when another copy holds the claim or let it go less than two
  seconds before. The host starts the copies of one event together, and the
  next event only after all of them answered.
- A radius or a stop that yields prints nothing and exits 0, so it cannot
  weaken the other copy's block. A guard that yields still gives its answer
  and writes no journal line, because a wrong match would otherwise open a
  call.
- `klin install` writes the hooks and the skill beside a plugin or a user
  file that already holds klin's hooks. The run says that the committed copy
  yields on this machine.

## Considered options

- Keep the plugin as the owner and write nothing. This leaves a teammate
  without the plugin ungated, and it does nothing for Cursor.
- Make the committed copy check at every event for a plugin and yield to it.
  Cursor records no enabled state klin can read, so a copy on disk would
  silence the only copy that runs.
- Judge a copy by its hook line. The copies run different lines, and the
  version each line runs can differ.

## Consequences

A copy that starts more than two seconds after the other copy finished acts
again. The first run of a plugin wrapper that downloads its binary can do
this once per version.

A person probed Cursor 3.22.7. It sends every copy the same payload, and it
also runs a Claude Code plugin that the project enables. A shell call reaches
the native copy and an imported copy as two different events, so a Cursor
shell call is named by its message and its command. Two equal commands in one
message then write one journal line between them, and each copy still refuses.
