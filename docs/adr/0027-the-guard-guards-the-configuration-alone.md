# The guard guards the configuration alone

> Amends ADR 0011 and ADR 0020. The reader rule and the three decisions stay.
> The guarded set shrinks to one file.

The guarded set had grown to five things: the configuration, each host's hook
file, the code owners, klin's state directory, and `refs/worktree/klin`. A
sixth class, the verification files, was `ask` on a table fixed in the binary.
Each addition answered a real hole, and together they refused work that has
nothing to do with klin.

A host's hook file is the clearest case. `.claude/settings.json` holds
permissions, MCP servers, the model, the status line and the plugin list
beside the hooks. A person who asks an agent to add an MCP server is asking
for an edit to that file, and the guard sees only the tool name, the file
path and the command. It cannot tell that edit from one that removes klin's
`PreToolUse` line, so it refused both.

The same reasoning reaches the rest of the set. klin cannot tell a loosening
of an eslint rule from a fix to one, or a reviewer added to CODEOWNERS from a
reviewer removed. Asking about every one of them spends a person's attention
at a rate that teaches the person to say yes without reading.

## The decision

The guarded set is `klin.json`. Nothing else.

Every route that reaches it still answers: an edit tool whose path is the
file, a redirect onto it, a whole-tree restore, a glob that matches the name,
`klin init`, and `klin turn reset`. A command outside the reader list that
names it is still `ask`, per ADR 0020, so the three decisions stand.

What leaves the set: each host's hook file, CODEOWNERS, klin's state
directory, and `refs/worktree/klin`. The verification table leaves with them.
An edit to any of these is now allowed with no question.

## What this gives up

An agent can remove klin's hook lines, and then no gate runs. An agent can
loosen a lint rule, lower a coverage threshold, or delete the reviewer who
would catch either. The guard no longer stands in the way of any of that.

This is a smaller loss than it reads as. ADR 0009 already says CI is
authoritative and the guard is feedback. A gate that only runs because a hook
survived was never a gate; it was a reminder. The reminder that a person
reads is worth more than the four that a person clicks through.

## The heredoc a command substitution opens

Section 9.4 says the body of a heredoc is data, and the guard found a heredoc
only where the `<<` sat outside every quote. A body passed as
`--body "$(cat <<'EOF' ... EOF)"` opens inside a double quote, so the guard
read none of the following lines as data. It read every line as a command,
and a paragraph that mentioned the configuration was refused. This refused
the `gh issue create` that published the ticket for this record.

A command substitution starts a command of its own, and a quoting context of
its own with it. The guard now resets its quote state at a `$(` and at a
backtick, so a `<<` inside one opens a body the way a bare one does.

One hole beside it stays open, and this record names it rather than fixing
it. The segmenter does not see where a command substitution ends, so a reader
inside one exempts the words that follow the closing parenthesis:
`gh issue create --body "$(cat <<'EOF' ... EOF)" klin.json` is allowed,
because `cat` heads the segment the `$(` opened. A guarded name in the words
before the substitution is still `ask`. The hole predates this record and is
the segmenter's shape, not the heredoc's; #90 owns the segmenter.
