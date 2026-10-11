# The guard follows a static `cd` and a copy destination

> Amends ADR 0033. Its trust rule stands: the guard answers only where it can
> prove a write, and allows everything else. Recorded for #426, 2026-10-11.

ADR 0033 gave up four kinds of command that a static reading can prove:

- anything behind a `cd`;
- `cp` and `install`, because each reads its first argument;
- a heredoc, because `<<` was on the list of shell klin does not read;
- a shell's `-c` text and `eval`, which the guard never read.

Each of these can write `klin.json` in a command the guard reads in full, and
each fell into the branch that allows. ADR 0033 named the cost: the old
parsers answered questions that no longer applied. A reader that proves more
writes and asks no more questions about commands that only name a file does
not bring that cost back.

## The decision

**The guard reads a command with one bounded lexer, and follows the write
destination wherever the text proves it. It never runs a shell.**

1. A `cd` with one static operand moves the directory that later relative
   paths resolve from. The operand must name a directory that exists. The
   `cd` must surely run in this shell: `;`, `&` or a newline before it, or
   `&&` after a `cd` the guard placed, and no pipe, background job, subshell,
   group or function. And the next command must run after it, so `&&`, `;` or
   a newline follows it. A `||` after a `cd` leaves
   every later relative path unproven. Any other `cd`, `pushd`, `popd`,
   `source` or `.` leaves them unproven too. An absolute path stays proven.
2. `cp` and `install` write their destination and never their sources. The
   destination is the last operand. When it names a directory that exists,
   or `-t` names one, each source's name inside it is written. `install -d`
   writes every operand. An option the reader does not know, a letter GNU
   and BSD spell differently, or an abbreviated long option gives the
   destination up. Under `-n` or `-u`, which may leave an existing
   destination as it is, only a destination that does not exist yet is
   written. So `cp -n x klin.json` passes, and `cp -n x .git/klin/new` asks.
3. `sh`, `bash`, `zsh`, `dash` and `ksh` with `-c` run the word after the
   flag in a child shell. With no script operand, they run the heredoc they
   read in a child shell. `eval` runs its words, joined, in this shell. The
   guard reads that text the way it reads the command, so a `cd` in `eval`
   reaches the next command, and a `cd` in a child does not. An option that
   takes an argument, such as `-o`, leaves the text unread. Nested text that
   holds a `$` is unread too, because the outer shell may expand it into a
   comment or a separator before the nested text splits. An `eval` that runs
   a `cd` moves the directory under the placement rules of rule 1.
4. A heredoc body is data unless rule 3 runs it, so the rest of its command
   line is read. `cat <<'EOF' > klin.json` is a redirect onto the
   configuration.
5. A comment, which an unquoted `#` at the start of a word begins, is text.
   A quoted path keeps its spaces as one path.
6. A shell token holding a `$` proves nothing about the file it names, as a
   wildcard proves nothing. An edit tool's path is not shell text, so a `$`
   in it is a character of the path.

The levels do not change. A redirect is `deny`. A writer, now with `cp` and
`install`, is `ask`. The guarded set does not change.

## What stays unread

The lexer flags `$(`, `${`, a backtick, a backslash, a here-string `<<<`, a
process substitution and a heredoc left open. A command that holds any of them
proves no path, as before. A quote left open allows the whole command. A
`CDPATH` the agent's shell sets can move a relative `cd` elsewhere. The guard
cannot see that shell's environment, so this is a known ceiling.

## What was ruled out

- Refuse a command because it names `klin.json` somewhere. ADR 0033 rejected
  that, and nothing here reverses it.
- Run the command in a sandbox, or ask a shell to parse it. The guard runs on
  every tool call, before anything runs.
- Parse Bash in full. The lexer reads a closed set of forms, and every other
  form stays unproven.
