# The turn is the window, and radius only reports

> ADR 0017 amends this record. The stamp is also the hook's base, it is a
> parented commit under `refs/worktree/klin/turn`, it moves by one rule on session
> start and prompt alike, and it is guarded. The paragraph "The stamp is not
> guarded" no longer holds. ADR 0019 moves it out of `.klin/`.

ADR 0008 took `diff-radius` from the Klin spec as a gate worth having. Building
it showed that the gate shape does not survive contact with what klin is. What
survives is a report on a window klin did not previously own.

cleat has no equivalent, so there is no behavioural specification to work from.
Its nearest relative, `report-hotspots.py`, is deliberately not a gate, and
`quality/STRATEGY.md` gives the reason: "churn alone chases harmless
refactors ... A report, not a gate: it prioritizes work, it does not block a
commit."

## Three findings forced the shape

klin had no window that matches a turn. Measured against the base, a long
branch reads as permanently wide. Measured against uncommitted work, one
`git commit` resets it, which is the defeat ADR 0008 rejects for scoping.
Claude Code fires `UserPromptSubmit` when the person speaks, and that is the
boundary. klin stamps the tree there, so everything inside the window is work
the agent chose. That makes "unprompted" literal.

A gate failure here has only one remedy, and it destroys work. Every other
klin failure asks for more: simplify the function, remove the escape, shorten
the document. This one would say the change is too wide, and the only way to
green is to undo it. An agent will take that route.

A `Stop` hook that does not block reaches nobody. On exit 0 its output goes
to the debug log, not to the model and not to the person. So report-only at
`Stop` is not a softer refusal, it is silence. `UserPromptSubmit` on exit 0 adds
its output to the model's context, and its `systemMessage` field reaches the
person.

## The decision

`klin radius` runs on `UserPromptSubmit`. It reports the turn that just ended
and stamps the one starting. It blocks nothing, it is not in the `CHECKS` table,
and it never runs in CI. On a pull request a reviewer already sees the diff
size, so a CI failure would add nothing. Mid-turn nobody is looking, which is
the only moment the number carries information.

It measures four things, from three `git diff --numstat` invocations against the
stamp: the raw line and file count, the gap under `-w`, the gap under `-M`, and
the number of distinct parent directories touched. The gaps are the strongest
signal. A person rarely asks for reformatting or for code to move between files,
so those lines name unprompted work directly. `docs/diff-size-research.md`
frames the same gaps as excuses that explain away a large diff. Judging an agent
turn inverts that reading.

Every invocation pins `--diff-algorithm=histogram` and passes `-M` or
`--no-renames` by name. Line counts move with both, so an inherited git setting
would make the same turn measure differently on two machines.

The report is printed only when the turn exceeds a value `init` derived, so its
presence is itself the signal. `init` takes the 90th percentile over the last
200 non-merge commits, for lines and for directories, and either one exceeded is
enough. Merge commits restate a branch and would double-count. A commit count
beats a day window, because a quiet month must not shrink the sample. Below 50
non-merge commits `init` writes no section and says why, since a percentile over
six commits would fire on almost every turn.

Nothing is excluded. A lockfile bump would dominate the numbers, so the report
names the largest single file instead. An exclusion list is a key that rots, and
naming the file is cheaper and never wrong.

The report states facts and asks for nothing. It arrives beside a new request
from the person, and any instruction in it invites the agent to abandon that
request. It describes what the project usually does rather than naming the
derived values, because a stated limit becomes a target.

## Consequences

The last turn of a session goes unmeasured, because the report needs a following
message. A second hook at `Stop` would close that, and one idea living in two
places drifts, so the gap stands.

When it cannot measure, it prints nothing and exits 0. That covers a missing
stamp on the first turn, no git repository, and a config with no `radius`
section. ADR 0005 says a missing key is an error naming the key, and this is an
exception to it, because an error on `UserPromptSubmit` becomes noise in every
message a person sends. `klin radius --report` is the person's command and does
raise the error.

The section sits in the config but outside `gate --list` and `--strict`. ADR
0010 says naming a gate is a claim that it runs, and this is not a gate.

The stamp is not guarded. Report-only leaves the agent nothing to gain by
deleting it, and a missing stamp already prints nothing.

`init` appends `.klin/` to `.gitignore`. `changed::files` lists untracked files,
so an unignored stamp would join the set every scoped gate judges, and that is a
wrong measurement rather than a mess.
