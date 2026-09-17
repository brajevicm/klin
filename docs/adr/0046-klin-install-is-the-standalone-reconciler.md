# `klin install` is the standalone reconciler

> Amends ADR 0023, ADR 0030 and ADR 0045, each of which named
> `klin init --hooks` as the explicit-hooks route.

klin maintains a native plugin for Claude Code, for Codex CLI and for Cursor.
A plugin carries the host hooks, the skill and a pinned wrapper that fetches a
pinned runtime, so it is self-sufficient and it stays the preferred local
product experience. ADR 0023 and ADR 0030 decided that, and nothing here
changes it. A plugin user installs no second binary.

Three cases still need explicit files: a host surface that loads no plugin, a
managed or team setup that wants the integration committed and covered by
CODEOWNERS, and a person who deliberately runs the standalone binary. Those
cases were served by `klin init --hooks`, which had two faults.

The first is that the flag sat on the wrong command. `init` writes the
configuration a person owns. Host integration is a different subject, and
hanging it off `init` gave one command two jobs, three flags and a confusing
story about what a plugin user must run.

The second is that the writer only ever added. It skipped an event that
already called klin, so it was idempotent for an install that was already
correct and blind to one that was not. A matcher from before #214, a command
an older klin wrote, a duplicate entry that ran the lifecycle twice, an entry
on an event klin no longer writes, and a half-written file all survived a
second run. A person on the binary route could only take a later fix by
deleting the hook file by hand.

## The decision

`klin install` is one command that opts a repository in, selects the hosts to
serve and reconciles the explicit hook files klin owns. `klin init` keeps the
configuration survey and `--pin`, and its `--hooks`, `--host` and `--global`
flags are gone. Spec 19.3 holds the contract.

Reconciliation, not addition, is the rule. klin recognizes the entries it
owns — a command that runs the klin binary on `radius`, `guard` or `gate` —
and brings them to the current contract: it adds what is missing, replaces a
stale command or matcher where it stands, removes a second copy on one event,
removes an entry on an event klin no longer writes, and leaves every entry
that is not klin's exactly where it is. One klin hook in a file is not read as
a complete install.

The marker goes to the repository root that `git rev-parse --show-toplevel`
names, not to the working directory, because a `klin.json` under
`repo/apps/web` opts nothing in.

A host is selected on evidence: `--host`, the host's own configuration in the
scope, or a native plugin klin can prove is enabled. A marker directory proves
the host and never the install. With no evidence and no `--host` the command
refuses and names the supported hosts, because a hook file klin invented gates
nothing.

The run resolves everything it can before it writes anything, and each owned
file is replaced whole through a neighbour and a rename. Where a filesystem
failure still lands between two writes, the output names what was written and
what was not.

The local user scope is `--user`, not `--global`. Section 19.0 reserves
`user` for one person on one machine, and `global` reads as a promise about
every repository everywhere, including the cloud and remote agents that a
local settings file never reaches.

`klin install` writes a person's configuration and integration files, so the
guard refuses it from an agent, beside `klin init` and `klin turn reset`.

## Consequences

A plugin user runs nothing new. A binary user runs `klin install` once, and
runs it again after an upgrade to take a hook contract fix; a second run over
a complete installation writes no file.

A person whose hook entry runs another klin command, such as `klin stats`,
keeps it. klin claims only the three commands its hooks run, so a fourth hook
command would have to join that list before klin could own it.

An agent cannot repair its own host integration. That is the same trade the
guard already makes for `klin init`: the file is a person's, and a person runs
the command in a reviewed commit.
