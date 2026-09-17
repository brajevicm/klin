# The generic harness contract, version 1

klin maintains a native integration for Claude Code, Codex CLI and Cursor.
Those three are first-class: they get built-in adapters, native plugins,
compatibility tests and klin's own documentation.

Every other harness integrates through the contract in this directory. You
translate your harness's lifecycle into klin's event shape, run the klin
binary, and translate the decision back. klin ships no adapter for your
harness, and using this contract does not make it first-class.

Start with [the porting guide](../../docs/HARNESS_INTEGRATION.md). It carries
the worksheet, the lifecycle mapping and the conformance levels. This file is
the wire shape alone.

## What is here

| File | What it is |
| --- | --- |
| `event.schema.json` | The event your integration sends klin on stdin |
| `response.schema.json` | The decision klin prints on stdout |
| `adapter.sh` | A reference shim: event in, klin command, decision out |
| `fixtures/` | One event of each kind, to run your integration against |

## The event

Your integration writes one JSON object to klin's stdin:

```json
{
  "klin_protocol": 1,
  "event": "pre_tool",
  "root": "/repo",
  "session": "5f2c1a9e",
  "tool": "write_file",
  "file_paths": ["/repo/klin.json"]
}
```

`klin_protocol` is the version of this contract, and it is what tells klin the
event is yours rather than a host's. klin speaks version 1. Any other version
is refused with a message naming the version klin does speak; it is never read
as some other harness's event.

`event` is one of `session`, `prompt`, `pre_tool` and `stop`, and it decides
which klin command you run:

| `event` | The command to run |
| --- | --- |
| `session` | `klin radius` |
| `prompt` | `klin radius` |
| `pre_tool` | `klin guard` |
| `stop` | `klin gate --hook --changed` |

`root` is the repository the agent is working in. Leave it out if you run klin
inside that repository already.

### Send only what you can prove

`file_paths` holds the paths you can prove the tool call will touch. `command`
holds the shell command the agent is about to run, when you know it. If you
cannot prove either one, leave it out. klin allows the call.

`tool` is a label for the journal and for the message a person reads. klin
never reads a file write out of it, and neither should your shim: do not guess
paths out of arbitrary tool arguments. A guessed path refuses work that was
never a threat, and teaches an agent to route around the guard.

## The decision

klin answers with one JSON object on stdout, and repeats the same answer in its
exit code:

```json
{ "action": "allow" }
{ "action": "deny", "reason": "klin: refused — this would change the configuration (klin.json)." }
{ "action": "block", "message": "<the gate report>" }
{ "action": "tell", "message": "<a note for the person>" }
```

- `allow` (exit 0) — let the call or the stop through.
- `deny` (exit 2) — refuse this tool call, and show the agent the reason.
- `block` (exit 2) — refuse this stop, and put the message in front of the
  agent so it can fix what the message names.
- `tell` (exit 0) — let the turn end, and put the message in front of the
  person.

klin prints at most one decision object, on a line of its own. A blocked stop
also prints the gate report as ordinary text for a person to read, so find the
decision by its `action` field rather than by taking the last line. A stop that
passes prints no decision at all: exit 0 and no `action` means the turn may end.

klin also writes a refusal to stderr, so it survives where your shim drops
stdout. If you can read only one channel, read the exit code: nothing but a 0
lets a call or a stop through.

There is no `ask`. klin asks a question on hosts that enforce one, and this
contract has no way to prove your harness does, so the class of call that would
be a question is refused instead. That is a real capability difference from the
first-class hosts, and the porting guide says how to record it.

## Trying it

```bash
integrations/generic/adapter.sh < integrations/generic/fixtures/pre-tool-path.json
```

The fixtures name `/repo`, so change `root` to a repository of your own that
holds a `klin.json`. A repository without one is not opted in, and klin stays
silent in it by design.
