# The guard asks when it cannot tell, and guards the stamps

> Amends ADR 0011. The reader rule stays. What changes is the decision for a
> command outside the reader list, and the guarded set.

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
and any non-reader command that names the state directory or `refs/klin`. The
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
directory of ADR 0019, and `refs/klin`. The state directory is guarded for a
reason of its own, recorded in ADR 0017: deleting the stamp turns every open
failure into held in one command.

## Consequences

The guard still reads no configuration. It may read `KLIN_STATE_DIR` and run
`git rev-parse --git-dir` to learn the state directory, and it must finish in
under 50 milliseconds, because it runs on every tool call.

A host with no `ask` decision gets `deny` for the ambiguous class, which is
ADR 0011's behaviour, and the host adapter records that per host.

ADR 0011's argument for a reader list over a writer list stands. A reader list
is short and closed. What changes is what happens outside it.
