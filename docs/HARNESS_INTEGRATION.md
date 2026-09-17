# Porting klin to another harness

This guide is for someone integrating klin with an agent harness klin does not
maintain. It carries the worksheet you fill in, the lifecycle mapping, and the
conformance level your answers land on. The wire shape lives beside the
schemas, in [`integrations/generic/README.md`](../integrations/generic/README.md).

## Where this sits

klin ships three different things, and they are not three routes to one
install. Section 19.0 of [the specification](SPEC.md) states the contract; this
is the short form.

Claude Code, Codex CLI and Cursor are **first-class**. klin maintains a native
plugin for each, a built-in adapter in Rust, compatibility tests against the
host's real payloads, and documentation it owns. Those three never route
through this contract.

Everything else is **custom**. You translate your harness's lifecycle into
klin's generic event, run the klin binary, and translate the decision back.
klin ships no adapter, no hook file and no skill placement for your harness,
and a custom integration is outside the compatibility promise that covers the
first-class plugins.

The boundary is there for accuracy. A harness can reach a working integration
through this contract today, and then say exactly what it enforces and what it
does not.

## What you are building

A translation shim, and no second klin. The generic event normalizes into the
same internal event the built-in adapters produce, and runs the same turn,
guard and gate loop from there:

```text
Claude / Codex / Cursor native event
               |
               v
         built-in adapter
               |
               v
          internal event
               |
               v
              klin

your harness's event
       |
       v
your translation shim
       |
       v
 generic event, version 1
       |
       v
 generic adapter -> internal event -> klin
```

Your shim is the whole port. It is small on purpose: read the harness event,
write klin's JSON, run one of three commands, read the answer back.

## The worksheet

Answer these about your harness before you write any code. The answers decide
what you can build and which conformance level you can claim.

1. What event opens a session or a turn?
2. What event fires when a person submits a prompt?
3. Can a tool call be intercepted before it executes?
4. Does that event give you file paths the tool will touch, the command the
   agent will run, or neither?
5. What event means the agent is about to stop?
6. Can that stop be blocked?
7. Can a block put a message back in front of the agent?
8. What identifies the repository root?
9. What stable session or conversation id can you send?
10. Where does the harness load project or user agent skills and instructions
    from?

Question 4 decides how much the guard is worth to you. A pre-tool event that
carries no proven path and no known command only notifies you that a call
happened, and klin allows every call it carries.

## The lifecycle mapping

| Your harness can | klin's responsibility | The command |
| --- | --- | --- |
| Start a session, or submit a prompt | Open or update the turn, and report the spread | `klin radius` |
| Intercept a tool before it runs | Guard the configuration against a proven path or command | `klin guard` |
| End a turn, and be blocked | Run every gate over the turn's window, and return the report | `klin gate --hook --changed` |
| Load agent skills or instructions | Carry klin's canonical skill | see [The skill](#the-skill) |

Each command reads one generic event on stdin. You do not need `--host`: the
`klin_protocol` field places the event. `--host generic` exists as an override
for `klin guard` and `klin gate`, and it refuses any payload that carries no
version of this contract.

You do not have to support every row. You do have to say which rows you support.

## Conformance levels

Name your level in your own documentation, and say what is missing. The levels
describe an integration, not a klin release.

### Full

The harness has all four:

- a session or prompt event that opens the turn;
- pre-tool interception carrying proven file paths, the command, or both;
- an end-of-turn event;
- a way to block that stop and put klin's report in front of the agent, so it
  can repair the failure in the same turn.

This is the closest a custom integration comes to the first-class agent loop.
One difference remains and cannot be closed through this contract: klin's
`ask` decision. On Claude Code an ambiguous call becomes a question a person
answers. Here it becomes a refusal, because nothing in this contract proves
your harness enforces a question. Fail closed and say so; do not weaken it to
an allow.

### Gate

The harness runs an end-of-turn check and reports the failure, but is missing
pre-tool interception, the ability to block a stop, or the channel that returns
the report to the agent.

Document exactly which of those is missing. A Gate integration still catches
regressions; it does not stop an agent from writing over `klin.json`, and it
must not be described as equivalent to a native integration.

### Manual / CI

The harness exposes no lifecycle hook klin can rely on. A person runs
`klin gate` by hand, and CI runs `klin gate --strict` on a checkout the agent
never touched. There is no automatic same-turn feedback, and none should be
claimed.

The CI half of this level is worth having on its own. Section 15.2 of the
specification calls a protected CI run the enforced level: it is the only place
a gate holds against an agent whatever the local integration does.

## The skill

klin's skill is one authored text at
[`plugins/klin/skills/klin/SKILL.md`](../plugins/klin/skills/klin/SKILL.md). The
binary embeds that source, so the plugin copy and the copy `klin install`
writes cannot drift apart.

Point your harness at that text. Do not fork it into your own repository, and
do not write a summary of it for your agent: a second copy drifts, and the
agent then reads two contracts. If your harness loads skills from a path,
`.agents/skills/klin/SKILL.md` is the conventional one, and it is where
`klin install` puts the standalone copy.

## Failure modes to get right

**An unknown protocol version.** If you send a `klin_protocol` klin does not
speak, klin refuses the event, names the version it does speak, and exits 2.
It does not fall back to reading your event as some other host's. Treat that
refusal as a version mismatch to fix.

**A repository that is not opted in.** A repository without a `klin.json` at its
root has not opted in. klin allows every call and blocks no stop there, and
prints nothing. That silence is correct. Do not add a check of your own that
speaks in its place.

**A stop you cannot block.** If your harness cannot block a stop, you are at the
Gate level. Report the failure where a person will read it, and say in your
documentation that an agent can still end its turn over a red gate.

## Promotion to first-class

Using this contract does not make a harness first-class, and klin claims no
first-class support for a harness on the strength of a shim. A harness becomes
first-class only through its own ticket, which:

1. proves the host's current official semantics against real payloads;
2. adds a built-in adapter, a native plugin, or both;
3. adds adversarial compatibility fixtures;
4. defines install, update and trust behavior for that host;
5. moves the harness into the first-class support matrix.

Until then, a custom integration is community-supported, and both klin and the
integration should say so in those words.
