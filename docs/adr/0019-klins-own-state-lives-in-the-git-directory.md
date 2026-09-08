# klin's own state lives in the git directory

> Supersedes ADR 0015. The argument in ADR 0004 for keeping state out of
> `target/` still holds.

ADR 0015 moved klin's state into `.klin/` at the tree root so that one
`.gitignore` line covered every file klin writes. The line was the cost it
minimised. There is a location that needs no line at all.

## The decision

klin's state lives under `klin/` in the directory that `git rev-parse
--git-dir` returns. Git never tracks it and never lists it as untracked.
`git clean -fdx` does not remove it. Each worktree has its own, because the
git directory is per worktree. It goes when the repository goes. klin already
requires git, so this costs nothing.

`KLIN_STATE_DIR` relocates the state. Under it klin uses a subdirectory named
by a hash of the git common directory and the worktree path, so two clones or
two worktrees never share a stamp. `~/.cache/klin` is the intended value.
`klin cache clean` removes the survey cache for the current tree, and with
`--all` every entry whose repository no longer exists.

The state is three things. The turn stamp of ADR 0017, as a `turn` file
beside the ref it names. The build stamp of ADR 0004. The survey cache of
ADR 0016. `klin gate --list` prints where they are.

## Consequences

`init` no longer edits `.gitignore`. klin writes nothing that git could see.

Anyone already running klin has `.klin-build-blocked` or `.klin/` in
`.gitignore`. Those lines become inert. `init` may say so when it sees one.

Moving a repository directory under `KLIN_STATE_DIR` orphans its state. A
missing stamp is a window from HEAD with a note, so the cost is one stop with
a wider window. Deleted repositories leave a few kilobytes behind, which
`cache clean --all` removes.

ADR 0004's reason for the location holds in both places. An agent empties
`target/` as a matter of routine and does not empty `.git/` or a home cache.
The stamps are also guarded per ADR 0017, which ADR 0004 could not assume.
