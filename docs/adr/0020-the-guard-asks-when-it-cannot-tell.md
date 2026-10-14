# The guard asks when it cannot tell, and guards the stamps

> Amends ADR 0011. The reader rule stays. What changes is the decision for a
> command outside the reader list, and the guarded set.
>
> ADR 0033 retires the reader rule. The three decisions below stand, and the
> `ask` class becomes a closed list of writers rather than everything outside
> the reader list.

ADR 0011 refused any command outside the reader list that named a guarded
path, and accepted the over-refusal as the price of a closed list. The price
turned out to be high. Six read-only commands were refused across the two
sessions that wrote the spec and its review, every one on a glob such as
`*.rs` or on a `find` over a directory. Each refusal cost a turn and had no
remedy the agent could take.

Two bugs made it worse and are fixed with this record. A token holding `*` was
split at the star and its prefix matched every guarded name when the prefix
was empty. The segment splitter ignored quotes, so a quoted `|` broke a
command in two.

## The decision

The guard sorts a tool call into three decisions.

`deny` is for a clear write: an edit tool whose path is guarded, a redirect
onto a guarded path, a whole-tree restore, `init` in any form, `turn reset`,
and any non-reader command that names the state directory or `refs/worktree/klin`. The
reason names the file and says a person changes it in a reviewed commit, or
names the command a person runs instead.

`ask` is for a shell command outside the reader list whose arguments name a
guarded path. Claude Code's PreToolUse hook accepts `ask` as a permission
decision, and Cursor's hooks answer `allow`, `ask` or `deny` in the same
words. The reason quotes the token that matched. The person decides, and the
agent loses no turn.

`allow` is everything else, including any reader naming a guarded path, and
any glob that does not match a guarded name when read as a pattern.

The guarded set is `klin.json`, each host's hook file, CODEOWNERS, the state
directory of ADR 0019, and `refs/worktree/klin`. The state directory is guarded for a
reason of its own, recorded in ADR 0017: deleting the stamp turns every open
failure into held in one command.

A second class is `ask` for a reason of its own. A verification file
configures a check that klin does not own: lint configuration, test
configuration, a coverage threshold, a CI workflow. An agent that edits one
of these can weaken every check that reads it without touching the
configuration, and klin cannot tell a loosening from a fix. The table of
these files is fixed in the binary, never a config key, and the class is
`ask`, never `deny`, because a fix to one of them is ordinary work.

A heredoc body is data. The guard matches the command words and every
redirect target, including a command after the terminator, and skips the text
between the delimiter and the terminator. This costs the guard the
interpreter case ADR 0011 relied on: a script an interpreter reads on stdin
is no longer matched. The guard is feedback, not a gate, and CI is
authoritative, so a route this open is the price of not refusing a note an
agent writes about the configuration.

## Consequences

The guard still reads no configuration. It may read `KLIN_STATE_DIR` and run
`git rev-parse --git-dir` to learn the state directory, and it must finish in
under 50 milliseconds, because it runs on every tool call.

A host with no `ask` decision gets `deny` for the ambiguous class, which is
ADR 0011's behaviour, and the host adapter records that per host.

ADR 0011's argument for a reader list over a writer list stands. A reader list
is short and closed. What changes is what happens outside it.

Amended 2026-09-09. The `deny` paragraph above ends with any non-reader that
names the state directory or `refs/worktree/klin`. That clause is withdrawn.
It overlapped the `ask` rule, which already covers every guarded path, and
it refused the plumbing an agent needs to read a stamp: `git rev-parse`,
`git cat-file`, `git for-each-ref`, `find` and `du`, none of which ADR 0011
lists. Those join the reader list, with `find` a reader only without
`-delete`, `-exec`, `-execdir` or `-ok`. The reason the clause existed is
answered elsewhere. A `turn` file that is gone is restored from the ref with
a red verdict. When the file and the ref are both gone, the prompt writes no
fresh stamp and the next stop judges the whole branch, so deleting a stamp
widens the window instead of closing it. The state directory and the ref
stay in the guarded set, so a shell command that would write to them is
`ask`. `docs/SPEC.md` 6.2 and 9.4 carry the rule.
