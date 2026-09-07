# A guarded name is refused unless it is a reader's argument

The guard had two rules pulling against each other. `command_writes_guarded`
matched the guarded literals anywhere in a command string, so a `grep` for
`klin.json` was refused, and a `gh issue create` whose body happened to
mention the config was refused with it. The same function also held an
allowlist of writers — `tee`, `cp`, `mv`, `rm`, `truncate`, `install`, `sed
-i`, `git checkout`/`restore` — so `perl -i`, `python3 -c` or a heredoc, `ed`,
`awk` with a redirect, `patch`, `git apply` and `git stash pop` walked past
it untouched. #33 and #34 each fixed one problem and reopened the other.

## The rule

A guarded name may appear as an argument to a listed reader, and nowhere
else: `cat`, `head`, `tail`, `less`, `grep`, `rg`, `diff`, `wc`, `stat`,
`ls`, `file`, `jq`, and `git` with `diff`, `show`, `log`, `status`, `blame`,
`add` or `commit`. Every other command that names a guarded path is
refused, including `echo`. A redirect onto a guarded path is refused
whatever the command, even a reader's own.

No allowlist of writers is needed, because the rule no longer asks what a
command writes. It asks what a command is. An interpreter — `perl`,
`python3`, `ed`, `awk`, `sed` — is never on the reader list, so a guarded
name in its inline script or its heredoc body is refused the same as a
guarded name on its command line, with no per-interpreter case to keep
current. `git add` and `git commit` are readers because neither changes a
guarded file's content; what an agent stages is only ever what a person
wrote.

The guarded set grows by one: `.github/CODEOWNERS`, per #35. A person who
cannot make a gate pass must not be able to remove the reviewer who would
catch it.

## Why a reader list instead of a smarter writer list

Every writer list is a race the guard loses by one interpreter. A reader
list is short and closed: the guard names what changes nothing, and refuses
the rest by default. Adding a tool later only ever removes a false refusal,
never opens a route a writer list would have missed silently.

The cost is a real one. A command that is neither a reader nor a git
subcommand on the list — a `gh issue create` whose text happens to include
`klin.json`, say — is refused even though it changes nothing. That is the
same shape of over-refusal #33 fixed for grep, narrowed to commands outside
the reader list rather than gone. The guard is feedback, not a gate a person
cannot work around by rewording a sentence or quoting the file's name
differently; ADR 0009 already says only CI is authoritative.

## Consequences

This ticket replaces #33 and #34. `WRITERS` and the `sed -i` and `git
restore`/`checkout` special cases it carried are gone; `restores_a_tree`
and `fills_in_the_configuration` stay, because neither is about what a
command writes to a named file — one is a whole-tree restore that never
names the files it would overwrite, the other is `klin init --add` filling
in the config by a name the guard would otherwise never see as guarded.

The tokenizer that looks for a guarded name splits on the punctuation a
shell, a heredoc body, or an interpreter's inline script wraps a filename
in, so `open('klin.json')` names the file as plainly as a bare word does.

Three details follow from the rule rather than sitting beside it. A command
substitution starts a new command, so `$(` and a backtick end a segment the
same way `;` does, and the interpreter inside one is read as the command it
is instead of borrowing the reader that wraps it. A glob is guarded when a
guarded name matches it, because `klin.*` reaches the file a bare name
would. A git global flag that takes a value — `-C`, `-c`, `--git-dir`,
`--work-tree`, `--exec-path` — is skipped with its value when the
subcommand is picked, so `git -C sub add` reads as `add`.
