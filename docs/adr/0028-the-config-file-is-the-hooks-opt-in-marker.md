# The config file is the hook's opt-in marker

> Amends ADR 0016 for the `--hook` path alone. Everywhere else the file stays
> optional and a missing value is still derived.

ADR 0016 made `klin.json` optional so that a person manages nothing: a tree
with no file is gated over the sections the survey supplies. That reading is
right for a person who runs `klin gate` in the tree they mean to measure, and
it is right for CI, which runs in one repository that chose klin.

It is wrong for a hook installed once for every repository a person opens. A
global Stop hook runs `klin gate --hook` in trees that never heard of klin.
There the survey either finds nothing to gate, which is exit 2 and a FAIL
line, or it derives gates a person never asked for and blocks a stop on them.
Both are noise in someone else's repository, and neither names anything the
agent can fix by editing code.

## The decision

Under `--hook`, `klin.json` is the marker that the repository opted in. When
no file resolves, the run exits 0 before it surveys anything, prints nothing
and writes no state.

`--config PATH` follows the same rule: a path that names no file is a tree
that did not opt in.

`klin gate` without `--hook` keeps the message it prints today. A person who
runs it in the wrong tree is asking a question, and the answer is the FAIL
line naming the file that does not exist and the sections that would fill it.

## What this gives up

A repository that would have been gated by derivation alone is now silent in
the hook until someone writes a `klin.json`. `klin init` writes one, so the
cost is one command, paid once per repository, by a person who wanted klin
there.

The silence is also unconditional. A file a person deleted by accident reads
the same as a repository that never opted in, and the hook says nothing
either way. CI still refuses the work, per ADR 0009, so the hole closes at
the place that is authoritative.
