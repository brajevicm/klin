# klin's own state lives in `.klin/`

> Superseded by ADR 0019. The state lives in the git directory, or under
> `KLIN_STATE_DIR`, and needs no `.gitignore` line.
>
> Supersedes the path in ADR 0004 and ADR 0012. The policy each records still
> holds. Only the location changes.

ADR 0004 put the build stamp at `.klin-build-blocked` beside the config, and
argued the location: not under `target/`, which an agent empties as a matter of
routine. That argument is untouched. What changed is that ADR 0014 adds a second
state file, so the root dotfile is now a growing pattern rather than one file.

Each such file needs its own `.gitignore` line, and a line someone forgets costs
more than clutter. `changed::files` lists untracked files, so an unignored stamp
joins the set every scoped gate judges, and the run measures the wrong thing
while reporting green.

So klin's state moves to `.klin/`, and one ignore line covers every file klin
ever puts there. The build stamp becomes `.klin/build-blocked` and the turn
stamp `.klin/turn`. Neither is guarded, for the reasons ADR 0004 and ADR 0014
each give.

`.agents/` was the alternative, and it was rejected. The name does not say who
owns the directory, so klin, an MCP server and a hook script could each claim it
and collide. It also reads as a cross-tool convention, and none exists. If one
emerges, klin can move again.

## Consequences

This changes shipped behaviour. Anyone already running klin has
`.klin-build-blocked` in `.gitignore`, and that line stops matching. `init`
appends `.klin/` when it is absent, which covers a project that re-runs it, and
not one that does not.
