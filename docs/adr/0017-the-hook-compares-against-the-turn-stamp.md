# The hook compares against the turn stamp

> Supersedes ADR 0009 and ADR 0013 for `--hook` mode. `klin gate` by hand and
> CI keep the base those records choose. Amends ADR 0014: the stamp is a
> parented commit under a ref, it moves by one rule, and it is guarded.

ADR 0009 made the merge-base with the default branch the local base. ADR 0014
built a turn stamp for the radius report, because the merge-base was the wrong
window for a turn. The stamp turned out to be the right window for the hook
too, and the merge-base the wrong one, for three reasons.

On the default branch with a remote, the merge-base is `origin/main`, so the
hook measures unpushed commits and then, after a push, nothing. With no remote
the merge-base is HEAD, so the hook measures uncommitted work and then, after
a commit, nothing. ADR 0013 exists to sort out when that "nothing" is honest.
On a long branch the merge-base is far behind, so every stop re-judges every
file the branch touched, and the hook slows as the branch grows.

A turn stamp has none of these. A commit inside the turn moves nothing. The
changed set is the files the turn changed, and an empty set is an honest
"nothing changed" rather than a hole.

## The decision

In `--hook` mode `before` is the turn stamp and `after` is the working tree.
The stamp is a commit made with `git commit-tree` over a tree from a temporary
index that holds every file `.gitignore` does not exclude, with HEAD at
stamping time as its parent. It is held under `refs/worktree/klin/turn`, so
`git gc` does not prune it, and the ref is never pushed. It sits under
`refs/worktree/` because git shares every other ref across the worktrees of
one repository, and each worktree has its own stamp. The `turn` file in the
state directory holds the commit id, the time and the verdict of the last
stop. A stop holds a lock on the state directory from before it measures
until after it writes the verdict, so two stops in one worktree cannot leave
an older green over a newer red.

One rule moves the stamp, applied on session start and on every prompt
submitted alike. The stamp moves to the current working tree when no stamp
exists or when the last stop ended green. Otherwise the stamp stays. So debt
an agent left behind stays new until it is fixed or a person accepts it,
across turns and across sessions. An acceptance does not move the stamp. An
accepted entry is a `before` entry, so the accepted site is held on the next
stop and the stamp moves when that stop ends green. Moving on acceptance
would make every other open finding inherited debt.

Session start follows the same rule, not a fresh stamp. A fresh stamp on
session start would photograph the mess a red stop left behind and read it as
held in the next session. That was a hole in the first draft of this rule.

`klin turn reset` moves the stamp regardless and prints that a person moved
it. It is the escape hatch for abandoned work, where a reviewed acceptance is
the wrong instrument. The guard denies it by name from an agent, as it denies
`init`. The hook never prints the command.

A missing stamp on a stop is a window from HEAD and a note that names the
missing stamp, because a stamp that was there and is gone was deleted.

## Why the stamp is guarded

ADR 0014 left the stamp unguarded because a report leaves nothing to gain by
deleting it. The stamp now holds the verdict that keeps a window open, so
deleting it converts every open failure to held in one command. That is the
single cheapest route to green in the design, and the guard exists for exactly
that shape of act. Guarding it costs nothing a reviewed commit would cost,
because a deleted stamp needs no review to restore.

## Consequences

`klin gate` without `--hook`, and CI, are unchanged: the pull request base,
the push event's previous commit, or the merge-base, with ADR 0013's rules
for a base equal to HEAD.

The hook's cost is proportional to the turn, not to the branch.

A commit inside a turn does not move the derivation commit of ADR 0016. The
derivation commit is the stamp's parent, fixed when the stamp is taken, so one
turn is judged against one set of derived values from start to end.

Amended 2026-09-09. The first text of this record moved the stamp when the
`accepted` section changed, said a commit inside a turn moved the derivation
commit, and relied on a monotone derived ceiling. The 2026-09-09 review of
`docs/SPEC.md` showed that the first forgave unrelated findings, the second
contradicted the stamp's fixed parent, and the third was false. SPEC.md 6.2,
6.5 and 6.6 carry the current rules, including the ref location and the lock.

The last turn of a session is measured at its stop like any other. ADR 0014's
gap about the last turn concerned the radius report, which still needs a
following prompt to be read.

The stamp appears in `git log --all`. A dangling commit would not, and would
be pruned instead.
