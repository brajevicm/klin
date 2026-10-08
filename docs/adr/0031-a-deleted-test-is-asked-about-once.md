# A deleted test is asked about once, and CI only notes it

> Amends ADR 0009 for one finding. A deleted test no longer fails CI.

Commit 3cd8058 made `inventory` fail on a deleted test function in every
window. The only remedy an agent could not take was an accepted entry, and
only a person writes one, in a reviewed commit. So every legitimate removal
of a test cost a person a config edit, and a second commit after the merge to
delete the entry once it stopped matching. #136 deleted ten tests of
behaviour ADR 0027 removed, and the gate asked for ten entries.

Removing a test is ordinary work. Merges into a table-driven test, renames
that edit the body, and removed behaviour all do it. Deleting a failing test
to reach green is one reason among several, and klin cannot tell which
reason it was. What costs quality is code that still exists and lost its
test, and that is a result `changed-coverage` (#54) judges without asking
why.

## The decision

In the hook, a deleted test file or test function blocks the first stop that
finds it, once, with a question: restore the test and fix the code if the
test failed, or say why the removal is intended and stop again. The stop
that blocks records the findings it put in front of the agent beside the
turn stamp. The next stop lets those deletions through as a NOTE, the
verdict is green, and the stop tells the person which tests went, through
the host's `systemMessage`. A test deleted after the question gets its own
question at the next stop that blocks. The agent's reply is not read.

Everywhere else a deleted test is a NOTE, and `--strict` does not escalate
it. The deletions are in the diff and in the NOTE, where a reviewer reads
them.

Three details carry the rest:

- The rule keys on `--hook`, not on the window kind. A stop whose stamp is
  gone judges a branch window, so a kind-keyed rule would ask nothing once
  the stamp went. Keying on `--hook` fails the safe way: a stamp that is
  gone takes the record of what was asked with it, so the stop asks again
  rather than asking less.
- The record of what was asked lives in the `turn` file and goes when the
  stamp moves. The build stamp reads as zero under a new prompt, and Cursor
  delivers a stop's follow-up as a new user message, so a record there would
  ask the same question again. The guard refuses an agent's write to that
  file, so forging the record is not the one-line bypass it would otherwise
  be (ADR 0032).
- Where a deletion is let through, its base entry carries `missing: 1`. The
  one judge holds the site, and an accepted entry that already names the
  test still takes the match, so no existing entry turns into one that
  matched nothing.

## What this gives up

CI no longer fails on a deleted test. ADR 0009 says local runs are feedback
and CI bounds them, and for this one finding that stops being true, even at
the enforced level. An agent that runs with no hook, such as a cloud agent
that opens a pull request, meets no question at all.

The agent's answer is not checked. An agent that deleted a failing test on
purpose can say the removal was intended. The question catches the
reflexive shortcut and not a determined one.

The record of what was asked is klin's own, on the same disk as the tree.
ADR 0032 puts the guard in front of it, which is the same worth the guard
has for `klin.json`: it refuses the routes klin can read as a write, and an
agent with no hook meets none of it.

A deletion does not stay red. Every other gate failure stays red across
turns until a person fixes, accepts or resets it. A deleted test is green
after one question.

A turn that deletes tests costs the agent one extra round.

## What was ruled out

- A person approves each deletion through a confirm command the guard puts
  to them. It adds a command, a guard rule, a fallback for Codex, which has
  no `ask`, and a click on ordinary work, and a script file still skips it.
- Tell the person and never block. The agent never gets the moment to
  reconsider, which is the one thing the hook adds.
- Read the agent's reply from the transcript, or look for evidence in the
  diff. The first is the agent's word in a format each host spells
  differently. The second is faked by touching a second file.
- A commit-message trailer, or a command that writes the accepted entry. The
  first is a one-line bypass, because an agent writes commit messages. The
  second still edits the config for every deletion.

## Amendment: the question asks why, and never for a repair (#502)

The question above offered two answers: restore the test and fix the code, or
say why. vNext asks only why (spec 9.2). The text never tells the agent to
restore the test, because klin cannot tell why it went and a repair
instruction pressures the agent to clear the question.

The ask shares the gate block of a FAIL in the same Stop, so a Stop with a
FAIL and an unasked deletion spends one block for both. When no gate block
remains under the prompt, the Stop reports the deletion, records no `asked`
entry, and leaves the stamp `red`. The next prompt keeps the stamp, so the
question comes under that prompt. After the question, the deletion is a
review item, which the Stop shows as a note. It is no reason for a gate block.
`klin status` names a deletion klin has not asked about yet under `unasked`.
