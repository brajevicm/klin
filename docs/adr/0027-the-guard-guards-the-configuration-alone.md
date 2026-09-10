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

Every route klin can read as a clear write to it still denies: an edit tool
whose path is the file, a redirect onto it, a glob that matches the name,
`klin init`, and `klin turn reset`. A command outside the reader list that
names it is still `ask`, per ADR 0020, so the three decisions stand.

The whole-tree restore leaves the deny list with them. ADR 0011 put it there
because the command never names the file it would overwrite, and the rule it
grew into matched the words `git`, `checkout` or `restore`, and any word
ending in a slash, anywhere in one command. That denied
`git checkout main -- src/`, which cannot reach a file at the root, and it
denied a command that only mentioned the three words in unrelated places. A
deny leaves an agent no remedy, so each of those cost a turn. The route it
closed also destroys every other change in the working tree, which a person
notices at once.

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

A second hole came with the first. The segmenter cut at the `$(` that opens
a substitution and never saw the `)` that closes it, so a reader inside one
lent its exemption to every word after the parenthesis:
`gh issue create --body "$(cat notes.md)" klin.json` was allowed, because
`cat` headed the piece the `$(` opened.

The fix is the same idea as the heredoc one. A substitution is a command of
its own, so it leaves the line it sat in. The guard reads the command inside
it separately, and the words after the closing parenthesis stay with the
command that owns them. `grep -rn "$(cat pattern.txt)" klin.json` is still
allowed, because `grep` owns those words and `grep` is a reader.

An unbalanced parenthesis inside a substitution ends it early. That splits
the command into more pieces than a shell would, so the guard can only ask
about more than it should, never about less.
