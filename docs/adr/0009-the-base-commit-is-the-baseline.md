# The base commit is the baseline, and the tool writes nothing

> ADR 0013 supersedes the paragraph in "Which two trees" that makes a base
> equal to HEAD a tool error with no exception.
>
> ADR 0017 supersedes "Locally the base is the merge-base" for `--hook` mode,
> where the base is the turn stamp. `klin gate` by hand and CI keep the bases
> below.

Until now a gate that ratchets held a committed JSON file: the findings a
person accepted, with the tool, version and config hash that produced them. A
run compared today's findings against that file. `init` wrote one per gate,
`--write-baseline` rewrote it, `--strict` failed when the file was looser than
the code, and the guard refused any write to it.

That file is gone. A run measures two trees, the base commit and the working
tree, and matches sites between them with the engine that already exists. A
finding fails when it is over the ceiling and either did not exist at the base
or is worse than at the base. Below the ceiling nothing is judged. The tool
never writes anything.

## Why

Every tool that started from a stored file grew the same machinery afterwards:
a command that tightens the file, a strict mode that fails when nobody ran it,
a rule that reports entries matching nothing, and a guard over the file. Each
is a chore for a person and a surface for an agent. The tools that started
from git, Semgrep's baseline commit, SonarQube's new-code period, reviewdog's
diff filter, added none of that. What they refined was which two trees to
compare.

The file also carried a problem the git model cannot have. A klin upgrade that
changed a measurement made every baseline read as drift, and a person rewrote
each one. Two trees measured by one binary cancel that out.

An end user's tree now holds `klin.json` and a CI workflow. Nothing else.

## Which two trees

The base is chosen once per run, and a base equal to HEAD is a tool error,
exit 2, never a pass. Comparing a tree to itself measures nothing, and a run
that reports green while measuring nothing is the worst failure this tool has.
That rule closes the shallow CI clone, the push to the default branch with no
event data, and the agent that runs `git commit -am wip` on the default branch
so the next stop sees an empty diff.

Locally the base is the merge-base with the default branch. That is feedback,
not control. An agent can move a local ref with git commands the guard does
not watch, and a long-lived branch compares against a merge-base that main has
since improved on. Both are bounded by CI.

In CI the base is what the host says it is. On a pull request klin compares
the merge result the host checks out against the tip of the target branch, so
a branch that left a function at 12 while main brought it to 6 reads as
worsened, not held. On a push it compares against the event's previous
commit, which covers a multi-commit push where the previous HEAD would not.
The workflow fetches history, and a missing base is the exit 2 above.

A renamed file is measured at its old path at the base, so a move is not a
tree of new debt.

## Accepting debt

A person adds one entry to an `accepted` list in the config, keyed the way a
site is keyed, with the value they allow:

```json
"accepted": [
  { "gate": "complexity", "file": "src/checkout.rs", "text": "fn checkout(", "cc": 14 }
]
```

The config is under the guard and under code owners, so this is the reviewed
act, and `git blame` on the line lands on the pull request that made it. Under
`--strict` an accepted entry that matches nothing is a failure, so the list
cannot grow loose the way a todo file does. The remedy is deleting one line,
not running a command whose output gets committed.

A gate that reads an external report has no base report in git. Its prior is
the sites at the base plus the accepted list, matched by the same engine, and
`init` may fill the accepted list for such a gate on day one. ADR 0008 refused
a list inside the config because the guard could not tell a config edit from a
baseline edit, and because provenance needed a home. The tool no longer writes
and there is no provenance, so both reasons are gone.

## Consequences

`--strict` has three failures left: an available gate that is neither
configured nor set to `false`, a config error, and an accepted entry that
matches nothing.

The five outcomes become three. New and worsened fail, held passes. Improved
and unmatched have nothing to compare against and are not outcomes.

The guard protects the config and the hooks. The baseline rules and the
refusal of the write flag go, because neither exists.

The differential test against cleat shrinks to measurement parity for escapes
and doc-size. Its three surviving cases write cleat's baseline at the base
commit and check the working tree against it. The looser-baseline case has no
equivalent here. ADR 0007's reason for matching cleat's keys, that the
differential test reaches every gate, weakens to a preference: every `baseline`
key is gone and the accepted list is ours alone.

ADR 0003's rule about held-out entries for unparseable files has no entries to
hold out. A file the grammar rejects is still named and still exit 2.

ADR 0008's last two sections, on the baseline staying outside the config and
on signing against automatic writes, are superseded.

Every run parses two trees. Under `--changed` in the hook the base version of
each changed file comes from `git show`, and the cost is one extra parse per
file. A whole-tree run in CI materializes the base in a detached worktree and
roughly doubles in parse time.

If klin must hold the ratchet where CI cannot fetch history, or on a host with
no pull request merge ref, this model cannot, and the stored file would have
to come back. Neither is a target today.
