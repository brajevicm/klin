---
name: klin
description: How to answer a klin quality gate — read the failure, fix the site it names, and leave the configuration alone. Use this whenever a klin line appears in a hook, whenever a stop is blocked by a gate or by a build failure, whenever a person asks about klin's gates, findings, radius, base or state, and before any work touches klin.json, a hook file or CODEOWNERS. Read it before you try to make a klin failure go away, because most of the fast routes to green are routes klin refuses.
---

# klin

klin compares the tree as this turn opened with the tree as it stands, and
fails only on debt the turn added. Existing debt is the floor. So a finding
klin names is a finding this turn created, and what it names is code to fix.

## How to read a failure

The stop hook prints a lead line, then each failing gate's own output, then
the derived values the run used. In the hook the exit code is the host's
protocol and not the verdict: exit 2 means "block this stop", and it does not
mean that klin failed to run.

1. Read the site each finding names. A finding is one violation at one site,
   not a report about the whole file.
2. Fix the code at that site.
3. Run `klin gate --changed` to see whether the fix holds.

A gate failure blocks one stop per turn. The second stop reports the same
failure and lets the turn end, so a turn that ends is not a turn that passed.
The failure stands, and CI refuses it.

A build failure is separate, and it blocks up to eight stops in one turn. No
gate is measured until the tree builds, so fix the build first.

## The routes to green that klin refuses

Each route below makes a gate pass without fixing anything. Every one of them
is visible in the diff a person reviews.

- Editing `klin.json`. An edit tool that names it is denied, and a shell
  command that names it is put to the person. Never add an entry to
  `accepted`, and never raise a ceiling.
- Deleting or renaming a test. The inventory gate reads that as a declaration
  that disappeared.
- Marking a test skipped, silencing a check, or swallowing an error. The
  escapes gate reads each of those as a new escape.
- Leaving a placeholder where the work belongs. The stubs gate names it.
- Moving text out of a document to get it under its ceiling, or deleting a
  citation instead of repairing the path it points at.
- Editing the host's hook file, such as `.claude/settings.json`, or
  CODEOWNERS. The guard allows an edit to both, because a person owns them by
  convention rather than by refusal. Leave them alone.
- `klin init` in any form, and `klin turn reset`. The guard denies both,
  because both are a person's command.

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
