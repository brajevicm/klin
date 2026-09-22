---
name: klin
description: How to answer a klin quality gate — read the failure, fix the site it names, and leave the configuration alone. Use this whenever a klin line appears in a hook, whenever a stop is blocked by a gate or by a build failure, whenever a person asks about klin's gates, findings, radius, base or state, and before any work touches klin.json, a hook file or CODEOWNERS. Read it before you try to make a klin failure go away, because most of the fast routes to green are routes klin refuses.
---

# klin

klin compares the tree as this turn opened with the tree as it stands, and
fails only on debt the turn added. Existing debt is the floor. So a finding
klin names is a finding this turn created, and what it names is code to fix.

## How to read a failure

The stop hook prints a lead line, then each gate that did not pass, with the
derived values it used above its row and its own output below. A gate that
passed prints only when it left a note you should read. In the hook the exit code is the host's
protocol and not the verdict: exit 2 means "block this stop", and it does not
mean that klin failed to run.

1. Read the site each finding names. A finding is one violation at one site,
   not a report about the whole file.
2. Fix the code at that site.
3. Run `klin gate --changed` to see whether the fix holds.

A gate failure blocks at most two stops per turn, and each block names its
number. After the first, a stop over a tree you did not change reports the
same failure and lets the turn end. A stop over a changed tree that still
fails blocks a second time, and after that no gate failure blocks again. A
turn that ends is not a turn that passed. The failure stands, and CI refuses
it.

A build failure is separate. It blocks a stop only after you changed the
tree, up to eight in one turn, and a stop over an unchanged tree is reported
and let through. No gate is measured until the tree builds, so fix the build
first. The report opens with where the command came from: a derived build
names its manifest, such as `tsc --noEmit from package.json beside
tsconfig.json`.

A build whose command the shell cannot find is not a build failure. klin
leaves a NOTE, judges the source as it stands, and CI runs the build. The
fix is to install the project's dependencies, not to remove the manifest or
the script that derived the command.

## The routes to green that klin refuses

Each route below makes a gate pass without fixing anything. Every one of them
is visible in the diff a person reviews.

- Editing `klin.json`. An edit tool that names it is denied, and a shell
  command that names it is put to the person. Never add an entry to
  `accepted`, and never raise a ceiling.
- Deleting a failing test. The inventory gate asks once why each deleted test
  went. If it failed because the code is wrong, restore it and fix the code.
  If the removal is intended, say why in your reply, and the stop after that
  lets it through.
- Marking a test skipped, silencing a check, or swallowing an error. The
  escapes gate reads each of those as a new escape.
- Leaving a placeholder where the work belongs. The stubs gate names it.
- Moving text out of a document to get it under its ceiling, or deleting a
  citation instead of repairing the path it points at.
- Editing the host's hook file, such as `.claude/settings.json`, or
  CODEOWNERS. The guard allows an edit to both, because a person owns them by
  convention rather than by refusal. Leave them alone.
- `klin init` in any form, and `klin install` in any form. Both write a
  person's configuration or integration; a person runs them.
- `klin turn reset`. A person owns the decision to start a new turn.

## Installation ownership

The native Claude Code, Codex CLI and Cursor plugins already carry this skill.
When a native plugin owns the selected host and scope, do not install a second
copy with the standalone route.

The standalone command is klin install. It opts a project into klin by writing
klin.json at the repository root, selects hosts from evidence or from
--host, and reconciles their hooks and this skill. It is a person-owned
command.

Project-scope skill paths are:

- Claude Code: .claude/skills/klin/SKILL.md
- Codex and Cursor: .agents/skills/klin/SKILL.md

Codex and Cursor intentionally share one project file. User-scope paths under
klin install --user are:

- Claude Code: ~/.claude/skills/klin/SKILL.md
- Codex and Cursor: ~/.agents/skills/klin/SKILL.md

User scope is local to one person's machine. It is not committed, does not
travel with the repository, and does not reach a cloud or remote agent. The
flag is --user, never --global.

When klin install finds no skill file, it writes the canonical skill. When
the file is byte-identical, it does nothing. When it differs, it reports an
explicit conflict and never overwrites the person's file. After a binary
update, rerun klin install to reconcile files klin can safely own; resolve a
different file as a person rather than losing its contents. The standalone
route copies this skill only; plugin slash commands remain plugin-owned.

klin init is separate. It surveys the tree, and klin init --pin writes
derived ceilings as policy a person reviews. It does not install host hooks or
this skill.

## Asking klin about its state

Ask the binary. Do not read klin's state directory, and do not compute a
verdict of your own, because a verdict you compute is not the verdict CI
produces.

- `klin gate --changed` runs the gates over the files this turn changed.
- `klin gate --list` prints every gate, which values a person pinned and
  which values klin derived, and where the state directory is.
- `klin radius --report` prints how far this turn has spread.

## When klin is missing

The plugin's `bin/klin` fetches the pinned release on first use. Where that
directory is unavailable, klin comes from PATH. Tell the person to run:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/brajevicm/klin/releases/latest/download/klin-installer.sh | sh
```
