# The guard answers only where it can prove a write

> Reverses ADR 0011's central argument, that a reader list beats a writer
> list. The three decisions of ADR 0020 stand, and so does the guarded set of
> ADR 0027 and ADR 0032.

ADR 0011 chose a reader list because it wanted to close the route: name the
commands that change nothing, and treat everything else as suspect. ADR 0020
softened the price from `deny` to `ask` but kept the shape, so the default
answer for a command the guard had not heard of was still a question.

That default is where every false positive comes from. The guard asks about
`echo klin.json`. It asks about a `gh issue create` whose arguments name the
file. It asks about `rm *.json` in a directory that holds no `klin.json`. It
asks about `git apply klin.json`, which reads the file as a patch rather than
writing it. Each question spends a person's attention on a command that
changes nothing, and a person who is asked four times learns to say yes
without reading.

The goal ADR 0011 was built for is not the goal any more. ADR 0009 already
says CI bounds a run and the guard is feedback. So a missed write costs
nothing, while a false question still costs a person a decision.

## The decision

The guard answers only where it can prove what a command does. It matches a
short, closed list of shapes that are writes under any reading, and allows
everything else.

The three decisions keep their levels. `deny` for an edit tool path, a
redirect, `klin init` and `klin turn reset`. `ask` for a command that writes a
guarded path. `allow` for everything else. What changes is which commands
reach an answer at all.

### A writer list replaces the reader list

A command asks when its command word is `rm`, `rmdir`, `unlink`, `shred`,
`mv`, `truncate` or `tee`, or `sed` or `perl` carrying `-i`, and a guarded
path sits among its arguments. Every argument of each of those is a file it
writes, so the guard needs no rule about which argument is which.

`cp` and `install` stay off the list although both write. Each reads its
first argument, so `cp .git/klin/turn /tmp/backup` would be a question about
a backup.

A writer is found behind the prefixes klin's own name is found behind: an
assignment, `env`, `npx`, `pnpm`, `bunx`, `time`, `nice` and `sudo`. Without
that, `sudo rm klin.json` would pass while `sudo klin init` is refused, which
is the same command word rule read two ways in one file.

### Paths resolve instead of matching names

The guard compares against this tree's `klin.json`, beside the root
`git rev-parse --show-toplevel` names, and this tree's own state directory,
which `state::dir` resolves. A path resolves from the directory the guard
runs in, with `.` and `..` taken out and the symbolic links its existing part
carries followed, so a name a host wrote through a link matches the one git
prints. A path the guard cannot resolve is allowed.

This is the guard's first `git rev-parse`, which section 9.4 already permits.
It runs only when a path needs proving, so a tool call that names none never
pays for it, and the two calls it makes cost about 11 milliseconds against a
budget of 50.

### Bail out to allow

The guard matches no path in a command that holds shell it does not read: an
unbalanced quote, `$(`, a backtick, `${`, `<<`, a backslash, or a `cd`
command word. A token holding a wildcard proves nothing about what it expands
to, so path matching skips that token.

An unbalanced quote allows the whole command, because the command then does
not say where its arguments end. A `>` inside an argument is a character of
that argument, so a commit message that holds one is not a redirect. The
other six suppress path matching alone. `klin init` and `klin turn reset`
name no path, so their deny is unaffected,
and a command substitution in command position counts as a prefix in front of
klin's own name, the way `env` and `sudo` do.

This deletes the heredoc reader, the command-substitution lifter, the blind
splitter and the nesting tracker, which were four parsers written to answer
questions the guard no longer asks. The parser may miss a write freely.

## What this gives up

Each of these is allowed after the change:

- `echo '{}' > *`, because a wildcard redirect target names no file the guard
  can read.
- A heredoc carrying a redirect onto `klin.json` on the same line.
- `cp`, `ed`, `patch`, `git apply`, `git checkout -- klin.json`, and
  `find -delete`.
- Another worktree's state directory, because the guard resolves this tree's
  own.
- Anything behind a `cd`.

Two ceilings are recorded here rather than written into the guard. A command
word is read by its basename, so a script of the tree's own at
`./scripts/rm` is read as `rm`, and its arguments are read as files it
writes. And
`truncate -s` has its size read as an argument like any other, which is
harmless while no size resolves to a guarded path.

One more follows from resolving the tree root. A monorepo whose `klin.json`
sits in a subdirectory is guarded at the root instead, where `Config::load`
would walk up from the working directory to find the file. The guard MUST NOT
read the configuration, and looking for the file it guards is close enough to
reading it to stay out of the guard.

## What was ruled out

- Keep the reader list and grow it. Every session finds another reader, and
  the list can only ever remove a false question after a person has already
  been asked one.
- Match a name anywhere in the command, as the guard did before ADR 0011.
  That is the over-refusal ADR 0011 was written to fix.
- Prove a write by opening the file, or by running the command in a sandbox.
  The guard runs on every tool call under a 50 millisecond budget, and it
  must decide before anything runs.
