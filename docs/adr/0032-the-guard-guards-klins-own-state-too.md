# The guard guards klin's own state too

> Amends ADR 0027. The guarded set is `klin.json` and klin's state directory.
> Nothing else comes back.

ADR 0027 shrank the guarded set to one file, and it was right about every
file it dropped. A host's hook file holds permissions and MCP servers beside
the hooks. A lint config holds rules a person asks an agent to change. Those
are files ordinary work edits, and klin cannot read a loosening apart from a
fix in any of them, so it refused work that has nothing to do with klin.

klin's state directory came out with them, and the argument does not reach
it. Nothing a person asks an agent to do edits `.git/klin/turn`. It is not a
file with two readings; it is klin's own record, and the only reason to write
it by hand is to change what klin concluded.

Two things have made that visible.

The guard already denies `klin turn reset`, which moves the stamp and reopens
the window a gate failed in. One line of shell over the same file does the
same thing and was allowed, so the deny bought nothing against an agent that
would take it.

ADR 0031 then put a second value there. A stop that blocks on a deleted test
records what it asked about, beside the stamp, and the next stop lets those
deletions through. Writing that record by hand skips the question. ADR 0031's
own third bullet rejects a window-kind rule because "the guard allows edits to
the state directory", while its chosen mechanism rests on the same directory.
That reasoning cannot hold both ways.

## The decision

The guarded set is `klin.json` and klin's state directory: a `klin` directory
under a git directory, which is where ADR 0019 put it. A worktree keeps its
own under `.git/worktrees/<name>/klin`, so the two names need not sit side by
side, and the rule reads a `klin` component anywhere after a `.git` one.

The three decisions of 9.4 are unchanged, and the state directory takes the
same ones the configuration takes. An edit tool whose path reaches inside it
and a redirect onto it are `deny`, and the reason names `klin turn reset` as
the command a person runs instead. A command outside the reader list that
names it is `ask`. Every reader still reads it: `cat .git/klin/turn` is
`allow`, as `cat klin.json` is.

`klin cache clean` stays open to an agent. The supported route to clearing
klin's state is a klin command, the way fixing the code is the supported
route past a gate.

## What this gives up

A person who wants an agent to clear a stuck state directory meets a refusal
and has to run the command, or clear it themselves. That is one refusal on
work that is rare, against a record klin reads to decide whether a gate holds.

It is still feedback and not a fence. An agent that writes the file through a
route the guard cannot read as a write reaches it, and an agent running with
no hook meets no guard at all. ADR 0009 is unchanged: CI is what bounds a
run. The guard is worth the same here as it is worth for `klin.json`, and no
more.

## What was ruled out

- Bring back the rest of ADR 0027's set. Its reasoning about those files
  still holds, and nothing in it has changed.
- Sign the record so a forged one is detected. klin has no secret to sign
  with, and a key on the same disk as the record is not one.
- Keep the record somewhere a person writes, such as `klin.json`. The
  `accepted` list is a person's, in a reviewed commit, and a per-turn record
  klin writes on every stop does not belong in a tracked file.
