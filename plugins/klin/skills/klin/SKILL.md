---
name: klin
description: How to answer a klin quality gate — read the failure, fix the site it names, and change the configuration only when the person asks. Use this whenever a klin line appears in a hook, whenever a stop is blocked by a gate or by a build failure, whenever a person asks about klin's gates, findings, radius, base, window or state, and before any work touches klin.json, a hook file or CODEOWNERS. Read it before you try to make a klin failure go away, because most of the fast routes to green are routes klin refuses.
---

# klin

At a stop, klin compares the tree as this turn opened with the tree as it
stands, and fails only on debt the turn added. Existing debt is the floor.
So a finding the stop names is a finding this turn created, and what it
names is code to fix. `klin check` compares the branch with its base.

## How to read a failure

The stop hook prints a lead line, then each gate that did not pass, with the
derived values it used above its row and its own output below. A gate that
passed prints only when it left a note you should read. In the hook the exit code is the host's
protocol and not the verdict: exit 2 means "block this stop", and it does not
mean that klin failed to run. On Cursor a block arrives as the next message
instead, with exit 0.

Among gate results, only a FAIL blocks a stop. A build failure blocks too,
as described below. A HOLE, an ERR line and a NOTE are limits of that stop.
They ask for no change to the source and spend no block, but a FAIL that the
same gate still found does.

1. Read the site each finding names. A finding is one violation at one site,
   not a report about the whole file.
2. Fix the code at that site.

The next stop judges the fix. Do not run `klin check` to see whether it
holds.

`klin check` exits 2 when klin or any one check could not run, even beside a
FAIL. Otherwise it exits 1 when a finding failed, 3 when a measurement was
incomplete, and 0. Read the rows, not only the exit code. A REVIEW row never
changes the exit code. An INFO row names functions over a derived complexity
ceiling only. It asks for no change, so do not change code for it.

A gate failure blocks at most two stops per turn, and each block names its
number. After the first, a stop over a tree you did not change reports the
same failure and lets the turn end. A stop over a changed tree that still
fails blocks a second time, and after that no gate failure blocks again. A
turn that ends is not a turn that passed. The failure stands, and CI refuses
it.

In a repository with a remote, when the history moved under the turn, such
as incoming commits, a branch change, or a reset that lost the turn's
history, the stop is advisory. It says so, reports what the gates found
without blocking on it, and takes a fresh stamp, so the next stop is
ordinary. A tree that does not build still blocks. `klin check` still
judges the branch.

A build failure is separate. The first one blocks. After that, only a stop
over a changed tree blocks again, up to eight in one turn, and a stop over
an unchanged tree is reported and let through. No gate is measured until the tree builds, so fix the build
first. The report opens with where the command came from: a derived build
names its manifest, such as `tsc --noEmit from package.json beside
tsconfig.json`.

A build whose command the shell cannot find is not a build failure. klin
leaves a NOTE and judges the source as it stands. `klin check` runs no
build, so the project's own CI must run it. The fix is to install
the project's dependencies, not to remove the manifest or the script that
derived the command.

## The routes to green that klin refuses

Each route below makes a gate pass without fixing anything. Every one of them
is visible in the diff a person reviews.

- Editing `klin.json` on your own. Change it only when the person
  explicitly asks for that change: the request names `klin.json`, or the
  setting or entry to change. "Make the gates pass" or "fix CI however you
  need" is not such a request. When the person gave the exact change, make
  it and show the diff. When you choose any value yourself, such as which
  entry to accept, its reason or a ceiling, show the change and write it
  only after the person says yes. Never add an entry to `accepted` or raise
  a ceiling on your own to make a gate pass. An entry the person asks for
  follows "Tailoring klin.json" below. klin does not refuse the edit, and klin
  judges under the edited file, so this rule is yours to keep. Never delete
  or move `klin.json`: without it klin is off, and klin asks the person
  first.
- Deleting a failing test. The stop asks once why each deleted test went.
  If it failed because the code is wrong, restore it and fix the code. If
  the removal is intended, say why in your reply, and the stop after that
  lets it through. The question is a FAIL, so it spends one of the turn's
  two gate blocks. `klin check` shows the deletion as a REVIEW item for the
  person who reviews the change.
- Marking a test skipped, silencing a check, or swallowing an error. The
  escapes gate reads each of those as a new escape.
- Leaving a placeholder where the work belongs. The stubs gate names it.
- Moving text out of a document to get it under its ceiling, or deleting a
  citation instead of repairing the path it points at.
- Editing the host's hook file, such as `.claude/settings.json`, or
  CODEOWNERS. The guard allows an edit to both, because a person owns them by
  convention rather than by refusal. Leave them alone.
- Editing or deleting klin's own record of the turn, under `.git/klin` or
  its ref. The guard refuses an edit and puts a deletion to the person. No
  command moves the turn's stamp: fix what the gate names.
- Pulling, merging, rebasing or switching branches to make a stop advisory.
  The stop names the move, and `klin check` and CI still judge the branch.
- `klin setup` in any form. It writes a person's configuration and
  integration; a person runs it.
- `klin update`. It replaces the binary that judges this work; a person
  runs it.

## Tailoring klin.json

A request to tailor, tune or set up `klin.json` is a request to propose a
change. The rule for `klin.json` above still holds, and this section applies
it.

1. When the repository has no `klin.json`, say that creating it turns klin on
   for this repository. Create it only after the person says yes.
2. Run `klin policy --suggest`. It prints a starter policy computed from the
   tree, and writes nothing.
3. Explain each suggested value from the `derived:` and `held:` lines under
   the JSON. Say what pinning the value changes compared with the value klin
   derives without it: a pinned value stays fixed, and a derived value moves
   with the tree. A complexity value you write makes that ceiling block,
   where the derived `lines` ceiling only reports. A suggested `layering`
   section has no derived value. Its `held:` line names the findings that
   section holds today.
4. Write a suggestion the person picks unchanged, because that is their exact
   change. A value you chose or edited is yours, and the rule above applies.
   `--suggest` prints only the values the file does not state yet, so merge
   the picked values into the existing `klin.json`. Keep every other key,
   nested value and `accepted` entry as it is.
5. When the person asks to accept a finding, build the `accepted` entry with
   its reason, in the shape `klin policy --schema` gives. The entry also
   needs each number the finding gives for a value its gate ratchets, such
   as `cc` and `lines` for `complexity`. Copy them from the finding, and add
   no value the finding does not carry. The entry, its numbers and its reason
   are values you chose.
6. After the write, show `git diff klin.json`, or the whole file when it is
   new, and `klin policy`, which marks each value pinned, derived or
   built-in. Tell the person to commit the file.

When a write needs a value that the person did not see, do not add it
yourself. Show the new value and wait for a yes again.

`klin check` judges under the edited file, so it cannot show that the edit
is sound.

## Proposing conventions from instruction files

When the person asks, read the rules in `AGENTS.md` or `CLAUDE.md` and
propose a `conventions` rule in `klin.json` for each instruction that one
convention can enforce in full. A convention only forbids: literal `text`, a
`code` pattern in a language klin parses, or a `files` glob, in the paths it
names. An instruction that needs judgment, or that requires something to be
present, such as documentation on every public method, is not a convention.
Leave it in the file and say why.

1. Read the rule shape from `klin policy --schema`, or from
   `klin policy --reference`, which says what each key does.
2. For each instruction, show the convention's name, its one `text`, `code`
   or `files` value, any `in` and `except`, and its `remedy`. A `code` rule
   reads one language: its `language`, or the one language of the source
   files in its scope. When the instruction covers more than one language,
   propose one rule per language. klin derives no convention, so every value
   is yours, and the rule for `klin.json` above applies: write it only after
   the person says yes. When the repository has no `klin.json`, step 1 of
   "Tailoring klin.json" applies first.
3. After the write, follow step 6 of "Tailoring klin.json", and show
   `klin policy conventions NAME`, which says what the convention forbids
   and where. It runs no check. A passing `klin check` does not show that
   the rule catches a breach either, because klin judges what is new against
   the base. When it is practical, show the rule catching a sample breach in
   a throwaway repository.

## Installation ownership

The native Claude Code, Codex CLI and Cursor plugins already carry this skill.
A repository may commit the standalone copy beside them, and where both copies
of the hooks run, one of them yields on each event.

The standalone command is klin setup. It opts a project into klin by writing
klin.json at the repository root when none exists, selects hosts from
evidence or from --host, and reconciles their hooks and this skill. It is a
person-owned command.

Project-scope skill paths are:

- Claude Code: .claude/skills/klin/SKILL.md
- Codex and Cursor: .agents/skills/klin/SKILL.md

Codex and Cursor intentionally share one project file. User-scope paths under
klin setup --user are:

- Claude Code: ~/.claude/skills/klin/SKILL.md
- Codex and Cursor: ~/.agents/skills/klin/SKILL.md

User scope is local to one person's machine. It is not committed, does not
travel with the repository, and does not reach a cloud or remote agent. The
flag is --user, never --global.

When klin setup finds no skill file, it writes the canonical skill. When
the file is byte-identical, it does nothing. When it holds an earlier klin
skill, it replaces it. When it differs, it asks the person on a terminal
before it replaces it, and without a terminal it replaces it and prints the
path. A skill that a link sends outside its own path is written only on a
yes at a terminal. After a binary update, rerun klin setup when klin update
names an integration that is not current. The standalone
route copies this skill only; plugin slash commands remain plugin-owned.

klin setup --pin also writes derived ceilings into klin.json as policy a
person reviews. A pinned complexity ceiling blocks.

## Asking klin about its state

Ask the binary. Do not read klin's state directory, and do not compute a
verdict of your own, because a verdict you compute is not the verdict CI
produces.

- `klin check --changed` runs the gates over the files this branch changed
  against its base, the window CI judges. It can name debt an earlier turn
  added. Run it only when the person asks for it. `klin check NAME...` runs
  only the named gates, and `--json` prints one JSON object.
- `klin status` reads the configuration, each host integration and the
  turn's window, with why it is red, and runs no check. Its window verdict
  is the last stop's, not the tree as it stands.
- `klin policy` prints every gate, which values a person pinned and which
  values klin derived, and where the state directory is.
- `klin report` shows what klin caught in the newest session, and
  `--since 7d` reports the last seven days.

## When klin is missing

The plugin's `bin/klin` fetches the pinned release on first use. Where that
directory is unavailable, klin comes from PATH. Tell the person to run:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/brajevicm/klin/releases/latest/download/klin-installer.sh | sh
```

A person updates a klin from the installer with `klin update`. A plugin
install updates with its plugin.
