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

The README installs klin in one command run from the repository: the
installer, then the installed binary by its full path,
`installer=$(curl …) && sh -c "$installer" && ~/.local/bin/klin install`. The
release configuration puts the binary in `~/.local/bin`, so the full path
works before PATH does. The download is held in a variable rather than piped,
because a pipeline exits with the status of its last command: a `curl` that
failed would hand `sh` nothing, `sh` would exit 0, and a klin an earlier
install left behind would run.

The terminal that ran the installer still has no `~/.local/bin` on PATH, and
a host started from it runs the hooks with that PATH. So each hook line klin
writes looks in `~/.local/bin` after PATH. Before this, the README's new
terminal hid the gap.

`--user` keeps the refusal. A person's home shows the hosts that person runs,
and a home that shows none has no team whose hosts klin cannot see.

## Consequences

A repository with no host directory gains `.claude/`, `.codex/` and
`.cursor/` on its first install, with a skill under `.claude/skills/` and
`.agents/skills/`. A person who wants fewer names them with `--host`.

A person who sets `KLIN_INSTALL_DIR` gets the binary somewhere else, and the
README's full path does not reach it. That person chose the directory and
runs `klin install` from there.

Homebrew and npm stay with #315.
