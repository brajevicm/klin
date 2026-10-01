# `klin install` writes every host a repository does not show

> Amends ADR 0046, which refused a run with no host evidence and no `--host`,
> and ADR 0053, which installs klin in two commands. Spec 19.3.

Installing klin took two commands with a new terminal between them. The
installer puts `klin` in `~/.local/bin`, and the shell that ran it does not
hold that directory on PATH yet. `klin install` then needed `--host` in a
repository that showed no host directory, because it refused to guess. A new
repository often shows none: a person clones it and has not started a host in
it yet.

The refusal rested on one reason, that a hook file klin invented gates
nothing. That is true and it is harmless. A repository serves a team, and klin
cannot see which hosts the team's other members run. A hook file for a host
nobody runs is never read. Codex asks a person to trust new hooks, and only a
Codex user sees that step.

## The decision

**Where the repository shows no host directory, `klin install` writes all
three first-class hosts,** and it says so and names `--host`. `--host` still
narrows the run to the hosts it names, and a repository that shows one or more
host directories still gets only the hosts it proves.

A klin plugin enabled in the person's own home does not count here. It is
evidence of one person's machine, not of the repository, and counting it would
make a plugin user commit one host where a teammate commits three.

The README installs klin from the repository: the installer piped into `sh`,
`source $HOME/.local/bin/env`, then `klin install`. The release configuration
puts the binary in `~/.local/bin`, and the dist installer writes that `env`
script to put the directory on PATH, so the shell that ran the installer finds
`klin`. fish sources `env.fish` instead. Where `~/.local/bin` is on PATH
already, the installer writes no `env` script, and the `source` line fails
while `klin install` still runs.

The lines are not chained. A `curl` that fails hands `sh` nothing and `sh`
exits 0, so a person who pastes the block at once runs any klin an earlier
install left behind. The README once held the download in a variable
and chained the lines, inside an outer `sh -c` for fish, to stop that. The
command was hard to read, and no other installer README we surveyed guards
it. A person who runs the lines one at a time sees the failed download first.

A terminal that ran the installer but skipped the `source` line has no
`~/.local/bin` on PATH, and a host started from it runs the hooks with that
PATH. So each hook line klin writes looks in `~/.local/bin` after PATH.
Before this, the README's new terminal hid the gap.

`--user` keeps the refusal. A person's home shows the hosts that person runs,
and a home that shows none has no team whose hosts klin cannot see.

## Consequences

A repository with no host directory gains `.claude/`, `.codex/` and
`.cursor/` on its first install, with a skill under `.claude/skills/` and
`.agents/skills/`. A person who wants fewer names them with `--host`.

A person who sets `KLIN_INSTALL_DIR` gets the binary somewhere else, and the
README's `source` line does not reach it. That person chose the directory and
runs `klin install` from there.

Homebrew and npm stay with #315.
