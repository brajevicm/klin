# klin Specification

Status: pre-1.0, updated 2026-09-28. This document describes klin as it
ships from `main`. Until the stability contract of #344, a release may still
change any surface. A section that describes behavior klin does not ship says
so.

This specification, accepted ADRs and klin's CLI tests define klin's behaviour.
ADR 0025 retires the external behavioural reference obligation. Gaps and
conflicts are resolved against klin's requirements and recorded here and in
CLI tests.

Purpose: Define a tool that gates an agentic coding harness deterministically,
with at most one committed configuration file, no baseline that a person
maintains, green on the day it arrives, and tighter under a schedule a
person pins once.

The structure follows the Symphony service specification. This draft is not
bound by the ADRs under `docs/adr/`. Where it reverses one, it says so and
gives the reason, so each reversal can be accepted or refused on its own.
Section 0 lists them.

## Normative Language

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, RECOMMENDED, MAY
and OPTIONAL are to be read as RFC 2119 describes them.

Implementation-defined means the behavior is part of the contract, but this
document does not prescribe one policy. An implementation MUST document the
policy it chose.

The vocabulary of `CONTEXT.md` applies. This document adds two terms, Window
and Derived, and asks `CONTEXT.md` to take them.

## 0. Decisions this draft reverses

| ADR | What it decided | What this draft does instead | Why |
|---|---|---|---|
| 0005 | A missing key is an error, never a default. `init` writes every section. | A missing key is derived from the base tree at run time and printed. `init` only pins a derived value into the file. | The goal is a tool a person manages nothing for. A derived value that the run prints is not the silent default ADR 0005 feared. Section 5. |
| 0009, 0013 | The hook compares against the merge-base with the default branch. | The hook compares against the turn stamp. `klin gate` and CI keep the merge-base. | The merge-base is the wrong window for a turn. It is empty after a commit on the default branch, it grows with the branch, and ADR 0013 exists only to patch that. Section 6. |
| 0010 | Under `--strict` an unaccounted gate is exit 2. | Every applicable gate runs unless its section is `false`. The failure has nothing left to catch. | A gate whose section is derived cannot be unaccounted. Section 10. |
| 0007 | Config keys preserve compatibility for differential tests. | One key vocabulary across sections. The differential test goes. | ADR 0009 already weakened the reason to a preference. `sources` in one section and `roots` in the next is a cost a user pays for a test klin no longer needs. |
| 0003 | A file no grammar reads is exit 2 everywhere. | In the hook it is a NOTE. Outside the hook it stays exit 2, unless the base held it and no grammar read it there either (8.6). | An agent cannot fix a grammar. Blocking a stop on it costs a turn per stop with no remedy. |
| 0011 | A non-reader naming a guarded path is refused. | It is `ask`. Only a clear write is `deny`. | Four read-only commands were refused in the session that wrote this draft. Section 9.4. ADR 0027 keeps the `ask` and shrinks the guarded set to `klin.json`. |
| 0004 | The host's cap on consecutive blocks bounds the loop. | klin bounds its own blocks. | The cap is not in the host's current documentation. Section 9.3. |
| 0015 | klin's state lives in `.klin/` at the tree root, with one `.gitignore` line. | State lives in the git directory, or under `KLIN_STATE_DIR`. No ignore line. | Per-worktree state has a native home that git never tracks and never lists. Section 7.4. |
| 0002 | The plugin ships no binary. | The plugin ships a `bin/klin` wrapper that fetches the pinned release on first run. | The first reason, a pin beside committed baselines, went with ADR 0009. The second, `bin/` unavailable for organization-distributed plugins, is handled by falling back to PATH. Section 19. |
| 0014, one line | The turn stamp is not guarded, because a report leaves nothing to gain. | The stamp is in the guarded set, and a missing stamp widens the window instead of closing it. | The stamp now holds the verdict that keeps a window open. A stamp that is gone is restored from its ref, or the stop judges the whole branch, so deleting it buys nothing. Sections 6.2 and 9.4. The rest of ADR 0014 stands. ADR 0027 reverses this row again: the stamp left the guarded set with everything but `klin.json`, and 9.4 carries the current rule. |

ADR 0001, 0008 and 0012 stand as written. ADR 0006 stands for its choice of
`ast-grep-core`, and ADR 0037 replaces the rule shape it named.

Every row above was accepted on 2026-09-08 and is recorded as an ADR: 0016
for the optional config and derivation, 0017 for the turn stamp as the hook's
base, 0018 for one key vocabulary, 0019 for the state directory, 0020 for the
guard's `ask` decision and the guarded stamps, 0021 for the unreadable file in
the hook, 0022 for klin bounding its own blocks, and 0023 for the plugin
wrapper. Each superseded ADR carries a note pointing forward.

This draft was reviewed on 2026-09-08 (`docs/spec-review-2026-09-08.md`).
Every finding of that review is applied here. The review's fix table maps to
sections 5.1, 5.4, 6.2, 6.5, 6.6, 7.3, 8.3, 9.4, 10 and 14.

A second review on 2026-09-09 found six contract gaps. Four are applied: the
derived ceiling is not monotone (5.4, 6.6, 7.3), an acceptance no longer moves
the stamp (6.2, 16.1), roots are discovered in both trees (3, 5.4), and the
dated ceiling uses one zone (5.5). Two were refused: a `complete` flag that
holds the window open on an unreadable file, because such a file yields no
sites in `before` and its later readable form is judged `new`, so the debt is
not hidden, and report provenance for SARIF, because a working tree has no
revision id. The `run` key in 8.3 answers the freshness problem instead. The
review also added the coverage line (8.6, 11), `inventory` over test
declarations (8.2), and three escapes patterns.

A third review on 2026-09-09, of the draft above, found six more. Five are
applied: the day-one promise is narrowed to the same tree (5.1), stops in one
worktree take a lock (6.5), the stamp ref moved under `refs/worktree/` (6.5),
`inventory` has its own judge (16.4), and ADR 0016 and 0017 are amended to
match 5.4, 6.2 and 6.6. The sixth, a content fingerprint of the working tree
as report provenance, was replaced by deleting the report before `run` (8.3).
The review also added the coverage regression rule (8.6, 10), verification
files to the guard's `ask` list, which ADR 0027 later removed again (9.4),
the empty test body to `stubs` (8.2),
a finding id (11.2), and the paired scenarios of 17. `TODO` and `FIXME` stay
in the `stubs` table because #106 had decided it.

A fourth review on 2026-09-09 read the draft for contradictions and found
five that would have shaped code, a data type or a test. Each resolution was
judged against the goals before it went in. Derived values split into
numbers from the derivation commit and path sets unioned with `after`, and a
site under an undiscovered path is `new` (3, 4.3, 5.4, 7.1, 12). `inventory`
ratchets existence under the one judge, which reverses the own judge of the
third review (4.4, 4.5, 8.2, 16.4). A deleted stamp is restored from its ref
or widens the window to the branch, and the state-directory `deny` goes (6.2,
7.4, 9.4, 14, 16.1). The failure model is keyed on `--hook` and `--strict`,
never on CI, and a config error in the hook never blocks (8.6, 10, 14). The
build stamp counts blocks under a prompt counter in `turn`, and a red verdict
is written before a build block (6.2, 9.2, 9.3, 16.1, 16.3). ADR 0016, 0017,
0020 and 0022 carry the amendments. A re-read after the edits found thirteen
smaller conflicts, most between the pseudocode of 16 and the prose it
illustrates, and they are fixed in place: the stamp rule names the first
session (6.2), a check with no ceiling judges every entry (16.4), `inventory`
receives `before` (16.4), the prompt counter is not written into a deleted
stamp (16.1), the edit tool on the state directory is `ask` (9.4, 15.1), the
build stamp is one record per prompt (16.3), and an unwritable state
directory reports a build failure instead of blocking (14). The same pass
fixed the escapes row count

(8.2), the `sarif` exception in 8.2, the `OK:` line with notes (8.6), the
finding id under a rename (11.2), the report path under `.gitignore` (8.3),
the report age check as the second clock (12), and the per-worktree survey
cache (6.6).

A fifth pass on 2026-09-09 tidied every open ticket against this draft and
settled the readings the tickets had made where the draft was silent. Seven
went into the text: the `Test` and `Tests` affixes (8.2), the empty document
ceiling (5.4), what `init --force` keeps (5.7), `--sarif PATH` (11.3), the
heredoc body as data (9.4), the tools out of scope for `compare` (8.3), and
the name-based resolution rule with its first languages (8.4). One check
left: `test-hygiene` is an escapes `patterns` row over the test roots, and
its schedule was the forced paydown 7.3 refuses (8.4).

An independence audit on 2026-09-10 (#127) found that 7.1 and the CLI tests
disagreed on how identical sites match. The tests describe the behaviour
klin needs, so the rule they pin is now written down (4.4, 16.5). Three
behaviours changed with it. A finding prefers an entry it did not rise
against, so a stale accepted entry never fails a site `before` holds (4.4,
7.3). An accepted entry wins a full tie against a `before` entry at the same
site, so a merge does not turn it into a strict failure by chance of line
position (4.8). An accepted entry must give a number for every value the
gate ratchets, so no value grows unjudged behind it (4.8). ADR 0008 and 0009
carry notes.

`klin stats` became an attention and value report on 2026-09-16 (#172), and
that reverses five earlier decisions. Stories before tallies goes: the default
report is the attention and value summary, and the stories moved to `--all`.
"Nothing in the reader names a check" goes: the human label for a gate is
presentation metadata on the catalogue row, so the reader keeps no vocabulary
of its own. The person's counted unit is the Regression, one finding site per
identity in a window, in place of the gate-level intervention, which stays the
hook's unit. Finding ids no longer key nothing: a current id is the primary
stats identity, with a conservative fallback for a record that carries none and
the rename limitation stated. And `regression` left the avoid list, where
`shortcut` now sits. The journal's records are unchanged: only their
interpretation moved (9.5, 11.5, CONTEXT.md, ADR 0034).

The compact configuration of #180 on 2026-09-14 finished what ADR 0038 began
(ADR 0040). `klin.json` holds a person's decisions and `{}` is complete. Every
check resolves its own policy from the facts of 4.3, and no component
manufactures a section. `doc_size` pins documents in a map, `doc_citations`
reads no policy, `inventory` and `lockfile` read `in` and `except`, `build`
is an override, and `project` and `version` are retired (5.2, 5.3). `init`
writes `{}`, and `init --pin` writes guardrails instead of topology (5.7).

## 1. Problem Statement

A coding agent optimizes for a green result at the end of its turn. The
cheapest routes to green are the ones a reviewer dislikes most: a silenced
check, a skipped test, a deleted test, a stub left behind, a copy instead of a
reuse, a refactor nobody asked for, a dependency that does not exist. A
language linter catches none of these, because each is legal code.

klin closes those routes with deterministic checks that run at the end of every
agent turn and, optionally, in CI. It solves four problems:

- It measures two trees and refuses only what got worse, so a project with
  existing debt adopts it on day one and stays green.
- It needs no file to start. Every fact it needs it derives from the tree and
  prints. A person pins a fact in `klin.json` only to override it.
- It hands each failure to the agent as its next task, with a remedy that adds
  or fixes code rather than deleting work.
- It runs offline, without a network, a database or a service, in the time
  budget of a hook.

Boundaries:

- klin is not a linter. Language linters and type checkers own style and
  types. klin ratchets what they report, and adds the checks they cannot make.
- klin is not a test runner. It reads results a suite already wrote.
- A local hook is feedback. Only a CI run on a checkout the agent never touched
  is a control. Section 15 names the two conformance levels.

## 2. Goals and Non-Goals

### 2.1 Goals

- Zero configuration works. `klin.json` is optional, and every value in it is
  an override of a value klin would otherwise derive and print.
- klin writes nothing into the working tree. Its own state lives in the git
  directory or in a cache directory the person names.
- One install step per host. For Claude Code that step is the plugin. For
  every other host it is the binary plus one command that writes the hooks.
- A run compares two trees and fails on new or worsened debt only.
- A ceiling derived from the tree makes day one green and close to what the
  project already does. A dated schedule a person pins once makes it tighter
  from then on.
- A ceiling change never fails a site the base holds, so tightening is safe by
  construction.
- Every check is deterministic: same two trees, same configuration, same
  binary, same output, on any machine.
- Every check runs offline, in-process where a parser is needed, and finishes
  inside a hook's time budget.
- The hook protocol works for Claude Code first and for any host that offers a
  session-start event, a pre-tool event, a prompt event and an end-of-turn
  event.
- CI is optional. When present, it is authoritative.

### 2.2 Non-Goals

- Matching another tool's numbers. A ratchet needs self-consistency only
  (ADR 0001).
- Judging style, formatting or naming. Linters own those.
- Executing a project's test suite, or running anything five times. The
  section 13 performance fixture is the explicit measurement exception.
- An MCP server, a web UI, a dashboard or a service (ADR 0008).
- Preventing a determined person or agent from working around a local hook.
- Forcing an agent to pay down debt it did not add. Section 7.3.

## 3. System Overview

Nine components, in one binary.

1. **Config** loads `klin.json` when it exists, resolves paths against its
   directory, and rejects a key klin does not know.
2. **Survey** derives repository facts needed before a check is selected:
   documents, manifests and test roots. An Automatic source check discovers
   its supported files itself when it runs. `complexity` derives ceilings
   from the derivation commit, and `reachability` derives proven families
   there; neither expensive derivation runs unless that check is selected.
3. **Window Chooser** picks the two trees a run compares and says which.
   Section 6.
4. **Checks** measure one tree each and return Findings with a Site identity
   and values. A check knows nothing about the other tree. `inventory` is the
   one exception: its measure of `after` reads the `before` sites, because
   the value it ratchets is whether a site still exists (16.4).
5. **Ratchet Engine** matches Findings between the two trees plus the accepted
   list, and sorts each into new, worsened or held (ADR 0009).
6. **Gate Runner** runs every applicable gate in catalogue order, which is
   cheapest first, prints a status row per gate, and returns one exit code.
7. **Host Adapter** reads a host's hook event and writes the decision in the
   host's shape. Section 9.
8. **Guard** refuses or questions an agent's tool call that would change the
   configuration, the hooks or the code owners.
9. **Reporter** prints text for a person and JSON for a harness. SARIF
   output for a code scanning host is not shipped (11.3).

Three layers hold these. The measurement layer is checks and the ratchet
engine. The policy layer is config, survey and the window chooser. The
integration layer is the runner, the host adapter, the guard and the reporter.
The measurement layer MUST NOT read a host event, and the integration layer
MUST NOT read source code.

## 4. Core Domain Model

### 4.1 Tree

A tree is a set of files at one point. A run holds exactly two: `before` and
`after`. `after` is the working tree unless a command names a commit. `before`
is what the Window chose. A tree MUST be readable without a network.

### 4.2 Window

A window names the pair of trees a run compares and where each came from.
Three kinds exist.

| Kind | `before` | `after` | Who uses it |
|---|---|---|---|
| `turn` | the tree as it stood when the current turn's window opened | working tree | the Stop hook, `klin radius` |
| `branch` | the merge-base with the default branch, or the pull request base | working tree, or HEAD in CI | `klin gate` by hand, CI on a pull request |
| `push` | the push event's previous commit | HEAD | CI on a push |

Every run MUST print the window it used, as one line, before any gate output.
Every JSON report MUST carry it.

### 4.3 Derived

A value klin computed from the tree because the config did not pin it. Every
derived value MUST be printed with the word `derived` and the rule that
produced it, on the run that uses it. Two kinds exist.

A derived number, such as a ceiling, a radius percentile or a build command,
MUST be a pure function of one commit, the derivation commit of 6.6, and the
binary version, so it is cached by commit id. It is computed over the paths
that commit's own survey holds, never over a path found only in `after`.
Where its sample needs policy, it reads the policy recorded by that commit,
never the working-tree copy.

A derived path set such as the documents of `doc_size` or the manifests of
`build` is the union of the derivation commit's survey and a discovery walk
over `after`. Source roots and language lists are facts, not configuration
values: each source check selects every supported file from the run's one
repository walk, then applies its compact `in` / `except` policy. A site
present only in `after` matches nothing in `before` and is `new` (7.1).

### 4.4 Site

The identity a Finding is keyed by. A site is a file plus the text of its
declaration line, with the line number as a tie-breaker only (ADR 0008). A
check MAY define a different identity when a declaration line does not exist,
and MUST document it. Moving code within a file MUST NOT create a new site.
Renaming a file MUST NOT create new sites (ADR 0009). A finding MAY name a
site that `after` no longer holds, when the check ratchets existence (8.2).
Its `file` and `text` are then the `before` site's.

When one site has more than one finding or more than one entry, the findings
and the entries at that site are paired one to one, greedily, in this rank
order (16.5):

1. No ratcheted value rose against the entry.
2. The most ratcheted values exactly equal.
3. The smallest line distance. The matcher does not read a line from an
   accepted entry, so its distance is 0.
4. The finding's line. Findings arrive sorted by file then line (12).
5. Entry order: the accepted list in config order, then the `before` sites
   by line.

A finding matches at most one entry, and an entry at most one finding. A
finding is `worsened` only when no untaken entry at its site holds it. So a
stale accepted entry never fails a site that `before` holds (7.3), and a
value a person raised in the accepted list holds the site. The order also
lets a twin that moved keep its entry over a nearer twin whose values
changed, so moving code does not read as new debt, and it makes a twin
inserted between two twins the new one.

A function moved between files with its body unchanged SHOULD match its old
site. The RECOMMENDED second pass matches unmatched findings to unmatched
entries by a whitespace-normalized body hash, across files. This is additive
to ADR 0008 and does not change the primary key, so a move within a file and
a rename of a file still match on the primary key alone.

The body is everything below the first line, with every run of whitespace
collapsed. A declaration that fits on one line has nothing below it, so the
whole line is its key. A declaration that wraps keeps its later lines,
because the primary key is the declaration line and not the whole signature
(ADR 0008). Dropping the first line drops the name, so a rename whose body
did not change matches too, which is what `inventory` reads to tell a
renamed test from a deleted one (8.2). An edit to the body changes the hash,
so a move that also edits is `new`. This is an equality match on an
unchanged body and not a similarity match.

Only an entry whose site the `after` tree no longer holds is a move target.
From one site the second pass may take as many `before` entries as that site
lost, which is the count of its `before` entries less the count of its
findings, lowest line first. An entry an accepted one outranked is not lost,
so a copy is not a move: the original takes the primary match, and the copy
is `new`. klin drops a body hash from an accepted entry, so the accepted
list keeps the keys 4.8 names and no accepted entry is matched this way.

Pairing is one to one and ranks by 16.5 without the line distance, which
means nothing across files. A failure whose entry sits in another file names
that file after the values it was, so a function that moved and grew does
not read as a regression where it now sits.

### 4.5 Finding

One site with the values a check measured there. Fields:

- `file` (string) repository-relative path, REQUIRED
- `line` (integer) 1-based, REQUIRED where a line exists
- `text` (string) the declaration line, REQUIRED, part of the identity
- `values` (object) metric name to number or string, REQUIRED
- `outcome` (`new` | `worsened` | `held`) set by the engine, never by a check.
  A note about a site, such as an unmatched accepted entry or an unparsed
  file, is not a finding. It is a separate record under `notes` in the JSON
  (11.2), and its `outcome` names the kind of note rather than a verdict.

A value the check ratchets on is one where higher is worse. A check MUST name
those values. A value that is not ratcheted is carried for the report only.

### 4.6 Check and Gate

A check is one measurement and the judgement behind it. A gate is one
configured or derived instance of a check. A check declares:

- `name`, the command name and the `--gate` name
- `section`, the config key it reads
- `needs`, what the check needs of the base: nothing, the commit the window
  names, or that commit laid out as a tree beside the working one. Every
  Automatic check MUST judge against the base, so that a tree with no config is
  green (5.1). A check that judges one tree against a number exists only for a
  number a person pinned. A check that needs the commit or the tree is the
  kind `--strict` reaches, because it has a comparison or an accepted list to
  judge. `sarif` needs the commit and not the tree: it reads which lines the
  window changed and runs the scanner once, over the working tree only (8.3).
- `takes_scope`, whether a changed-file list narrows it. False says only that
  it does not, and a check sets it false for its own reason: `doc-size` and
  `lockfile` judge a set small enough that narrowing it buys nothing,
  `layering` and `sarif` read the whole tree by construction (8.3). The
  physical changed-file set is the default judgement boundary, and one of
  those reasons is that a check owns a broader bounded judgement unit
  instead, because a change elsewhere in the repository deterministically
  changes the meaning of evidence in a file the window did not touch.
  `public-api` judges the whole consumer-facing surface, `reachability` judges
  every member of each family, and `doc-citations` judges the whole derived
  root-document set. A check that owns such a unit MUST name it in its own
  contract in 8.2.1, and the runner never infers one: there is no dependency
  graph and no propagation rule above the catalogue. The turn window of ADR
  0014 says which work belongs to the turn. It does not require every finding
  of that turn to sit on a line the turn edited, and the two-tree ratchet, not
  the changed-file list, is what keeps old debt quiet.
- `gate_per_entry`, whether the section is a list of entries a person writes,
  each its own gate under its own `name`, rather than one section the whole
  check runs under. Only `sarif` sets it (8.3).
- `available`, whether the tree holds what an Automatic check applies to: a
  source root, a document at the tree root, an instruction file of 5.4 at the
  tree root for `doc_size`, a test, a manifest klin reads. It
  is answered from the facts of 4.3 alone and never from a derived number
- `activation`, what the section's absence means. An Automatic check runs
  over the tree's facts where it is available and derives its own policy. A
  Policy check, such as `conventions`, runs only once a person writes the
  policy. An Integration check, such as `sarif`, runs only once a person names
  the external tool (ADR 0038). No component derives a section for a check:
  each check reads the facts, resolves its own policy, and prints the
  provenance of each value it used (ADR 0040).

A check declares no cost of its own. The catalogue is one ordered table,
written cheapest first, and a run executes its gates in the order the
catalogue declares them (ADR 0036). A check that plans several gates, one per
entry a person wrote, keeps them together in that position, in the order the
section lists them. The order is a property of the table, so adding a check
in the right place is the whole of the decision.

A Policy or Integration check runs only when its section is present. An
Automatic check runs unless its section is `false`.

A run loads and validates the configuration once and reads every tree's file
list once, however many gates run and however many roots their sections
name. The configuration is the policy a person wrote; what a tree holds is a
fact the run reads for itself; the runner composes the two and a check
borrows what it needs (ADR 0038).

### 4.7 Ceiling

The value a measure may reach before its gate fails. A ceiling is derived, a
pinned number, or a pinned dated schedule. Section 7.3.

### 4.8 Accepted entry

Debt a person allows, keyed like a site, with every value the gate ratchets
and the amount of each they allow. Only a person writes one, in a reviewed
commit (ADR 0009). An entry that does not give a number for each of those
values is a config error, because a value it leaves out would grow unjudged
at that site. The one exception is a value a site of that gate may carry
or not: `lines` and `test_lines` of `complexity`, of which a function carries
at most one (8.2.1). An entry may leave either out. It then holds only a
finding that does not carry the value it left out, because a value a finding
carries and its entry does not is a rise (7.1). An entry that names `lines`
for a function in test code, with `test_lines` pinned, therefore matches
nothing. Pinned by
`an_accepted_entry_that_names_test_lines_holds_a_test_function`,
`an_accepted_entry_without_lines_holds_a_test_function_on_its_cc`,
`an_accepted_entry_that_leaves_out_lines_does_not_hold_a_production_function`
and `an_accepted_entry_that_names_some_of_the_values_is_a_tool_error` in
`tests/complexity.rs`. An entry that matches nothing is a NOTE, and a failure
under `--strict`.

The accepted entry takes the match when the finding holds against it, unless
a `before` entry the finding also holds against shares more values with it
(4.4). When the finding rose against the accepted entry but not against
`before`, or when `before` shares more values, the `before` entry takes the
match and the accepted entry matches nothing. That is how `--strict` tells a
person to delete the line once the code has moved off the accepted value.

### 4.9 Verdict

One run's result. Per gate: `ok`, `FAIL` or `ERR`. Overall: exit 0, 1 or 2.
Exit 1 means a gate failed. Exit 2 means klin could not measure something.

### 4.10 Note

Something klin reports and never fails on. A note has no ceiling.

## 5. Configuration Contract

### 5.1 The file is optional

`klin.json` at the repository root, when it exists. Discovery walks up from
the working directory. `--config PATH` overrides. Paths resolve against the
file's own directory. The file is under the guard and SHOULD be under
CODEOWNERS.

With no file, every Automatic check for which the tree holds applicable facts
runs. When
the two trees are the same, this MUST produce a green run, because nothing in
a tree is worse than itself. That is the first stop of the hook, whose first
stamp is the working tree as it stands. When the trees differ, as under `klin
gate` by hand against a merge-base, the run fails on new or worsened findings
only. A stale citation, a missing lockfile entry or a long document that the
base already holds is `held`, not `new`. A file klin could not measure at
the base either is a NOTE (8.6), so it keeps this promise too. A build that
already fails is judged under section 14 and is not part of this promise.

Under `--hook` the file is the marker that the repository opted in. When no
`klin.json` resolves, `klin gate --hook` MUST exit 0, print nothing and write
no state, before it surveys the tree. A `--config PATH` that names no file
reads the same way. `klin gate` without `--hook` keeps the exit 2 that names
the file and the sections that would fill it. ADR 0028.

One configuration per repository, at the root. Source checks discover every
supported package in a monorepo from that one root. Discovery walks up only to
find the file from a subdirectory. A configuration per package is not
supported.

### 5.2 Top-level keys

- `build` (string, list or `false`) OPTIONAL, ADR 0012. A build a person
  chose over the derived one: a command, a list of `{run, root}` entries, or
  `false` to build nothing. Absent, the hook derives one command per standard
  manifest when it builds (5.4). A derived command is never written into the
  file.
- `accepted` (list) OPTIONAL, section 4.8
- `radius` (object) OPTIONAL, ADR 0014, with `lines` and `directories` as whole
  numbers. Derived from history when absent.
- `journal` (object) OPTIONAL, with `prompt` a boolean. `false` turns off the
  prompt excerpt of 11.4; the excerpt is recorded by default. A configuration
  klin cannot read carries no excerpt either: the one case where klin cannot
  see this setting is the case where it MUST NOT record the text.
- one key per gate, named for its section, or `false` to exclude the gate.
  A check that runs as several gates holds them in its own section, as the
  named entries of `sarif` (8.3) and the named conventions (8.4) do. There
  is no top-level list of extra gates (ADR 0038).

`klin.json` is a person's policy over facts klin derives, and `{}` is a
complete configuration. It never describes the repository: roots, languages,
documents, manifests, test roots, families and build commands are facts.

A key klin does not know MUST be an error naming the key. `project` and
`version` are no longer keys, and a configuration that names either MUST be an
error saying to delete it: the repository's identity is a fact, and a binary
version freezes no semantics. A configuration schema epoch, should klin need
one, is a decision of its own (ADR 0040). A section with a `baseline` key MUST
be an error saying the key is gone (ADR 0009). Every section, and every
`radius`, `journal` and `build` entry, MUST reject every field its reader does
not read, including retired topology fields, with an actionable migration
error, and SHOULD name the field a person most likely meant. A section MAY pin
some keys and leave others to derivation. A pinned key MUST print as `pinned`
beside the derived ones.

### 5.3 Compact policy

`complexity`, `escapes`, `stubs`, `dead_symbols` and `reachability` are
Automatic. An absent section means use discovered facts and built-in language
support; `false` disables the check; an object holds only a decision a person
made. Every one reads `in` and `except`: a repository-relative path or
non-empty list, selecting that path and everything below it. They are paths,
not globs. An explicit `in` with no applicable file is exit 2.

`complexity` additionally reads `cc` and `lines`; either is a whole number or
a dated ceiling schedule and either may be omitted for derivation. It also
reads `test_lines`, of the same shape, which only a person pins, and which
holds the length of test code as `lines` holds the length of production code
(5.4, 8.2.1, ADR 0063). `escapes`
additionally reads `skip_test_idioms`, default `true`, which leaves `unwrap` and
`expect` out of Rust test code and `@ts-expect-error` out of TypeScript and
JavaScript test files, and nothing else (8.2, ADR 0049, ADR 0060). The key was
`skip_rust_tests`, and a section that still names it MUST get the migration
error of 5.2 naming `skip_test_idioms`. Pinned by
`the_retired_skip_rust_tests_key_names_the_key_that_replaced_it` in
`tests/escapes.rs`. `dead_symbols`
additionally reads name globs in `ignore`. `stubs` and `reachability` read no
other policy.

The retired `roots`, `languages`, `patterns`, `skip_dirs`, `exclude`,
`exclude_except` and `ceilings` fields, and a person-authored reachability
family list, MUST be rejected rather than ignored. Source discovery, supported
languages, marker patterns and reachability topology are properties of the
binary and tree, not knobs in `klin.json`.

`doc_size`, `doc_citations`, `inventory` and `lockfile` are Automatic too.
`doc_size` is a map of document path, from the configuration's directory, to
a ceiling, a whole number or a dated schedule (5.5). A document the map names
is judged under that ceiling, and `AGENTS.md` and `CLAUDE.md` at the tree root
keep their derived ceilings where the map does not name them (5.4), so a pin
never takes an instruction file out of scrutiny. An empty map is exit 2.
`doc_citations` reads no policy: its section is absent or `false`.
`inventory` and `lockfile` read only `in` and `except`. The retired `doc_size` entry list of `file` and `ceiling`,
the `file`, `roots` and `extensions` of `doc_citations`, the `name`, `path` and
`pattern` of `inventory`, and the `manifests` and `exclude` of `lockfile` MUST
be rejected with the replacement named (ADR 0040).

### 5.4 Derivation rules

Each check documents its rule. The rules for the shipped checks:

- Source files: every supported, non-ignored file in the repository's one
  tree listing, less the built-in skip set and the section's `in` / `except`
  scope. Each check owns its supported language table. The before tree uses
  the compact scope its own commit records, and today's scope when it records
  none, so narrowing scope cannot silently erase coverage. A derived sample
  instead uses the compact scope recorded by the derivation commit, so both
  the files and the policy that select them come from that one commit.
- Source roots and test roots: start at the directory of each source file
  and merge upward while the directory above holds nothing but source. A
  source root is a directory this finds that no other one holds. A source
  file that sits directly in a directory holding something else, such as
  `build.rs` beside a crate's `Cargo.toml`, starts at that directory, so the
  crate directory is a source root and holds the ones beneath it. A test root
  is a directory this finds that a test directory segment names or whose
  every source file carries a test affix, that no other test root holds, and
  that no other directory this finds holds unless that one is, or holds, the
  nearest directory above the test root that directly holds a `Cargo.toml`,
  `go.mod`, `package.json` or `tsconfig.json`. So a crate's `build.rs`, or a
  `jest.config.js` beside a package's `package.json`, keeps the package
  directory a source root and leaves its `tests/` a test root. Nor does a
  source file in a directory above the package remove that test root, such
  as a root `install.sh` above `rust/Cargo.toml` or a `crates/check.sh`
  above a workspace member, with or without a `build.rs` in the crate.
  `src/spec/` beside `src/schema.sql` and `src/lib.rs` is no test root,
  because `src`, which holds `lib.rs`, is a directory this finds between it
  and the crate directory, and it stays in its crate's source root. The
  survey knows no other manifest, so `Tests/` beside a Swift package's
  `Package.swift`, or `tests/` beside a Python project's `setup.py` and
  `pyproject.toml`, is no test root. Pinned by
  `a_build_script_at_a_crate_root_keeps_its_tests_as_a_test_root`,
  `a_workspace_member_with_its_own_build_script_keeps_its_tests_as_a_test_root`,
  `a_crate_below_a_directory_that_holds_a_script_keeps_its_tests_as_a_test_root`,
  `a_config_file_beside_a_packages_manifest_keeps_its_tests_as_a_test_root`
  and `a_directory_another_test_root_holds_is_no_test_root_of_its_own` in
  `tests/survey.rs`, and by
  `unwrap_and_expect_in_the_tests_of_a_crate_with_a_build_script_are_left_out_by_default`,
  `a_workspace_members_build_script_and_src_are_judged_beside_its_test_root`,
  `a_script_added_between_a_workspace_member_and_its_root_keeps_the_members_tests_left_out`,
  `a_directory_inside_src_is_no_test_root_beside_a_non_source_file` and
  `removing_a_non_source_file_from_src_keeps_a_held_site_under_it_held` in
  `tests/escapes.rs`.
- `doc_size`: the agent instruction files `AGENTS.md` and `CLAUDE.md` at the
  tree root, in the derivation commit and in `after`. The
  ceiling is the word count at the derivation commit, rounded up to the next
  50 and never below 50, so an empty document gets 50 rather than a ceiling
  its first word breaks. An instruction file the derivation commit lacks is
  not judged on that run. A
  NOTE names it and its word count, and it gets a ceiling when the stamp
  moves and the derivation commit holds it. Any other rule would read the
  ceiling from `after`, which 4.3 forbids. Every other document, such as a
  README or a changelog, grows by design and is judged only when the section
  pins it, because the gate exists for the instruction file that grows every
  turn (8.2, #343). A document the section pins takes its pinned ceiling
  instead, and is judged wherever it sits. With no section, `doc_size` runs
  only when the tree holds an instruction file at its root, and a tree whose
  root holds only other documents needs a section a person writes. `--file`
  on such a document with no pin is exit 2. A cache that holds a ceiling for
  any other document is no derivation of that commit, so klin derives again
  and `init --pin` never writes that ceiling into policy. Pinned by
  `a_readme_that_grows_past_its_base_word_count_passes_with_no_pin`,
  `an_instruction_file_that_grows_past_its_derived_ceiling_fails_with_no_pin`,
  `a_pinned_readme_is_judged_under_its_pin`,
  `a_readme_alone_under_an_empty_config_leaves_doc_size_needing_a_section`
  and
  `file_on_a_readme_under_an_empty_config_is_a_tool_error_naming_the_instruction_files`
  in `tests/doc_size.rs`, and by
  `pin_writes_no_readme_ceiling_a_cache_of_this_version_still_holds` in
  `tests/init.rs`.
- `doc_citations`: every Markdown file at the tree root in the union of 4.3,
  each read against the whole tree with the built-in extension list of 8.2.1.
  This set is the check's judgement unit on a changed run too (8.2.1). It is
  wider than the documents `doc_size` derives a ceiling for, so a README is
  read for its citations with no pin. Pinned by
  `a_root_readme_doc_size_does_not_judge_is_still_read_for_citations` in
  `tests/doc_citations.rs`.
- `inventory`: every file under a test root the survey finds, and every
  source file a test directory segment or a test affix of 8.2 marks wherever
  it sits, less the default skip set and hidden directories.
- `lockfile`: every manifest the survey finds that klin has a reader for,
  `Cargo.toml`, `package.json` and `go.mod`.
- `complexity.cc` and `complexity.lines`: the 95th percentile of each measure
  over every supported function selected by the compact scope recorded at the
  derivation commit, with the TypeScript and JavaScript suite callbacks in test
  files omitted as 8.2.1 states,
  rounded up to the next whole number, with a floor of `cc 10` and `lines 25`
  so a small clean tree is not held to a ceiling of 1. Below 50 functions the
  floor is the ceiling. A pinned `cc` wins over the floor, whatever its value
  (ADR 0059). Pinned by `a_percentile_below_ten_derives_a_cc_ceiling_of_ten`
  and `a_pinned_cc_below_ten_still_judges_at_the_pinned_value` in
  `tests/complexity.rs`. Test code of 8.2.1 stays in the sample of both
  measures, so no derived ceiling moves when tests are judged apart. Pinned
  by `test_code_stays_in_the_derived_sample_for_both_ceilings`. `lines`
  judges only production code. `complexity.test_lines` is never derived: a
  missing `test_lines` means test code is not judged on length, and not that
  a ceiling is derived for it (ADR 0063). A function found only in `after`
  never enters the percentile. No recorded section, or a recorded object with neither `in` nor
  `except`, selects the whole repository. A recorded scope that selects no
  supported function derives the floors and names its zero-function sample.
  A recorded configuration or complexity section that exists but cannot be
  read as compact scope falls back to the whole repository and emits a NOTE.
  A pre-compact section containing only retired fields has no compact scope
  and therefore selects the whole repository. Today's compact scope selects
  what is judged but never enters this sample; when it differs from the
  recorded scope, a NOTE names both. Two scopes are compared after each is
  sorted and loses every path another path of its list holds and every
  `except` path outside every `in`, so one selection written two ways is one
  scope. The fallback and the difference are `derivation` notes, which a stop
  nothing blocks still tells the person.
- `dead_symbols`: every Rust and TypeScript/TSX file selected by the compact
  scope. Its optional `ignore` list is a set of name globs.
- `reachability`: one family per directory of the derivation commit whose
  files share a basename prefix or suffix at a token boundary and one
  concrete extension, such as `src/commands/*_command.rs`, named for its
  root and pattern. A family is derived only when its complete cohort under
  that root holds at least three files, every one structurally measured,
  every one with an eligible declaration, and every one proven reached: an
  eligible declaration whose name has exactly one declaration under the
  index and a reference from another file. A member reached only through a
  name several files declare is not proof. A TypeScript destructuring
  declaration that binds the name is no declaration of it here, and one in
  another file counts as a reference from that file, as a lazy
  `const { default: Profile } = await import(…)` does for the file it loads.
  A shorthand binding counts too, so `const { Login } = await import(…)` in
  another file proves the member that declares `Login`. A destructuring
  inside a function body is no declaration, so a name only it binds proves
  nothing (8.2.1). Pinned by
  `a_destructuring_that_binds_a_member_name_is_no_second_declaration_of_it`,
  `a_member_only_a_destructuring_in_another_file_binds_still_proves_its_family`
  and `a_shorthand_binding_in_another_file_reaches_and_proves_a_member` in
  `tests/reachability.rs`. `*.rs`, `*.ts` and every other
  bare extension are never a family. The derivation reads the structural
  files under the derivation commit's source roots, less those under a
  source root that is also a test root, and a file under a test directory
  seeds no family. A test root inside a source root, such as a crate's
  `tests/` beside its `build.rs`, stays among the files read, so a reference
  from it can prove a member, and a test directory under a family's root
  stays in the cohort the family must prove. Pinned by
  `a_crates_integration_tests_beside_its_build_script_still_prove_a_family`
  and
  `a_test_directory_under_a_family_root_stays_in_the_cohort_beside_a_non_source_file`
  in `tests/reachability.rs`. A file under a test directory, or one whose
  basename carries a test affix of 8.2, is never a member in a tree a run
  judges.
  Of two candidates the
  broader wins where its whole cohort is proven, and a narrower one survives
  a broader one that is not. The policy is read from the derivation commit
  alone, never from the union with
  `after`, so the tree being judged cannot widen or weaken it, and it is
  cached under that commit. When nothing is proven the gate has no families
  to judge. A person may narrow the derived families only with `in` and
  `except`, or disable the check with `false`; a family list is invalid.
- `radius`: the 90th percentile over the last 200 non-merge commits, per
  ADR 0014, or no section below 50 commits.
- `build`: one entry per manifest, per ADR 0012, derived only by a hook run
  that runs the build. The `derived:` line names each command and the
  manifest it came from, and the hook prints it whether the build passes or
  fails, so a derived command never reaches the agent with no origin (ADR
  0040, ADR 0048). Manifests are a path set. A manifest the derivation commit
  lacks gets its entry from the fixed table on the turn that adds it.
  The derived value is the command the table names, such as `tsc --noEmit`,
  and it stays a function of the derivation commit. Which tool a checkout
  runs that command with is resolved when the build runs, per 9.3, and
  reaches neither the derived value nor the order the entries run in. A run
  that resolved a command against the checkout prints one `resolved:` line
  per such entry in its text report, beside the `derived:` line and above the
  gates, so the report names what ran. A `resolved:` line is a fact of the
  checkout and no derivation, so neither the journal nor the `--json` object
  carries it. klin never invokes `npx`, `npm exec` or any other command that
  could fetch a tool. A `build` a person wrote runs exactly as written.

A derived ceiling is not monotone. A percentile falls when simple functions
arrive and rises when simple functions leave. A tree of 96 simple functions
and four complex held ones has a ceiling at the floor. Delete 46 of the
simple ones and the next derivation puts the ceiling at the complex four,
though no surviving function got worse. klin keeps no history that could
prevent this, and a run prints the ceiling it used, so a person sees the
number move. The derived ceiling is the day-one default and nothing more. A
person who wants a ceiling that cannot loosen pins one. Section 7.3 names
what tightens.

### 5.5 Pinned ceiling shape

A pinned ceiling is either a number or an object of dated steps:

```json
"complexity": {
  "cc": 12,
  "lines": { "2026-09-08": 90, "2027-01-01": 70, "2027-07-01": 60 }
}
```

The run uses the lowest step whose date is on or before today. A schedule
with no step yet due MUST be an error. Today is the system date in UTC, so
two machines on one day agree on the step. A run MAY take
`KLIN_TODAY=YYYY-MM-DD` for tests. A run that uses a schedule MUST print the
date it used on the `derived:` line for that ceiling. This is the shape issue
#87 proposes for one gate, applied to every ceiling.

### 5.6 Exclusion

A gate is excluded by setting its section to `false`. Naming an excluded gate
on the command line is exit 2.

### 5.7 What `init` does now

`init` writes `{}` when no configuration exists, which is the repository's
opt-in marker (5.1, ADR 0028), and says so. It reads no tree and derives
nothing, because a run derives what the file leaves out. It MUST NOT change an
existing configuration.

`init --pin` writes today's suggested guardrails as policy a person reviews:
the complexity `cc` and `lines` the derivation commit gives, and no
`test_lines`, which nothing derives (pinned by
`pin_writes_no_test_lines_because_klin_never_derives_it` in `tests/init.rs`), a `doc_size`
ceiling for each instruction file of 5.4 that commit holds at the tree root and
for no other document, because it pins what a run derives, and the `radius`
values history gives. The `doc_size` rule is pinned by
`pin_writes_a_document_ceiling_only_for_the_instruction_files` in
`tests/init.rs`. It writes a value only where the configuration states
none, so a pinned number, a dated schedule, a section set to `false`, the
`accepted` list and the `journal` preference stay as a person wrote them. It
creates the file when there is none. It MUST NOT write repository topology: no
roots, languages, document entries, manifests, test roots, reachability
families, build commands or package structure (ADR 0040). The snapshot flags
`--add` and `--force` are gone.

`init` MUST write only the config. The guard denies `init` in every form from
an agent. `init` MUST NOT edit `.gitignore`, because klin writes nothing that
git could see.

`init` carries no host integration. `klin install` owns it, and the `--hooks`,
`--host` and `--global` flags of `init` are gone (19.3, ADR 0046). A command
line that names one is a usage error.

`init` is a convenience, not a step. A tree with no `klin.json` is fully
gated.

### 5.8 The configuration reference

`klin reference` prints the configuration reference as Markdown on stdout and
exits 0. It reads no configuration and no tree, so it runs anywhere.

Every check declares its keys beside the code that reads them, and the
reference prints one table per section. Each row states the key, what it
holds, whether klin derives it when absent or only a person pins it, the
derivation rule of 5.4 where there is one, and the default where there is
one. A section that reads no key says so. The reference states the top-level keys of 5.2 the same way, and
the dated ceiling shape of 5.5.

Every key the reference names is read through its declaration, so a key
renamed in the declaration is renamed where it is read. A key inside one of them, such as the `run` of a `build`
entry, is stated in what the key above it holds and is not a row of its own.
The sections the reference prints, and the built-in language coverage it
prints beside them, come off the same table of checks a run gates from, so a
check cannot be gated and left out of the reference.

`schemas/klin.json` is generated from those same declarations by `klin
reference --schema`. SchemaStore registers the exact filename `klin.json`, so
editors can discover completion, hover text and structural validation without
an inline `$schema`, editor setting or extension. The schema is editor
guidance only: `klin` remains the authority for semantic validation, and
normal commands never read or validate against the schema artifact.

The reference MUST also state what the key tables alone do not say:

- the built-in source extensions each check discovers, printed from the
  tables in the binary, and that those tables are capability rather than
  configuration
- the shared default skip list and git-ignore behavior
- the `in` / `except` path shape and the retired topology keys that compact
  sections reject
- the path-to-ceiling shape of `doc_size` and the built-in citation extensions
  of `doc_citations`
- that `publish = false` and `"private": true` do not make a package not
  applicable to `public_api` (ADR 0050)
- that a file which leaves compact scope is reported as lost coverage under
  the base-era scope rule of 8.6

`docs/REFERENCE.md` holds the printed reference, and a CLI test fails when the
committed copy differs from what the binary prints, so CI fails on a reference
that drifted. A person regenerates it with `klin reference > docs/REFERENCE.md`.

## 6. Window Selection

### 6.1 The hook uses the turn window

In `--hook` mode `before` is the turn stamp. The stamp is a git tree object of
the working tree, including uncommitted and untracked files that `.gitignore`
does not exclude. `after` is the working tree. Committing inside the turn does
not move either.

This reverses ADR 0009 for the hook only. The merge-base is the right window
for a pull request and the wrong one for a turn. On the default branch with a
remote it measures unpushed commits and then nothing. With no remote it
measures uncommitted work and then nothing. On a long branch it re-judges
every file the branch touched at every stop. The turn window has none of
these, and it is the window ADR 0014 already built for radius.

### 6.2 When the stamp moves

One rule, applied on session start and on every prompt submitted alike:

> The stamp moves to the current working tree on a first session, or when
> the last stop ended green. Otherwise the stamp stays where it is.

So debt an agent left behind stays `new` until it is fixed or a person accepts
it, across turns and across sessions. The stamp persists in the state
directory between sessions, and a session start that finds a red verdict
keeps the old stamp rather than photographing the mess. A first session is a
worktree whose state directory does not exist yet. Its first stamp is the
working tree as it stands, which treats a person's uncommitted work as prior,
and that is correct.

An acceptance does not move the stamp. An accepted entry is a `before` entry
under 7.1, so the site it names is `held` on the next stop, and the stamp
moves when that stop ends green. Accepting finding A while finding B is still
open leaves B `new`. A rule that moved the stamp on acceptance would make B
inherited debt, which is the route to green this section exists to close.

A person who abandons the work has a third route: `klin turn reset` moves the
stamp to the current working tree and prints that a person moved it. The guard
denies the command by name from an agent, the way it denies `init`. Without
this command a red window that nobody acts on degrades into a report that
everyone learns to ignore.

A stamp describes the current turn only while current HEAD history still holds
the commit it was taken over. That commit is the stamp's parent, HEAD at
stamping time, and not the stamp itself: the stamp is a synthetic sibling of
its parent and is never an ancestor of a later commit. So the rule is keyed to
commit history and not to branch names.

> A red stamp stays while the HEAD it was taken over remains in current HEAD
> history. Where current HEAD no longer holds that commit, the stamp cannot
> describe the current turn.

A commit made inside the turn keeps the parent an ancestor of HEAD, so the
turn window stays, and so does `git checkout -b` at the same HEAD and any
branch that descends from the parent. A switch to divergent history, a
detached checkout of an unrelated commit, a hard reset, and a rebase that
drops the parent each take the parent out of HEAD history. The stop then
prints a NOTE that names the reason, judges a branch window from the base of
6.3 for the current checkout, and writes that base as the stamp, red, keeping
the prompt counter and dropping the `asked` record of 8.2 and `intervened`,
because both belong to the turn the checkout left. The handoff records of
9.1 belong to a host session and not to the turn, so the fallback leaves
them: a report the host is about to submit is still recognized, and a
session's next prompt clears its record as always.
The journal records the stop as `branch-fallback` (11.4).

The recovery copies go with it. The stop deletes `refs/worktree/klin/turn`
and `refs/worktree/klin/mark`, because both are copies of a turn the checkout
left and a copy of that turn is the one thing a later stop must not read. A
stop that loses the `turn` file after this widens to the branch window, which
is the window the fallback already judged, so the deletion forgives nothing.
The ref MUST NOT instead be moved to the base: a stamp restored from the ref
reads its parent as `<commit>^`, which names the stamped HEAD for a synthetic
stamp and the commit before the base for an ordinary one, so a moved ref would
name a parent no stop ever took and would pass this section's test on history
that holds no base at all.

klin MUST tell a proven "not an ancestor" apart from a question git could not
answer. `git merge-base --is-ancestor <parent> HEAD` exits 0 for an ancestor,
1 for a proven divergence, and other codes when git refused the question. Only
the proven divergence takes the fallback above. A stamp that names no parent,
which is the shape a stamp taken over an unborn HEAD has, keeps the turn
window.

A stamp is missing when the state directory exists and holds no `turn` file.
A `turn` file that is gone while the ref of 6.5 remains is restored from the
ref with a RED verdict, and a NOTE says so. When the file and the ref are
both gone, the stamp was deleted. The rule above writes no fresh stamp then,
because a fresh stamp would photograph whatever the deletion was meant to
hide. The next stop prints a NOTE that names the missing stamp, judges a
branch window from the base of 6.3, and writes that base as the stamp with
the verdict the gates gave. So deleting a stamp widens the window to the
whole branch. When no base of 6.3 resolves either, the stop judges from HEAD
and says so.

The stamp holds the stamped commit id, the time, the verdict of the last
stop, a prompt counter, the prompt mark of 6.2.1, and the `asked` record of
8.2. `klin gate --hook` writes the verdict and adds to `asked`. A fresh stamp
holds an empty `asked`, so the record goes whenever the stamp moves. `klin radius` applies the rule above and raises the
counter by one on every session start and prompt submitted, whether or not
the stamp moved. The counter is what makes the per-turn block budget of 9.3
literal, because the stamp itself moves only after a green stop. The state directory
the stamp sits in is guarded (9.4).

#### 6.2.1 The prompt mark

The stamp waits for a green stop, and the radius report cannot. Keyed to a
frozen stamp, the report measures one window that grows with every prompt,
calls it the turn that just ended, and prints on every prompt once that
window passes its value. ADR 0024.

So klin takes a second mark. The prompt mark is a commit over the same tree
the stamp commits, with HEAD as its parent, under `refs/worktree/klin/mark`.
It moves on every session start and on every prompt submitted, whatever
verdict the last stop left. It is held in the `turn` file beside the stamp and
written in the same atomic write.

The prompt mark is the window the radius report measures, and nothing else
reads it. A stop judges the stamp, `klin turn reset` moves the stamp, and the
derivation commit of 6.6 comes from the stamp. A mark the `turn` file has lost
is read from the ref. With neither, the report prints nothing, which is what
it does with any window it cannot measure. Deleting the mark costs a report
and no block, so the mark needs no recovery beyond its ref.

### 6.3 `klin gate` by hand and in CI

Candidates, in order:

1. `GITHUB_BASE_REF`: the pull request target, as
   `refs/remotes/origin/<target>` then as a local name. Kind `branch`.
2. The push event's `before` commit from `GITHUB_EVENT_PATH`, when it is not
   the null commit. Kind `push`.
3. The merge-base of HEAD with the default branch, trying the remote HEAD
   symbolic ref, then `origin/main`, `origin/master`, `main`, `master`. Kind
   `branch`.

The first candidate that resolves wins. None resolving is exit 2 with the list
of what was tried. No flag overrides the list.

### 6.4 A base equal to HEAD

Outside the hook, when `before` equals HEAD and the working tree is clean:

- A remote source passes and prints that the trees are the same.
- A local source passes only when the remote default branch holds HEAD.
  Otherwise exit 2 naming the unpushed commits.
- No remote tip is exit 2 under `--strict` and a pass otherwise.

This is ADR 0013 unchanged, and it now applies to `klin gate` and CI only.

### 6.5 Materialization

Under `--changed` the `before` version of each changed file comes from `git
show`, at its old path for a rename. Without `--changed` the `before` tree is
a detached worktree under a temporary directory, removed after the run. A root
the `before` tree lacks measures nothing there.

The turn stamp is a commit made with `git commit-tree` over a tree from a
temporary index, with HEAD at stamping time as its parent, so both paths above
work on it unchanged. The RECOMMENDED stamping sequence is `git add -A` with
`GIT_INDEX_FILE` pointing at an `index` file in the state directory, then
`git write-tree`, then `git commit-tree -p HEAD`, then `git update-ref
refs/worktree/klin/turn <commit>`. The ref keeps `git gc` from pruning the stamp,
makes it visible to `git log --all`, and is the copy a stop restores the
`turn` file from when that file is gone (6.2). The ref is never pushed. The `turn` file
in the state directory holds the time, the verdict, the prompt counter and
the `asked` record of 8.2 beside the commit id, and `intervened`, set once a
stop under the stamp spends a gate block (9.5). A fresh stamp holds neither.
What a host session was handed is not in the `turn` file: it lives in the
handoff records of 9.1, one file per session, so no write of the stamp can
replace it.
It MUST be written to a temporary name and renamed into place, so a hook that
dies mid-write leaves the previous stamp, not a torn one. Two sessions in one
worktree share one window and one `turn` file. A stop MUST hold an advisory
lock on the state directory from before it measures until after it writes the
verdict, so stops in one worktree run in order and the last verdict describes
the last tree. Without the lock an old green stop that finishes after a new
red one would write green, and the next prompt would move the stamp over the
red debt. A stop that cannot take the lock within the hook budget writes no
verdict, spends no build block and no gate block, and says so. It still
measures and reports, but another stop may be writing the counts of 16.3,
so a block it spent could exceed the budget of 9.3. It reads its window
read-only: from the `turn` file, or else the ref, with no restore of a
missing file, no re-anchor of an abandoned stamp and no replacement written
(6.2, 16.1).

The ref is `refs/worktree/klin/turn`, not `refs/klin/turn`. Git shares
`refs/` across the worktrees of one repository, with `refs/worktree/`,
`refs/bisect/` and `refs/rewritten/` as the exceptions, so a stamp under
`refs/klin/` in one worktree would replace the stamp of another and leave it
for `git gc` to prune.

### 6.6 The derivation commit

Derived values come from one commit, the derivation commit. Under the turn
window it is the stamp's parent, which is HEAD at the time the stamp was
taken. Under the branch and push windows it is `before` itself. It is never
the stamp, because a stamp is a new commit on every turn and a cache keyed by
it would never hit.

The window names its derivation commit, and a run takes that commit from the
window before any fact of the tree reaches a check. A fact read before the
window is known is read again under it. A run by hand or in CI, whether
`klin gate` or one check's own command, derives from `before` whatever turn
stamp the checkout holds, so a change it judges never moves its own ceiling,
even when the change is committed. The hook derives from the stamp's parent
even when the state directory cannot be written, because the turn ref keeps
the stamp. A run by hand that resolves no base has no `before`, and it
derives from the stamp's parent, or from HEAD when no stamp is readable.
`klin init --pin` judges nothing and has no window, so every value it pins
comes from that same commit, the stamp's parent or HEAD, as `radius` does.
Pinned by `a_branch_run_derives_from_the_base_and_not_from_a_committed_change`,
`init_pin_with_no_turn_stamp_derives_from_head_and_not_from_the_base`,
`init_pin_derives_from_the_stamps_parent_and_not_from_the_base_or_head`,
`a_branch_run_by_hand_derives_from_the_base_and_not_from_a_turn_stamp`,
`a_push_run_derives_from_the_commit_the_push_started_from`,
`a_document_ceiling_comes_from_the_same_base_as_the_complexity_ceiling`,
`a_check_run_on_its_own_derives_from_the_same_base_as_the_gate`,
`doc_size_run_on_its_own_derives_from_the_base`,
`a_stop_whose_state_directory_is_unusable_still_derives_from_the_stamps_parent`
and `a_commit_inside_the_turn_does_not_recalibrate_until_the_stamp_moves` in
`tests/survey.rs`, and by
`reachability_on_its_own_derives_its_families_from_the_base_as_the_gate_does`
in `tests/reachability.rs`.

The survey MUST cache its result under `survey/<commit>.json` in the state
directory, because its numbers and its path sets at that commit are a pure
function of the derivation commit and the binary version. The `after` walk
of 4.3 is not cached and is unioned in at run time. The first stop after a
commit pays one whole-tree parse.
Every stop between two commits reads the cache. The cache is per worktree
like the rest of the state directory (7.4).

A commit inside a turn does not move the derivation commit. The stamp's parent
is fixed when the stamp is taken, and the stamp moves only under 6.2. So a
turn is judged against one set of derived values from start to end, whatever
the agent commits along the way. The derivation commit changes only when the
stamp moves, which is after a green stop, and a person sees the new ceiling on
the `derived:` line of the next run.
A compact-scope edit changes what is judged immediately, but changes a derived
complexity ceiling only when the derivation commit advances.

## 7. Ratchet Semantics

### 7.1 Outcomes

Three outcomes (ADR 0009):

- `new`: over the ceiling in `after`, no matching site in `before` or in the
  accepted list. FAIL.
- `worsened`: matched, and a ratcheted value rose. FAIL. A value the finding
  carries and the matched entry does not counts as a rise, so a value that
  starts to be judged at a site is judged from nothing.
- `held`: matched, no ratcheted value rose. Pass.

A `complexity` site ratchets both `cc` and `lines`, so a function over its
`cc` ceiling is `worsened` when only its `lines` rise, even under the `lines`
ceiling. A function in test code with no `test_lines` pinned ratchets `cc`
alone, because nothing judges its length (8.2.1). Pinned by
`a_test_function_whose_length_grew_is_held_while_its_cc_holds_with_no_test_lines`.
A function over its `cc` ceiling that leaves test code, such as one whose
`#[cfg(test)]` is removed, carries `lines` again, and the base entry does
not, so it is `worsened`. Pinned by
`a_test_function_that_becomes_production_code_is_judged_on_its_length`. No `+N` or percentage tolerance applies (ADR 0062). Pinned by
`a_function_whose_length_grew_since_the_base_fails_too` in
`tests/complexity.rs`.

Below the ceiling nothing is judged. An accepted entry is a `before` entry. A
finding matches at most one entry, and an entry at most one finding.
Identical sites match in the rank order of 4.4.

A site under a path the derivation commit's survey did not hold matches
nothing in `before`, whatever `before` holds there, so when the check judges
it at all it is `new`. Whether the check judges it follows its rule for a
path without a number (4.3, 5.4). A path that was not measured was never
held, so a directory that becomes a root, or a file that becomes a known
language, cannot bring inherited debt with it.

A path under a root the derivation commit's survey held stays held when a
root that survey did not hold also contains it: a root `build.rs` that makes
the crate directory a root keeps the base's sites in `src` held, and only the
paths outside `src`, such as the `build.rs` itself, match nothing in `before`.

### 7.2 Scope

`--changed` restricts both findings and entries to the changed files, so an
untouched file's debt never reads as new or as fixed. It announces the file
count it judged. Under the turn window the changed files are the files the
window changed, and an empty set is an honest "nothing changed", not a hole,
because a commit does not empty it.

The runner also makes the full `Change` set available to each gate in a
changed run, separately from the list of files the gate judges. A `Change`
keeps its current path and, where applicable, its base path, so additions,
deletions, modifications and renames remain distinguishable. `Context.only`
continues to mean judgement and report scope; it is not a change or
invalidation model.

### 7.3 Tightening

The invariant that makes tightening safe:

> A ceiling change MUST NOT fail a site the base holds. A site over the
> ceiling in both trees with no ratcheted value risen is `held`, whatever the
> ceiling is.

Because of it, lowering a ceiling affects new code only. What tightens, in
order of how much work it does:

1. A person pins a dated schedule once (5.5). This is the mechanism. `init`
   MAY offer to write one from the derived ceiling to a target over a period,
   under a flag such as `--tighten 18m`, and MUST NOT write one unasked.
2. A person pins a lower number.

The derived ceiling tightens nothing. It can rise when simple code leaves the
tree (5.4). A pinned number or schedule is the only ceiling that cannot
loosen, and `init` writes one from today's derived value in one command.

Forced paydown, where a touched file must leave with less debt, is NOT
RECOMMENDED. Its only remedy is a refactor nobody asked for, which the radius
report exists to discourage.

### 7.4 What the tool writes, and where

Nothing into the working tree. `init` writes `klin.json` and hook files, and
only when a person runs it.

klin's own state is six things: the turn stamp with the prompt mark of
6.2.1, the build stamp, the handoff records of 9.1 under `handed/`, the
event claims of 9.8 under `claims/`, the cache, which holds the survey of 6.6
and the structural cache of 8.4, and the journal of 9.6. All are per working
tree. The cache is safe to delete. All
six are guarded, because the
guard guards the directory they share (9.4). Deleting the turn stamp buys
nothing, because a stop without one judges the whole branch (6.2). They live
in the state directory:

- By default, `klin/` under the directory `git rev-parse --git-dir` returns.
  Git never tracks it, never lists it as untracked, `git clean` never removes
  it, each worktree has its own, and it goes when the repository goes.
- When `KLIN_STATE_DIR` is set, under that directory, in a subdirectory named
  by a hash of the git common directory and the worktree path. Two clones or
  two worktrees MUST never share a stamp. `~/.cache/klin` is the intended
  value.

An implementation MUST print the state directory under `klin gate --list`.
`klin cache clean` MUST remove the cache for the current tree, and with
`--all` the cache under every entry of `KLIN_STATE_DIR` whose repository
no longer exists. It MUST NOT remove a stamp, and it MUST leave the journal
alone: the journal is history, not a cache. A repository path that does not
resolve on the machine running the command, such as a worktree a container
does not mount, is no proof that the tree is gone, and a stamp is the one piece
of state whose loss lets a block go unspent.

ADR 0004's argument for the location holds in both: an agent empties `target/`
as a matter of routine and does not empty `.git/` or a home cache. ADR 0015's
reason for `.klin/` was one `.gitignore` line, and neither location needs one.

Anyone already running klin has `.klin-build-blocked` or `.klin/` in
`.gitignore`. Those lines become inert. `init` MAY say so when it sees one.

## 8. Check Catalogue

### 8.1 Criteria

A check earns a place when all four hold:

1. **Deterministic and offline.** Same inputs, same output, no network, no
   clock except a pinned dated ceiling.
2. **Additive remedy.** The fix adds or improves code. A check whose only
   remedy is undoing work is a report, not a gate (ADR 0014).
3. **Not a linter's job.** Either it needs git, a second tree, a second
   artifact such as a document or a lockfile, or knowledge across files. Or
   it ratchets a linter's own output.
4. **Names an agent failure.** Something an agent does when it wants green
   cheaply.

### 8.2 Tier 1: build these

Every check here except `sarif` compares two trees. "New against `before`"
means a site present in `after` and absent in `before` fails, and a site in
both is held. `inventory` runs the other way: a site in `before` and gone
from `after` fails, as a rise of its `missing` value from 0 to 1.

| Check | Agent failure it names | Identity | Judgement | Derivable | Status |
|---|---|---|---|---|---|
| `escapes` | silenced check, swallowed error, skipped test | file + line text | `count` rises | yes | shipped |
| `complexity` | tangled function written in a hurry | file + declaration | `cc`, `lines` rise | yes | shipped |
| `dead-symbols` | private declaration left unreferenced | file + declaration | `dead` rises from 0 to 1 | yes | shipped |
| `reachability` | implementation file nothing in the repository uses | file | `unreached` rises from 0 to 1 | yes | shipped |
| `doc-size` | instruction file that grows every turn | document | words over a ceiling derived from the derivation commit | yes | shipped |
| `doc-citations` | document that cites a file that moved | document + path | new against `before` | yes | shipped, needs the base comparison |
| `radius` | unprompted wide change | turn | report only | yes | #91 |
| `stubs` | placeholder left behind | file + line text, or file + kind for a comment marker | `count` rises | yes | **new** |
| `inventory` over tests | deleted test file, deleted test function | test file path, or test function site | `missing` rises | yes | shipped |
| `lockfile` | dependency added without a lockfile entry, pin removed, pin the lockfile does not record | manifest + name | `unlocked`, `unpinned`, `stale` rise | yes | shipped, Rust, npm and Go |
| `sarif`, `after` only | anything a linter reports, on a line the window changed | file + rule + message | new on a changed line | no | shipped, section 8.3 |

`inventory` has two identities. A test file is keyed by path. A test
function is keyed like a complexity site, file plus declaration line, and
only functions the language's test convention marks count, such as a
`#[test]` item, a `test_` function or an `it(` call. That table is fixed in
the binary and 8.2.1 states it. `inventory` ratchets one value, `missing`,
which is 0 for every test site in `before` and 1 for a site `after` no
longer holds, so a vanished site is `worsened` under the one judge of 16.4.
Both identities reach that judge together and count in one unit, `test
site(s)`. The second pass of 4.4 matches a renamed test by body hash before
it counts as missing, so a rename with the body unchanged is `held`.

Removing a test is ordinary work, and deleting a failing one is the
cheapest route to green in section 1. klin cannot tell which it was, so it
asks once and does not judge the answer (ADR 0031):

- Under `--hook`, a vanished site is `worsened` and blocks the stop, and the
  remedy asks the agent to restore the test and fix the code if the test
  failed, or to say why the removal is intended and stop again. A stop that
  blocks records the site id (11.2) of every finding it reported in the
  `turn` file, beside the stamp (6.5). A vanished site that record holds is
  let through: its `before` entry carries `missing: 1`, so the one judge
  holds it, and the run lists it in a NOTE. The report names every finding
  it records, so nothing reaches that record unasked. The record goes when
  the stamp moves, and the guard refuses an agent's write to the file that
  holds it (9.4). The rule keys on `--hook` and not on the window kind,
  because a stop whose stamp was deleted judges a branch window (6.2), and a
  stamp that went takes the record with it, so such a stop asks again.
- Everywhere else, every vanished site is let through the same way and is a
  NOTE, `--strict` included.

An accepted entry that names a vanished site still takes the match, by
entry order (16.5), so it neither fails nor turns into an entry that matched
nothing. Nothing klin prints asks for one.

A deletion that removed the subject too is a NOTE: for a file, the subject
file went in the same window, and for a function, the file that held it
went. The subject of a test file is
the file in `before` whose path equals the test's path with the test
affixes stripped: a `test_` or `spec_` prefix, a `_test`, `_spec`, `.test`
or `.spec` suffix before the extension, a `Test` or `Tests` suffix on the
basename as in `FooTest.java` or `FooTests.swift`, and a `tests/`, `test/`,
`spec/` or `__tests__/` directory segment. The table is fixed in the binary
and printed with the NOTE.


`lockfile` proves one thing: every dependency the manifest names has an entry
in the lockfile beside it, no pin the base held is gone, and the lockfile
records the version of each exact pin. It cannot prove
that a package exists in a registry, because it runs offline. A dependency
that does not exist fails the project's own install, when the `build` step
runs one. A derived build runs no install, so a declared dependency that was
never installed leaves the build's tool absent, and 9.3 makes that a NOTE
that lets this gate speak. Workspace members, path dependencies and optional dependencies are
implementation-defined and MUST be documented per manifest format, which
8.2.1 does for the five formats that ship.

The first version covers `Cargo.toml` against `Cargo.lock`, `package.json`
against `package-lock.json`, `pnpm-lock.yaml` and `yarn.lock` against
`package.json`, and `go.mod` against `go.sum`. The pnpm reader recognizes a
`lockfileVersion` major of 4 or later and reads package names from the direct
keys of its `packages` mapping. It accepts the path-shaped keys of older
lockfiles and the `name@version` keys of newer ones, including peer suffixes.
The Yarn v1 reader recognizes the `# yarn lockfile v1` marker and reads names
from its top-level, comma-separated selectors. The Yarn 2+ reader recognizes
the top-level `__metadata.version` value of 4 or later and reads names from
its top-level locators. These readers scan only the package-key shapes they
need and do not parse YAML values. A pnpm or Yarn file without its recognized
marker, section or key shape is one NOTE per run and no finding. `poetry.lock`
and `uv.lock` need a TOML reader and remain follow-ups. A manifest whose
lockfile format klin cannot read is one NOTE per run and no finding, so such a
manifest never reads as a pass.

Two more checks belong to this tier by the criteria and are not in the core
list of section 18, because each takes weeks and carries an unsolved problem:

| Check | Agent failure it names | Identity | Judgement | Status |
|---|---|---|---|---|
| `duplication` | copy instead of reuse | block in changed lines, and tree share | `share` rises | #48 |
| `sarif` with `compare` | a linter's findings compared across two trees | file + rule + message | `count` rises | #47, section 8.3 |

`stubs` is new. Its patterns per language are a fixed table the way escapes
are, and every pattern in the first table is an executable body that does
nothing: `todo!()`, `unimplemented!()`, `pass` as the sole body of a
function, `raise NotImplementedError`, `throw new Error("not implemented")`,
an empty body on a function the language's test convention marks, and an
elision comment such as `// ...` or `# rest of the code` as a sole body. The
sole-body patterns match function bodies only, because `pass` as the body of
a class or an exception is ordinary Python. The comment markers `TODO`,
`FIXME`, `XXX` and `HACK` are in the table too, as #106 decided. A project
that tracks work in such comments sees a new one fail once, and a person
accepts it or the agent moves the note to the tracker. An abstract
declaration whose body is meant to be empty, such as a trait method or a
protocol, MUST NOT match. Identity is file plus line text, ratcheted on
`count`, exactly like escapes, except for a comment marker. A comment is not
a declaration, so every comment marker in one file is one site, keyed by the
file and the row kind and ratcheted on its count (4.4, 8.2.1, ADR 0064). It
SHOULD share the escapes engine and differ only in the table. #106 shipped
the line patterns and #114 the body shapes, which the function walk reads.

The escapes table gains three rows for test-disabling constructs it lacks:
`fit(`, `fdescribe(` and `pytest.mark.xfail`. `skipif` is not a row, because a conditional skip states which platforms a test supports. The
other focus and skip markers, `.only`, `.skip`, `xit`, `#[ignore]`,
`@Disabled`, `t.Skip` and `XCTSkip`, are already there.

For the same reason a Rust `#[cfg_attr(P, ignore)]` or
`#[cfg_attr(P, ignore = "...")]` is a skipped test only where `P` is always
true under Rust's cfg rules, such as `all()`, `not(any())` or a nest of
these. Such a test never runs, as under a bare `#[ignore]`. A configuration
option, such as `windows` or `feature = "slow"`, may hold on one target and
not on another. So `any(windows, not(any()))` always holds and is a site, and
`all(windows, not(any()))` states where the test runs and is none. A
predicate that never holds, such as `any()` or `false`, skips nothing, so it
is no site either. `true` always holds, and so does `test`, because a test
runs only where `cfg(test)` is set. `ignore` counts at any place after the predicate, and inside a nested
`cfg_attr` whose predicate always holds too. The pattern finds `#[ignore` and
`#[cfg_attr` with any whitespace or comment between their tokens, and the
Rust grammar then reads the `cfg_attr` it found, so whitespace and comments
inside the attribute change nothing.

#### 8.2.1 Measurement rules of the shipped checks

The table above names what each shipped check measures. This section states
how, so a second implementation reproduces klin's own numbers and so a rule
that looks wrong is disputed as a rule, not rediscovered in source. Every
rule here is pinned by a CLI test under `tests/`, named beside it. Section 5.8
owns the compact policy keys and built-in language coverage, and links here.

**`doc-size` counts words.** A word is a maximal run of characters that are
not Unicode whitespace, as `White_Space` defines it. A no-break space and an
em space split words the way an ASCII space does. Nothing is stripped: a
heading marker, a fence, a backticked span, an emphasis marker and every
token inside a code block are words. A byte sequence that is not valid UTF-8
is decoded lossily, and each replacement character is part of a word, never
an error. The count compares to the ceiling with `words > ceiling` failing,
so a document exactly at its ceiling passes, and lands inside the two
percent margin that adds a WARN line. Pinned by
`a_word_is_a_run_of_non_whitespace_so_markup_counts_and_unicode_spaces_split`
and `a_byte_that_is_not_utf8_is_read_as_one_word_not_an_error` in
`tests/doc_size.rs`. Known limit: the count rewards terse markup and
punishes fenced examples equally. That is a design choice for a separate
ticket, not a defect of the rule.

**`escapes` and `stubs` aggregate matches into sites.** A site is one file
plus the text of one line with leading and trailing whitespace trimmed, except
for a `stubs` comment marker, whose site is keyed by its file and kind. Every
match of every pattern in the language's table, on every line whose trimmed
text is equal, lands on that one site. Its `count` is the number of those
matches. Its label and remedy are the ones of the first pattern, in table
order, that matched a line with that text, and its line is the first line
that pattern matched. The built-in language table comes first. A line that
carries two kinds is one site labelled by
the earlier row, and a second copy of that line, indented differently, adds
its matches to the same site rather than opening another. `escapes` reads
the text as written, so a pattern inside a string literal is a match. Unless
`skip_test_idioms` is `false`, it leaves each language's test idioms out of
that language's test code and counts them on the coverage line as skipped in
tests. Each language's table names its test idioms and its test code, and a
language that names none has every row judged in a test. The Rust test idioms
are `unwrap` and `expect`, and Rust test code is an inline `#[cfg(test)]`
module or a `.rs` file under a test root the survey finds in the tree being
read (5.4). The TypeScript and JavaScript test idiom is `@ts-expect-error`, a
row of its own, and their test code is a test file of 5.4: one under a test
root, or one a test directory segment or a test affix marks. It needs no
description, the same way a reason on `#[ignore]` silences nothing. Each tree
is classified over its own files, so a root that stops being test-only has its
production sites judged. Every other row is judged in a test as anywhere else,
so a `#[ignore]`, an `#[allow(...)]`, an `unsafe { }`, a `@ts-ignore`, a
`@ts-nocheck` or a non-null `!` inside a test is a site, and a
`@ts-expect-error` in production code is a site (ADR 0049, ADR 0060). Pinned
by `a_site_inside_a_cfg_test_module_is_not_a_production_site`,
`a_ts_expect_error_in_a_typescript_test_file_is_left_out_by_default`,
`every_other_typescript_escape_is_still_judged_in_a_test_file`,
`skip_test_idioms_turned_off_judges_the_idioms_of_every_language`,
`skip_test_idioms_turned_off_judges_the_test_module_too`,
`unwrap_and_expect_in_a_file_under_a_test_root_are_left_out_by_default`,
`a_skipped_test_under_a_test_root_is_still_an_escape`,
`a_skipped_test_inside_an_inline_test_module_is_still_an_escape`,
`allow_and_unsafe_in_rust_tests_remain_escapes`,
`skip_test_idioms_turned_off_judges_a_file_under_a_test_root_too` and
`production_rust_beside_a_test_root_is_judged_as_before` and
`a_root_that_stops_being_test_only_has_its_new_production_unwrap_judged` in
`tests/escapes.rs`.
A Rust `cfg_attr` that carries `ignore` is a skipped test only where its
predicate always holds, as 8.2 states. Pinned by
`a_cfg_attr_whose_predicate_always_holds_is_a_skipped_test`,
`a_skipped_test_is_found_through_whitespace_comments_and_nesting`,
`a_cfg_attr_on_test_or_a_true_literal_is_a_skipped_test` and
`a_cfg_attr_whose_predicate_may_not_hold_is_no_skipped_test` in
`tests/escapes.rs`.
`stubs` throws away a match that lies wholly inside a quoted span on one
line, judges a test module like any other code, and refuses the key. Pinned by
`repeated_lines_of_two_kinds_fail_as_one_site_labelled_by_the_first_pattern_with_every_match_counted`,
`a_line_carrying_two_escape_kinds_counts_both_under_the_first` and
`the_same_line_twice_in_one_file_is_one_site_whose_count_ratchets` in
`tests/escapes.rs`. Known limit: the label hides the second kind on a mixed
line. A finding that says `unwrap x4` may hold two `expect` calls.

`stubs` keys a comment marker by its file and the row kind,
`comment marker`, in place of a line text (ADR 0064). Every marker match in
one file lands on that one site, its `count` is the number of matches, and
an accepted entry names `comment marker` as its `text`. So a typo fix inside
a marker, a change from `TODO` to `FIXME` and a move within the file hold
the count, and a new marker or one moved in from another file raises it. The
gate pairs each marker match with one base match of the same trimmed text in
the same file, and the matches left over are the lines the base file lacks.
The site's line is the first of them, and its `new_lines` value lists them
all as one string, such as `"1, 3"`, so a failure names each one. The first
named line may be an edited marker and not the new one. A line that holds a
marker and a code stub is two sites, and a code stub keeps its line text, so
an edited `todo!()` line is a new site. An accepted entry written before
ADR 0064 for a line that holds a marker and a code stub or a body shape keys
the site of that code stub or body shape. Where the base holds that site,
the base entry shares the exact count and takes the match (4.4), so the
accepted entry matches nothing: a NOTE, and a failure under `--strict`. The
marker is then held at the base. Where the base does not hold that site, the
accepted entry holds it, `--strict` does not name it, and the marker fails
as new. The pairing is `n log n` in the marks of a file, the line count is
linear in the file for each pattern, and the quoted-span lookup is a binary
search. Known limit: a reworded marker, or a marker deleted while another is
added in the same file, holds the count.
Pinned by `a_typo_fix_inside_an_existing_marker_is_held`,
`a_new_marker_in_a_file_that_holds_one_raises_its_count_and_names_the_new_line`,
`every_marker_line_the_base_file_lacks_is_named`,
`an_edited_not_implemented_line_is_a_new_site`,
`a_marker_moved_within_a_file_is_held_and_one_moved_to_another_file_is_new_there`,
`a_marker_and_a_body_shape_on_one_declaration_line_are_two_sites`,
`an_accepted_marker_entry_names_the_row_and_holds_at_its_count`,
`an_accepted_entry_for_a_line_that_held_a_marker_and_a_code_stub_holds_the_code_stub`,
`an_accepted_entry_for_a_mixed_line_the_base_holds_is_stale_and_the_base_holds_both_sites`
and `a_file_of_two_hundred_thousand_distinct_markers_is_judged_in_seconds`
in `tests/stubs.rs`.

**`stubs` judges three body shapes.** The function walk of `complexity`
reads them, over every grammar the built-in stubs table supports, and
`stubs` records each one at the declaration line of the function that holds
it, as one more match on the site of that line. A shape is read off a body that
holds a run of statements, so a concise arrow body such as `() => value` is
one expression and does the work of one. A function whose body holds one
`pass` statement is a `pass body`. A function whose body holds no statement,
and one comment that starts with `...` or with `rest of the`
case-insensitively once the comment markers are off it, is an `elided body`.
A function whose body holds no statement, and whose declaration the
language's test convention marks, is an `empty test`. A comment is not a
statement in any of the three.

A body meant to be empty is not a shape. A declaration that carries no body,
such as a trait method without a default or an interface method, has none. A
Python declaration under a decorator that names `abstractmethod`,
`abstractproperty` or `overload` has none, and neither does one in a class
whose bases name `Protocol` or `ABC`. Both are read as whole names, the last
segment of a dotted one, so `StoreABC` is not `ABC` and a route argument
that spells `overload` exempts nothing. `pass` in a class body, an exception
class included, is not a function body and never matches. A function that
starts on the declaration line of a function that holds it is not judged
either, because the line it would be reported at is not its own: a callback
written inside the call that declares a test carries the test's line and not
its shape. Pinned by
`a_pass_body_fails_and_the_same_declaration_with_a_body_stays_green`,
`an_elided_body_fails_and_a_comment_that_elides_nothing_stays_green`,
`an_empty_test_body_fails_and_a_test_rewritten_with_the_same_declaration_stays_green`,
`an_empty_body_under_a_multi_line_tokio_test_is_an_empty_test`,
`pass_on_an_exception_class_and_on_an_abstract_declaration_is_not_a_stub`,
`a_callback_on_the_line_of_a_test_declaration_is_not_an_empty_test` and
`a_decorator_or_a_base_whose_text_only_spells_a_marker_does_not_hide_a_pass_body`
in `tests/stubs.rs`. Known limit: another shape that stands in for work,
such as `return null` or `{}` on a function no test convention marks, is not
judged, and adding one is a spec change with its own legitimate-use fixture.
Second known limit: a text the grammar rejects keeps its line patterns and
loses its shapes, and the run says nothing about the loss, so a file that
does not parse can only under-report. A parser resource error is not a
grammar rejection: `stubs` MUST propagate the named error of section 13,
including in a direct `stubs` command or a run selecting only that gate.
Pinned by `an_oversized_source_preserves_the_named_resource_error_in_stubs`
in `tests/stubs.rs`.

**`dead-symbols` judges private declarations.** The structural index supplies
module-level functions, methods, types, constants and variables from Rust and
TypeScript, with TSX treated as TypeScript. A declaration is dead when no
reference with the same name exists outside its own declaration. A
TypeScript destructuring declaration is judged by the names it binds: it is
dead only when none of them has a reference outside the declaration. A
renamed binding such as `{ add: loaded }` is judged by its local name, and a
nested or defaulted pattern binds every name inside it, never a default value
or a computed key. A name a `const`, `let` or `var` destructuring declaration
binds is no reference to that name, at the top level and in a function body,
so such a destructuring keeps no other declaration of a name it binds alive.
The head of a C-style `for` is a declaration, so `for (let [first] = [0]; ; )`
binds `first` as a `let` statement does. A name a parameter, a `for…in` or
`for…of` head, a `catch` clause or a Rust `let` binds, or an assignment such
as `[first] = load()` writes, still reads as a reference, unless a pattern
writes it as a shorthand such as `{ name }`, which never reads as one. A
field name reads as a reference too (ADR 0035). A name
resolves to every same-name declaration, so ambiguity keeps each declaration
alive. Declarations marked externally visible, Rust `main`, and functions the
shared test convention recognizes are not judged. The `ignore` list adds name
globs. A glob is matched against each name a declaration binds, so a
destructuring declaration is left out only when every name it binds matches
one. The check is name-only: it does not resolve imports, types, reflection,
framework entry points or external callers. The index reads a string as text,
with two exceptions. Where a Rust attribute item holds `serde(...)`, as the
attribute itself or directly inside `cfg_attr`, the string value of `default`,
`skip_serializing_if`, `serialize_with`, `deserialize_with` or `getter`, plain
or raw, is the path of a function the derive calls. The string is read by its
value, every escape decoded, and a line continuation drops its newline and the
whitespace after it. A string with an escape klin cannot decode names nothing.
The Rust grammar reads the decoded value as one path expression, and the name
that path ends in is a reference, and no other segment is. So generic
arguments, a qualified-self prefix such as `<T as Trait>` and a const-generic
block, with any literal or comment inside it, never change which name that
is, and `Accessor::<u8>::get` references `get` alone. A value the grammar does
not read as exactly one path names nothing. It
is an ordinary reference, so a changed run widens on it and `reachability`
counts it. The same `serde` tokens inside a macro call are not an attribute,
so no path they spell is a reference. `with` names a
module, which no one reference stands for, so its string stays text, as do a
`default` with no value and the string of any other key, such as `rename`.
Inside a string literal that a Rust macro call receives or a `macro_rules!`
body holds, plain or raw and read by the same decoded value, the name of each
`{name}` or `{name:spec}` capture is a reference, and so is a width or a
precision the spec names, such as `WIDTH` and `PREC` in `{v:>WIDTH$.PREC$}`.
So a name only `format!("{name}")` or `format!(r#"{name}"#)` uses is alive. `{{` is a brace, and a position such as `{0}` or `{}` names
nothing. The macro is not resolved, so a string any macro receives is read
this way, and under the name-only rule an extra reference can only make a
declaration look used. A
declaration that becomes dead
after being referenced at the base is `worsened`; a dead declaration already
held at the base is one NOTE and never fails. When it can, a worsened finding
names the first base file that held a lost reference. `--report` prints the
complete current dead-symbol list. A changed run that is not strict builds
declaration state only for the files in its effective judgement scope, over
both trees and under the base's own path and rename semantics (6.5). That
scope is the run's physical scope plus the files that declare a name a changed
file references on one semantic side and not the other. A declaration that did not
move can still turn from referenced to dead when its last caller changed, so
the physical scope alone is not the semantic impact scope. A name a changed file references on both sides cannot flip
one, so it widens nothing. The names come from the same structural facts the
index is built from, never from a textual diff,
and a name with several declarations widens to all of them, which keeps the
ambiguity rule of ADR 0035: fail less, never more. A changed file whose
working-tree text the grammar could not read widens nothing from its base
reference names: that hole is reported as a hole, and missing evidence never
becomes proven deadness. One effective scope drives the state of both trees,
the ratchet, the base and accepted matching, `lost_reference`, the notes, the
counts and the coverage line, so no run reports a finding at a site it says it
did not judge. `Context.only` keeps its runner meaning: the check derives this
scope locally. Its evidence stays whole: both
trees keep the complete index of 8.4, so a judged declaration is alive on a
reference from any measured file, changed or not, and a lost reference in an
unchanged file still explains a worsened finding. A whole run, a strict run
and the check by hand build state for every eligible declaration. Pinned by
`a_new_private_unreferenced_rust_function_fails_as_new`,
`a_private_typescript_main_is_judged`,
`losing_the_last_reference_is_worsened_and_names_the_old_reference_file` and
`one_typescript_reference_keeps_duplicate_names_alive` in
`tests/dead_symbols.rs`; the destructuring rule by
`an_object_destructuring_declaration_whose_binding_is_used_passes`,
`an_array_destructuring_declaration_whose_binding_is_used_passes`,
`a_renamed_binding_used_by_its_local_name_passes`,
`a_nested_or_defaulted_binding_keeps_its_declaration_alive`,
`a_destructuring_declaration_whose_bindings_are_all_unused_still_fails`,
`a_destructuring_declaration_with_one_used_binding_passes`,
`a_property_key_a_computed_key_and_a_default_value_bind_nothing`,
`two_unused_destructurings_of_one_name_do_not_keep_each_other_alive`,
`a_destructuring_inside_a_function_keeps_no_declaration_of_its_names_alive`,
`ignore_globs_match_every_name_a_destructuring_binds`,
`a_changed_caller_that_drops_the_last_binding_reference_worsens_the_destructuring`,
which also requires a second run over the structural cache to print the same,
and `a_destructuring_that_loses_a_reference_in_two_callers_names_the_first_base_file`;
the `serde` strings by
`a_private_function_only_a_serde_default_names_passes`,
`a_private_function_only_a_serde_skip_serializing_if_names_passes`,
`a_private_function_no_serde_key_names_still_fails`,
`a_serde_attribute_inside_cfg_attr_names_its_function_too`,
`a_serde_path_references_only_its_last_segment`,
`a_serde_with_module_names_no_function`,
`serde_tokens_inside_a_macro_call_name_no_function`,
`a_raw_string_serde_path_names_its_function`,
`a_generic_qualified_serde_path_references_its_terminal_callable`,
`a_qualified_self_serde_path_references_its_terminal_callable`,
`an_escaped_serde_string_is_read_by_its_value`,
`a_const_generic_block_in_a_serde_path_keeps_its_terminal_callable`,
`a_brace_in_a_char_literal_of_a_const_generic_block_keeps_the_terminal_callable`,
`a_brace_in_a_string_or_a_comment_of_a_const_generic_block_keeps_the_terminal_callable`,
`a_continued_serde_string_is_read_without_the_whitespace_after_the_newline` and
`removing_a_serde_attribute_in_a_changed_file_worsens_an_unchanged_helper`,
with `a_member_a_serde_string_names_is_reached` in `tests/reachability.rs`;
the format captures by `a_private_const_only_a_format_capture_uses_passes`,
`a_private_const_only_a_raw_format_capture_uses_passes` and
`a_private_const_only_a_width_or_a_macro_rules_capture_uses_passes`; the test
convention by
`a_tokio_test_passes_inline_and_under_a_test_directory` and
`a_multi_line_test_attribute_marks_its_function`; the report cap
is covered by `report_lists_every_current_dead_symbol_without_the_note_cap`,
and the
judgement scope by
`a_changed_run_builds_no_state_for_the_declarations_it_does_not_judge` and
`unrelated_historical_debt_outside_the_changed_scope_stays_silent`, and the
semantic impact scope by
`removing_the_last_reference_in_a_changed_caller_worsens_an_unchanged_declaration`,
`a_reference_another_unchanged_caller_still_holds_is_no_regression`,
`deleting_the_only_caller_worsens_the_unchanged_declaration`,
`every_declaration_of_an_affected_name_stays_conservatively_in_scope`,
`a_changed_file_that_keeps_its_reference_names_widens_nothing` and
`a_changed_caller_the_grammar_cannot_read_guesses_no_deadness`.

In a changed run that is not strict, which includes the hook, the two trees
`dead-symbols` and `reachability` compare share one base extraction. The run's
change set (4.5) is the authority: a working-tree file it does not name, and
that the base lists under the same name, holds the base's bytes at the same
path. The working tree takes the base extraction's outcome for that file and
does not extract it again. An added, modified or renamed file, and a file the
base does not list under the same name, such as one renamed by case alone
where git does not see the rename, is extracted from the working tree. The
base is laid out whole with each renamed file at its current path, so its old
bytes are read under the grammar of that path, and an extension-changing
rename such as `.ts` to `.tsx` reads them as TSX. The TypeScript rule for a
declaration file (8.2) reads that path too, so a rename such as `.d.ts` to
`.ts` reads the old bytes as a file that is no declaration file, pinned by
`a_declaration_file_renamed_to_a_module_is_read_at_the_base_under_its_new_name`.
Each tree still selects its
own files under its own scope and keeps the facts it selected; a
name-resolving check builds its own index lazily from those facts, and no parse
tree outlives its extraction. A strict run, a run that
is not changed, and the check by hand extract both trees. Known limit: a
change git does not report, such as an edit to a file marked `assume-unchanged`
or `skip-worktree` or bytes a clean filter hides, reads as the base's bytes in
a changed run. That run's scope already leaves the file's own findings out,
and the names the file declares and references are the base's.

The same runs keep the base's structural outcomes between runs in the
structural cache, one file per base commit under `cache/structural/<commit>`
in the state directory (7.4). The file name is the full object id of the
commit the run compares against, so a branch name never keys it, and a red
turn reads the cache of its own stamp. For each path of the base tree that
the run's change set does not name, the file holds the outcome the base
extraction came to: facts without a parse tree, unsupported, unparsed or
foreign. A later run over the same commit takes those outcomes in place of
reading, parsing and extracting the base's copy of each file. It still
extracts every path its own change set names, at the path and under the
grammar the base tree gives it. A run that extracted an outcome the file did
not hold writes the whole file again through the atomic replacement of ADR
0041, and that write keeps the four newest files of the structural cache and
removes older ones, so an evicted commit costs a later run one extraction.
Each file carries an identity: an explicit schema epoch, the binary version,
a checksum of the extraction sources and `Cargo.lock` the binary was built
from, the commit, and a checksum of the configuration's root, git's
configuration and the attribute files git reads outside the tree, which decide
the bytes a checkout of the commit writes. A file that is missing,
carries another identity, fails its body checksum or does not decode to the
end reads as no file, and the run extracts the base as it would without one.
The cache changes what a run costs and never what it reports: the findings,
notes, coverage and exit codes are the ones a run without it gives. Strict
runs, whole runs and the check by hand neither read nor write it, and `klin
cache clean` removes it. Known limit: the identity does not name the
system-wide attributes file or the behavior of a filter program, so a change
to either between two runs over one commit, with git's configuration
unchanged, reads the outcomes the earlier checkout gave.
`tests/structural_cache.rs` pins the repeated run, a damaged file, a file
copied from another commit, a smudge filter added between two runs, the
four-file bound, and repeated red stops through a prompt and a branch switch. `tests/structural_views.rs` repeats each changed
caller over the cache and requires the same output.

A run that reads that cache lays the whole base out without checking the base
commit out. It registers the base commit as a linked worktree with no file on
disk, reads the commit into that worktree's index, and writes from the index
only the base files a gate may read: every file the index holds, less the
source files whose facts the cache already holds. The base tree's file list is
the index's, under the rules of 4.3, so no walk and no ignored-path discovery
runs in the base; a fresh checkout holds tracked files only, so the base's
ignored set is empty either way. A submodule entry is no file, as an
uninitialized submodule is an empty directory. A symbolic-link entry is no
file where git writes a link, and is a file where `core.symlinks` is false and
git writes the target path as a plain file. Every file the layout writes is
written before any rename moves it, so git converts each one under the base
commit's own attribute topology, which is the topology a checkout of the
commit reads. Renames then move files to today's paths, as they do for a base
checked out whole.

The layout changes what a run costs and never what it reports. A strict run, a
run that is not changed, and the check by hand check the base out whole. So
does a run with no state directory, with no cache to name, with a cache it
cannot read, whose index holds a sparse checkout or any other shape this
layout does not read, or whose git commands refused. klin asks git itself what
`core.sparseCheckout` and `core.symlinks` are worth, so every spelling git
reads as a boolean reads the same way here, and a value git will not read as a
boolean sends the run to the checkout rather than to a default. A layout that
could not be completed removes its worktree before the run checks the base out
on the same directory, so no half-laid base reaches a gate. `tests/structural_cache.rs`
and `tests/structural_views.rs` compare each warm run with the same run over a
removed cache and require the same findings, notes, coverage and exit code.

The shared structural view keeps imports and module declarations alongside
declarations and references, and keeps unparsed and unsupported outcomes as
coverage data. The project's Change data remains separate from the structural
scope, so a consumer can reuse facts without losing which paths changed.
`layering` and `public-api` consume those facts directly through the
module-resolution layer; neither builds a `SourceIndex`, because they resolve
modules and exported paths rather than declaration and reference names. `dead-symbols` and `reachability` request
their own name index only when they judge names, so each measurement builds at
most one index and the structural cache remains a cache of facts only. A
measurement holds its facts sorted by path, the one order its index also reads
them in, so the two views of a measurement never disagree on order.
`tests/structural_views.rs` requires a cached base of imports, module
declarations and an unparsed file to be read and parsed only for the changed
file, and the cache round-trip unit test pins the import and module fields.

The findings, notes, coverage and exit codes of a changed run are the ones
two independent extractions give. `tests/structural_views.rs` pins the
gate, changed and hook callers for edits, additions, deletions, both rename
classes, scope movement, name ambiguity, a lost reference, an unsupported
language, unparsed files and a case-only rename git does not see. With
`KLIN_DIFF_BIN` naming an earlier build, each of those tests also requires
that build to print the same normalized output for the gate, strict,
changed, hand `--report` and hook callers over the same trees.

**`reachability` judges files of a derived family.** The derivation commit
proves each family from a directory, concrete extension and basename pattern;
the report exposes its name, roots and pattern, while configuration may only
narrow all derived families with `in` and `except`. A member is reached when another
file holds a reference with the name of one of its eligible declarations:
functions, types, constants and module-level variables that are not entry
points. A TypeScript destructuring declaration in another file that binds
that name reaches the member too, in whatever form it binds it, shorthand
included, so `const { default: Profile } = await import(…)` reaches the file
it loads while `Profile` is still unused. A named TypeScript re-export in
another file, `export { x } from "./m"` or `export { x as y } from "./m"`,
is a reference to `x` from the file that holds it, under the same name-only
rule, so a name several files declare still reaches each of them. A re-export
names a declaration by the name the declaration exports it under: `default`
for `export default function Profile` or `export default class Profile`, and
its own name otherwise. So `export { default } from "./m"` and
`export { default as Profile } from "./m"` name that declaration, and
`export { Profile } from "./m"` does not. A re-export proves a member only
where one declaration under the index answers to that name, so a `default`
re-export proves nothing while several files export a default, and proves the
only default of the index wherever the re-export points. The rule
reads the export facts the structural extraction already holds and resolves
no module. A star re-export, `export * from "./m"` or
`export * as ns from "./m"`, names nothing and reaches no member (ADR 0061).
`dead-symbols` reads no re-export as a reference, because it judges private
declarations, which no re-export can name. Pinned by
`a_member_only_a_named_re_export_in_another_file_names_is_reached`,
`named_re_exports_in_another_file_prove_a_family`,
`a_default_member_a_bare_default_re_export_names_is_reached`,
`a_default_member_a_renamed_default_re_export_names_is_reached`,
`a_default_member_a_re_export_of_its_own_name_leaves_unreached`,
`a_re_export_of_the_only_default_proves_its_member`,
`a_default_re_export_proves_no_member_while_several_files_export_a_default` and
`a_member_only_a_star_re_export_names_stays_unreached` in
`tests/reachability.rs`, and by
`a_re_export_of_its_name_in_another_file_keeps_no_private_declaration_alive`
in `tests/dead_symbols.rs`. Methods are not eligible, because a
name such as `run` or `get` recurs across unrelated types and the name-only
rule would reach every file that declares one. Exported declarations are
eligible, unlike in `dead-symbols`, because a family says its files are
wired inside this repository. A reference from the file itself reaches
nothing. Resolution is
the structural index's name-only rule, so a name several files declare
reaches every one of them: ambiguity makes a file look reached and never
unreached. The index covers the whole tree in each family's language partition, so
every run resolves against unchanged callers. The check takes no
scope: a changed run judges every member of both trees, because the edit that
strands a member is an edit to its caller and not to the member, so a run
narrowed to the changed files would judge no member at all and report every
site as held at the base. A derived family is small by construction, and the
index is built over the whole partition either way, so the whole judgement
costs a lookup per member, and the check reads no second extraction of its
own. The physical changed-file set is therefore not this check's judgement
boundary, and the wider boundary raises no old debt: a member unreached in
both trees stays one NOTE under the ordinary two-tree ratchet, and never
fails because a run re-judged it. Identity is the
repository-relative path, so a file two families match is judged once,
under the first family in the list, and an accepted entry names the path. A
file under a test directory, or one a test affix marks, is no member in either
tree a run judges (5.4), so a test written beside a family's files is judged
by no family. A
measured member with no eligible declaration is measured and not judged,
and is neither unreached nor unsupported. A file that leaves the tree is
`inventory`'s and no finding here. A new unreached member fails as new, a
member that loses its last external reference is `worsened`, and one
unreached in both trees is one NOTE. The remedy names the first proven
reached sibling of the family in path order, and none when every sibling is
unreached or reached only through a shared name. The remedy also names the
conflict with a public-api break over what an unreached member held, on
every unreached finding because a check reads no other gate's findings, and
keeps the two decisions apart. The task decides the public contract: it is
restored where the task keeps it, and the break is left for a person to
accept where the task removes it. Whether the implementation is still needed
decides the file: it is kept and wired in, or deleted only if unused. A
public-api identity names no file, and an unreached member may still be live
through a caller this check does not resolve, so neither decision implies the
other. Pinned by
`an_unreached_file_that_held_a_public_api_break_names_the_conflict_and_not_a_bare_delete`.
The check does not resolve imports, `mod foo;`, side-effect imports,
star re-exports, string registries,
dependency injection, framework discovery by name or attribute, macro or
build-generated callers, or callers outside the tree, which belong to the
module graph of `layering` or to no V1 check; a family wired that way is narrowed by path or accepted by a
person. Two files that reference only each other read as reached. Pinned
by `a_new_command_file_nothing_references_fails_as_new`,
`losing_the_last_external_reference_is_worsened`,
`one_ambiguous_reference_reaches_every_file_that_declares_the_name`,
`a_file_with_only_entry_points_or_methods_is_measured_and_not_judged`,
`the_remedy_names_a_proven_sibling_and_not_one_reached_by_ambiguity`,
`a_family_the_base_proves_is_derived_and_judges_a_new_working_tree_member`,
`a_changed_run_judges_a_member_a_dispatch_edit_stopped_referencing`,
`the_stop_hook_blocks_a_turn_that_left_a_member_unreached`,
`a_changed_run_reports_one_surface_the_whole_run_reports_too`,
`legacy_unreached_debt_stays_a_note_in_a_turn_that_edits_another_file`,
`a_new_test_file_in_a_family_directory_is_no_member`,
`a_test_directory_under_a_family_root_stays_in_the_cohort_it_must_prove`,
`a_destructuring_in_another_file_that_binds_a_member_name_reaches_it` and
`a_plain_declaration_elsewhere_or_a_destructuring_in_the_same_file_reaches_no_member`
in `tests/reachability.rs`, and by
`a_caller_only_turn_judges_the_whole_family_off_the_shared_extraction` in
`tests/structural.rs`. Known limit: a destructuring inside a function body is
no declaration, so a name only it binds reaches no member. That is right for
`function boot() { const [Login] = list; }`, whose binding names a local
value, and wrong for an unused `const { default: Profile } = await import(…)`
inside a function, which leaves the file the import loads unreached. Second
known limit: a member's own destructuring declaration, such as
`export const { Profile } = factory;`, is judged by its pattern text, which
no reference spells, so a file that uses `Profile` does not reach it. Third
known limit: a declaration that a later statement exports as the default,
such as `function Reset() {}` with `export default Reset;` or
`export { Reset as default };`, keeps its own name, so a `default` re-export
does not reach it and `export { Reset } from "./m"` does.

**`doc-citations` reads backticked paths, not Markdown links.** On each line,
backticks pair from the left, and an unpaired trailing backtick opens
nothing. A span is a citation when, after trimming and dropping everything
from the first colon on, it holds no space and no `*`, it ends with one of the
built-in source, document and manifest extensions the reference prints, and it
holds a `/` or a `.`. So `` `src/a.rs:12` `` cites
`src/a.rs`, and `` `*.rs` ``, `` `a b.rs` `` and `[a](src/a.rs)` cite
nothing. A run's root is the tree root, and `--root` by hand names others.
Any citation resolves when a root holds a file at that path. A path with
a `/` resolves nowhere else. A bare filename no root holds directly
resolves when exactly one file under the roots has that basename, is
ambiguous when several do, and resolves nowhere when none does. Identity is
the document plus the cited path after trimming and the colon strip, so the same stale string on two lines is one site with `count` 2, and the
same string moved to another line is held. Pinned by
`a_span_that_is_not_a_path_is_not_read`,
`a_wildcard_a_link_and_a_line_suffix_are_read_as_the_syntax_says`,
`a_bare_filename_resolves_when_exactly_one_file_under_the_roots_has_that_name`,
`two_candidates_is_ambiguity_reported_with_both`,
`a_stale_citation_moved_to_another_line_is_held` and
`the_same_stale_string_cited_once_more_is_worsened_with_the_count` in
`tests/doc_citations.rs`. Known limit: a citation split across two lines, a
path in a Markdown link, and a path with a space are never read.

The check takes no scope: a changed run judges every derived root document of
both trees, because the edit that breaks a citation is a move, a rename or a
new basename clash in the source, and not an edit to the document that cites
it, so a run narrowed to the changed files would judge no document at all.
The derived root-document set is bounded by 5.2, and each document already
resolves against whole-tree path facts, so the whole judgement costs one
resolution per citation. The physical changed-file set is therefore not this
check's judgement boundary, and the wider boundary raises no old debt: a
citation broken in both trees stays held under the ordinary two-tree ratchet,
and never fails because a run re-judged it. One root-document set drives the
findings, the base and accepted matching, the coverage counts and the
moved-target remedy alike, so a changed run never reports a document that its
coverage says it did not measure. `--file` and `--root` by hand are
unchanged: a person naming a document still judges that document only. Pinned
by `under_changed_a_move_breaks_the_citation_of_a_document_the_window_did_not_touch`,
`the_stop_hook_blocks_on_a_move_that_breaks_an_untouched_documents_citation`,
`under_changed_a_stale_citation_of_an_untouched_document_stays_held`,
`under_changed_a_new_basename_clash_makes_an_untouched_citation_ambiguous`,
`under_changed_removing_a_basename_clash_leaves_an_untouched_citation_silent`
and `file_and_root_by_hand_judge_that_document_against_the_base` in
`tests/doc_citations.rs`.

**`complexity` measures each function on its own.** The unit is a function
node of the language's grammar, and an accessor or initializer body counts
as a function of its own. Cyclomatic complexity starts at one and adds one
for each decision node and each boolean operator in the language's tables,
walked through the function's body. A nested function is not walked: it is
excluded from the count of the function around it and measured as a site of
its own, at its own declaration line. A TypeScript or JavaScript callback
passed directly as an argument to a suite container call in a test file is not
a function site. The calls are direct `describe`, `context`, `suite`,
`fdescribe` and `xdescribe` calls; direct `.only` and `.skip` calls on
`describe`, `context` and `suite`; and curried `.each` calls on those same
three containers, in the form `describe.each(data)(name, callback)`. A callback
may be wrapped in parentheses or a TypeScript `as`, `satisfies`, non-null or
type assertion. Calling the value returned by `describe(...)` or
`describe.only(...)` does not make that call a suite container. The suite
callback is omitted from both trees' judged function population and from the
derived `cc` and `lines` sample.
Functions inside it remain measured, including `it` and `test` callbacks,
hooks and helpers. The same callback in a file that is not a test file is
measured as before. A test file is one that
5.4 marks under a test root or by a test directory segment or test affix. The
`default` arm of a Java or Swift
`switch`, the `else` arm of a Kotlin `when`, and a single unguarded
catch-all arm of a `match` or `case` add nothing. `lines` is the
count of source lines from the first line of the declaration to the last
line of its body, both inclusive, so a one-line function is 1.

Test code is judged on `cc` as production code is, and on length only
against `test_lines`. Test code is every function in a test file of 5.4, one
under a test root or one a test directory segment or a test affix marks, and
every function inside a Rust item marked `#[cfg(test)]`, such as an inline
test module or a lone helper function, helpers and fixtures included. Each
tree is classified over its own files, and a file the change renamed is
classified at the base under the path the base holds it at, so a test moved
into production code is new production code there. Pinned by
`a_function_marked_cfg_test_outside_a_module_is_test_code` and
`a_test_file_renamed_into_production_code_is_judged_as_production_code`. With
`test_lines` pinned, a function in test code over it is a finding, and
`lines` judges only the rest. A finding in test code carries its length as
`test_lines`, and one in production code as `lines`. A finding that moves
between the two carries a value its base entry does not, so it is
`worsened` (7.1). That holds for any such move of a site over its `cc`
ceiling, whichever length ceiling is stricter and even when its length is
under both. Pinned by
`a_test_over_cc_moved_into_production_code_is_worsened_with_both_ceilings_pinned`,
`production_code_over_cc_moved_into_a_test_is_worsened_with_both_ceilings_pinned`
and
`a_function_over_cc_that_changes_class_is_worsened_under_both_length_ceilings`.
With no `test_lines`, no function in test code fails on length, its finding
carries no length, and the `OK:` line says how many test functions were not
judged on length and names each file that holds one and that the change added
or renamed, since nothing else checks how long those tests are. The change
set says which files those are, so a file a wider scope brings in is not
named. Pinned by
`a_file_a_wider_scope_brings_in_is_not_named_as_added`. A failure names the ceilings in force, with `test_lines`
only where one is pinned. Pinned by
`with_no_test_lines_a_test_function_past_the_lines_ceiling_does_not_fail`,
`with_no_test_lines_a_test_function_past_the_cc_ceiling_still_fails`,
`with_test_lines_pinned_a_test_function_past_it_fails`,
`with_only_lines_pinned_a_production_function_is_judged_and_a_test_function_is_not`
and
`with_no_test_lines_the_coverage_line_says_test_code_was_not_judged_on_length`
in `tests/complexity.rs`.

The hand-checked numbers per language are the contract, in
`*_functions_carry_their_hand_checked_numbers` for Rust, Python, TypeScript,
Go, Java, Ruby, Swift and Kotlin, with
`a_nested_function_is_measured_on_its_own_not_folded_into_the_one_around_it`,
`a_fall_through_arm_is_not_a_decision`,
`a_guarded_catch_all_arm_is_still_a_decision` and
`an_accessor_or_initializer_body_is_measured_like_any_other_function`,
`a_growing_suite_callback_in_a_test_file_is_not_a_complexity_finding`,
`a_long_test_callback_inside_a_suite_is_still_measured`,
`suite_callbacks_do_not_raise_the_derived_lines_ceiling` and
`a_suite_callback_in_a_production_file_is_still_measured` in
`tests/complexity.rs`. Known limit: which grammar nodes are decisions is per
language and fixed in the binary. Two languages that express one construct
differently may count it differently, and the fixtures are the record of
which choice was made.

**`inventory` ratchets the existence of two things.** A test file is the
repository path of a file the base commit's tree listing holds that 5.4 calls
a test, and its `missing` is 1 when the working tree holds no file there. A
test function is a site of ADR 0008 inside such a file, in both trees, read
off each tree's file list and kept only where the
language's test convention marks it: a `fn`, `def` or `func` declaration
whose name starts with `test_`, a `func Test` declaration, an `it(` or a
`test(` call at the start of the declaration line, an `@Test` annotation on
the declaration line or on the run of marker lines directly above it, and a
Rust attribute whose path ends in the segment `test`, with or without
arguments, such as `#[test]`, `#[tokio::test]`,
`#[tokio::test(flavor = "multi_thread")]` or `#[async_std::test]`, among the
attributes and comments directly above the function. The Rust grammar reads
that attribute, so an attribute over several lines, or with whitespace or a
comment between its tokens, marks the same test. `#[rstest]`, `#[test_case(...)]`
and any other attribute whose last segment is not `test` mark nothing. The
same convention decides which functions `dead-symbols` leaves unjudged and
which empty body `stubs` calls an `empty test`. A marker an identifier runs
into matches nothing, so `myfunc Test` is not a declaration, and a call marker counts at
the start of the line only, so `def helper(test_arg): return it(test_arg)`
is not a test. Where the grammar holds the annotation inside the function's
own node, as it does for Java, the declaration line is the first line of
that node that carries more than an attribute or an annotation, so the site
names the method and the body hash of 4.4 leaves the name out. Its
`missing` is 1 when no function in the working tree takes it, by site first
and then by body hash, one to one on each pass. `in` and `except` narrow
both identities the same way, and both trees are read under the scope the
base commit records, and under today's when the base records none (8.6), so a
narrowing lets a deletion through only once it is committed. A file the working tree's grammar refuses holds no function site,
so the functions in it are not judged and the file is the unparsed refusal
of ADR 0003. A parser resource error MUST propagate as the named error of
section 13, including its file, line, measured byte count and ceiling; it is
not a grammar rejection. Pinned by
`deleting_a_test_function_from_a_file_that_stays_blocks_the_stop_and_asks_why`,
`a_deleted_test_function_is_a_note_that_fails_nothing_outside_the_hook`,
`the_stop_after_the_question_passes_and_leaves_a_green_verdict`,
`a_prompt_between_two_stops_does_not_ask_about_the_same_test_again`,
`a_stop_whose_stamp_was_deleted_still_asks_about_a_deleted_test`,
`a_test_function_renamed_and_moved_with_its_body_unchanged_is_held`,
`a_function_whose_name_only_holds_a_marker_is_not_a_test_site`,
`a_test_name_with_no_attribute_above_it_is_a_test_site`,
`deleting_a_tokio_test_from_a_file_that_stays_is_a_vanished_test_site`,
`an_in_that_names_one_file_judges_the_functions_in_it`,
`a_test_file_beside_its_source_is_judged_with_no_configuration`,
`an_except_added_only_in_the_working_tree_does_not_let_a_deletion_through` and
`a_test_file_no_grammar_reads_is_named_and_exits_two` in
`tests/inventory.rs`. Known limit: the convention table is fixed in the
binary, so a project whose tests carry another mark has no function
identity, and only its test files are ratcheted.

**`lockfile` reads the manifest and the lockfile beside it.** A site is the
manifest's repository path plus the dependency name, and it carries three
values, all higher is worse: `unlocked` is 1 when the lockfile holds no entry
for the name, `unpinned` is 1 when the manifest's specifier is a range or
absent, and `stale` is 1 when the manifest states an exact version, the
lockfile holds the name, and no version it records for this manifest is that
pin. A dependency that one table states as a range and another as an exact
version is `unpinned` and is still judged for staleness at its exact version.
A version is the pin when it equals it. A pin of one or two numbers, such as
Cargo's `=1.2` or npm's `1.2`, also holds every release that starts with it
and a dot, so `=1.2` holds `1.2.5` and `1.0.0-alpha` does not hold
`1.0.0-alpha.1`. A version that names a path, `link:` or `file:`, is a
workspace package with no registry release to compare, and it holds every pin.
A lockfile that records several versions of one name is stale only when none
of them is the pin. A range is never judged for staleness, because it has no
single version to compare. Exact means a Cargo requirement that
starts with `=`, an npm specifier that starts with a digit and holds no
operator and no wildcard segment, and every Go `require`, which states one
version. A path, git or
workspace dependency has no registry behind it and no version to pin, so it
carries 0 for every value and can never fail. The manifest tables read are
Cargo's `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]` and
`[workspace.dependencies]`, including the `[dependencies.name]` sub-table
form, the dotted-key form, an inline table on one line or several, and any
table whose last path segment is one of those names, so a target-specific
table is read too. A statement only ever takes a value away, so two tables
that name one dependency read the same in either order, and a `features` line
undoes no version another line stated. A Cargo `package` field gives the name
the lockfile is searched for, so a renamed dependency is found. A comment is
cut off before the line is read, so a brace inside one opens no inline table.
The npm tables are `dependencies`, `devDependencies`,
`optionalDependencies` and `peerDependencies`. Go reads both forms of the
`require` directive. A `replace` directive applies to the version it names,
or to every version when it names none. One that sends a module to a local
path gives it no registry, and one that sends it to another module or version
makes klin look up that module and judge that version, because `go.sum`
records the replacement. The lockfile is the nearest one at or
above the manifest's own directory, which is how a workspace member finds the
one lockfile its members share. `Cargo.lock` gives the `name` and `version` of
each `[[package]]` block, in either order, `package-lock.json` gives the keys of
`packages` with everything up to the last `node_modules/` stripped and the
keys of the nested `dependencies` tree that version 1 writes, and `go.sum`
gives the first field of each line, with the second field as its version once
a `/go.mod` suffix is cut off. So a Go module is stale when `go.sum` holds no
line for the version its `require` states. npm and pnpm resolve each name
for each manifest, so klin judges a manifest's pin against the entry for the
manifest's own directory, relative to the lockfile, and then against the
root's. npm writes that entry as `<directory>/node_modules/<name>`, or as a
top-level key of the version 1 `dependencies` tree, and a link takes the
version of the package it links. pnpm writes it under `importers`, or in the
top-level dependency tables of a lockfile that has no importers. The pnpm
`packages` mapping and the npm entries nested in another package's
`node_modules` count only toward `unlocked`, so a pinned name that the
lockfile holds only there is stale. Cargo and Yarn record no resolution per
manifest, and klin judges a pin against every version they record. A
dependency the base manifest did not name is a
site only when it is `unlocked` or `stale`, so a new dependency with a range
and a lockfile entry is not a finding, while a pin the base held and a
lockfile entry the base held are both `worsened` when they go. Staleness the
base already had is held, so a new one is `new` and one that grew is
`worsened`. `pnpm-lock.yaml` gives names from the direct
keys under its `packages` mapping: older versions use `/name/version` or
`/name@version`, and current versions use `name@version`, with a peer suffix
ignored after the version: the `(...)` of current versions and the `_...` of
version 5. The version in the key is the one it records.
`yarn.lock` v1 gives names from top-level selectors after
`# yarn lockfile v1`; Yarn 2 and later gives them from top-level locator keys
after `__metadata.version`. Both give the version from the `version` line
under each key. Selectors and locators are split at the
package-name separator, preserving scoped names. The line readers recognize
only those markers and key shapes; an unrecognized shape is a NOTE and no
finding rather than an empty lockfile. That NOTE is filed under the manifest
and names the lockfile. A lockfile only the base held unreadable is named at
its base path, followed by `at the base`. A JSON lockfile klin cannot parse
is a tool error naming the file. A manifest with no lockfile in either tree is a
NOTE and no finding, and a lockfile only the base held makes every
dependency of that manifest `unlocked`, so deleting a lockfile fails. A
manifest klin cannot parse now and that did not parse at the base is a NOTE
naming the manifest in every run, hook or not (8.6). It judges none of that
manifest's dependencies, and every other manifest is still judged, so a
fixture that is invalid on purpose does not turn the gate red. A manifest
klin cannot parse now that the base did not hold is a tool error outside the
hook, because the work added the hole: the error says to make it parse or to
add it to `except`. In the hook it is the same NOTE, because the agent cannot
edit `except`. Once the change is in the base, the manifest is a NOTE in
every run. A manifest that parsed at the base and does not parse now
is a tool error naming the file, because the work broke it and the agent can
fix it. A manifest that did not parse at the base and parses now is judged
against a base that named no dependency. Every manifest and lockfile is read
once per tree, the base's through one git process. The base judges a manifest
the window renamed at the path it had there, beside the lockfile it had there,
so a rename keeps its base. A manifest renamed from another format, such as
`Cargo.toml` to `package.json`, has no comparable base, because its base bytes
were written for another reader, so it is judged as one the base did not
hold. A lockfile several manifests share is parsed once. An accepted entry that gives no `stale` holds
a `stale` of 0, so an entry written before the value existed stays valid and
holds no staleness. The remedy has one part for each value that failed, in the
text and in each finding's `fix_advice`: every nonzero value of a new finding,
and each value that rose on a worsened one.
`unlocked` asks for the project's own install, so the lockfile records the
dependency, `unpinned` asks for the base's exact version back or an exact
version for a new dependency, and `stale` asks for an install again, so the
lockfile records the pinned version. The failure names a dependency that the
lockfile beside the manifest does not lock at an exact pin. Only the npm reader can
reject a manifest, because the Cargo and Go readers are line scans. The check
judges every manifest under `--changed` as well, because a lockfile change
judges a manifest whose own text did not change and the whole set is a
handful of files. Pinned by
`a_new_rust_dependency_with_no_lockfile_entry_fails_as_new`,
`a_rust_lockfile_entry_that_went_fails_as_worsened`,
`a_rust_pin_that_became_a_range_fails_as_worsened`,
`a_new_dependency_with_a_range_and_a_lockfile_entry_does_not_fail`,
`a_path_a_git_and_a_workspace_dependency_are_not_judged_for_unlocked`,
`a_deleted_lockfile_fails_every_dependency_of_its_manifest`,
`a_workspace_lockfile_above_the_member_manifest_is_found`,
`a_sub_table_and_a_target_table_are_read_like_any_dependency_table`,
`a_dotted_key_states_one_field_and_undoes_nothing_another_line_stated`,
`an_inline_table_written_over_several_lines_is_read_as_one_dependency`,
`a_brace_inside_a_comment_hides_no_dependency_below_it`,
`a_renamed_dependency_is_locked_by_the_package_the_lockfile_records`,
`both_npm_lockfile_versions_hold_a_dependency_in_the_base_state`,
`an_unrecognized_lockfile_format_is_a_note_and_judges_no_manifest`,
`pnpm_lockfile_key_styles_hold_and_missing_dependencies_fail`,
`both_yarn_lockfile_formats_hold_and_missing_dependencies_fail`,
`a_malformed_lockfile_is_a_tool_error_naming_the_file`,
`a_derived_manifest_klin_cannot_parse_is_a_note_and_every_other_manifest_is_judged`,
`a_derived_manifest_that_parsed_at_the_base_and_does_not_parse_now_is_a_tool_error`,
`a_derived_manifest_the_change_adds_and_klin_cannot_parse_is_a_tool_error_and_passes_the_hook`,
`a_derived_manifest_klin_cannot_parse_that_the_change_only_renamed_is_a_note`,
`a_renamed_manifest_is_judged_against_the_lockfile_beside_it_at_the_base`,
`a_lockfile_only_the_base_could_not_read_is_named_at_the_base`,
`a_manifest_renamed_to_another_format_has_no_base_to_hide_behind`,
`a_derived_manifest_that_did_not_parse_at_the_base_is_judged_once_it_parses`,
`a_manifest_klin_could_never_parse_is_a_note_and_no_tool_error`,
`two_manifests_that_share_one_lockfile_are_each_judged_against_it`,
`under_changed_a_changed_lockfile_with_an_unchanged_manifest_is_still_judged`,
`a_manifest_pin_the_lockfile_records_at_another_version_fails_as_worsened`,
`a_new_pin_the_lockfile_records_at_another_version_fails_as_new`,
`npm_judges_the_version_under_a_package_root_and_not_a_nested_one`,
`every_lockfile_reader_fails_a_pin_it_records_at_another_version`,
`a_range_with_a_lockfile_entry_at_any_version_passes`,
`staleness_the_base_already_had_is_held`,
`a_lockfile_with_several_versions_of_one_name_is_stale_only_when_none_equals_the_pin`,
`a_new_dependency_missing_from_the_lockfile_gets_a_remedy_that_names_the_install`,
`a_finding_with_several_values_prints_the_remedy_for_each`,
`a_pnpm_5_peer_suffix_is_no_part_of_the_name_or_the_version`,
`a_go_module_a_replace_sends_to_another_version_is_judged_at_that_version`,
`an_exact_pin_beside_a_range_of_the_same_dependency_is_still_judged_for_stale`,
`an_npm_pin_with_only_a_nested_lockfile_entry_is_stale`,
`an_npm_workspace_link_is_judged_at_the_version_of_the_package_it_links`,
`each_npm_workspace_manifest_is_judged_against_its_own_entry_and_then_the_root`,
`a_cargo_lock_block_is_read_whatever_the_order_of_its_fields`,
`only_a_pin_with_fewer_than_three_numbers_holds_the_versions_it_is_a_prefix_of`,
`a_go_replace_applies_only_to_the_version_it_names_and_judges_its_target`,
`a_pnpm_manifest_is_judged_against_its_own_importer_and_not_the_package_pool`,
`a_pnpm_5_root_dependency_section_is_its_importer`,
`a_worsened_finding_prints_only_the_remedy_of_the_value_that_rose`,
`a_stale_pin_fails_under_a_condition_that_names_it` and
`an_accepted_entry_that_gives_no_stale_holds_no_staleness` in
`tests/lockfile.rs`. Known limit: the Cargo and Go readers are line
scans, so a manifest that states a dependency in a shape the scan does not
know contributes no site rather than a wrong one. Known limit: a Yarn
lockfile records no resolution per manifest, so in a Yarn workspace a pin
passes when any workspace resolved the name to it. This can hide staleness,
and it cannot report staleness that is not there.

**`layering` judges resolved dependencies against a person's layers.** It is a
Policy check: with no section it does not run. The section holds `layers`, a
map of layer name to a layer's `in` and `can_use`, and the optional shared
`in`, `except` and `acyclic`. A layer's `in` is a path or a list of paths of
5.3, never a glob. `can_use` lists the layers a layer may depend on, `null`
lets it depend on every layer, and an absent `can_use` lets it depend only on
itself. A layer may always depend on itself. A `can_use` that names no layer,
a layer or shared `in` that holds no Rust or TypeScript file, and a file two
layers hold are config errors. `roots`, `languages`, `skip_dirs`, `name`,
globs, `allow_same` and package topology are refused.

The check builds the module graph of ADR 0043 for both trees from the
structural facts of 8.4 and each tree's own file list. A Rust target root
comes from a Cargo manifest's `lib` and `bin` targets, implicit ones included,
and, where no usable manifest sits above a file, from `src/lib.rs`,
`src/main.rs` or a file directly in `src/bin`. From each root the check follows
`mod` declarations to `name.rs` or `name/mod.rs`, and a literal `#[path]` from
the directory of the file or of the inline module that holds it. A file two
targets reach is a module of each. A dependency is a path a `use` tree or a
path outside an import writes from `crate`, `self` or `super`, resolved to the
deepest module it names. A path whose first segment names a module the same
file declares at that path's nesting, with `mod name;` or inline and directly
in a module rather than inside a block, resolves as if it started with
`self::`, so `pub use inner::X;` and `inner::f()` beside `mod inner;` depend on
`inner`. A module a block declares directly, a function body or a `const` or
`static` initializer alike, is an item of that block and is reached by no bare
path from outside it. A module that module declares is its child as usual. A
target's edition comes from its manifest, where Cargo's default is 2015, and a
conventional root is read as edition 2024. In a `use` tree of an edition 2015 target, a first segment
starts at the target root instead, and resolves only where the root declares a
module of that name. A Rust path that resolves to its own module
names that module's items and is no edge, so `use self::Kind::*` closes no
cycle. A path from another name may be another crate or a local item, so a
`use` tree that writes it is counted as external and not resolved, and a path
outside an import that writes it is not read. Every TypeScript file is a
module. A relative specifier resolves when exactly one of these files exists:
the specifier itself with a TypeScript extension, the `.ts` or `.tsx` file a
`.js` specifier stands for, or `.ts`, `.tsx`, `index.ts` or `index.tsx` after
it. A bare specifier, an alias and a relative specifier that names a file of
another kind are counted as external. A `mod` declaration is containment and
never a dependency.

A module is its resolver's identity and holds one or more physical files, each
once and in no meaningful order (ADR 0058). A Rust or a TypeScript module holds
one; a resolver for a later language may group several. Structural facts stay
per file. Each file a resolver attaches counts on its own in the coverage
line, so a module of ten files is ten attached files. A dependency is a
*site*: the module that writes it, the module it reaches, and the exact file
and line that write it. A resolver or a surface derivation runs only over a
tree whose file list holds a path of its language: a Rust source or a
`Cargo.toml` for Rust, a TypeScript source or a `package.json` for
TypeScript. A source the grammar refuses still counts, so the resolver still
places it. The file list is scanned once while the topology is built.

Path policy is physical. Each file of a tree is placed once: whether the scope
selects it and which layer holds it. Each module is then folded once over its
files: its scope is inside where the scope selects every file, outside where it
selects none, and mixed otherwise; its layer is a layer where every file sits
in that one layer, none where no file sits in a layer, and mixed otherwise. A
dependency is judged where the scope selects the file that writes it and the
module it reaches is inside, by the layer of that file and the folded layer of
that module. A module that is mixed is never placed by one of its files. A
dependency on a module whose files straddle the scope is unresolved as below
and gets no verdict. A dependency from a file in a layer on a module whose
files straddle the layers is unresolved as below and gets no layer verdict,
but it is still judged for cycles, because only the scope decides that.

For every dependency the section judges, where the file that writes it and
the module it reaches sit in layers and the source layer may not use the
target layer, the edge is forbidden. With `acyclic` true, the strongly
connected components are found over modules, with every judged pair of
modules one edge however many sites write it. A judged dependency whose two
modules share a component is cyclic, and a module that imports itself is
cyclic. The check first judges *semantic edges*: the module that writes a
dependency, the module it reaches, and the edge's text, which names its kind,
the two layers for a forbidden edge, and the module it reaches by its file and
the inline modules after it. A module's semantic identity is its identity
under current paths and, for a Rust module, the kind and root of the target
that owns it, so a file two targets reach is a different module under each.
The working tree's semantic edges are paired with the base's before any
finding exists: a semantic edge the base holds is held wherever its sites now
sit, so evidence that moves between the files of one module, splits across
them or joins in one of them is held. Then each edge is reported where it is
written: one finding per file and text, which carries `edge` at 1 and is held
only where the base holds every semantic edge it merges. A line that starts to
reach another module, an inline module of the same file included, is new, and
so is a file two targets reach whose targets swap what each reaches. An
accepted entry is matched by the file and text a person wrote and never
follows a move, so under `--strict` an entry its edge moved away from is
stale. The check needs the commit
and not the runner's tree: it reads the whole base through the run's one
shared checkout, so a changed run lays out no partial tree for it. A new cyclic edge prints one shortest cycle through it,
which explains the finding and is no part of its key. Today's policy judges
both trees. The base places a file the window renamed under the path it had at
the base, and its finding names the current path, so a move into another layer
is new debt and a move inside a layer is held.

A module that two files answer, a module no file answers, a path above the
crate root, and a TypeScript specifier with no candidate or with two are
unresolved. Where the scope selects the file that writes it, or where a manifest
writes it, each is a NOTE in the hook and exit 2 elsewhere, unless the base
holds it too, which is a NOTE in every run (8.6). A
file on disk that the file list leaves out, such as generated source git
ignores, is counted as external. The `OK:` line counts the dependency sites judged, the
files attached by a manifest and by a conventional root, the Rust files no
target reaches and the external dependencies. In a changed run that is not
strict, the working tree takes the base's facts for every unchanged file, as
`dead-symbols` does. Pinned by
`a_new_forbidden_dependency_fails_as_new`,
`a_forbidden_dependency_the_base_holds_is_held`,
`an_allowed_and_a_same_layer_dependency_pass`,
`can_use_null_lets_a_layer_use_every_layer`,
`a_file_in_two_layers_is_a_configuration_error_naming_both`,
`retired_topology_and_unknown_keys_are_refused`,
`a_super_path_inside_an_inline_module_resolves_from_that_module`,
`a_path_attribute_that_retargets_an_unchanged_file_is_new_debt_in_a_changed_run`,
`a_manifest_that_moves_the_library_root_changes_what_an_unchanged_file_reaches`,
`a_workspace_member_that_inherits_its_edition_is_attached_by_its_manifest`,
`a_file_two_targets_reach_is_judged_as_a_module_of_each`,
`a_module_two_files_answer_is_unresolved_by_hand_and_a_note_in_the_hook`,
`a_module_two_files_answered_at_the_base_too_is_a_note_by_hand_and_under_strict`,
`a_second_copy_of_a_form_the_base_could_not_resolve_is_new`,
`typescript_relative_imports_resolve_and_package_imports_are_counted_not_guessed`,
`two_typescript_files_one_specifier_names_are_unresolved`,
`a_new_cycle_fails_and_a_cycle_the_base_holds_is_held`,
`a_module_that_imports_itself_is_a_cycle`,
`a_rust_path_to_its_own_module_is_no_cycle`,
`a_retarget_to_an_inline_module_of_the_same_file_is_new`,
`a_typescript_re_export_is_a_dependency`,
`a_package_renamed_with_its_manifest_keeps_its_base_debt`,
`a_missing_target_root_a_manifest_names_is_unresolved_whatever_the_scope`,
`a_module_declaration_is_containment_and_not_a_dependency`,
`a_cycle_closed_through_a_bare_child_path_is_a_cycle`,
`a_cycle_the_base_held_through_a_bare_child_path_stays_held`,
`a_call_through_a_child_the_file_declares_is_a_dependency_on_it`,
`a_first_segment_that_names_no_declared_module_stays_external`,
`a_child_declared_inside_an_inline_module_is_not_reached_from_beside_it`,
`a_module_declared_inside_a_function_is_not_reached_by_a_bare_path_beside_it`,
`a_module_declared_inside_a_constant_initializer_is_not_reached_by_a_bare_path`,
`a_child_of_a_block_local_module_is_reached_from_that_module`,
`a_bare_use_path_in_edition_2015_starts_at_the_crate_root`,
`a_bare_use_path_from_edition_2018_starts_at_the_declared_child`,
`a_file_renamed_inside_its_layer_keeps_its_base_debt`,
`a_file_renamed_into_another_layer_is_placed_in_its_base_layer_at_the_base`,
`a_changed_run_beside_a_gate_that_lays_out_changed_files_judges_the_whole_base`,
`a_cached_changed_run_reads_and_parses_only_the_changed_file`,
`an_accepted_forbidden_edge_is_held`,
`a_tree_with_no_typescript_path_dispatches_only_the_rust_resolver`,
`rust_source_the_grammar_rejects_still_dispatches_the_rust_resolver`,
`typescript_source_the_grammar_rejects_still_dispatches_the_typescript_resolver`,
`a_file_two_targets_reach_that_swaps_what_each_target_reaches_is_new`,
`an_accepted_edge_does_not_follow_its_dependency_to_another_file` and
`without_a_section_the_gate_needs_one_a_person_writes` in
`tests/layering.rs`. A module of several files has no resolver yet, so the
unit tests
`multi_source_work_is_linear_in_files_modules_sites_and_unique_edges`,
`a_straddled_destination_is_ambiguous_and_never_judged`,
`a_site_is_its_file_and_line`,
`a_moved_semantic_edge_is_held_once` and
`a_semantic_edge_is_paired_before_its_findings_are_made` in
`src/layering.rs`, and
`a_module_of_many_files_attaches_each_file_and_names_each_site` in
`src/modules/mod.rs`, pin it over a graph built in memory. Known limit: a path
inside a macro's tokens, a bare Rust path that names no module its file
declares, a TypeScript `import()` or `require()`, `tsconfig` paths and package
exports are not dependencies in V1. A name a block or a function binds in the
type namespace, such as a local `use`, a local type alias or a generic type
parameter, shadows a declared child module of the same name in Rust, and klin
still reads a bare path through that name as a dependency on the child.

**`public-api` judges the consumer-facing contract a library or package
exposes.** Klin derives public API from standard Rust library and TypeScript
package entry points. You normally configure nothing: the section is absent,
or `false` to exclude the gate, and any object under it is a config error,
because surfaces, roots, languages and entry points are facts of the tree. It
is an Automatic check that needs the commit and takes no scope: a changed run
judges every file of both trees, because a manifest, an entry point or a
re-export can change what an unchanged file means to a consumer. It reads the
structural facts of 8.4 and the module graph of ADR 0043 for each tree under
that tree's own topology, and it builds no name index, no second resolver and
no parser of its own.

A *surface* is what a consumer addresses. For Rust it is a Cargo library
target, named by its crate name and matched between trees by that name and
its package manifest path. Two libraries with the same crate name retain
separate surfaces: unchanged items pass, and a removal or contract change is
judged only against the library that owned it. Adding or removing a namesake
library does not change the identity of an existing surface. Moving the
library root within its package keeps the surface's identity. Pinned by
`libraries_with_the_same_crate_name_keep_their_own_items`,
`a_namesake_library_does_not_hide_a_contract_change_or_removal` and
`removing_a_namesake_library_reports_only_its_surface`.
A binary target and a Rust directory no
manifest names are not surfaces. Implicit and custom library roots and every
library package of a workspace are found the way ADR 0043 finds targets. For
TypeScript a surface begins only at explicit package metadata that names a
checked-in TypeScript, TSX or declaration source file: an `exports` string,
each `exports` subpath whose target reduces to exactly one such file across
its conditions, and, without `exports`, the first of `types`, `typings`,
`main` and `module` that names one. Generated JavaScript is never mapped back
to source, `src/index.ts` is never guessed, and a package none of whose
entries names a supported source is not applicable and is said so on a
`NOTE:` line, not a hole. Publication metadata does not decide a surface: a
Cargo package with `publish = false` and an npm package with
`"private": true` keep their surfaces, because a workspace sibling, a path
dependency or a git dependency can still consume them (ADR 0050).

An *item* is what a consumer names under a surface, and its identity is the
surface, the exported path or name and the item's kind, never the file that
declares it. In this check, *external* means outside the crate or package
that declares the item, including a sibling in the same repository, and never
means published. The check judges only external items. From a Rust root the
check follows every plain `pub` declaration, every `pub mod`, every `pub use`
leaf and every `pub extern crate`, which re-exports the crate under its alias:
an alias renames the item, a glob exposes every public item of the module it
reaches less every name the globbing module binds itself at any visibility
in the namespace the name lives in (an item it declares, a child module or an
`extern crate` alias as a type, and a name a `use` outside a function binds
in both), a name a module neither declares nor re-exports by name comes from the one glob of it
that provides the name and is a hole where two do, a re-export of a module
exposes everything under it, and a plain
`pub` item inside a private module is external only where a `pub use` exposes
it.
`pub(crate)`, `pub(super)`, `pub(self)` and `pub(in ...)` are never external.
A Rust module already on the active surface traversal is a coverage hole:
its cyclic module re-export gives unbounded public paths. A named re-export
already being resolved is also a hole, because its cyclic name cannot be
resolved. A module-cycle hole names the file, line and source text of the
re-export that closes the cycle. Both end in a report rather than an abort;
separate finite aliases of one module are still followed independently.
Pinned by `a_module_re_export_cycle_is_a_named_hole_instead_of_unbounded_paths`
and `a_named_re_export_cycle_is_a_named_hole_instead_of_recursing_forever`;
`a_module_cycle_names_the_export_that_closes_it` pins the closing source site.
A re-export whose path starts, with or without a leading `::`, at a name of
the crate's extern prelude that reaches a library target the tree holds is
followed into that library's source, globs included, and each item it
reaches is measured there under the surface that re-exports it. Those names
are each normal dependency the target's manifest takes by path, target-specific
ones included, under its rename where the manifest writes `package =` and
under the library's own crate name otherwise, and each alias an
`extern crate`, public or private, at the top of the crate root gives one of
those dependencies, as `pub extern crate wgpu_types as wgt;` does. The path
names the manifest of the library, so a crate name two libraries of the tree share
reaches the one the dependency names. A dependency without a path, such as a
registry version, names a crate the tree does not hold even where a library of
the tree has its name, and so does a name a `use` binds, which never enters
the extern prelude. A first segment the module binds itself, by a `use`
outside a function, as a type it declares or, below the crate root, as an
`extern crate` alias, names a local item, so the path is opaque even where a
dependency has that name, and so is an alias the crate root gives a crate
that is no dependency of its manifest. An associated item of an `impl` or a
`trait` is no item of the module, so it hides no name. So an item moved into a sibling crate and
re-exported under its old name keeps its identity: an unchanged contract
passes and a changed one fails as changed. A re-export of a whole crate root,
such as that `pub extern crate` itself, is an opaque item, because the items
of a library the tree holds are judged under its own surface. Pinned by
`an_item_moved_into_a_workspace_sibling_and_re_exported_under_its_name_passes`,
`an_item_moved_into_a_workspace_sibling_with_a_changed_contract_fails_as_changed`,
`a_re_export_through_a_pub_extern_crate_alias_of_a_sibling_is_judged_the_same_way`,
`a_name_a_sibling_provides_through_a_glob_is_measured_where_the_glob_reaches`,
`a_re_export_after_a_leading_path_separator_reaches_an_extern_prelude_name_and_no_use_alias`,
`a_crate_the_manifest_takes_from_a_registry_stays_opaque_though_the_tree_holds_its_name`,
`a_crate_name_two_libraries_of_the_tree_share_reaches_the_one_the_manifest_names`,
`a_dependency_the_manifest_renames_is_followed_under_its_new_name`,
`a_dependency_inherited_from_the_workspace_is_followed_from_the_workspace_path`,
`a_dependency_inherited_from_a_workspace_below_the_tree_root_is_followed`,
`a_dependency_a_package_inherits_from_its_own_workspace_is_followed`,
`a_target_specific_path_dependency_is_followed`,
`a_private_extern_crate_alias_of_a_sibling_is_followed`,
`a_name_two_globs_provide_is_a_hole_where_a_re_export_names_it`,
`a_re_export_of_a_crate_the_tree_does_not_hold_stays_opaque`,
`a_local_use_named_like_a_dependency_keeps_its_path_opaque_though_its_item_changes`,
`a_private_item_or_import_hides_the_name_a_glob_of_a_sibling_provides`,
`an_extern_crate_alias_named_like_a_dependency_keeps_its_path_opaque`,
`only_a_module_level_binding_in_the_same_namespace_hides_a_name_a_glob_provides`,
`a_use_inside_a_function_named_like_a_dependency_leaves_the_dependency_followed` and
`a_glob_of_a_workspace_sibling_lists_its_items`.
A public inherent method, associated constant or associated type is an item
under its type, pinned by `an_item_of_an_inherent_impl_is_an_item_under_its_type`.
From a TypeScript entry
file the check follows exported declarations and namespaces, default exports,
local export clauses, and named, aliased,
type-only and star re-exports through the module graph's own edges. An
exported file no entry reaches is not package API. TSX is TypeScript.

An item is *measured* where its declared contract is canonical, and *opaque*
where klin proves it exists and no more. The canonical contract is written by
the language's structural adapter and never by the check: it drops bodies,
initializers, comments and decorators, one space stands between tokens, a
private member leaves except a private constructor, which stops a consumer
constructing the class, a private tuple position becomes `_`, and a binding
name that is not contract becomes `_`. The contract is complete: it shows
every fact whose own change can fail an item, so the `was` and `now` lines of
a failure always differ, and a construct the adapter cannot canonicalize is
opaque, never guessed (ADR 0054). A Rust type or variant with a directly
written `#[non_exhaustive]` renders it, and every other attribute leaves,
`#[cfg_attr(...)]` included. A Rust struct's private named field leaves and
its field list ends in `..`, whether or not the field sits under `#[cfg]`. A
Rust trait method with a default body renders `{ .. }` where one without
renders `;`, and a trait's associated `const` with a default renders `= ..`.
A TypeScript overload set keeps the source order of its
signatures inside one file, the groups of different files are ordered by their
text, so a renamed file never changes a contract, and an implementation
signature that follows overload signatures leaves the set. Inside a class or
interface body the overloads of one method, call signature or construct
signature likewise stay together in source order and lose their
implementation, though the public and protected properties a constructor
implementation declares through its parameters stay as members, and the
members keep one order whatever order the source wrote them in. An object
type literal keeps every member in source order, its overloads included, so
reordering any of its members fails. A `this` parameter shows as `this` with
its type. A TypeScript
parameter with a default carries `?` where no required parameter follows it,
and `= ..` where one does, because a caller passes `undefined` to reach that
default. A default on a constructor parameter that declares a property is
always `= ..`, because that property is never `undefined` the way an optional
one may be. Its initializer never shows. So
`#[non_exhaustive]` added to a type or a variant, a private field added to a
struct whose fields were all public, a default body or a default `const`
removed and a change that only reorders a TypeScript overload set each fail.
The `cfg` declarations of one Rust item are ordered by their text, so
reordering them passes. Rust covers
functions with qualifiers, generics, receiver and parameter types, return
type and `where` clause; structs, unions, enums with their variants, fields
and explicit discriminants; traits with their supertraits and associated-item
signatures; type aliases; and `const` and `static`
with their type alone. TypeScript covers functions and overload sets, classes
with their heritage and public and protected members, interfaces, type
aliases, enums and variables. A type the compiler would infer is written as
`?`, so an inferred contract is visibly partial and never fabricated from a
body. A re-export of a crate the tree does not hold or of another package, an
enum variant re-exported by path, a `* as ns` export and an anonymous default
export are opaque, and the
normalized clause that exposes them is the contract klin compares. A
TypeScript namespace an `export` statement declares, written `namespace`, or
`module` with a name that is no string, `declare` or not, is the item
`NAME (namespace)`, where `NAME` is the first name a dotted name such as `A.B`
writes, because that is the name the declaration binds. It is opaque, and its
clause is the contract klin compares: `declare` where written, unless a
declaration file or an ambient namespace around it makes the namespace ambient
already, `namespace` and its name as written, and in braces each member a
consumer can see, spelled by the rules above, so a body and an initializer
leave and an inferred type is `?`. A namespace is ambient where `declare`
makes it so, where an ambient namespace holds it, and in a declaration file: a
file whose name ends in `.d.ts`, `.d.mts` or `.d.cts`, or a `.ts` file whose
name holds `.d.`. So `declare` added in a declaration file passes. The body
of an ambient namespace that holds no export clause and no export assignment
exports every declaration it holds except an import alias written without
`export`. Any other body exports only what it writes `export` on and the names
its export clauses list. A name a clause lists is spelled as the member of the
body that binds it, or as the name itself where no member binds it, followed,
where the clause exposes it under another name or as a type only, by `as`,
`type` where `export type` or a `type` before the name makes it type-only, and
the name a consumer reaches it by. A variable that destructures binds each
name its pattern binds, never a default value or a computed key, and is
spelled whole for each of them, with each default its pattern writes shown as
`= ..`, because whether a default is there can change the type of the name it
binds. Its value never shows. The export clause itself is never spelled,
so reordering one passes, and adding `export {}` fails as changed only where it
hides a member. A statement that declares nothing leaves, a nested namespace is
spelled the same way, an import alias is spelled as written, the overloads of
one function stay together in source
order and lose their implementation, and the members keep one order whatever
order the source wrote them in. So a new namespace is a new item and passes, a
namespace the base exposed that is gone fails as removed, one whose clause
changed fails as changed, and an edit to a function body, an initializer or a
member the namespace does not export passes. A dotted name stays as written, so
rewriting `namespace A.B` as a namespace `B` inside `A` fails as changed. A
namespace declared without `export` and exposed by a later export clause or a
default export is the opaque item that clause exposes, judged by the clause,
except where the file also declares a function, class or other declaration of
that name: the clause then exposes that declaration alone, and the members of
the namespace go unjudged. A namespace an `export` statement declares hides the
name a star export provides. Pinned by
`a_new_exported_declare_namespace_in_an_entry_file_is_an_item_and_passes`,
`an_exported_namespace_the_working_tree_lacks_fails_as_removed`,
`an_exported_namespace_whose_declaration_changed_fails_as_changed`,
`a_plain_namespace_shows_what_it_exports_and_an_edit_no_consumer_sees_passes`,
`a_declare_namespace_shows_every_member_it_holds_and_dropping_declare_fails`,
`an_export_clause_added_to_a_declare_namespace_fails_as_changed`,
`under_an_export_clause_a_member_without_export_is_hidden_and_dropping_export_fails`,
`a_name_an_export_clause_lists_shows_under_the_name_the_clause_gives_it`,
`a_name_a_destructuring_declaration_binds_shows_as_that_declaration_where_a_clause_lists_it`,
`a_namespace_in_a_declaration_file_is_ambient_and_shows_every_member`,
`a_member_edit_inside_a_nested_namespace_or_module_fails_as_changed`,
`a_module_with_a_name_is_a_namespace_and_a_dotted_name_is_its_first_name`,
`a_dotted_name_stays_as_written_so_nesting_it_fails_as_changed`,
`a_namespace_a_later_clause_exports_is_the_opaque_item_of_that_clause`,
`a_clause_that_exports_a_function_and_a_namespace_of_one_name_exposes_the_function_alone`
and `an_exported_namespace_hides_the_name_a_star_export_provides`.

Base and working tree are derived independently. A base surface the working
tree lacks fails once, at the surface. For every item of a surface both hold,
an item gone fails, a measured contract that changed or is no longer declared
fails, an opaque clause that changed fails, and everything else passes: a new
surface, a new item, a widened visibility, an opaque item that became
measured. Each break carries `break` at 1 with the surface as its file and
`NAME (KIND)` as its text, so an intentional break is an accepted entry under
that identity, and the base holds no break by construction. For a Cargo library, the
finding's `file` is always `CRATE (MANIFEST)`, such as
`shared (a/Cargo.toml)`, while the consumer name in `--report` stays `shared`.
The owning manifest is part of accepted-entry matching, JSON site identity,
removed-module grouping and unresolved-hole evidence even when the crate
name is unique in either tree. An accepted break for one library cannot hold
the same item break in a namesake, and an unqualified crate-name entry for a
Cargo library matches nothing and follows the normal stale-entry rules;
klin never rewrites accepted entries.
Pinned by `accepting_a_namesake_break_cannot_hold_another_librarys_break`,
`identical_breaks_in_namesake_libraries_have_distinct_json_ids`,
`an_unqualified_rust_surface_acceptance_is_stale` and
`a_removed_module_of_two_surfaces_with_one_name_prints_each_item_once`.
Where one change removes a module and the items inside it, the text report
prints them as one group, the module's line and then the lines of the items it held, while the
11.2 object, the journal, the identities and the accepted entries keep one
finding per item. Pinned by
`a_removed_module_prints_as_one_group_and_json_keeps_each_item`. The remedy
keeps the base's contract only where the task allows it, says that for a
changed contract a new item beside the unchanged one keeps the base's contract
where that serves the task, proposes no name or design, and tells the agent
not to change what the task asked for only to satisfy the gate. In the hook
it adds that a stop blocked on a break the task intends is answered in the reply and
followed by another stop, which the block policy of 9.3 may let end, and
that the reply accepts nothing: a person accepts the break with an accepted
entry in a reviewed commit, and CI refuses it until then. Every stop prints
the same wording, so a stop that passes does not claim it blocked, and the
wording promises no end, because a build failure still blocks under 9.3.
Outside the hook the remedy names only the person's accepted entry and CI,
and no second stop. Pinned by
`a_break_in_the_hook_names_the_intended_change_route_and_leaves_acceptance_to_a_person`
and `a_break_by_hand_names_person_acceptance_and_no_second_stop`. A glob over
a crate the tree does not hold, a star export of another package, a name two
globs or two stars provide, an export form klin recognizes and cannot list,
such as TypeScript's `export =` or an exported `declare module` whose name is a
string, a
path through a module no file answers, and an unresolved module or specifier
inside a surface are holes: a `NOTE:` in the hook and exit 2 elsewhere, while
other findings still print, because a green run must not imply a surface it
claims to support was completely measured. A hole the base holds too is a
NOTE in every run (8.6). Unresolved-hole matching includes the same
owner-qualified Cargo surface identity as compatibility findings: a form
that moves from one namesake surface to another is new at its destination,
even when its source file, text and underlying reason are unchanged. Pinned
by `an_unresolved_form_moved_to_a_namesake_surface_is_new` and
`export_equals_and_an_ambient_module_are_still_holes`. The `OK:` line counts
the items and surfaces judged, how many are measured and opaque, the library
targets and entry points found, and the packages or targets with no supported
surface.
`klin public-api --report` prints the working tree's derived contract without
judging it: each surface with its discovery source, each item with its
identity, kind, origin, measured or opaque status and canonical signature,
each hole, and each package or target not applicable. A language's surface
derivation runs only over a tree that holds a path of that language, by the
same rule as its resolver in `layering`. Pinned by every test in
`tests/public_api.rs`. Known limits: a module bound by `use` and then
re-exported by its bare name, a macro, a trait implementation's semantics,
`cfg` evaluation, an attribute written through `#[cfg_attr(...)]`, a
registry dependency that `[patch]` or `[replace]` points into the tree,
`extern crate self as` an alias, a re-export by name through a glob of a
module that binds the name in either namespace, which is opaque,
`typesVersions`, conditional exports that do not reduce to one source file,
the top-level names a declaration file exports without writing `export`,
`tsconfig` paths and a package alias are outside V1, a declaration file
renamed to a name that is none, such as `index.d.ts` to `index.ts`, is read
at the base under its new name (8.2.1), so a member only the declaration file
exported goes unjudged, a declaration no surface
exposes is not judged, even where an exposed contract names it, and a generic
parameter renamed is a changed contract.

None of these rules asks another implementation to agree with klin. They
state what klin's own tests hold, per ADR 0025, so a change to one is a
change to the spec and to a test in the same commit.

### 8.3 The linter seam

The seam ships in two steps. The first needs no second tree and fits the hook
budget. The second is deferred until a user asks for the cases it catches.

**Step one, the `after` tree only.** A `sarif` entry names a command that
writes a SARIF report from the working tree, or a report that is already
there:

```json
"sarif": [
  { "name": "eslint", "run": "npx eslint -f sarif -o out/eslint.sarif .", "report": "out/eslint.sarif" },
  { "name": "semgrep", "report": "out/semgrep.sarif", "differential": true }
]
```

With `run`, klin deletes `report`, executes the command in the working tree the
way it runs `build`, under the same limit and deadline (9.3), then reads
`report`. A report that is missing after `run` is ERR, and so is a command
still running at the limit or the deadline, named with the one it reached. The
report path SHOULD be under `.gitignore`, so the stamp of 6.5 does not carry
it. The report is fresh by construction, because the only file at that path is
one the tool wrote over the tree klin is about to judge. The command's exit
status is not judged, because a linter exits non-zero when it finds something.
In the hook this is the RECOMMENDED form. Without `run`, klin reads the report
as it finds it, and that form belongs in CI, where the same job wrote the
report one step earlier. There, a report older than any file the window changed
is ERR, because a report that predates the change cannot describe it. Executing
the tool in the `after` tree has no dependency problem: the working tree has
its dependencies installed, or the build would fail first.

Each entry of the section is its own gate, named by its own `name`, because
nothing in a tree says which scanner an entry runs. The section MUST be a list,
and an entry with no `name` is a config error naming the key. A `sarif` gate
reads the base commit and never the base tree, so it runs one scanner over the
working tree and no `before` worktree is laid out for it.

Each result is keyed by file, rule id and message. A result fails when its
line falls inside a hunk the window changed. A result in a file the tree does
not track is one whole changed range, so every result in it is judged.

**Where a result sits.** A result's `physicalLocation.artifactLocation.uri`
reaches klin in four forms, because every scanner writes a location its own
way: a path relative to the repository, an absolute path, a `file://` URI, and
a path under an entry of `originalUriBaseIds`, which may name a further entry
in turn. klin percent-decodes each uri, drops a `file://` scheme, resolves a
base id through at most four entries, strips the tree's own directory off an
absolute path, following the real directories both name so that a tree reached
by a symlink still places, and resolves the `.` and `..` segments of what is
left. A location that lands outside the tree, in any of the four forms, and a
result with no physical location at all, is a NOTE naming the uri and is not
judged, because a path klin cannot resolve says nothing about which lines
changed. A location with no `region.startLine` starts at line 1, which is the
line a result about a whole file is judged on.

Several results of one rule in one file are one finding with a `count`, so an
accepted entry holds that site at the count a person accepted. An accepted
entry that matches nothing is a NOTE, and exit 1 under `--strict`, as for
every other gate (10).

A report klin did not write is judged fresh by modification time: klin
compares the report file's time against the time of each file the window
changed, and a report older than one of them is ERR.

A result on a line the window did not change is held, whatever the base held
there, and the gate's `OK:` line says how many results it judged and how many
it held. With `differential: true` the tool already reports only what is new,
so every result fails and that line says instead that it judged every result
the scanner reported. Scoping to changed lines, not changed files, is what
keeps an agent that touches a file with thirty old warnings green. A reformat
that moves every line is the known weakness, and the radius report already
names such a turn.

This step runs the tool once, needs no `before` worktree, and delivers most
of what the goal names: eslint, tsc, clippy and ruff findings become klin
failures exactly where the agent wrote the line. The same `run` and `report`
contract is the one a coverage reader or a test-result reader takes later.

**Step two, `run` in both trees.** An entry marked `compare: true` is
executed in both trees and its results are matched by site and ratcheted on
`count`. This catches a rule count that rose on an unchanged line, and a
finding that moved. It is deferred for a reason the review stated: the
`before` worktree has no installed dependencies, and a symlink of today's
dependencies runs today's tool over old source, which is not a pure function
of the `before` commit and breaks section 12. A design for step two MUST
solve that before it is written. A tool that compiles the tree, such as
clippy or `tsc`, is out of scope for `compare`: a bare `before` worktree
would fetch dependencies, which section 12 forbids, and a build is outside
the budget of section 13. The first candidate is a tool that is one binary
and reads no project dependencies, such as ruff.

### 8.4 Tier 2: build when tier 1 is green

`conventions` (#42, ADR 0037), `public-api` over the module graph and a
derived public surface (#46, ADR 0044),
`reachability` over one reference extractor (#49, #51), `layering` over one
module graph (#50, ADR 0043), `changed-coverage`
and `crap` over one coverage reader with a postflight run (#53, #54, #55,
#70), `hotspots` as a report (#60), SARIF output (#65).

The reference extractor is the `syntax` module (#49, ADR 0035). It owns the
grammars, the parser and the file no grammar read, and it hands a check
declarations, imports, module declarations and references, so no check holds
another language's node kinds. It resolves a reference by name to every
declaration of that name under the roots. That errs toward "referenced", so
`reachability` and other structural checks fail less, never more.

That conservative answer is evidence for judging a tree and not for writing
policy over it. `reachability` judges a file reached through an ambiguous
name, and derives a family only from members proven reached through a name
one declaration holds, per 5.4. `dead-symbols` and `reachability` are
shipped over this extractor; a third structural language is an adapter in
`syntax`, and neither check branches on a language.

Rust and TypeScript are the first structural languages, and TSX is TypeScript
rather than a language of its own. A file in a language no structural adapter
reads is counted as not measured on the coverage line of 8.6, so a green run
over such a tree is visibly a run over nothing. The module graph of ADR 0043
resolves an import or a Rust `mod foo;` to a file. The extractor keeps each
specifier as written for it, with the inline modules that hold an import, a
module declaration or a qualified path, every leaf path of a Rust use tree, and
every path outside an import that starts at `crate`, `self`, `super` or the
name of a module the file declares outside a block at that path's nesting.

A `test-hygiene` check, a count of habits across the test roots against a
dated ceiling, was considered and is not a check. A habit that rose is an
escapes `patterns` row with `roots` set to the test roots, and a schedule on
a whole-tree total fails a tree nobody changed, which is the forced paydown
7.3 refuses.

**`conventions` judges the project's own rules.** A convention names one
thing the project forbids, where it applies, and what to do instead:

```json
{
  "conventions": {
    "single-git-boundary": {
      "code": "Command::new(\"git\")",
      "except": ["src/project/git.rs", "tests"],
      "remedy": "Use the shared Git boundary."
    },
    "no-old-flags": {
      "text": "config::Flags",
      "remedy": "Use the explicit execution context."
    },
    "no-scratch-files": {
      "files": "**/scratch.*",
      "remedy": "Remove temporary scratch files."
    }
  }
}
```

The section is a flat object keyed by the convention's name, and the name is
the convention's identity. A convention MUST state `remedy` and exactly one of
`text`, `code` and `files`. It MAY state `in` and `except`, and a `code`
convention MAY state `language`. Any other key is a config error that names
the convention and the key. There is no regex and no example.

- `text` is literal text, matched line by line. No character in it has a
  special meaning. A comment or a string that holds the text is a match. A
  file that holds a NUL byte is not text and is not read.
- `code` is a code pattern. `$NAME` holds one piece of code and `$$$ARGS`
  holds a list. It matches code, so a comment or a string that reads like the
  pattern is not a match. Its languages are `rust` and `typescript`, and a
  `.tsx` file is TypeScript.

A `code` pattern MAY be a fragment: the piece of code a person means, with
nothing written around it. klin reads the fragment in each place its language
lists. For Rust, those places are code as written, an expression, a match
arm, a type and a field. For TypeScript, they are code as written and a type.
A reading counts when the grammar reads the fragment there with no error and
no token it had to supply, and one node is the whole fragment. A file matches
through every reading, and klin never chooses among them. A node that two
readings match is one match. A pattern that is only a hole is not a reading,
because it would match every node. A pattern that no place reads is a config
error that names every place klin tried.

| `code` | Reads as | Matches | Does not match |
|---|---|---|---|
| `_ => Ok(0)` | a match arm | `_ => Ok(0),` and `_=>Ok(0)` | `_ => Ok(1)`, the text in a comment |
| `RefCell<$T>` | a type | `records: RefCell<Records>`, `Rc<RefCell<Cache>>` | `Cell<u8>`, the text in a string |
| `Command::new("git")` | an expression | `Command::new("git").arg("status")` | `std::process::Command::new("git")` |
| `Array<Foo>` | code as written, a type | `let a: Array<Foo>`, `const n = Array<Foo>` | `Array<Bar>` |

A reading matches the code as the grammar reads it, so a path written in full
is a different piece of code from a path written short, and an element with
one more attribute is a different element. A name such as `config::Flags`
inside `use crate::config::Flags;` is part of a longer path, so a `text`
convention says it better.
- `files` is a glob over repository-relative paths. `**/` crosses any number
  of directories, `*` and `?` stay inside one, and `[...]` is a class. It
  reads no file.

`in` and `except` each take a repository-relative path or a non-empty list of
them. A path names itself and everything below it, and it is never a glob:
`src/syntax` holds `src/syntax/mod.rs`, and `src/main.rs` holds only itself.
With no `in`, a convention applies to the whole repository, and `except`
takes paths out of that scope. An absolute path, a path outside the
repository and a glob are config errors. An `in` path the working tree holds
nothing at leaves its convention measuring nothing there, so it is exit 2, and
a NOTE in the hook. An `except` path the working tree holds nothing at is a
NOTE.

A convention reads what the shared walk reaches. It never reads the default
skip set, a hidden directory, or what git ignores. It never reads the
configuration file either, because that file states every literal a convention
forbids.

A `code` convention with no `language` takes the one language that the files
in its scope are written in. Two languages, or none, is a config error that
lists them and asks for `language` or a narrower `in`. Which grammar reads the
pattern is never evidence of the language. A pattern that no grammar of its
language can read is a config error. A pattern that one grammar of the
language reads and another does not matches nothing in the files the other
reads: a JSX pattern matches no `.ts` file. A pattern that matches nothing in
the tree is a valid convention.

Each convention is judged as its own gate, `conventions/<name>`, through the
ratchet of section 7 and the accepted list of 4.8. Two conventions on one line
are two findings, each with its own remedy and its own accepted entry. A
`text` or `code` site is the file and the text of the line the match starts
on, and `count`, the matches at that site, is the ratcheted value. A `files`
site is the path, at line 0 and a count of 1.

A site that moves to another line of its file is held, and a site in a file
git renamed keeps its key, as 4.4 has it. A site that moves or is copied to
another file is `new`. No body hash pairs a site across files. The scope and a
`files` glob read the path each tree holds, so a file renamed out of `except`
brings its sites in as `new`, and a path renamed into a `files` glob is `new`.

An accepted entry whose gate names a convention the section defines no longer
is a config error. When the section is absent or `false`, no convention gate
runs, and its entries wait for it.

A failure names the convention and prints its `remedy` as the fix. A JSON
finding carries `convention`, `logical_gate`, `matcher`, `count` and `remedy`
in its values, and `language` for a `code` convention (11.2).

`klin conventions --report` reads the configuration and writes nothing. It
prints a summary with one row per convention. `--report <name>` explains one
convention.

```text
3 conventions: 2 new sites, 1 can't run

exhaustive-cli-dispatch    1 new                      src/main.rs:4
no-bad                     Can't read the pattern.
no-records-side-channel    1 new                      src/main.rs:7

For details, run: klin conventions --report <name>
```

The first line counts the conventions, the new sites, the sites that got
worse, and the conventions that cannot run. Each row names the first thing to
act on, in this order:

1. why the convention cannot run
2. its new and worse sites, with the place of the first one
3. an `in` path that matches nothing
4. the files klin cannot parse
5. its held and accepted sites
6. `Clear`, when it matches nothing

```text
window: branch — base ead8a4f, the merge-base with main

exhaustive-cli-dispatch

Forbids _ => Ok(0) in src/main.rs (1 file).
Reads as a match arm in Rust. The language is derived from the files in scope.

  src/main.rs:4   _=>Ok(0),   New

Fix: Handle every CLI command explicitly.
```

The detail prints the window line of 4.2 first. Then it prints:

- what the convention forbids, where, and how many files it reads
- for a `code` convention, what the pattern reads as, and whether the
  language is derived or pinned
- each `in` or `except` path that matches nothing, and each file klin cannot
  parse
- why the convention cannot run, when it cannot
- each site with its outcome against the base and the accepted list
- the remedy, after `Fix:`

The summary prints no window line, and it counts against the same base. When
no base resolves, both views say so and give no site an outcome. A convention
that cannot run makes the report exit 2. A name that the section does not
define is exit 2.

A run walks each tree once. It reads each file once for every `text`
convention, and it parses each file once for every `code` convention. ADR
0037.

### 8.5 Tier 3: defer with a reason

`guard-suites` (#43) and
`manifests` (#44): one stack each.
`db-migration-safety` (#57): deterministic for raw SQL only. `asset-path`
(#59): the ticket expects false positives, which fails criterion 1 in
spirit. `flaky-test-runner` and an MCP server: refused in ADR 0008.

### 8.6 Per-check contract

Every check MUST:

- state its identity rule, its ratcheted values and its derivation rule in
  its module docstring
- print one remedy per failure that names what to change
- print, per failure, the site it matched in `before` or the accepted entry it
  matched, and the value on each side, with the ceiling in force beside them,
  so an agent fixes the right thing and a person can dispute a wrong match. A
  `new` finding says that nothing matched. A check that prints a held site
  says the value the base holds it at, for the same reason. Section 11.1
  fixes those line shapes.
- print one `OK:` line with what it judged on success, plus any `NOTE:`
  lines, and nothing else. A ratcheting check writes the state it measured in
  its own vocabulary and its own counts, and the ratchet writes why those
  findings do not fail, because only the comparison knows that. The reasons
  are distinct and a green line MUST NOT claim more than the comparison
  proved, on the one line the ratchet writes for the whole gate:
  `, all held at the base` when a base site holds every finding,
  `, all on the accepted list` when a person-authored entry holds every one,
  `, N held at the base and M on the accepted list` when both hold some, and
  nothing at all when the run judged no finding. A run that measured no file
  judges no finding, so it claims no comparison either, and its coverage says
  what it measured. No check composes that qualifier for itself. A check that
  judges documents one by one and holds them against the base itself, which is
  `doc_size` alone, still says per document what the base holds that document
  at, because that line names a value and not the gate's pass reason.
  What it judged includes the coverage: how many
  files it found, measured, not measured, excluded and could not read, so a
  green run over an unexpectedly small scope is visible on its one line.
  Section 11.1 writes the boundary between those five counts down once, and
  every check uses it.
  A check that judges documents one by one, such as `doc_size`, says each on
  a line and the gate's coverage on one
  line for the gate.
- name every file that is present in both trees, was measured in `before`,
  and was not measured in `after`. Such a file left through compact scope, a
  file the grammar stopped reading, or a discovery rule the tree no longer meets.
  In the hook it is a NOTE. Under `--strict` it is exit 2, per section 10.
  The rule binds a check whose scope is a set of source files. A check whose
  scope is a list a person writes, a report, or the very set it ratchets,
  such as `inventory`, has nothing to lose this way that it does not already
  judge. A compact source check measures `before` under the `in` / `except`
  scope the base commit records, and under today's when the base records none:
  today's scope applied to both trees could never show a file that a narrowing
  this run took away. Every other rule is today's on both trees. A scoped run reports the
  loss among the files in its scope. The JSON carries the loss as a `lost`
  record under the gate's notes (11.2).
- name a file it could not measure. Outside the hook that is exit 2, with or
  without `--strict` (ADR 0021), when the base could measure the file or did
  not hold it, because the change opened that hole. A file the base held and
  could not measure either is a NOTE in every run. It is not a hole the work
  opened, and no run could ever end green around it, so a tree that holds one
  is green the day klin arrives (5.1). The rule covers a file no grammar reads
  and a form a resolver supports and could not resolve. The base's side is the
  check's own measurement of `before`: a file counts when that measurement
  could not read it, and a form counts when the base holds a form in the same
  file with the same text and reason, at any line, paired one to one. A check
  that reads the base's copy of a renamed file under today's path, which every
  check but `conventions` does, does not count a file the window renamed from
  a path another grammar reads, such as `.ts` to `.tsx`, because the base may
  have read it under its own path. So that file stays exit 2. A rename between
  two paths one grammar reads, such as `.js` to `.mjs`, counts. `conventions` reads the base's copy under
  the path the base holds it at, so its own measurement already decides. In
  the hook each of them is a NOTE, because the agent has no remedy. A file the
  base did not hold stays exit 2 outside the hook, a `lockfile` manifest
  included (8.2.1): the change that adds a fixture invalid on purpose also
  adds it to `except`, and once it is in the base it is a NOTE.
  Pinned by
  `a_file_no_grammar_read_at_the_base_either_is_a_note_by_hand_and_under_strict`,
  `a_file_the_base_parsed_and_the_change_broke_is_exit_two_by_hand`,
  `an_extension_changing_rename_reads_the_base_bytes_under_the_new_grammar`,
  `a_file_no_grammar_read_at_the_base_either_is_a_note_after_a_rename`,
  `a_rename_one_grammar_reads_at_both_paths_keeps_the_note`,
  `a_glob_the_base_holds_too_is_a_note_by_hand_and_under_strict` and
  `an_unparsed_file_is_named_by_each_caller_as_before`.
- run under `klin gate` and under its own subcommand with the same output
- carry tests through the binary only, on a throwaway tree with a base

## 9. Hook Protocol

### 9.1 Host adapter

Hosts come in two kinds, and a port starts by naming which kind the host is.

A shell-hook host runs a command with a JSON event on stdin and reads a
decision from stdout or from the exit code. Claude Code, Codex CLI, Cursor
and Hermes are this kind. Each one is one module under the host adapter that
implements the adapter's trait: the name `--host` takes, the marker
directory, the hook file, the guard's tool matcher, the key its hook file
lists plugins under if it has one, how an unlabelled event is recognised as
this host's, how the event is read, and how a decision goes back. A port of a
shell-hook host adds that module and registers it in the adapter list. No
other module names a host. The trait MAY grow when a host needs a seam no host
had — the prompt event's name, the tree the event names, the stop channel — and
the existing adapters fill the new method with what they already did.

A program-hook host loads a module in its own process and has no shell hook.
OpenCode and Pi are this kind, and klin cannot be the hook. A shim outside
this crate translates the host's event into the harness protocol of 9.7,
spawns the same `klin` command, and translates the answer back. The shim is
the whole port, and klin gains no variant for it. A shim MUST NOT fabricate a
first-class host's payload to reach klin: the harness protocol exists so that
a harness klin does not maintain speaks as itself.

Two facts shape both kinds. The stop answer is the report text, and the
adapter chooses the channel it goes out on: the exit code, stderr, or a JSON
field. A host's guard event carries a list of paths, because one event may
name more than one file, and the adapter maps that list onto the record
below.

One adapter, chosen from the event's shape or from `--host`, maps a host
event to one internal record:

- `tool` (string), `file_paths` (list), `command` (string) for the guard
- `blocked_before` (bool), the host's flag that the hook already blocked this
  turn
- `session` (string) when the host sends one

And maps one internal decision to the host's output. For Claude Code:
pre-tool decisions go out as `hookSpecificOutput.permissionDecision` with
`allow`, `deny` or `ask` and a reason. Stop blocks are exit 2 with the report
on stderr. A stop that ends with something for the person, such as a note
about a file no grammar read or a deleted test the run let through, writes
it as a JSON `systemMessage` on stdout under exit 0. A stop whose host event
klin cannot read writes that note to stderr and exits 1 instead, by the rule
of 16.5, because klin does not know whose shape to tell it in. Prompt and
session-start text go to stdout on exit 0.
For Codex CLI, `allow` is exit 0 and both `deny` and `ask` are exit 2 with
the reason on stderr, because Codex rejects `permissionDecision: ask` on
`PreToolUse` as unsupported. Stop blocks also use exit 2, and Codex makes the
reason a continuation prompt inside the same turn, which runs no
`UserPromptSubmit`. A stop that tells the person uses `systemMessage` too,
because Codex rejects plain text on a stop that exits 0.

For Cursor, an event carrying `cursor_version` is Cursor's, and Cursor sends
that field on every request. The adapter is
tried after Codex and before Claude Code, because those events also carry
Claude Code's fields. `preToolUse` names `Write`, `Edit` and `Delete`.
`beforeShellExecution` carries the shell command at the top level of the
event rather than under `tool_input`; klin reads a top-level `command` on
that event alone, because `beforeMCPExecution` carries the MCP server's own
launch command there and the agent did not run it. `conversation_id` is the
session. Cursor sends no `stop_hook_active`, so `blocked_before` is always
false and klin's own gate block count and gate tree bound the blocks (9.3). `loop_count`
counts the follow-ups one conversation has already taken and MUST NOT be
read as `blocked_before`. Guard decisions that refuse go out as `permission`
on stdout: both a deny and an ask carry `deny` and the reason in
`agent_message`, and exit 2, because Cursor 3.20.21 accepted `ask` on a
shell event but did not enforce it. A question the host does not enforce
fails closed, as it does on Codex. An allow is exit 0 with no stdout, matching
Claude Code and Codex, so a user-scope plugin stays silent in a tree that
never wrote `klin.json`. Stop blocks print a JSON
`followup_message` on stdout and exit 0. Cursor 3.21.18 did not submit the
follow-up of a stop hook that exited 2, and did submit one from a hook that
exited 0 (measured 2026-09-23, `docs/cursor-compatibility.md`). Nothing
enforces an exit-0 block: a Cursor that ignored stdout would let the turn
end, which fails open. The journal still records the stop as blocked. A stop
that tells the
person writes a JSON `followup_message` on stdout under exit 0. Cursor submits
that follow-up as the next user prompt. Before delivery, klin records a hash
of the exact report or message in a handoff record, one file per session id
of the stop's event under `handed/` in the state directory, for a block and
a told stop alike. Only that session's own hooks write its file, and a host
runs one session's hooks in order, so no write for one session can replace
another session's record, sequentially or concurrently. A prompt of that
session with that hash consumes the record and moves neither the prompt
counter nor the mark. Every different prompt of that session, including one
that starts with `klin:`, clears the record and opens a turn normally. A
prose prefix is not a protocol marker. A block whose report klin could not
record is reported and not blocked, because the host would submit it as a
person's prompt and gain a fresh budget; the block its count already took
stays spent. ADR 0045, ADR 0052.

A told message is also recorded as told under the current prompt, less its
window line, whose age moves each minute and says nothing new. A later stop
under that prompt whose message is the same apart from that line tells
nothing, so a message Cursor submits and the agent answers without a change
cannot replay forever. A different message is told, and the session's next
prompt clears the record. A stop that tells nothing because of this carries
`told-before` in its `flags`. A host that submits a told message hears only
one klin recorded first: a stop that lost the state lock (6.5), or whose
handoff record would not write, tells it nothing, because an unrecorded
message would replay. Every such stop records an empty `told` (11.4). ADR
0052.

Cursor runs a project hook from the workspace root and a user hook from
`~/.cursor`, so the working directory is not the tree on a user-scope
install. Every Cursor request carries `workspace_roots`, and a tool event
also carries the `cwd` a relative path in a command stands on. The adapter
names the tree from `cwd`, then from the first `workspace_roots` entry, and
every command a hook runs measures that tree instead of the working
directory: the guard, the hook-mode gate, and the `radius` that moves the
stamp and the mark of 6.2.1. A host that runs its hooks in the tree names
none, and klin reads the working directory as before. `radius --report` is a
person's command with no event, so it reads the working directory.

Each host names the event a person's prompt raises: it is the last `radius`
line in that host's hook table (`UserPromptSubmit` for Claude Code and Codex,
`beforeSubmitPrompt` for Cursor). The spread report of 9.2 rides that event,
and no module outside the adapter names it. The adapter places the payload
once and the resulting event carries its host, named tree and whether it is
this prompt event; guard, gate and radius do not place or parse it again.

In hook mode the exit code is the host's protocol, not the verdict. On Claude
Code, Codex and the harness protocol, exit 2 means "block this stop", whatever
caused it. On Cursor a block is a `followup_message` under exit 0, as above.
The verdict of section 4.9 lives
in the report and in the `turn` file. Outside hook mode the exit code is the
verdict.

Codex CLI sends Claude Code's
event fields plus `turn_id` on every turn-scoped event. `turn_id` alone
places an event as Codex, and it is tried before Claude Code's fields.
`permission_mode` places nothing, because both hosts send it. A session
start is not turn-scoped, so Codex's is read in Claude Code's shape, which
runs the same `radius`. Codex's `Bash` tool carries a shell command in
`tool_input.command`. An MCP tool carries its own arguments, so the guard
reads a `command` there only when the tool has one. `apply_patch` carries
one or more file paths in its patch headers. The adapter is the only module
that reads a host's JSON.

### 9.2 Events

| Event | Command | Blocks | Writes |
|---|---|---|---|
| session start | `klin radius` | never | `turn` per 6.2, its prompt counter, and the mark of 6.2.1 |
| pre-tool | `klin guard` | deny or ask | nothing |
| prompt submitted | `klin radius` | never | `turn` per 6.2, its prompt counter, and the mark of 6.2.1 |
| stop | `klin gate --hook --changed` | each stop that changed the tree while the build fails, up to eight per turn, and for gates the first failing stop and one more over a changed tree | `build-blocked`, and the verdict in `turn` |

The shared hook lines call `klin` from PATH:

```
klin guard
klin radius
klin gate --hook --changed
```

The host adapter reads which host called from the event, so no flag is
needed in the hook line. `--host NAME` overrides detection.

A stop in a tree that holds no `klin.json` prints nothing and blocks
nothing, per 5.1.

### 9.3 The block policy

ADR 0004 and ADR 0012 hold in policy. A build failure blocks each stop until
the tree builds. A gate failure blocks at most two stops under one prompt,
the second only over a changed tree (ADR 0052). The build stamp in the state
directory carries the facts between the processes.

ADR 0004 relies on the host's cap on consecutive blocks. That cap is not in
the current Claude Code documentation. klin MUST bound its own blocks (ADR
0022). The first failing stop spends gate block 1 of 2. After it, a failing
stop over the tree that block was taken over spends no block: the hook
reports and lets the turn end, so an agent that says a break is intended and
stops again without an edit ends the turn. A failing stop over a different
tree spends gate block 2 of 2, whether the failure is the same finding, a new
one, or a tool error. After gate block 2 no gate failure under that prompt
blocks, whatever the tree. Each gate block names its number in the turn.

klin MUST prove gate block 2 from its own record: the tree gate block 1 was
taken over, recorded in the build stamp, and a current tree that differs
from it. The host's `blocked_before` flag says a block happened and never
which tree it saw. Where klin's record holds no gate block and no build
block, the flag counts as a gate block klin never recorded, so the stop
spends none. It never authorizes a second. When klin cannot read that tree, cannot hash the
current tree, or cannot write the record of the block, the hook reports and
spends no block. That holds for the first gate block too: a block klin cannot
record would read as unspent at the next stop, and a host that sends no
prior-block flag would take it again at every stop, so it blocks nothing, by
the rule of 14 that a state klin cannot keep blocks nothing.

A host that submits the block report as another prompt records that exact
report in the session's handoff record of 9.1 before delivery; the matching
prompt consumes it
without opening another turn, so it raises no prompt counter and brings no
fresh gate budget. The match is by exact text, and a host can submit text
klin did not hand off: Cursor merges every stop hook's answer, and another
hook's `followup_message` can win (9.1). So a stop whose host says it follows
a message the host submitted by itself keeps the build stamp of the prompt
that opened the chain, whatever the prompt counter says, and writes it back
under the current counter. The build stamp names the host session that took
it, and a chain keeps only its own session's stamp. A stop that follows no
automatic message opens its prompt's budget: where its session's stamp names
an earlier prompt, the stop writes a fresh stamp for the current prompt, even
if it spends no block. So a chain that a clean stop opened never inherits the
cap of an earlier prompt. Cursor says so with a `loop_count` above 0, and
Cursor 3.21.18 returns the count to 0 for a person's message
(`docs/cursor-compatibility.md`). That rule never adds a block: where a host
does not reset the count for a person's message, it only withholds that
prompt's fresh budget. A genuine
later prompt brings a fresh budget of two gate blocks and eight build
blocks. A deleted test is the one gate failure that
does not stay red: the stop that blocks on it records the question beside
the stamp, and the next stop lets it through as a NOTE and ends green (8.2,
ADR 0031). A deletion already asked about fails nothing, so it is no reason
for gate block 2. A new deletion may spend a gate block that remains, and
once both are spent it is reported and asked about under a later prompt. A
build failure blocks at each
stop that changed the tree since the last build block, until the tree
builds, up to eight in one turn, and then the hook reports, says that it
stopped blocking, and lets the turn end. A stop over a tree the last build
block already saw spends no block: the hook reports the failure, says the
tree did not change, and lets the turn end, because a block over a tree the
agent did not touch teaches it nothing (ADR 0048). The build
stamp holds the count, the prompt counter of 6.2 the count was taken
under, and the tree of 6.5 the last block was taken over, apart from the
gate block count and the tree the last gate block was taken over. A build
block changes only the build fields and a gate block only the gate fields.
A count taken under an earlier prompt reads as zero, so every turn
has eight build blocks and two gate blocks, and only the stop writes the
build stamp. A build-failure
stop writes a RED verdict before it blocks, so the next prompt does not move
the turn stamp over a tree that does not build. Each block names its number
in the turn, and a failing build's report opens with the `derived:` line of
5.4 when the command was derived.

A derived build for a JavaScript project runs the tool that project
installed. From the directory the entry runs in, and then each directory
above it up to the root klin measures and never above it, the nearest
`node_modules/.bin` that holds the tool names it, written relative to the
directory the entry runs in. The tool on `PATH` runs when the checkout
installed none.

A tool the project installed is there even when it cannot run, so a broken
install, such as one whose link points nowhere or one the host cannot
execute, fails its own build and klin does not quietly compile with another:
the 127 of ADR 0048 is an absent tool only for a command klin did not resolve
to a binary in the checkout. klin never invokes `npx`, `npm exec` or any
other command that could fetch a tool, and it starts no package manager. This
applies to a derived JavaScript command alone: a derived `cargo` or `go`
command, and every command a person wrote, run exactly as they read, in the
environment the hook itself was given. A tool a project installed but did not
put on `PATH` is therefore found, and the NOTE below is not told for it.

Each build command runs for at most 300 seconds, and klin stops every build and
`run` command of one run at 600 seconds after klin started, so a later command
gets only what is left before that deadline, and a command with no time left
does not start. The deadline counts klin's own work between commands too. The
host gives the Stop hook 900 seconds, so when klin starts as the stop begins,
no command is still running when the host's time runs out, and whatever klin
does after its last command has at least 300 seconds. Time the host spends
before klin starts, such as a `cargo run` that compiles klin first, is not
counted. When a command reaches its limit or the deadline, klin stops it and
the build fails with a message that names the command and the limit or deadline
it reached. `klin.json` cannot change either: a person whose build takes longer
sets `build` to `false` and lets CI build. A run MAY take `KLIN_COMMAND_LIMIT`
in seconds for tests. It sets the limit of each command, from 1 to 300 seconds,
and the deadline is twice it. Any other value is an error, so the override can
shorten the limit and the deadline and never raise them.

Each command runs in a process group of its own. When the command's shell
exits, when it reaches its limit or the deadline, and when a hangup, an
interrupt or a terminate signal ends klin, klin sends a kill signal to every
process left in that group before it goes on. So a command that starts a
process in the background leaves nothing running that could change the tree
after klin judged it. A process that makes a group of its own leaves klin's
reach, and so does every process when klin is killed with a signal it cannot
catch.

A build whose shell exits 127 is not a build failure. The shell could not
find the command, so the tool is absent and the code is unjudged. The hook
records the build as unmeasured: one NOTE names the command, quotes the
shell, and says that klin judged the source as it stands, that CI runs the
build, and that the action left is to install the project's dependencies or
for a person to set `build` to `false`. The gates then run over the tree,
so a `lockfile` finding for the dependency that was declared and never
installed reaches the agent, and the NOTE is told at a stop nothing blocks.
The exit code is the whole test: klin reads no shell message and guesses no
tool name. An absent tool skips its own entry and no other, so a later entry
that fails still blocks, and the NOTE stands only for a build in which every
entry that ran passed (ADR 0048).

Two facts in ADR 0014 about where hook output goes on exit 0 need one more
check against the current documentation before #91 lands. The documentation
read for this draft says Stop-hook stdout on exit 0 reaches the model. If that
holds, the last turn of a session can carry a radius report at its stop.

### 9.4 The guard's three decisions

The guarded set is this tree's `klin.json` and this tree's own state
directory. The guard resolves both: the configuration beside the tree root
`git rev-parse --show-toplevel` names, and the state directory of 7.4. A
host's hook file, CODEOWNERS, `refs/worktree/klin`, every verification file,
another project's `klin.json` inside the tree, and another worktree's state
directory are ordinary files, and an edit to one of them is `allow`. ADR
0027, ADR 0032 and ADR 0033 record why.

The guard answers only where it can prove that a tool call writes a guarded
path. Everything else is `allow`, and a write the guard misses costs nothing,
because ADR 0009 makes CI authoritative. ADR 0033 records the reversal.

- `deny`: an edit tool whose path is the configuration or reaches inside the
  state directory, a redirect onto either, `init` in any form, `install` in
  any form, and `turn reset`. The reason names the file and says a person changes it in a
  reviewed commit, or names the command a person runs instead: `klin turn
  reset` for the stamp, and `klin cache clean`, which stays open to an agent,
  for the cache. Nothing else denies.
- `ask`: a command that writes every argument it takes, with a guarded path
  among its arguments. The writers are `rm`, `rmdir`, `unlink`, `shred`,
  `mv`, `truncate` and `tee`, and `sed` or `perl` carrying `-i`. `cp` and
  `install` are not writers here, because each reads its first argument. The
  reason quotes the token that matched. The person decides.
- `allow`: everything else, including every command that only names a guarded
  path.

A path resolves from the directory the guard runs in, with `.` and `..` taken
out and every symbolic link its existing part carries followed. A path the
guard cannot resolve is `allow`. A token holding a shell wildcard proves
nothing about the file it stands for, so the guard matches no path against
it, and `rm *.json` in the tree root passes.

The guard matches no path in a command that holds shell it does not read: an
unbalanced quote, `$(`, a backtick, `${`, `<<`, a backslash, or a `cd`
command word. An unbalanced quote allows the whole command, because the
command then does not say where its arguments end. The other six suppress
path matching alone. `init`, `install` and `turn reset` name no path, so their `deny`
stands behind any of them, and a command substitution in command position is
a prefix in front of klin's own name.

A command splits into segments at `;`, `&`, `|` and a newline, and the split
MUST honor single and double quotes (issue #90). A redirect is an unquoted
`>`, so a `>` inside an argument is a character of that argument and not a
redirect. A writer and klin's own name are read by the basename of the
command word, and each is found behind the same prefixes: an assignment,
`env`, `npx`, `pnpm`, `bunx`, `time`, `nice` and `sudo`. So a script of the
tree's own named `rm` is read as `rm`, and `sudo rm klin.json` asks.

For Codex CLI, an `apply_patch` call is judged by every path in its `*** Add
File:`, `*** Delete File:`, `*** Update File:` or `*** Move to:` headers.
Patch body text is data. A deny for any path wins; otherwise an ask wins over
allow.

The guard MUST NOT read the configuration. It runs before the config loads.
It MAY read `KLIN_STATE_DIR` and run `git rev-parse` to resolve the tree root
and the state directory, and it reaches for git only when a path needs
proving. It MUST finish in under 50 milliseconds, because it runs on every
tool call.

### 9.5 What the hook prints

On a block, one lead line that says how many gates failed, what to do, and
which gate block of two the stop spends, then the report. On a stop that
spends no gate block, the same report and a line saying why klin does not
block again and that the window stays open until a person fixes, accepts or
resets it. Never the command that accepts debt, and never `turn reset`.

The hook's text report is focused. It prints every gate that did not pass,
`FAIL` and `ERR` alike, each with its own `derived:` and `pinned:` lines and
its own output. A gate that passed prints nothing, unless it left a note the
hook tells (8.2, 14): a file no grammar read, a file no semantic adapter
measured, a deleted test let through, a derived ceiling whose scope fell
back, a dependency a resolver could not resolve, a build tool the shell could
not find, or a file the run could not read or stopped measuring. Such a gate
prints its provenance, its row and its output without its `OK:` line, so the
note stands with the gate it belongs to. The run-level lines and the closing
count stay. The 11.2 object and the journal record every gate, whatever the
text printed. `klin gate` outside the hook prints every gate.
The `--json` form is available for a host that reads JSON.

A stop that nothing blocks can end a turn, and on a turn that caught a
regression (11.5) it tells the person what happened in one `systemMessage` of
9.1. A green stop names what the turn caught and what became of it: `klin
caught 2 regressions this turn. All 2 were fixed after klin flagged them.` A
red pass-through names what still needs the person: ``1 regression still needs
your attention. `klin stats --turn` shows it.`` The line and the report of 11.5
come from the same words and the same counting rule, with one exception:
where every regression still open in the turn is a `public-api` finding, the
red pass-through names the problem and not the count, ``Public API
compatibility breaks still need your attention. `klin stats --turn` shows
them.``, because one intended removal can be several breaks (ADR 0054). The
counting and `klin stats --turn` do not change. Pinned by
`a_red_pass_through_whose_open_regressions_are_all_public_api_names_the_breaks_not_a_count`.
Neither the line nor the report says who authored a fix: the journal proves a
regression was present and later absent from a measurement, and nothing more. A turn that caught none prints no such
line, and a session start prints none. A fix in a later prompt of the same turn
still gets its line, because the turn stamp records the intervention (6.5)
until the stamp moves.

At most once every seven days, and only when the journal holds seven days,
one sentence about the week follows the line, with the command that lists it:
``klin caught 9 regressions in the last seven days. 8 were fixed after klin
flagged them. `klin stats` shows the rest.`` Where the week's regressions were
all fixed, the command sentence reads ``klin stats` shows them.`, and where
none were, the sentence ends after the count. A command in the message stands
in backticks, and the message never names a command that accepts debt. The
notes the run left (8.2, 14), the turn-end line and the weekly line join in one
message, in that order. The stop reads the journal for this only where the turn
stamp records an intervention, or where the prompt spent a gate block
(16.3), it reads the bounded tail of 11.4 and never the whole file, and its
journal line records which parts it printed (11.4). The word `shortcut` appears
in no message klin prints for a person.

A red pass-through under a prompt that spent a gate block also checks the
journal for a `prompt` line carrying the stop event's session id. If the event
has no session id, it checks nothing. If no such line exists, the stop adds a
note to its `systemMessage`: ``klin: no prompt event reached this session; klin
grants no fresh gate blocks until `klin radius` runs on session start and on
prompt submitted.`` It still exits 0 and changes neither the block nor the verdict.
The note is a `note` in `told`, and the stop carries `no-prompt-event` in its
`flags`.

### 9.6 The journal

Every `klin gate --hook` stop MUST append one line, the record of 11.4, to
`journal.jsonl` in the state directory — the stop that blocks, the stop that
passes, and the stop that could not run its gates alike. Two stops write
nothing: a tree that holds no `klin.json` runs nothing, per 5.1, and a config
only a person can fix ends the stop before it reads anything, per 14.

Three more events append the other kinds 11.4 defines. `klin radius` on a
`UserPromptSubmit` event MUST append a `prompt` line; a `SessionStart` event
moves the mark and raises the counter of 6.2 the same way but appends no
line of its own, because it opens a window and ends no turn. The guard MUST
append a `guard` line for an `ask` or a `deny`, and MUST append none for an
`allow`, because the guard runs on every tool call under its 50 millisecond
budget (13) and an allow tells a reader nothing. An allow the host refused
anyway is a `deny` line under the reason `host-refusal`: a custom integration
on a protocol version klin does not speak refuses every call (9.7), and the
line is the only record of why the agent is blocked. `klin turn reset` MUST
append a `reset` line.

The write MUST NOT change a block or a pass: it is best-effort, a failed
append prints nothing to the agent, and a state directory klin cannot write
costs the record and nothing else, by the rule of 14. The hook never prunes
the file, and `cache clean` leaves it alone (7.4). Only the reader of 11.4
tolerates what an interrupted writer can leave: a truncated last line.

### 9.7 The harness protocol

A harness klin does not maintain speaks klin's own event shape rather than
another host's, so no port has to fabricate a first-class host's payload.
`klin_protocol` names the version, and it is the field that places the event:
no host klin maintains sends it, so the protocol's adapter is tried before all
of them. `--host harness` overrides detection, and refuses a payload that
carries no version of this protocol. The adapter's name, and so the `host` of
a journal line (11.4), is `harness`. Before 1.0 the protocol was called
`generic`, and that name names no host now.

Version 1 carries `event`, one of `session`, `prompt`, `pre_tool` and `stop`,
and then `root`, `session`, `prompt`, `tool`, `file_paths`, `command` and
`blocked_before`. `harness-protocol/event.schema.json` is the checked-in
schema. The adapter maps these onto the one internal record of 9.1 and nothing
further: klin gains no second turn, guard or gate engine for a custom harness.

The protocol carries only evidence the harness proves. `file_paths` holds the
paths the harness can prove the call will touch, and `command` holds a command
only where the harness knows the one the agent is about to run. Missing
evidence stays missing: klin MUST NOT read a file write out of a tool name or
out of opaque tool arguments, and a call that proves neither a path nor a
command is allowed.

The decision is one JSON object on stdout, under
`harness-protocol/response.schema.json`: `allow`, `deny` with a `reason`,
`block` with the report as its `message`, or `tell` with a note as its
`message`. Exit 0 carries `allow` and `tell`, exit 2 carries `deny` and
`block`, and a refusal also goes to stderr so it holds where stdout goes
unread. At most one decision is printed, on a line of its own, beside the
report text a blocked stop also prints. A stop that passes prints no decision,
as it does on every other host, so exit 0 with no decision ends the turn. No host-specific field of Claude Code, Codex CLI or Cursor appears in
this protocol. There is no `ask`: nothing here proves a question the harness
enforces, so the ambiguous class of 9.4 fails closed as it does on Codex and
Cursor, and 19.4 has the integrator record that difference.

A `klin_protocol` klin does not speak fails clearly and closed. klin names the
version the event sent and the version it speaks, refuses every tool call and
blocks every stop while that mismatch stands, and MUST NOT read the event as
another host's.

### 9.8 One copy per host event

A host may run several copies of klin's hooks for one event on one machine: a
native plugin beside committed project hooks or user-scope hooks, and Cursor,
which runs the hooks in Claude Code's settings files beside its own by
default. Exactly one copy takes effect per event. klin compares no hook
line and knows nothing about the other copies. Each copy reads the same
payload, because the host sends one event to all of them.

**The event's identity.** An event's identity is the values of the fields that
name it, as each host sends them (`docs/HOST_COMPATIBILITY.md`):

- Claude Code and Codex CLI: `session_id`, `hook_event_name`, `prompt_id`,
  `turn_id`, `tool_use_id`, `source`, `stop_hook_active` and
  `last_assistant_message`, where the payload holds them.
- Cursor: `conversation_id`, `generation_id`, `session_id`,
  `hook_event_name`, `tool_use_id`, `tool_name`, `tool_input`, `command`,
  `cwd`, `status` and `loop_count`. Cursor sends every copy the same payload
  in its own shape, including a copy it imported from Claude Code's settings
  or a Claude Code plugin. A shell call is the one exception: Cursor's own
  hook receives it as `beforeShellExecution` and an imported one as
  `preToolUse` on the `Shell` tool. So a shell call is named by
  `conversation_id`, `generation_id` and its command alone.
- A custom harness (9.7): none. It runs one copy per event.

An event has an identity only when its payload names a session and one field
that scopes the event inside the session: `prompt_id`, `turn_id`,
`generation_id`, `tool_use_id` or `source`. A payload without both cannot tell
two copies of one prompt from two prompts, or two sessions from each other.
Such an event is run by every copy, as before this section.

**The claim.** Each copy that acts on an event first claims its identity in
`claims/` in the state directory (7.4). The claim is held while the copy acts.
A copy yields when another copy holds the claim, or when that copy let the
claim go less than two seconds before. The host starts every copy of one event
together and starts the next event only after all of them answered. So a late
copy arrives within a process start of the first one, and two different
events with one identity are a model turn apart. A claim older than the window
belongs to an earlier event, and the copy that finds it takes it. A copy that
cannot write the state directory acts, because an event run twice is the
older failure and an event run by no copy would drop a block.

**What a copy that yields does.**

- `klin radius` prints nothing and exits 0. It moves no stamp, no mark and no
  prompt counter, writes no journal line and consumes no follow-up of 9.1.
  The claim comes before the follow-up is consumed.
- `klin gate --hook` prints nothing and exits 0, and runs no gate. It cannot
  weaken the other copy's block: Claude Code and Codex apply the most
  restrictive answer across hooks.
- `klin guard` gives its answer as every copy does, and writes no journal
  line. The guard's answer is the same in every copy, and a copy that let a
  call through because another call had the same identity would open that
  call. So only the journal line depends on the claim.

Two different events, and events from two sessions on one worktree, never
have one identity. A late copy whose wrapper took longer than the window to
start, such as a plugin wrapper that downloads its binary on first run, acts
on the event again, so for that one event the prompt counter moves twice.

## 10. Runner and CI Contract

- `klin gate` runs every applicable gate in catalogue order, which is cheapest
  first (4.6), and prints a status row per gate, the full output of each
  failing gate, and one summary line.
- `--gate NAME` runs one gate. Naming an excluded or unknown gate is exit 2.
- `--strict` adds four failures: an accepted entry matching nothing, a
  same-tree comparison klin cannot explain (6.4), a file measured in `before`
  and not in `after` though present in both (8.6), and a survey that finds no
  source root. The last one closes the hole the retired unaccounted-gate
  failure of ADR 0010 used to close: a CI job in the wrong directory or over
  a clone with no base would otherwise apply every gate to nothing and print
  green. A config error is exit 2 in every mode but the hook (14), so it is
  not on this list.
- `--hook` with `--strict` is a usage error: the reason goes to stderr with
  exit 1, which no host reads as a block. The two flags name two callers.
- `--changed` scopes to the window's changed files.
- `--list` prints applicable gates with one `pinned:` line per value their
  section states, excluded gates, and gates that need a section a person
  writes. It derives nothing, so a value a section leaves out is said by the
  run that derives it.
- CI SHOULD run `--strict` and MAY name gates on the command line. Naming
  them is no longer required to catch a deleted section, because a deleted
  section is a derived one.
- CI MUST fetch enough history to resolve the base. `fetch-depth: 0` is the
  RECOMMENDED setting.

## 11. Output Contract

### 11.1 Text

Stable, tested line shapes. `ok    NAME`, `FAIL  NAME`, `ERR   NAME` for
rows. `OK:` for a passing gate's one line, printed under its row by the
runner and on its own by the gate's subcommand, because 8.6 asks for the same
output from both. `FAIL:` for a failure with its remedy under it. `NOTE:` for
a note. One `window:` line first. One `derived:` or `pinned:` line per value
a gate used, above that gate's row, and the hook's build lines above every
row.

Every `OK:` line ends in the gate's coverage, as `(N file(s) found, N
measured, N not measured, N excluded, N unreadable)` when the check has
unmeasured files; otherwise it keeps the four-count shape
`(N file(s) found, N measured, N excluded, N unreadable)`. The boundary is
the same for every check: `found` counts every file the check's own discovery
rule reached under its roots, before anything dropped one; `excluded` counts
the ones an exclusion dropped; `unreadable` counts the ones it reached and
could not read or parse; `not measured` counts known-language files for
which this check has no structural adapter; `measured` counts the ones it
judged. A scoped run counts only the files in its scope. A check whose scope
is not a set of files counts the thing it discovers — a document, a manifest,
a test file the base holds — and its module docstring names that thing.

Every failure line ends in what the ratchet judged it against: `— matched the
base site at FILE:LINE`, `— matched the accepted entry for FILE`, or `—
nothing matched` for a `new` finding, with `, ceiling C` after it wherever
the gate has a ceiling of its own. So an agent fixes the site the ratchet
compared, and a person can dispute a wrong match.

### 11.2 JSON

One object on stdout. Fields:

- `status`, the row of 11.1 for the run as a whole, and `summary`, the one
  line 11.1 ends with
- `window` `{kind, before, after, how}`
- `derived` list of `{section, key, value, rule}`
- `gates` list of `{name, status, findings, notes, coverage, ms, held, facts,
  names, work, graph, surface}`,
  where `status` is the row of 11.1, `findings` and `notes` are how many that
  gate left in the two lists below, `coverage` is the
  `{found, measured, not_measured, excluded, unreadable}` counts of 11.1,
  or null for a
  gate that could not run far enough to measure a scope, `ms` is how long the
  gate's own measure and judge took, and `held` counts the findings the run
  let through because a base site or an accepted entry carried them, which is
  one quantity and not two: a gate that also drops sites its window never
  reached counts those in its `coverage` and never in `held`. `accepted`
  counts how many of `held` a person-authored entry carried rather than a base
  site, which is the one number the `OK:` line's qualifier of 8.6 turns on, so
  the text and the JSON read the same comparison. `held` less `accepted` is
  what the base held. Both are null for a gate that never got that far, and
  `accepted` is 0 for a ratcheting gate no accepted entry matched. `facts` is
  `{reads, parses, extracted, shared, cached, ms, cache_read_ms,
  cache_write_ms}`, with `states` beside them for `dead-symbols`, for a gate
  that reads structural
  facts (8.4), and null for any other gate or for one that never got that far:
  `reads` and `parses` count the files of both trees whose content this gate
  read and parsed, `extracted` counts the files whose structural outcome it
  extracted itself, `shared` counts the files whose outcome it took from an
  extraction the run already held, which is an earlier gate's extraction of
  the same tree or, for `dead-symbols` and `reachability` in a changed run
  that is not strict, the base's extraction of an unchanged working-tree file
  (8.4), `cached` counts the files whose outcome it took from the structural
  cache of 8.4, which a file counts under the first time a gate of the run
  takes it and under `shared` after that, `ms` is
  the part of the gate's `ms` spent on its own extractions, and
  `cache_read_ms` and `cache_write_ms` are the parts spent reading and
  writing the structural cache. `states` counts the declaration states
  `dead-symbols` built over both trees (8.4), and no other gate's `facts`
  carries the key. A run extracts each file of a tree once.
  Each gate still selects its own files and resolves names over those files
  alone, so `facts` and `names.base_ms` are the fields of a row that depend on
  the other gates a run selects. `names` is `{base_ms, before, after}` for
  `dead-symbols` and `reachability`, with `lost_ms` beside them for
  `dead-symbols`, and null for any other gate or for one that never got that
  far. `base_ms` is the part of the gate's `ms` spent laying the base tree out
  and naming and reading the structural cache of 8.4, so it holds
  `facts.cache_read_ms`, and only the first gate of a run that needs the base
  pays it. `names.layout` divides the whole base's layout into its parts,
  each in milliseconds: `worktree_add_ms` (every git command that laid the
  linked worktree out), `changes_ms` (looking the change set up), `renames_ms`
  (moving renamed files to today's paths), `cache_name_ms` (naming the
  structural cache, which asks git for the checkout identity of 8.4),
  `ignored_ms` (asking git what the base tree ignores) and `walk_ms`
  (walking the base tree's directories into its file list). `written` is how
  many base files a light layout (8.4) wrote from the base commit's index, and
  null for a base checked out whole. A base checked out whole walks the
  checkout, and a light layout reads its file list from the index, so
  `ignored_ms` and `walk_ms` are zero on a light layout. A run lays the
  whole base out once, so the parts sit on the row of the first name-resolving
  gate that asks for them and are null on every other row, and on every row
  of a run whose base was laid out scoped. `facts.cache_read_ms` is reading
  and decoding the cache alone. `before` and `after` are each `{measure_ms,
  index_ms, query_ms, files, declarations, references, distinct_names}` for
  one tree.
  `measure_ms` is the part spent selecting and measuring the tree's files,
  which holds that tree's share of `facts.ms`. `index_ms` is the part spent
  building the tree's name index, and `query_ms` is the part spent judging the
  tree's declarations or family members through that index. `files`,
  `declarations`, `references` and `distinct_names` count what the index
  holds: its files, their declarations, each name's reference sites once per
  file and line, and the names of each language partition. `lost_ms` is the
  part spent building the working tree's findings and the lost references
  that explain them. The counts depend only on the trees and the selection.
  `footprint` is what the facts `dead-symbols` held over both trees cost, and
  null for any other gate or for one that never got that far. A file the two
  trees hold as one extraction counts once. It holds the populations `files`,
  `declarations`, `references`, `imports`, `module_declarations`, `exports`,
  `export_leaves`, `qualified_paths` and `extern_crates`; the owned bytes
  `path_bytes`, `declaration_name_bytes`, `declaration_text_bytes` and
  `reference_name_bytes`; how many declarations carry a signature, an owner or
  an exported alias, and the bytes each of those holds, as `signatures`,
  `signature_bytes`, `owners`, `owner_bytes`, `exported_aliases` and
  `exported_alias_bytes`; over reference names it also carries
  `reference_distinct_names`,
  `reference_canonical_allocations`,
  `reference_canonical_allocation_ratio_milli`,
  `reference_canonical_bytes`,
  `reference_representation_before_bytes` and
  `reference_representation_after_bytes`; the JSON row also carries the
  floating-point `reference_canonical_allocation_ratio` for the same
  allocation/distinct-name ratio; `nestings`, `nesting_entries` and
  `nesting_bytes`
  over every value that carries inline module names; `import_text_bytes`,
  `export_text_bytes` and `module_text_bytes`, which hold each statement's own
  text and the names and paths it carries, with a qualified path under the
  module bytes; `extern_crate_bytes`, the crate name and alias of each
  `extern crate`; and `sizes`, which includes `name` and gives the size of one
  `file_facts`,
  `declaration`, `reference`, `import`, `module_declaration`, `export`,
  `export_leaf` and `extern_crate` without the bytes their strings and lists
  own. Every value
  depends only on the trees and the selection (8.4). `footprint.references`
  counts the reference values the facts hold, so it is at least
  `names.before.references` plus `names.after.references`, which count the
  sites an index keeps, one per file and line for each name.
  `work` is `{reads, parses}` for the file-local gates
  `complexity`, `escapes` and `stubs`, counting source contents read and parsed
  over the current and base trees; it is null for other gates or for a gate
  that never got that far. `graph` is `{modules, sources, dependencies, edges,
  dispatches, ms}` for `layering` and `public-api`: the modules of both
  trees' module graphs, the files those modules hold, the dependency sites,
  the distinct pairs of modules those sites join, how many times each
  structural language's resolver ran, by language name, and the part of the
  gate's `ms` spent resolving them and, for `layering`, finding their cycles.
  It is null for other gates or for a gate that never got that far. `surface`
  is `{surfaces, items, measured, opaque, holes, dispatches, ms}` for
  `public-api`: the surfaces and items derived over both trees, how many items
  are measured and opaque, the holes inside the surfaces, how many times each
  language's derivation ran, and the part of the gate's `ms` spent deriving
  them. It is null for other gates or for a gate that never got that far.
- `findings` entries per 4.5 with `id`, `condition`, `fix_advice`,
  `ceiling`, and `matched`, which is the `before` site or accepted entry as
  `{file, line, text, accepted, values}`, or null for a `new` finding. The
  list carries the run's failures, so a `held` site is not in it: a held
  finding is not a failure, and the gate that prints one says so on its own
  `OK:` line. `ceiling` is the ceiling in force as text, or null for a gate
  whose only ceiling is the value the base holds. `id` is a hash of the gate name,
  the file and the declaration text, so it is the site identity of 4.4 in one
  token, and a harness can follow one finding across stops without parsing
  the rest. The id follows the path, so a file rename changes it while the
  site of 4.4 survives. Two findings at one site share the id, because 4.4
  keys a site by its file and its declaration text and pairs the findings
  there one to one. The list also carries the `error` and `unparsed` records
  of a run that could not measure something (14). Those name no site's
  values, so they carry no `id`, `ceiling` or `matched`, and their `outcome`
  says which kind each one is.
- `notes` entries of `{gate, outcome, file, text}`, plus the site's `line`
  and `values` where the note has them. `outcome` says which kind each one
  is: `unmatched` for an accepted entry that matched nothing, `unparsed` for
  a file a grammar refused in the hook or that the base held refused too
  (8.6), `lost` for a file `before` measured
  and `after` did not (8.6), `not-measured` for a known-language file with
  no structural adapter, `derivation` for a derived ceiling whose recorded
  scope fell back or differs from today's (5.4), `unresolved` for a
  dependency form `layering` supports and could not resolve, or a form inside
  a public surface `public-api` recognizes and could not resolve, in the hook
  or held at the base (8.2.1, 8.6), and `note` for what a check left out of
  its count. Outside the hook any other `unresolved` record is in `findings`,
  beside `error` and `unparsed`.
  `text` carries the reason, as the `NOTE:` line printed it.
- `exit` integer, the code the run returns. It is not read off
  `status`: a build failure that has spent its blocks is an `ERROR` run that
  exits 0, so a harness that wants the process's answer reads `exit` and one
  that wants the verdict reads `status`. Under `--hook`, 16.3 decides the
  stop's code after this object is built, so the journal line of 11.4 is the
  record that carries it in `exit`: 2 for a stop that blocks, 0 for one that
  passes.

A finding has no column, so the JSON carries none rather than a wrong one.

### 11.3 SARIF

Not shipped: klin has no `--sarif` flag. #65 recorded the design below and
closed without shipping it. #207 keeps the problem for the next product
decision.

`--sarif PATH` would write SARIF 2.1.0 to the file `PATH` names, with one rule
per gate and one result per failing finding (#65). Text output on stdout is
unchanged by the flag, which is why the log goes to a file and not to stdout
the way `--json` does. `--sarif` with `--json` would be a usage error.

### 11.4 The journal record

One JSON line per event of 9.6. Every kind shares four fields:

- `schema` integer, 1 for this record. A reader MUST skip a line whose
  `schema` it does not know, and MUST count what it skipped so a report can
  say so. A schema bump without an upgrade in the reader MUST NOT ship.
- `version`, the klin version that wrote the line, and `time`, seconds since
  the epoch.
- `kind`, `"stop"`, `"prompt"`, `"guard"` or `"reset"`.
- `session`, the host's id for its grouping of turns, null where the event
  carried none or named no host, and always null on a `reset` line, which
  `klin turn reset` writes over no host event at all.

The `stop` line is the 11.2 object the stop's run built — the gates, a build
failure, or an error alike — plus what only the hook knew:

- `host`, the adapter's name, null where the event was unreadable.
- `prompt`, the counter of 6.2 the stop ran under.
- `hook` `{blocked, delivery, gate_spent, gate_blocks, gate_block,
  build_blocks, blocked_before, continued}`. `continued` is whether the host
  said this stop follows a message it submitted by itself (9.3). `blocked` is
  whether this stop blocked, whatever exit code its host takes
  for a block (2 on Claude Code, Codex and the harness protocol, 0 on
  Cursor, 9.1).
  `delivery` is `block` or `none`; `follow-up` and `report` are reserved for
  a host whose stop cannot block (#67). `gate_blocks` and `build_blocks` are
  the build stamp of 16.3 as this stop left it, and `gate_spent` is whether
  `gate_blocks` is above zero. All three are cumulative, so a later stop sees
  them too. `gate_block` is the number of the gate block this stop itself
  spent, 1 or 2, and null on every other stop: an intervention is a failing
  gate on a line whose `gate_block` is set (ADR 0034, ADR 0052).
  `blocked_before` is the host's flag.
- `verdict`, `green`, `red`, or `none` for a stop that wrote no verdict, with
  a `why` string beside `none` that names the reason this stop had and no
  other: the lock timed out, the state directory could not be readied, it held
  no stamp klin could read, or the stamp klin read could not be written back.
- `timing` `{total_ms, build_ms, lock_ms, base_remove_ms, base_prune_ms,
  klin_ms}`, where `klin_ms` is the total less the build, so the budget of 13
  reads straight off it. A stop that laid the whole base out removes its
  linked worktree before it writes this line, so `total_ms` holds the removal,
  and `base_remove_ms` and `base_prune_ms` are the parts `git worktree remove
  --force` and `git worktree prune` took, zero for a stop that laid out no
  whole base.
- `asked`, the site ids this stop asked about, as the turn stamp records
  them (8.2).
- `flags`, the unusual paths this stop took, empty on a clean stop:
  `turn-restored` (16.1), `branch-fallback` (a stop that judged a branch
  window because no stamp resolved, or because the commit the stamp was taken
  over is outside current HEAD history, 6.2), `count-unwritable` (a build
  stamp that would not write, 14), `told-before` (9.1, a message this prompt
  already told a host that submits it as a prompt, told nothing this time),
  `no-prompt-event` (16.3, a prompt that
  spent a gate block found no prompt line for the stop's session).
- `told`, the parts of the `systemMessage` this stop printed for the person,
  empty where it printed none: `note` (8.2, 14, 16.3), `turn` and `weekly`
  (9.5). A reader finds the last weekly line from it.
- `window`, the window the stop judged, which the line carries even where its
  run compared nothing against a base, so a reader knows which turn stamp
  each stop judged against (11.5).
- `config_hash`, a hash of the config file in force, so a later reader can
  tell a fix from a config change without a schema bump. Recorded and not
  read.

The `prompt` line is the `UserPromptSubmit` event `klin radius` ran on:

- `prompt`, the counter this event raised it to.
- `text`, the prompt's first line cut at 80 characters. Absent where
  `journal.prompt` (5.2) is `false`, where the configuration would not load,
  or where the event carried no prompt text.
- `radius` `{lines, formatting, moved, directories, wide}`, the facts `klin
  radius` measures (ADR 0014), present only where radius could measure them:
  a first prompt, with no mark yet to measure from, carries none. `wide` is
  whether the turn spread past the project's usual change, the same test
  that decides whether `klin radius` prints its note.

The `guard` line is one `ask` or one `deny`, never an `allow`: the guard
runs on every tool call under its 50 millisecond budget (13), and an allow
tells a reader nothing.

- `decision`, `"ask"` or `"deny"`: the answer the host delivered, not the one
  the guard reached. A host with no question to ask refuses instead (9.1), so
  an ask that reached the agent as a refusal is recorded as `"deny"`.
- `reason`, a hyphenated tag naming why the guard decided as it did:
  `config-write` and `state-write` for a proven write an edit tool's path or
  a redirect names, `config-mention` and `state-mention` for a command that
  only names a guarded path as a writer's argument, which klin cannot prove
  the way it proves the first two (ADR 0033), and `init` and `turn-reset`
  for one of klin's own subcommands that only a person runs.

The `reset` line is `klin turn reset`:

- `prompt`, the counter the stamp carried over. A reset does not end the
  turn, so the counter is unchanged by it (6.2).

A reader on the stop path MUST NOT read the whole file, whose length is the
worktree's whole lifetime. It reads backwards from the end and stops at the
first line older than the history the stop's decision needs: the seven-day
cutoff the weekly line of 9.5 reads, or the second before the turn stamp was
taken where the turn reaches further back. The stamp's own second cannot be
the bound, because the `klin radius` run that appends a `prompt` line takes
the stamp after appending it. That stopping line is also how the stop
tells a journal reaching further back from one beginning inside the window,
which the weekly line of 9.5 asks about. One bounded reader answers for the
whole stop: the turn end and the `no-prompt-event` check of 16.3 read one
tail and not one each.

The bounded reader MUST NOT stop at a line it could not parse or whose
schema it does not know: a truncated last write is not an older line, and a
line from a newer klin is skipped and counted like any other. It MUST NOT
report a `prompt` line absent because a byte or line limit was reached. A
worktree holding no readable stamp bounds nothing and reads the whole file.
`klin stats` (11.5) keeps the whole-file reader, because its windows ask for
history the stop path never needs.

The file is a public surface. `klin stats --json` (11.5) is what a harness
reads, and the two readers of the design read nothing else.

### 11.5 `klin stats`

`klin stats` reads the journal of 11.4 and reports what needs the person's
attention and what klin was worth, for the person and not for the agent. It
re-runs no gate and reads no working tree: it interprets the record. `--since
Nd` sets the window, seven days by default. `--all` adds the evidence and the
audit trail. `--json` prints the facts instead of the text. The command exits 0
whatever it finds: it reports and judges nothing.

Two more scopes replace the window of days, and the three exclude each other:

- `--turn` reads the lines at or after the time the current turn stamp was
  taken (6.2), and none where no stamp is readable. A journal time is a whole
  second, so the last `reset` line bounds the scope too: a reset starts the
  report over, as it starts the judgment over.
- `--session` reads the lines carrying the newest `session` id the journal
  holds, and the `reset` lines among them, which carry none and still set aside
  the regressions before them.

#### The counted unit

A **regression** is one finding site that a blocked stop put in front of the
agent because it was new or worse than the base. The blocked stop is one whose
line records a `gate_block` (11.4), and a build block is none. A line an older
klin wrote records no `gate_block`, and its `blocked` stands in (ADR 0052). It
is the person's unit, and
the word `shortcut` MUST NOT appear in any text `klin stats` or a turn end
prints.

The same site over several blocked stops in the window is one regression, with
its latest outcome. A regression's identity is the finding `id` of 11.2 where
the record carries one. That id hashes the gate, the path and the declaration
text, so a rename produces a different id. The reader MUST stay conservative
and MUST NOT merge two ids because their text resembles each other. A record
carrying no usable id — `doc-size` is one such shape — is keyed by the smallest
conservative key its recorded fields allow, which is its gate, file, line and
text. Such a record MUST NOT be dropped, and two distinguishable findings MUST
NOT be merged. A reader MUST NOT deduplicate by line number alone.

The **outcome** is a relation between the lines, which the writer never stores:

- `fixed-next`, the site was absent from the first stop after it was flagged
  that measured its gate
- `fixed-later`, the site was absent from a later measuring stop, and the
  record says how many measured stops it took
- `config-changed`, the site went from a measurement taken under a
  `config_hash` (11.4) other than the one in force when it was flagged. The
  report MUST NOT call that a code fix
- `set-aside`, a person moved the turn stamp before the site went. The report
  MUST NOT call it fixed, MUST NOT call it currently open, and MUST NOT say it
  is still in the tree, which the report cannot see
- `open`, no stop in the window measured its gate without it
- `asked-once`, a later stop recorded the site, by its gate, file and line, as
  one klin let through after it asked (8.2), or as a deleted test function
  whose file went too. The code is as the agent left it and the fix is the
  person's to make, so it is a question and not a regression: it stays out of
  the regression count and keeps its own audit entry. Pinned by
  `a_deleted_test_klin_let_through_after_asking_counts_only_as_asked_once`,
  `the_stop_that_lets_a_deleted_test_through_says_no_regression_was_fixed`,
  `a_deleted_test_restored_after_the_block_counts_as_caught_and_fixed_next` and
  `a_deleted_test_whose_file_went_after_klin_asked_is_not_a_fixed_regression`
  in `tests/stats.rs`

A gate the stop's `gates` list of 11.2 carries no row for did not run, and a
row that says `ERR` measured nothing. Neither ends a regression, and neither
counts as a measured try. Two sites under one failing gate end apart from each
other: a site absent from a stop that measured its gate went, whether or not
another site kept that gate red.

The report MUST NOT say who authored a fix. The journal proves a site was
present and later absent from a measurement, and nothing about who edited the
code. `The agent fixed all 12.` is therefore forbidden, and `All 12 were fixed
after klin flagged them.` is the form.

#### Measurement confidence

One decision per report says whether klin measured the window whole. The
conditions are the smallest conservative set the records support: a source file
a grammar refused (`unparsed`), a file `before` measured and `after` did not
(`lost`), a known-language file with no structural adapter (`not-measured`), a
dependency or public-surface form a check supports and could not resolve
(`unresolved`), a gate row that says `ERR`, and a journal line the reader could
not take. Not every note lowers confidence: a `near-ceiling`, `derivation`,
`deleted` or `note` record does not.

A window the reader could not place outranks all of them, because it read no
line at all. `--turn` with no readable turn stamp (6.2), and `--session` over a
journal holding no `session` id, are the two. Such a report MUST say it could
not tell where the window began, and MUST NOT say no regressions were found: it
looked at nothing. A window of days always begins somewhere.

The default text summarizes the problem in one clause and never dumps the rows:
`Stats may be incomplete: 2 files weren't measured.`, or the narrower `Stats may
be incomplete: 2 source files couldn't be parsed.` where every unmeasured file
is one a grammar refused. `--all` carries the rows behind it.

#### The default report

The default report is an attention and value summary, never a fixed dashboard
or a table. Its opening state is the strongest condition the window holds, in
this order: measurement doubt, then known open regressions, then the
uncertainty a reset left, then everything resolved, then nothing found, then no
data. `Nothing needs your attention.` MUST be printed only where no known open
or set-aside state remains **and** measurement confidence supports the claim.
Where open regressions and set-aside ones both exist, the open line comes first
and the set-aside line follows it.

```text
Nothing needs your attention.

klin caught 12 regressions this week. All 12 were fixed after klin flagged them.
```

```text
1 regression needs your attention.

klin caught 12 this week. 11 were fixed after klin flagged them.

src/io.rs:12  response.unwrap()
```

```text
7 regressions were set aside when you restarted.

klin caught 8 this week. 1 was fixed after klin flagged it.
```

```text
Stats may be incomplete: 2 files weren't measured.

klin caught 12 regressions this week. All 12 known regressions were fixed.
```

```text
Nothing needs your attention.
No regressions were found this week.
```

```text
klin is on. Your first recap appears after the agent finishes a task.
```

The last is the report for a journal with no line at all. The value sentence
names the unit once: where the opening state already said `regression`, the
count sentence reads `klin caught 12 this week.` Where the window holds a
`config-changed` regression, `N resolved after the config changed.` follows the
count, so the difference between the caught count and the fixed count is never
left for the reader to guess.

The default report names at most three open sites, each on one line: where the
site is, then the recorded text, the measured values where the record carries
no text, or the gate's own noun where it carries neither. A fourth is `and N
more · klin stats --all`, which `--all` itself does not print.

The default report MUST NOT carry, merely because the journal records it: the
stop or run count, `klin_ms` or any timing, the previous window's better or
worse comparison, the guard deny count, guard ask history once resolved, the
deleted-test question count once resolved, prompt excerpts, every fixed
regression, gate names as a dashboard, zero-valued rows, reset history beyond
the current set-aside uncertainty, or per-day history. The journal keeps
recording all of it, and `--all` and `--json` carry it.

#### `--all`

`--all` prints the default report and then the evidence: the title `klin, SCOPE
in PLACE`, where SCOPE is `this turn`, `this session`, `today`, `this week`,
`this month` or `in the last N days`, and PLACE is `this repository` where the
repository has one worktree and `this worktree` where it has more. Then comes
every regression of the window, grouped into the turns they were caught in and
newest first.

A chapter is headed by the day it happened on — `Today`, `Yesterday`, the
weekday name inside seven days, then `YYYY-MM-DD`, with the local offset read
from the system once per report and UTC as the fallback. Where the journal
recorded an excerpt for the prompt the turn ran under, `You asked: "EXCERPT"`
follows it. Inside a chapter the sites group under the gate's human label and
its count, each site carrying its outcome sentence, and its remedy where it is
still open:

- `Fixed after klin flagged it on the next measured try.`
- `Fixed after klin flagged it N measured tries later.`
- `Resolved after the config changed.`
- `Set aside when you restarted.`
- `Still open.`

`Audit` follows, one dated line per item: `klin asked you before X` for a guard
`ask` and `klin refused X` for a guard `deny`, where X names the reason tag of
11.4 (`an edit to klin.json`, `a command that named klin.json`, `an edit to
klin's own state`, `a command that named klin's own state`, `klin init, which
only you run`, `klin install, which only you run`, `klin turn reset, which
only you run`, and `a tool call` for a tag the binary does not know). A reset reads `You restarted, and N regressions
were set aside.`, or `You told klin to start over.` where it set none aside. An
`asked-once` regression reads `a test deleted from FILE:LINE, SITE. The agent
said why.`, where SITE is the declaration line without a trailing `{` or `:`,
and `FILE deleted. The agent said why.` where the site names no declaration
because the whole file went. A reset is not an ask and a guard `deny` is not an
ask: only a guard `ask` is described as asking the person.

`Measurement` follows where the window was not measured whole, with the files
and counts behind the confidence clause.

#### Human labels

The words a report gives a gate's findings are presentation metadata on the
catalogue row of 4.6, a singular and a plural. `klin stats` MUST NOT keep a
second gate-name vocabulary of its own, and MUST NOT match on a gate name to
choose words. Every shipped row MUST supply its labels, and a row that does not
MUST NOT compile. Labels change no gate identity, no config section, no journal
record, no accepted entry and no judgement.

A gate the binary holds no row for is read and printed under its recorded gate
name, so a journal line for a gate klin no longer has stays readable. One
`sarif` entry runs under the name a person gave it, so the catalogue row owns
the generic noun and the journal keeps the specific historical identity.

#### `--json`

`--json` prints one object of facts and none of the person's sentences. A
factual field is not removed because the default text stopped printing it.

- `window` `{scope, days}`, where `scope` is `turn`, `session` or `days` and
  `days` stands only beside `days`
- `counts` `{caught, open, fixed-next, fixed-later, config-changed, set-aside,
  asked-once}`, over deduplicated regression identities and latest outcomes.
  `caught` excludes `asked-once`
- `episodes`, one entry per regression identity, newest first, with no grouped
  `more` count: `{gate, label, id, key, file, line, text, values, remedy, time,
  first, last, prompt, outcome, tries, config_changed}`. `id` is the finding id
  of 11.2 or null, and `key` is the conservative identity `{gate, file, line,
  text}` a reader falls back to. `tries` is how many stops measured the gate
  after the site was flagged
- `audit`, one entry per guard answer, reset and `asked-once` regression,
  newest first: `{time, kind, decision, reason, file, line}`, where `kind` is
  `guard`, `reset` or `asked-once` and a field that does not apply is null
- `confidence` `{whole, gap, unscoped, unparsed, lost, not_measured, unresolved,
  errored, skipped}`, where `gap` is the summary clause or null and `unscoped`
  is the clause for a window the reader could not place
- `activity` `{stops, klin_ms}`, the run count and klin's own time, the
  `klin_ms` of 11.4 and never the project's build (13)
- `stops`, `skipped` and `unreadable`, the same facts beside the object's root
- `earlier` `{caught, open}` for the window before this one, or null where the
  journal does not reach back over it

The report carries no score, no color, no glyph, no praise, no joke, no
estimate of time saved, no count of bugs prevented, and no value claim the
journal cannot prove. No word of the agent's glossary appears in the text.

## 12. Determinism

- Every `git diff` klin runs MUST pin `--diff-algorithm=histogram` and pass
  `-M` or `--no-renames` by name (ADR 0014).
- Findings MUST be sorted by file then line before matching and before
  printing.
- Grammars are compiled into the binary. A grammar version change is a klin
  version change, and the survey cache and the structural cache both key on
  the version.
- No check MAY read the network.
- The only clock a judgment reads is a pinned dated ceiling (5.5), read in UTC,
  and `KLIN_TODAY` overrides it. The report age check of 8.3 compares file
  times, and the command limit and deadline of 9.3 stop a build or `run`
  command that outlives them. These are the two other places time enters a
  verdict, and in both two machines can judge one tree differently: file times
  can differ between checkouts, and a command that ends just under its limit or
  the deadline on one machine and just over it on another passes on the first
  and fails on the second. Every other field a verdict depends on is a pure
  function of the trees. The `ms` of 11.2 and the `time` and `timing` of 11.4
  are measurements about the run, recorded and never judged, so they do not
  break determinism.
- A `run` entry in 8.3 is deterministic only when the tool it runs is. klin
  MUST record the command it ran beside the results.
- A derived number or reachability family is a pure function of the derivation
  commit and the binary version. Other derived path sets are the union of that
  commit's survey and `after`; source checks discover their own facts (4.3).

## 13. Performance Budget

Before parsing grammar-backed source, klin MUST refuse any source line longer
than 65,536 UTF-8 bytes, excluding its line terminator. This deterministic
resource ceiling applies equally to both trees and every language read by the
shared parser. The error MUST name the file, one-based line, measured byte
count and ceiling as a `source-line resource ceiling exceeded` error (exit 2),
rather than silently exclude the file or truncate its site identity. Survey
and tolerant readers that already omit refused parses omit this source too.
The source-line ceiling is defense in depth, not a bound on total retained
site text or process memory. Complexity's survey MUST collect numerical
metrics without materializing function site text or body hashes. Its retained
function sites MUST share one text per source-row and holder-row pair within
a file; an owned finding text is materialized only for a function over its
ceilings. Dense lines below the source-line ceiling MUST retain every
supported function in both trees and the survey. The CLI pins are
`an_oversized_source_line_reports_a_named_resource_error`,
`the_source_line_resource_ceiling_is_inclusive_and_counts_utf8_bytes`,
`dense_sub_ceiling_lines_keep_every_function_in_both_trees_and_the_survey`,
`a_minified_bundle_reports_a_resource_error_in_json` and
`an_oversized_test_source_preserves_the_named_resource_error` and
`an_oversized_source_preserves_the_named_resource_error_in_stubs`.

The Stop hook, excluding the project's own build, SHOULD finish within 5
seconds on a tree of 2,000 source files when scoped with `--changed` to 20
files and the survey cache is warm. The first stop on a new base MAY take the
whole-tree survey and SHOULD finish within 30 seconds on the same tree. A
whole-tree `--strict` run in CI SHOULD finish within 60 seconds. An
implementation MUST measure all three on a fixture and record the numbers in
the release notes when they move by more than a third.

The guard MUST finish within 50 milliseconds.

The dense structural rows have a separate large-repository product budget. The
`structural_300k` row holds exactly 325,077 source lines in 10,000 source
files, and `structural_1m` holds exactly 1,033,827 source lines in the same
10,000-file split. Excluding the project's own build, the 20-file warm Stop
row, the first-stop cold survey and the whole-tree strict run SHOULD finish
within these limits:

| Workload | `structural_300k` | `structural_1m` |
| --- | ---: | ---: |
| Warm Stop hook, 20 changed files and a warm structural cache | 5 seconds | 5 seconds |
| Cold survey | 30 seconds | 60 seconds |
| Whole-tree `--strict` run | 20 seconds | 45 seconds |

These are product requirements, not the current implementation's medians.
They are rounded limits with operating headroom, chosen from the final
controlled measurements in #193: 2,388 ms and 2,733 ms for the warm rows,
22,834 ms and 47,507 ms for the cold rows, and 12,168 ms and 30,724 ms for
the strict rows. The 100-file warm row remains a fixed-repository scaling
check, not a second product budget. The existing 2,000-file budgets above are
unchanged.

Before a release, the dense rows MUST be rerun on the controlled reference
machine and compared with the preceding controlled release. A median that
misses one of the limits above, or rises by more than one third against its
preceding controlled row, MUST be explained in the release notes before the
release proceeds. This is a release checklist rule: ordinary contributor
tests continue to verify fixture shape and semantics, and do not use
machine-specific wall-clock or RSS values as an oracle.

The normative measurement fixture is the ignored `performance_fixture` CLI
test, selected by `cargo test -- --ignored perf`. It generates a deterministic
temporary repository with `rust/` and `web/` projects: the 2k row holds 1,000
`.rs` and 1,000 `.ts`/`.tsx` files, and the 10k row holds 5,000 of each. A
small one-percent TypeScript subset is `.tsx`; both rows include Rust and
TypeScript manifests, tests, imports and references, and held escape/stub
sites. Each row changes 10 files in each language. The test runs warm hook,
cold survey and whole-tree strict five times and prints the median; the hook
row explicitly excludes the project build. It also sends 1,000 deterministic
read, write, shell, quoted-path, glob, heredoc, configuration-name and
multi-path patch events through the real guard binary path five times, as a
separate row. The test records the klin version, fixture counts, cache state,
changed-file counts, iteration count and median milliseconds; it does not
enforce the budgets on contributor hardware. The dense rows additionally
change 40 more files in each language and print a 100-file warm row. The dense
fixture asserts that structural counters grow with that delta: changed files
are extracted from both trees, while unchanged base files remain cached and
shared.

The same test also measures two source-dense structural rows. The
`KLIN_PERF_ROW` environment variable selects one row:
`KLIN_PERF_ROW=structural_300k cargo test --release --test performance --
--ignored perf` or `KLIN_PERF_ROW=structural_1m`. Without the variable the
test runs the 2k, 10k and guard rows as before. `structural_300k` holds 10,000
source files and exactly 325,077 source lines. `structural_1m` holds the same
5,000 Rust plus 5,000 TypeScript split and exactly 1,033,827 source lines.
Each dense row reports the primary 20-file delta and the additional 100-file
warm delta; the latter is a fixed-repository scaling check, not a product
budget. Both rows keep 50 `.tsx` files, tests, entry points and held escape
and stub sites.

Each dense file repeats one structural unit: two constants, a struct or
interface, a type alias, an `impl` or class with a method, a function in one of
256 duplicate-name buckets, an exported value function that imports and calls
its neighbour, and a branching function. `structural_300k` writes one unit per
file and `structural_1m` writes three or four. The rows differ in source volume
and keep the same shape, so fixture generation asserts that both hold 270 to
290 declaration lines per 1,000 source lines. Generation also asserts file
counts, the language split, the TSX count, exact LoC, the exact declaration
count, an FNV-1a digest of every generated path and byte, and representative
structure. Configured Rust and TypeScript module families exercise
`complexity`, `dead-symbols` and `reachability` through the real binary. Run
with the current binary, the dense rows also write a `layering` section with
one layer per language and `acyclic` set, so every row builds both module
graphs and finds their cycles, and each gate row prints `graph_modules`,
`graph_sources`, `graph_dependencies`, `graph_edges`, `graph_ms` and
`graph_dispatches_<language>`, so a row tells semantic modules from the
physical files they hold, and dependency sites from the distinct module pairs
those sites join. The `public-api` row also prints
`surface_dispatches_<language>`. The warm hook asserts that `layering`
reads and parses no source of its own, because it takes every structural
outcome an earlier gate of the stop already held. A binary named by
`KLIN_BIN` reads no `layering` section, so its rows leave the section out.
The fixture's `web/package.json` names `./src/index.ts` under `exports`, and
its `rust/src/lib.rs` is the implicit library root, so every row derives one
Rust and one TypeScript public surface, and the `public-api` gate row prints
`surface_surfaces`, `surface_items`, `surface_measured`, `surface_opaque`,
`surface_holes` and `surface_ms` beside its `graph` counters. The warm hook
asserts that `public-api` reads and parses no source of its own, for the same
reason as `layering`. A binary before #46 has no `public-api` row, and its
rows leave the counters out.

Each dense row runs warm hook, cold survey and whole-tree strict five times
and prints the median total and every gate's median `ms`. Where available it
also prints deterministic content-work and structural-fact counters:
`work_reads`, `work_parses`, `facts_reads`, `facts_parses`, `extracted`,
`shared`, `cached`, `cache_read_ms` and `cache_write_ms`. The `dead-symbols`
and `reachability` rows also print the `names` group of 11.2 as
`names_base_ms`, `names_lost_ms`, and each tree's values under
`names_before_` and `names_after_`, so a warm row separates the base layout,
each tree's measurement, index build and name queries, and the lost
references from the rest of the gate's time. The row that laid the base out
also prints the parts of `names.layout` as `names_layout_<name>`, and a warm
hook row prints the stop's `timing` values as `stop_<name>`, so the worktree
checkout, the base file list, the cache and the worktree removal are told
apart. A targeted warm row also times, on the fixture's repository and apart
from every stop, the git commands a lighter base layout would use, five times
each with the median printed as `worktree_<name>_ms`: a whole detached
`worktree add`, its `worktree remove --force`, a `worktree add --no-checkout`,
a `read-tree` into it, an `ls-files --stage` listing of it, a `checkout-index`
of the changed files' base paths into it, its removal, and `worktree prune`.
Those medians are an estimate of a candidate, not a measurement of klin. The `dead-symbols` row also
prints the `footprint` group of 11.2, each counter as `footprint_<name>` and
each type size as `footprint_size_<name>`, so one row carries the population,
sparsity and byte proxies of the facts that run held. `KLIN_PERF_CASE=warm20` or `warm100` runs
one targeted warm scenario of either dense row, so a 1M warm row needs no cold
or strict run. A row that runs one
targeted warm scenario prints the structural cache's file count and bytes
after its stops and the peak RSS of one more warm hook, where a full row
prints them beside the peak memory. A second warm hook
row removes the structural cache of 8.4 before each stop, so it measures a
stop that extracts the base and writes the cache, beside the first row's
stop that reads it. After the rows, one untimed stop writes the structural
cache again, because the cold rows' `cache clean` removed it, and the output
prints the file count and the bytes on disk of that cache. The warm hook rows
read each
gate's values from the journal line of the stop it timed. The cold and strict
rows read them from `--json`. A gate's `ms` covers its whole run:
reading, parsing and extracting the files that no earlier gate of the run
extracted, building a structural index where name resolution needs one, and
its own algorithm. Its `facts.ms` is the first part, so `ms` less `facts.ms`
is the time of its index and its algorithm. A run extracts each structural
file of a tree once, so the
first gate that reads a file pays for the extraction, and a later gate counts
that file in `facts.shared` (11.2). In the warm hook, `dead-symbols` and
`reachability` take the base's facts for every unchanged working-tree file
(8.4), so their `extracted` is the base's structural files plus the changed
ones, and their `shared` is the unchanged ones. Where the structural cache of
the turn's base holds the base's outcomes, `extracted` is the changed files of
both trees and `cached` is the unchanged ones. `complexity` walks a parse of
its own,
which no extracted fact replaces, so its `ms` still covers its parsing. A row
taken with an earlier binary through `KLIN_BIN` prints no fact counters when
that binary records no `facts`. The output records source LoC, declarations,
digest, file and language counts, cache state, changed files, iteration count,
version and host platform, and excludes project build time from hook timing.
On a platform with `/usr/bin/time`, it also prints the controlled peak RSS of
one warm hook, one warm hook without the structural cache, one
`gate --changed --json --gate dead-symbols` and one strict run;
missing resource reporting is not a test failure. These rows record
measurements and add no wall-clock or RSS budget.

`KLIN_PERF_ROW=source_areas` selects the root-count rows: the same 2,000
source files, 1,000 Rust and 1,000 TypeScript, split over 2, 100 and 500
directories that hold nothing but source under a manifest directory that holds
more, so each is a source area the tree listing discovers. Each
row is the whole-tree strict run, five times, median. A run reads a tree's
file list once and asks git once what it ignores, whatever the root count, so
the three rows MUST read alike; a row that grows with the root count is a walk
or a process per root coming back (ADR 0038). `KLIN_BIN` names another klin
binary for the harness to run, so a row can be taken under an earlier release
beside the current one. Such a run does not assert that every dense gate
reported a time, because an earlier release may lack one.
`KLIN_PERF_SCOPE=rust` gives the 2k, 10k and dense rows a `complexity` section
of `"in": "rust"`, recorded at the base, so the derived ceilings sample one
source area. Without it, or with `whole`, the section is absent and the
sample is the whole repository.

`KLIN_PERF_CONFIG` chooses the configuration of the 2k, 10k and dense rows.
`build-off`, the default, writes `{"build": []}`. `empty` writes `{}`, so the
hook derives the build, and puts stand-in `cargo` and `tsc` commands first on
the path, so the row measures the build's preparation and no compiler.
`legacy` pins the configuration the running binary's own `init --force`
writes after the base and commits it, which only a binary before #180 reads,
so it is taken with `KLIN_BIN`. Comparing it with `build-off` under the
current binary measures what discovering the facts costs (ADR 0040).

A check that cannot take scope, such as a whole-tree duplication share, MUST
say so in `gate --list` and MAY be skipped by the hook under a `hook: false`
key on its section.

## 14. Failure Model

Two flags change the failure model, and nothing else does. `--hook` turns a
failure the agent cannot fix into a NOTE, or sends it to stderr with exit 1,
so a stop is never blocked on it. `--strict` turns a hole a person's CI must
not miss into exit 2. `klin gate` with neither flag exits 2 on every hole ADR
0003 named that the base did not hold too (8.6), and passes with a NOTE on the
holes that `--strict` adds (10).
klin cannot see CI, so no row says "CI". A row that names no mode behaves the
same in all three.

| Class | Behavior |
|---|---|
| Config error: unknown key, malformed section, schedule with no due step | exit 2 before any gate runs, naming the file and key. Hook: the same text on stderr with exit 1, never a block, because the agent cannot edit the file it names (9.4) |
| No base resolves outside the hook | exit 2 naming what was tried |
| `turn` file missing in the hook, ref present | restored from the ref with a RED verdict, and a NOTE says so |
| `turn` file and ref both missing in the hook | a branch window from the base of 6.3, or from HEAD when none resolves, a NOTE names the missing stamp, and the stop writes that base as the stamp |
| The commit the stamp was taken over is outside current HEAD history in the hook | a branch window from the base of 6.3 for the current checkout, a NOTE names the commit HEAD no longer holds, and the stop writes that base as the stamp, red, keeping the prompt counter, dropping `asked` and `intervened`, leaving the handoff records of 9.1, which belong to a host session, and deleting both `refs/worktree/klin/turn` and `refs/worktree/klin/mark` (6.2) |
| Git cannot answer whether HEAD history holds that commit | the turn window stays, because only a proven divergence is a turn the checkout left (6.2) |
| A file no grammar reads | Outside the hook: the gate names it and exits 2, other findings still print. A file the base held and no grammar read there either is a NOTE, `--strict` included (8.6). Hook: a NOTE, told to the person through `systemMessage` on a stop that ends (9.1). |
| A deleted test (8.2) | Hook: blocks the first stop that finds it, once. The next stop lets it through as a NOTE, tells the person, and ends green. Outside the hook: a NOTE, `--strict` included. |
| The build fails in the hook | block with the build output, no gate runs. Outside the hook the build step does not run (ADR 0012). |
| Host event unreadable in the hook | report to stderr and exit 1, never block |
| Host event unreadable in the guard | allow |
| A `run` entry exits without writing its report, or the report is not SARIF | that gate is ERR, in every mode. The command's exit status alone is not judged (8.3). |
| An accepted entry that names a pattern row klin retired from a built-in table | the NOTE and the `--strict` failure of 4.8 name the row and where it went, so the first run after an upgrade states its cause. The outcome is the one 4.8 gives every unmatched entry. A row a project deleted from its own `patterns` is not one of these |
| Survey cache unreadable | recompute, overwrite |
| State directory unwritable | the hook reads the stamp it can find, per the two rows above, writes no verdict and no build count, prints why, and never blocks on it. A build failure and a gate failure are reported, not blocked, because no count could bound the blocks. On a host that submits a told message as a prompt, the stop tells nothing, because klin could not record the message (9.1). |
| Survey finds no source root | `--strict`: exit 2 naming the directory surveyed. Otherwise a NOTE naming it, and in the hook the turn ends. |

## 15. Trust Model and Conformance Levels

`docs/THREAT_MODEL.md` carries the reader-facing form of this section: the four
trust zones, which behavior is local feedback and which is re-measured outside
the agent's environment, and where the boundaries end. It states this section,
9.4 and 19.2 and adds no rule of its own. The contract is here.

### 15.1 Feedback level

Hooks only. klin puts every failure in front of the agent, blocks on a gate
failure at most twice per turn (9.3), keeps the window open until the failure
is fixed, accepted or reset by a person, and refuses its edits to the config. The config and klin's own state
directory are the guarded set (9.4).
Nothing prevents a PATH
shim or a `chmod -x`, and the guard sees only the tool calls the host shows
it. This level is what a person gets with no CI, and this document makes no
stronger claim for it.

### 15.2 Enforced level

Feedback level plus: a CI run with `--strict`, on a checkout the agent never
touched, against a protected branch, with `klin.json`, the workflow, the hook
settings and CODEOWNERS under CODEOWNERS. At this level a gate holds against
an agent, and loosening it takes a reviewed commit by a person.

One finding is the exception. A deleted test is a NOTE in CI (8.2), so at
this level it holds only through the hook's one question, the guard in front
of the record that question leaves (9.4), and the reviewer who reads the
diff. An agent that runs with no hook meets none of the three. ADR 0031 and
ADR 0032 record the cost.

The README MUST name both levels and say which one a setup reaches.

### 15.3 Secrets and safety

klin reads no secrets and sends nothing anywhere. The `build` and `run`
commands execute a project's own shell lines, which the config owner wrote,
under the guard. A derived `build` command is one of a fixed table keyed by
manifest, printed before it runs.

## 16. Reference Algorithms

### 16.1 The hook's window

```
state = KLIN_STATE_DIR/hash(common_dir, worktree) if set else git_dir()/klin

read_stamp():
  stamp = read(state/turn)
  if stamp is not None: return stamp
  commit = resolve("refs/worktree/klin/turn")
  if commit is None: return None
  note("turn file missing, restored from the ref")
  stamp = Stamp(commit, parent=parent(commit), time=None, last_verdict=RED)
  # parent(commit) is commit^, which names the stamped HEAD only because the ref holds a
  # synthetic stamp. 6.2 is why no other commit may be written to that ref.
  write_atomic(state/turn, stamp)
  return stamp

holds_head(commit):                    # 6.2, the three answers stay apart
  status = run("git merge-base --is-ancestor " + commit + " HEAD")
  if status == 0: return True                              # ancestor of HEAD, or HEAD
  if status == 1: return False                             # proven divergent
  return None                                              # git could not answer

left_behind(stamp):
  return stamp.parent is not None and holds_head(stamp.parent) is False

hook_window():
  stamp = read_stamp()
  if stamp is not None and left_behind(stamp):
    note("the turn started from a commit HEAD no longer holds, judging the branch")
    delete_ref("refs/worktree/klin/turn")                  # no restore of what HEAD left
    delete_ref("refs/worktree/klin/mark")                  # 6.2.1
    before = branch_stamp(stamp, mark=None)
    return Window(BRANCH, before, WORKING, "the turn HEAD left behind")
  if stamp is None:
    note("stamp deleted, judging the branch")
    return Window(BRANCH, branch_stamp(stamp, mark=mark_of(stamp)), WORKING, "stamp missing")
  return Window(TURN, stamp.commit, WORKING, "since " + stamp.time)

branch_stamp(stamp, mark):             # the base this stop judges, written back red
  before = choose_window(strict=False).before or HEAD      # 16.2
  write_atomic(state/turn, commit=before, parent=before, time=now, last_verdict=RED,
               prompt=prompt_of(stamp), mark=mark,
               asked=[], intervened=False, followup=None)
  return before

on_session_start_or_prompt():          # one rule for both events
  stamp = read_stamp()
  tree  = write_tree(index=state/index, add_all=True)
  if event == PROMPT: report_radius(read_mark(), tree)   # 6.2.1, never on a session start
  if stamp is None and not exists(state): move_stamp(tree)   # first session here
  elif stamp is not None and stamp.last_verdict == GREEN: move_stamp(tree)
  elif stamp is None: note("stamp deleted, the next stop judges the branch")
  move_mark(tree)                                  # 6.2.1, on both events, whatever the verdict
  if exists(state/turn): bump_prompt(state/turn)   # prompt += 1, whether or not the stamp moved

turn_reset():                          # a person's command, denied by the guard
  tree = write_tree(index=state/index, add_all=True)
  move_stamp(tree); move_mark(tree); print("a person moved the window")

move_stamp(tree):
  commit = commit_tree(tree, parent=HEAD)
  update_ref("refs/worktree/klin/turn", commit)
  write_atomic(state/turn, commit=commit, parent=HEAD, time=now, last_verdict=None,
               prompt=current_prompt(state/turn), mark=current_mark(state/turn))

move_mark(tree):                       # 6.2.1, the window the radius report measures
  commit = commit_tree(tree, parent=HEAD)
  update_ref("refs/worktree/klin/mark", commit)
  write_atomic(state/turn, mark=commit)

read_mark():
  return mark_of(state/turn) or resolve("refs/worktree/klin/mark")

derivation_commit(window):
  return stamp.parent if window.kind == TURN else window.before

roots(window):
  return survey(derivation_commit(window)).roots | survey_roots(window.after)
```

### 16.2 `klin gate` and CI window

```
choose_window(strict):
  for (ref, kind, source) in candidates():
    before = resolve(ref)
    if before is None: continue
    if before == HEAD and tree_is_clean():
      if source == Remote: return Window(kind, before, WORKING, "same, remote agrees")
      tip = remote_default_tip()
      if tip is None:
        if strict: fail("cannot tell whether history is hidden")
        return Window(kind, before, WORKING, "same, no remote to ask")
      if commits_between(tip, HEAD) > 0: fail("unpushed commits hidden")
      return Window(kind, before, WORKING, "same, remote holds HEAD")
    return Window(kind, before, WORKING)
  fail("no base resolves")
```

### 16.3 The hook run

```
hook(event):
  host = Host.detect(event)
  window = hook_window()
  at = derivation_commit(window)
  survey = cached_survey(at) or survey(at)
  count = read(state/build-blocked)
  mine = count is not None and count.session == event.session
  if count is None or (count.prompt != turn.prompt and not (host.continued(event) and mine)):
    count = Count(prompt=turn.prompt, session=event.session, builds=0, gate_blocks=0)
    if mine and not host.continued(event) and not lost_the_lock:
      write_atomic(state/build-blocked, count)     # a person's prompt opens its budget (9.3)
  count.prompt = turn.prompt                       # a continued chain keeps its record (9.3)
  failure = build(config_or(survey), changed_files(window))
  unbuilt = None
  if failure and failure.exit == 127:
    unbuilt = note("unbuilt", failure.command, failure.shell_said); failure = None
  if failure:
    write_verdict_atomic(state/turn, RED)
    if lost_the_lock: report(failure); return 0         # 6.5: another stop may be counting
    tree = tree_of(working_directory)
    if count.builds > 0 and tree == count.build_tree: report(failure, "the tree did not change"); return 0
    count.builds += 1; count.build_tree = tree; write_atomic(state/build-blocked, count)
    if count.builds > 8: report(failure, "stopped blocking after eight"); return 0
    block(derived_lines + failure + "block N of 8")
  (failed, errored, reported, told) = run_gates(config_or(survey), window, scope=changed, unbuilt)
  if failed == 0 and errored == 0:
    write_verdict_atomic(state/turn, GREEN)
    if told: tell(report)                          # systemMessage on stdout, exit 0 (9.1)
    return 0                                       # see tell() below
  write_verdict_atomic(state/turn, RED)
  if lost_the_lock: pass_through("another stop held the state directory"); return 0
  if no_state_directory: pass_through("could not record a gate block"); return 0
  if count.gate_blocks >= 2: pass_through("the gate has blocked 2 stops"); return 0
  flagged = count.builds == 0 and host.blocked_before(event)
  if count.gate_blocks == 0 and not flagged:
    number = 1
    tree = tree_of(working_directory)              # None when git cannot hash it
  else:
    if count.gate_tree is None: pass_through("no record of the last gate tree"); return 0
    tree = tree_of(working_directory)
    if tree is None: pass_through("no record of the last gate tree"); return 0
    if tree == count.gate_tree: pass_through("the tree did not change"); return 0
    number = count.gate_blocks + 1
  count.gate_blocks = number; count.gate_tree = tree
  if not write_atomic(state/build-blocked, count):
    flags += "count-unwritable"
    pass_through("could not record a gate block"); return 0
  add_asked_atomic(state/turn, reported)           # 8.2, cleared when the stamp moves
  if host.follows_up(event) and not record_atomic(state/handed/hash(event.session), followup=hash(report)):
    report(); return 0                             # 9.1: the echo would open a turn
  block(report + "gate block " + number + " of 2")

tell(message):                                   # 9.1, ADR 0052
  if host.follows_up(event):
    handed = state/handed/hash(event.session)
    if lost_the_lock: return                       # an unrecorded message would replay
    if handed.told == hash(without_window_line(message)): flags += "told-before"; return
    if not record_atomic(handed, followup=hash(message), told=hash(without_window_line(message))): return
  deliver(message)

pass_through(why):
  report(); say(why)
  if count.gate_blocks > 0 and event.session and no_prompt_line(event.session):
    systemMessage("klin: no prompt event reached this session; klin grants no fresh gate blocks until `klin radius` runs on session start and on prompt submitted.")
    flags += "no-prompt-event"
```

`reported` is the site id (11.2) of every finding the run printed, and `told`
counts the notes a person needs even when nothing blocks: a file no grammar
read, a file measured in `before` only, and a deleted test the run let
through. Only a stop that blocks on a gate adds to `asked`, so a question the
agent never saw is asked again at the next stop that blocks. `inventory`
reads `asked` under `--hook` (8.2).

The build stamp is one record per prompt: the prompt counter it belongs to,
the number of build blocks and the tree the last build block was taken over,
and the number of gate blocks and the tree the last gate block was taken
over. The two pairs never share a field (ADR 0052). A record an older klin
wrote holds `tree` for the build tree and `gate_spent` for one gate block,
and no gate tree, so it never proves a second gate block.
A passing build does not reset the build count, so a tree that builds, breaks
and builds again inside one turn still gets eight blocks in that turn and no
more. The tree is the one 6.5 hashes for the stamp, read through an index of
the build stamp's own, `build-index`, so a build block costs one hash of the
working tree and leaves the turn stamp's first-session marker alone. `unbuilt` is one
note the runner adds to the run's notes and counts as told, so a stop nothing
blocks still tells it. The host's `blocked_before` flag is a second opinion for the first gate
block only, because after a build block that flag is true while no gate
block is spent (ADR 0004). It never proves a second gate block (ADR 0052).

### 16.4 Evaluate one gate

```
evaluate(check, section, window, scope):
  before = check.measure(window.before, section) if check.needs is the tree else []
  after  = check.measure(window.after,  section, before)   # only inventory reads `before`
  entries = accepted(config, check.name) + before   # accepted first: entry order breaks ties (16.5)
  (after, entries) = restrict(after, entries, scope)
  ceiling = section.ceiling            # derived from the derivation commit, pinned, dated, or none

  after = [f for f in after if ceiling is None or over(f, ceiling)]
  before_over = [e for e in entries if ceiling is None or over(e, ceiling) or e.accepted]
  return judge(after, before_over, check.ratcheted)
```

`before_over` is what makes 7.3 hold: an entry below today's ceiling is not
judged, and an entry above it is matched, so a lower ceiling never turns a
held site red. A check with no ceiling, such as `inventory`, judges every
finding and every entry, so its `missing: 0` entries stay in and a vanished
site matches its `before` entry.


`inventory` fits the same judge by ratcheting existence. Its measure of
`after` emits a finding for every site the `before` measure holds, with
`missing: 1` where `after` has no match by site and then by body hash, and
`missing: 0` where it has one. The `before` entries carry `missing: 0`, so a
vanished site is `worsened` under the one `judge` and nothing else changes.
A vanished site the run lets through, outside the hook or in a stop's
`asked` record (8.2, 16.3), has a `before` entry of `missing: 1` instead, so
the same judge holds it and the run lists it in a NOTE. A site the accepted
list names is matched like any other, ahead of the `before` entry by entry
order. A vanished site whose subject went in the same window is a NOTE, not
a finding.

### 16.5 Match one site

```
match_site(findings, entries, ratcheted):
  # findings are sorted by line; entries are the accepted list in config
  # order, then the `before` sites by line
  candidates = []
  for i, f in enumerate(findings):
    for j, e in enumerate(entries):
      rose     = any(f[m] > e[m] for m in ratcheted)
      shared   = count(m for m in ratcheted if e[m] == f[m])
      distance = 0 if e.accepted else abs(e.line - f.line)
      candidates.append((rose, -shared, distance, i, j))
  pairs = []
  for (_, _, _, i, j) in sorted(candidates):
    if taken(findings[i]) or taken(entries[j]): continue
    take(findings[i]); take(entries[j])
    pairs.append((findings[i], entries[j]))
  return pairs, untaken(findings), untaken(entries)
```

The judge runs this once per file and declaration text (4.4). An untaken
finding is `new`. An untaken `before` entry has no outcome. An untaken
accepted entry matched nothing (4.8). The second pass of 4.4 takes the
untaken findings and entries as its input, grouped by body hash rather than
by site, with `distance` fixed at 0. Each group sorts both sides by file and
line before it runs, so what is left of the rank order, `i` and `j`, is the
finding's file and line, then the entry's. Only an entry whose site the
`after` tree lost reaches this input (4.4).

Inserted twin. `before` holds `fn f() {}` at lines 1 and 5. `after` holds it
at lines 1, 3 and 6, all with equal values. Every pair shares every value,
so distance decides: line 1 takes line 1 at distance 0, then line 6 takes
line 5 at distance 1. Line 3 is untaken and `new`. Pairing by line order
would pair line 3 with line 5 and name line 6, which nobody wrote, as new.

Moved twin. `before` holds `fn twin()` with cc 2 at line 3 and cc 1 at line
20. `after` holds cc 2 at lines 19 and 50. Both findings share both values
with the cc 2 entry. Line 19 is nearer, so it takes that entry and is held.
Line 50 takes the cc 1 entry and is `worsened`, cc 1 to 2. Pairing by
nearest line would pair 19 with the cc 1 entry at 20 and report a function
that only moved as worse.

Stale accepted entry. `before` holds `fn f()` with cc 3 and 5 lines. The
accepted list holds it at cc 2 and 4 lines. `after` holds it at cc 2 and 5
lines. The finding shares one value with each entry. It holds against the
`before` entry and rose against the accepted one, so the `before` entry takes
the match and the site is held (7.3). The accepted entry matched nothing.
Without the first rank, entry order would give the match to the accepted
entry and report a site that got no worse as `worsened`.

## 17. Test and Validation Matrix

One seam, the binary, on a throwaway tree with a base (AGENTS.md).

Core:

- Config: absent file runs Automatic checks, discovery, override, relative
  paths, unknown key, `baseline` key refused, `false` exclusion, compact source
  objects accept only human policy, retired topology keys, `project` and
  `version` are actionable errors, a misspelt key or field names the one a
  person most likely meant, pinned beats derived, dated ceiling picks the right step, and a
  schedule with no due step is an error.
- Survey and source facts: one project, a monorepo, a tree with no source,
  complexity ceilings on a tree with fewer than 50 functions, lazy cache hit
  and miss, and cache keyed by binary version.
- Structural extraction: `complexity`, `dead-symbols` and `reachability` in
  one run judge as each does alone under one scope, overlapping scopes and
  disjoint scopes, with a language only `complexity` reads, a file one gate
  excepts and another reads, a file the grammar rejects, a `--changed` run,
  and a file only one tree holds; a file two structural gates read is
  extracted once per tree, under `--changed` too; in a changed run that is
  not strict, `dead-symbols` and `reachability` extract a file the working
  tree did not change once for both trees, and their edits, additions,
  deletions, both rename classes, scope movement, name ambiguity, lost
  references, unsupported languages, unparsed files and a case-only rename
  git does not see judge as two independent extractions do. A repeated
  changed run reads the base's outcomes from the structural cache and judges
  the same. A missing, truncated, extended or changed cache file, and one
  copied from another commit, reads as no cache and judges the same. A smudge
  filter added between two runs judges as a run without the cache, a write
  keeps the four newest cache files, and repeated red stops keep their turn
  base through a prompt and a branch switch.
- Window: each candidate in order, each ADR 0013 branch outside the hook, the
  hook with a deleted `turn` file restores it from the ref with a red
  verdict, the hook with file and ref both deleted judges the branch and the
  prompt before it writes no fresh stamp, the stamp holding through a commit, the
  derived values holding through a commit inside a turn, the stamp not moving
  after a red stop on a prompt and on a session start, accepting finding A
  holds A and leaves B failing and the stamp where it was, `turn reset`
  moving it and saying so, the stamp surviving `git gc`, a hook killed
  mid-write leaving the previous `turn` file intact, the survey cache hitting
  on the second stop of a turn and missing after a commit.
- Ratchet: new fails, worsened fails, held passes, rename keeps sites, moved
  function keeps its site within a file and across files, a function renamed
  with its body unchanged keeps its site, a one-line function that moved
  keeps its site, a moved body that was reindented keeps its site, a moved
  body that was edited is `new`, two bodies that moved take the entry that
  shares their values, the lower of two bodies in one file takes the moved
  entry, an accepted entry beside the base entry does not free it for a
  copy, a function that moved and grew names the site it matched, a copy of
  a function beside its original is `new` and does not inherit the
  original's match, a twin inserted between two twins is the new one and its
  neighbours hold, a moved twin keeps its entry over a nearer twin whose
  value changed (16.5), a lowered ceiling fails no held site, accepted entry
  holds a site, an accepted entry the base also holds at the same values
  stays matched under `--strict`, a stale accepted entry does not fail a
  site the base holds, an accepted entry that names some of the values, or
  gives one a value that is not a number, is exit 2, unmatched accepted
  entry is a NOTE and a strict failure, a failure prints the site it matched
  and both values.
- Measurement rules (8.2.1): a document whose words are split by Unicode
  spaces and hold markup is counted at the ceiling boundary, a byte that is
  not UTF-8 is one word, a repeated line of two escape kinds fails as one site
  with the first label and every match counted, a backticked span that is not
  a path is not read, a nested function is measured on its own.
- Each check: over, at, under the ceiling, a file it cannot read with
  `--hook`, with neither flag and with `--strict`, of which the first is a
  NOTE and the other two exit 2, scope restricts both sides, `--json` shape
  with coverage counts,

  a tree with no config and existing debt of this check's kind is green, a
  new root added in the window is measured, a first file in a new language
  is held to the floor, a new document is a NOTE with its word count and is
  judged once the stamp moves, a directory that becomes a root in the window
  brings no held sites with it, and a function under an `after`-only root
  does not move the derived ceiling.

- `inventory`: a deleted test file whose subject was deleted too is a NOTE, a
  deleted test function or file blocks the first hook stop that finds it and
  asks why it went, the next stop passes and tells the person which tests
  went, a prompt between the two stops does not ask again, a stop whose
  stamp was deleted still asks, outside the hook a deletion is a NOTE under
  `--strict` too, an accepted entry naming a deleted test matches, a renamed
  test function with its body unchanged is held, a deleted test function
  whose file went too is a NOTE.
- `sarif`: a result on a changed line fails, on an unchanged line in a changed
  file is held, `differential` fails every result, `run` writes the report
  before it is read, a report older than a changed file is ERR.
- `lockfile`: a manifest entry with no lockfile entry fails, a removed pin
  fails, a pin the lockfile records at another version fails and a base one
  is held, a path dependency is not judged, a new dependency with a range and a
  lockfile entry is green, a deleted lockfile fails every dependency, a
  workspace lockfile above the member manifest is found, both npm lockfile
  versions are read, an unreadable format is a NOTE, a malformed supported
  lockfile is a tool error, a manifest klin cannot parse at either commit is a
  NOTE, a manifest the work broke or added unparseable is a tool error outside
  the hook, and two manifests that
  share a lockfile are each judged against it.
- `layering`: a new forbidden edge fails and a base one is held, same-layer
  and `can_use: null` dependencies pass, overlapping layers and retired keys
  are config errors, nested Rust module context, a literal `#[path]`, a
  manifest-only target change and a file two targets reach resolve, an
  ambiguous Rust or TypeScript module is exit 2 and a NOTE in the hook, a new
  cycle fails and a historical one is held, containment is no dependency, a
  rename inside a layer is held and one into another layer is new, a changed
  run beside a gate that lays out changed files judges the whole base, and a
  cached changed run parses only the changed file.
- `public-api`: an unchanged library passes and the `OK:` line counts what was
  judged, root `pub`, a `pub mod` chain and a `pub use` are external while a
  private module's `pub` child and restricted visibility are not, a
  binary-only package is not applicable, a custom library root and each
  workspace library are surfaces of their own, an alias renames, a glob lists
  its module, a re-export of another crate is opaque and judged on presence, an
  item moved into a workspace sibling and re-exported under its name passes
  with its contract unchanged and fails as changed otherwise, through a
  `pub extern crate` alias and a glob inside the sibling too, a whole crate
  re-exported is opaque, a re-export of a crate the tree does not hold stays
  opaque, and so does one of a registry dependency named like a library of the
  tree or through a `use` alias after `::`, a dependency's path picks one of
  two libraries of one name, a renamed dependency, a target-specific one, one
  inherited from a workspace at or below the tree root or from the package's
  own workspace, and a private
  `extern crate` alias are followed, a name two globs provide to a re-export
  by name is a hole, a path from a name a local `use` binds stays opaque where
  a dependency has that name, a private item or import hides the name a glob
  of a sibling provides and an associated item, a binding in the other
  namespace or a `use` inside a function does not, an `extern crate` alias
  named like a dependency keeps its path opaque, an associated constant of an
  inherent `impl` is an item under its type, a glob of a sibling lists its
  items, a
  body, comment, format or binding-name change passes, a changed signature
  and a removed item fail and an addition passes, a removed library fails once
  at the surface, a source move behind an unchanged identity passes by hand
  and in a changed run, an accepted break is held, a glob of another crate
  and a module no file answers are holes by hand and a NOTE in the hook, the
  section is absent or `false` and any object is refused, `init` writes no
  section; root and subpath exports are surfaces and generated JavaScript is
  not reverse-mapped, `types`, `typings` and a direct TypeScript `main` or
  `module` are entries, a package with no supported entry is not applicable,
  a file outside the traversal is not API, named, default, alias and star
  re-exports are items, an ambiguous star and a star over another package are
  holes, an external re-export is opaque, TSX is TypeScript, an inferred
  contract is partial and a body change behind it passes, a TypeScript
  signature change fails, a subpath removal fails once, a changed manifest
  changes an unchanged file's meaning in a changed run, and a cached changed
  run parses only the changed file.
- `conventions`: an unknown key names the convention and the key, a missing
  remedy, zero or two matchers, a language on a `text` or `files` rule, and an
  absolute, escaping or glob path are config errors, `in` and `except` select
  a path and everything below it from wherever klin runs, a literal holds its
  regex characters as text, a code pattern skips a comment and a string, holes
  work in Rust, TypeScript and TSX, a fragment reads as a match arm or a type
  whatever its spacing, a fragment with two readings matches through both, a
  node two readings match counts once and two nested nodes count twice, a
  fragment no place reads names every place tried, a pattern that is only a
  hole is refused, a scope in two languages or in none is a
  config error, an `in` path that names nothing is exit 2, a hidden directory
  is not read, two conventions on one line are two findings, an accepted entry
  for a removed convention is a config error, a move within a file and a
  rename inside the scope are held, a move or a copy to another file and a
  rename out of `except` are `new`, `--report` summarizes every convention in
  a row that names what to act on first, `--report <name>` explains one
  convention with its sites and its fix, an unknown name is exit 2, and
  neither view writes anything.
- Coverage: a file present in both trees and measured in `before` only is a
  NOTE in the hook and exit 2 under `--strict`, whether it left through an
  exclusion or a grammar error.
- Concurrency and worktrees: an old green stop that finishes after a new red
  stop does not write green, two worktrees keep independent stamps through
  `git gc --prune=now`, a stop that cannot take the lock writes no verdict.
- Guard, the files that left the guarded set: an edit to a host's hook file,
  CODEOWNERS, a lint config, a test config, a coverage threshold, a CI
  workflow and klin's state directory is allowed.

Paired scenarios. Every `stubs`, `escapes` and `inventory` pattern carries
two fixtures, one regression that fails and one legitimate change that stays
green, because deterministic detection is not correct judgement:

- A failing test is deleted, fails. The same test moves to another file with
  its body unchanged, green.
- A test body is emptied, fails. A test body is rewritten with the same
  declaration, green.
- A `pass` body lands on a function, fails. A `pass` body lands on an
  exception class, green.
- A new root or an exclusion changes what is measured, and the coverage line
  says so on the same run.
- A finding survives a commit, a new prompt and a new session, and its `id`
  is the same in each JSON report.
- Accepting finding A holds A and leaves B failing.
- A `run` command that exits without writing the report is ERR, and a run
  that writes it is judged on the new report.
- Runner: gates run in catalogue order, every gate runs after a failure, ERR
  beats FAIL in the exit code, `--gate` on an excluded gate, `--list` shows
  derived and pinned, no source root is exit 2 under `--strict` and a NOTE in
  the hook.
- Hook: build failure blocks every stop that changed the tree and stops
  after eight, an unchanged tree is reported and not blocked again, a
  command the shell cannot find is a NOTE and the gates run, a derived
  build's failure names its command and its manifest, a new prompt
  restores the eight, a build failure writes a red verdict and the next
  prompt does not move the stamp, a gate failure blocks once and a second
  time only over a changed tree and never a third, a build block leaves the
  gate's tree alone, a host flag alone spends no second gate block, a
  follow-up prompt a host submits brings no fresh budget, a second
  session's prompt in the same worktree does not spend it, unreadable event never blocks, the verdict is
  written.

- Guard: one test per deny route including `init`, `install` and `turn reset`, one per
  ask route, every reader allowed including `git rev-parse` and `git cat-file`
  on the ref, every file that left the guarded set allowed for both an edit
  and a write, a heredoc opened inside a command substitution allowed, glob
  does not match by empty prefix, quoted pipe does not split, under 50
  milliseconds.
- Init: plain `init` writes `{}`, `--pin` writes only guardrails and keeps
  every value a person wrote, `--add`, `--force`, `--hooks`, `--host` and
  `--global` are usage errors, and it never touches `.gitignore`.
- Install: the marker lands at the repository root from a nested directory, a
  marker a person wrote is kept, a host with no evidence is refused with the
  supported names, a plugin-owned host gets no entries, each host's file
  carries one entry per event in that host's shape, a stale matcher and a
  stale command are replaced, a partial install is repaired, a duplicate entry
  of klin's is removed, an entry of klin's on an event klin no longer writes is
  removed, another tool's entries survive, a second complete run writes no
  file, an unreadable host file leaves every file untouched, a written line
  exits 0 when no binary resolves and only the stop of an opted-in tree says
  how to install it, `--user` writes the person's own file and
  no home-directory `klin.json`, and the project form outside a repository is
  refused.
- State: default under the git directory, per worktree, `KLIN_STATE_DIR`
  relocates it, two clones never share a stamp, an unwritable directory never
  blocks, `cache clean` removes only klin's files.
- Install: the plugin wrapper fetches the pinned version once and verifies the
  checksum, a second run makes no network call, a missing network on first run
  lets the turn end with a message.

Extension:

- Fixtures with hand-checked values for complexity per language (ADR 0001).
- A `run` sarif entry over a tool that needs no dependencies.
- The performance fixture of section 13.

## 18. Implementation Checklist

Core, in this order:

- [x] Guard: fix the glob prefix and quoted splitting, add the `ask` decision
- [x] State directory under the git directory, `KLIN_STATE_DIR` override,
      `cache clean`
- [x] The turn stamp as a commit with HEAD as parent, under `refs/worktree/klin/turn`
- [x] Amend ADR 0016 and 0017 to match 5.4, 6.2 and 6.6
- [x] Amend ADR 0016, 0020 and 0022 for the fourth review of section 0

- [x] One stamp rule for session start and prompt, `turn reset` for a person,
      the `turn` file written atomically with its prompt counter, a lock on
      the state directory for the whole stop, the build stamp counting blocks
      under the prompt counter and stopping after eight, a red verdict before
      a build block

- [x] The hook reads the turn window, writes the verdict, restores a missing
      `turn` file from the ref, and judges the branch when both are gone
- [x] The guard denies `init` and `turn reset`, asks on a non-reader that
      names `klin.json`, and allows every file that left the guarded set

- [x] Survey at run time from the derivation commit, cached by it, derived
      values printed. Derived numbers come from the derivation commit's own
      paths. Roots, languages, documents and manifests are the union of that
      survey and the `after` walk, and a site under a path the survey did not
      hold is `new`.
- [x] `doc-citations` and every other derivable check compare to `before`
- [x] No source root is exit 2 under `--strict`, and `--list` says derived or
      pinned per key
- [x] Derived complexity ceilings, floor and minimum sample, floor for a new
      language
- [x] Pinned dated ceilings in every check that takes a ceiling, in UTC, date
      printed
- [x] One key vocabulary, old names print the new one, differential test
      retired
- [x] Unreadable file is a NOTE in the hook
- [x] Coverage counts on every `OK:` line, matched site and both values on
      every failure, a finding `id`, all in the JSON, and the coverage
      regression NOTE and strict failure
- [x] Host adapter, Claude Code first, then Codex (ADR 0030). What the README
      may claim about hosts is #169's audit, not this row.
- [x] Three escapes rows: `fit(`, `fdescribe(`, `xfail`
- [x] Cross-file move matching by body hash (4.4)
- [x] `inventory` over test files and test functions, with the
      deleted-subject NOTE and the body-hash rename match
- [x] `stubs`, sharing the escapes engine, executable function bodies only,
      the empty test body included, with a legitimate-change fixture per row
- [x] `sarif`, `after` only, delete `report`, `run`, read `report`, scoped to
      changed lines
- [x] `lockfile`, Rust, npm and Go, with pnpm, yarn, Poetry and uv deferred
- [x] `CONTEXT.md` takes Window and Derived, README names the two
      conformance levels
- [x] New ADRs for each row of section 0 that is accepted, and one for the
      stamp as a parented, ref-held commit

Next, after core is green, each with an unsolved problem named in section 8:

- [ ] `duplication`
- [ ] `sarif` with `compare`, once the dependency problem of 8.3 has a design

Distribution, in this order, because each step depends on the one before:

- [x] Release pipeline: four binaries and a checksum file per tag (#62)
- [x] Install script, one per release, so a URL under a tag pins the version
      and the script needs no `--version` flag (19.1)
- [x] The Claude Code plugin with `hooks.json` and the `bin/klin` wrapper (#66)
- [x] Explicit hooks for Codex and its host adapter (#68)
- [x] Explicit hooks for Cursor and its host adapter (#67)
- [x] `klin install`, the standalone route's reconciler (#216)
- [x] The standalone agent skill and its project/user reconciliation contract (#217)
- [x] The GitHub Action
- [ ] Homebrew tap, `cargo install`, npm wrapper (#64). None of them ships,
      and 19.1 records why the spec advertises no install command for them.

Recommended:

- [x] Configuration reference (#18) generated from each check's declared keys
      and derivation rules, section 5.8
- [ ] `--sarif` output (11.3). #65 closed without it.

Before calling it 1.0. This list is the v0 draft's. #207 holds the current
1.0 criteria, and #344 will write the stability contract:

- [x] Performance numbers from section 13 recorded on a fixture
- [x] Large-repository budgets and the controlled-release regression rule in
      section 13 (#182)
- [x] A task comparison with and without klin on a small set of agent tasks,
      recording regressions caught, legitimate changes blocked, extra repair
      turns and hook latency. This is a benchmark, not a test. Three
      publishable rounds ran, and the last one was inconclusive
      (`docs/benchmark-result-2026-09-25.md`). Since 2026-09-25 the benchmark
      is internal regression tooling and no longer gates 1.0 (#207).
- [ ] The hook-output facts in 9.3 verified against the host's documentation
- [x] Cursor adapter, or the README stays silent on it

## 19. Installation and Distribution

### 19.0 Support status, delivery, and the words for a scope

Two separate axes place a host. Support status says who keeps an integration
working. Delivery says how klin reaches the host. A first-class host is
reached by either of two delivery mechanisms, so the axes do not collapse into
one list of integration types.

Support status:

1. **First-class integration.** A host klin maintains as part of its
   compatibility promise: Claude Code, Codex CLI and Cursor. Each has a
   built-in adapter, compatibility evidence klin owns
   (`docs/HOST_COMPATIBILITY.md`), and the native plugin and standalone routes
   klin supports for it.
2. **Custom harness integration.** A harness-specific integration maintained
   outside klin's compatibility promise, which maps its harness into the
   harness protocol (9.7, 19.4). Using the protocol does not make a host
   first-class.

Delivery and interoperability:

1. **Native plugin.** The native plugin klin ships for a first-class
   host. It is the native alternative to the standalone route: Claude Code and
   Codex install and update it, and Cursor's verified route is a local copy a
   person makes from a release tag. It carries
   the host hooks, the klin skill and a pinned wrapper that fetches a pinned
   runtime. Section 19.2.
2. **Standalone route.** The klin binary and `klin install`, the route
   documents lead with on every first-class host, because it alone gives the
   person the `klin` command. It owns explicit
   hook files a repository or a person commits or keeps, standalone skill
   placement, managed and manual installations, and every first-class host
   surface that has no native plugin. Sections 19.1 and 19.3.
3. **Harness protocol.** klin's versioned event and decision contract of 9.7,
   which a custom harness integration implements. klin states the protocol and
   ships a reference adapter for it in `harness-protocol/`; it ships no adapter
   for any particular harness.

"Standalone integration" is not a category: the standalone route delivers a
first-class integration and is not a tier of support.

A person on a first-class host takes either route, and documents lead with the
standalone route: the installer, then `klin install` (ADR 0053). A plugin is
self-sufficient. The wrapper it carries fetches the runtime, so a plugin user
installs no binary for the hooks to work. It gives the person no `klin`
command, and it MUST NOT install one: the wrapper names the command that does,
once (19.2). A custom harness integration uses the binary of 19.1 and not this
route (19.4).

No route gates a repository by itself. Under `--hook` a `klin.json` at the
repository root is the marker that the repository opted in, and a tree without
one stays silent (5.1, ADR 0028). Installing a plugin makes klin available to
every repository the host opens. It opts none of them in.

The contract uses five scope words, and no others:

- **project** — integration the repository owns, committed with it. It is
  portable with the repository wherever the host loads project configuration.
- **user** — integration for one person on one machine. It is not portable
  with the repository, and it MUST NOT be described as reaching a cloud or
  remote agent.
- **plugin** — a first-class integration the host manages and klin maintains.
- **managed** — host policy or configuration an organization controls.
- **custom** — a harness that is not first-class and implements the harness
  protocol of 9.7.

`global` is too broad a word for a user-scope install, so this contract does
not use it for one. The CLI uses the same word: `klin install --user` writes
the user scope, and no flag is called `--global`.

### 19.1 The binary

A pushed tag `vX.Y.Z` builds the binary for macOS and Linux, on x86_64 and
arm64, and attaches the four archives, a `.sha256` beside each one, a
`sha256.sum` over all of them, and the install script to a GitHub release.
The release stays a prerelease until the promotion of 19.2 marks it Latest,
so `releases/latest` names the last promoted release. `dist` runs that
pipeline, so its artifact names and its install script are what a route
consumes. ADR 0026 records that choice. `klin --version` prints the version
the binary was built from, which is the tag without its `v`. Every route below
downloads from that release and MUST verify the checksum.

Two routes ship:

1. The install script the release carries, `klin-installer.sh`, run through
   `sh`. It detects the platform, verifies the checksum it was generated
   with, and puts `klin` and `klin-update` in `~/.local/bin`, or in the
   directory `KLIN_INSTALL_DIR` names. Each release carries the script that
   installs that release, so a URL under a tag pins a version.
2. A GitHub Action, `action.yml` at the root of this repository, that
   installs a pinned version and runs `klin gate --strict`. For CI only. A
   workflow pins it by the release tag, `uses: brajevicm/klin@vX.Y.Z`, so one
   tag names the binary, the plugin and the Action.

The plugin wrapper of 19.2 is not a third route into this list. It fetches the
same release for the plugin alone, into the plugin's own cache.

No other channel ships. There is no Homebrew tap, no npm package and no
published crate, and this document MUST NOT print an install command for one.
Each of them is a distribution channel with its own release obligations, and
none is required to make klin work on a supported platform, so each stays
deferred (#64). #315 ships Homebrew and npm once releases are public. A future
channel is added here only once it ships.

The supported binary targets are macOS and Linux, on x86_64 and arm64. The
plugin wrapper resolves that same set, so the public contract and the release
targets stay one list. On Linux the install script refuses a glibc older than
the one on the runner that built the binary, which is 2.35 on `ubuntu-22.04`.
klin ships no musl build. Native Windows is not a supported target: klin
builds no Windows binary and ships none. WSL is not a documented supported route
either, because klin has no compatibility evidence for it. A person on Windows
has no shipped klin install today.

### 19.2 The first-class plugins: Claude Code, Codex CLI and Cursor

One plugin directory serves all three hosts. It holds `hooks.json` with the
three hooks of 9.2, Cursor's flat `hooks/cursor.json` with the same three
commands on Cursor's event names, one skill that tells the agent how to read a
failure and what it may not touch, two slash commands that run the gates and
list them, and a `bin/klin` wrapper. Each host's manifest names its hooks
file, so none of them has to find it by convention. Cursor's manifest MUST
name `./hooks/cursor.json`, because the default `hooks/hooks.json` is Claude
Code's nested shape. ADR 0045.

The plugin is self-sufficient. It carries the hooks, the skill and the pinned
wrapper, and the wrapper fetches the pinned runtime on first run. A plugin
user MUST NOT be told to install the standalone binary of 19.1 to make the
plugin work. The one exception is the managed case below, where the host gives
the plugin no usable `bin/`.

**Claude Code.** klin maintains a native Claude Code plugin, the host-managed
route. The project route of 19.3, under `.claude/`, stays the portable and
manual alternative, and it is what a person takes where user settings are not
available. A plugin a person installed, and the settings that enable it, live
on that person's machine. This document MUST NOT claim that they exist in a
remote or cloud environment.

**Codex CLI.** klin maintains a native Codex plugin for the Codex surfaces
that load plugins. Codex CLI reads the same manifest and the same `hooks.json`
shape. It substitutes the literal `${CLAUDE_PLUGIN_ROOT}` into a plugin's hook
line, and the hook lines do not rely on the variable being exported, because a
shell default form such as `${CLAUDE_PLUGIN_ROOT:-}` was left unsubstituted
and expanded to nothing. Claude Code exports the variable and reads the bare
form the same way, so the hook lines name the plugin root in that form and no
other, and the same lines run on both hosts. Codex finds the plugin through a
marketplace file of its own at `.agents/plugins/marketplace.json`, which names
the same plugin at the same release tag as Claude Code's
`.claude-plugin/marketplace.json`. The install is `codex plugin marketplace
add brajevicm/klin` and `codex plugin add klin@klin`. Installing a plugin does
not trust its hooks: Codex skips an untrusted plugin's hooks until the person
reviews and trusts the current hook definition through the CLI `/hooks`
surface, and a fresh session then runs them, so the install documentation
names both steps. The Codex IDE extension's contract loads no plugins, so
klin's plugin support MUST NOT be described as covering it; that surface takes
the standalone route of 19.3.

**Cursor.** klin maintains a native Cursor plugin. Cursor finds it through
`.cursor-plugin/marketplace.json` at the repository root, which points at the
same directory. Cursor documents only a path source, so this entry names the
directory in the marketplace's own tree and no tag. Cursor Teams import that
repository under Dashboard → Plugins → Team Marketplaces. A person without a
team marketplace copies `plugins/klin` from the release tag the manifests pin
to `~/.cursor/plugins/local/klin` and
reloads the window, which is a user-scope install for that machine alone. A
copy made again from that tag takes a released plugin and never a wrapper from
an unreleased branch (ADR 0029). The copy instructions a document gives MUST
be idempotent: a second run leaves one usable copy and never nests one plugin
inside another. The Team Marketplace import has no recorded verification
(`docs/cursor-compatibility.md`), so a document MUST label it as such rather
than present it as a verified route. Cursor skips a symlink whose target sits
outside that folder. The Cursor hook lines name
`${CURSOR_PLUGIN_ROOT}/bin/klin` in that form and no other, the way Claude
Code and Codex name `${CLAUDE_PLUGIN_ROOT}`. Cursor expands both variables. A
project `.cursor/hooks.json` (19.3) stays the portable and manual route, and
it is the route for an environment that loads a repository's hooks but not a
person's own. A user-scope Cursor hook or skill is local to that machine. This
document MUST NOT call it global, and MUST NOT imply that it reaches Cursor
Cloud Agents.

**The release a marketplace installs.** Claude Code and Codex read a
marketplace from the repository's default branch, and a relative path there
copies the plugin as `main` holds it. Their two entries MUST therefore name
`plugins/klin` at the release tag `vX.Y.Z` that the manifests pin, through the
`git-subdir` source both hosts document. Each entry MUST spell the path as its
host documents it: `plugins/klin` for Claude Code, `./plugins/klin` for Codex.
A plugin installed from either marketplace then holds the files of the release
its manifest names, as long as the tag does not move, and a commit to `main`
that changes `plugins/klin` reaches a plugin user only with the next release.
The release commit rewrites the `ref` with the manifests. A release MUST push
its tag alone and MUST stay a prerelease until the promotion. `main` MUST take
the tag, and the release MUST become Latest, only after the release smoke of
`docs/HOST_COMPATIBILITY.md` passed. CLI tests fail when a `ref` is not `v`
followed by the crate version, when the release would rewrite a marketplace
file anywhere but its `ref`, and when the release configuration lets
`cargo-release` push or lets a release become Latest before the promotion. ADR
0029 records the decision.

The pre-tool matcher names the union
`Write|Edit|MultiEdit|NotebookEdit|Bash|apply_patch|mcp__.*` of the tools
Claude Code and Codex CLI emit, so the guard reads edits and MCP calls on both
hosts. A name missing from this matcher leaves that host's corresponding tool
call unguarded; a name the host never emits is dead text. `klin install` uses
the same union for both hosts, keeping the standalone route aligned with the
plugin. ADR 0030 records the decision.

The wrapper is a shell script. It reads the version from the plugin manifest
beside it, so the plugin carries one pin. On first run it downloads that
release into `~/.cache/klin/bin/<version>/klin`, verifies the checksum, and
executes it. That install removes every other version that no session ran for
seven days: another host's plugin may pin another version into the same cache,
and removing it at once would make the two fetch in turn. A `radius` run
touches its version to mark it used. Every later run executes the cached
binary with no network call. When the download fails, the wrapper runs a
`klin` that PATH resolves if there is one, and otherwise prints one line
saying so and exits 0, so a turn is never blocked by a missing network. This
is the one place klin touches the network, and it is install, not measurement.

The plugin never installs a `klin` command for the person (19.0). Once per
machine, at the first `radius` run that printed nothing, the wrapper says in
one `systemMessage` that the CLI exists and names the command that installs
it. The hint is never a `followup_message`: Cursor submits a stop's
`followup_message` as the next prompt, which would hand the installer to the
agent. It says nothing where PATH resolves a `klin` other than the wrapper
itself, and nothing under Cursor, which shows no message at a prompt. A file
beside the cache records that the hint was given.

Every line the wrapper or a hook prints on exit 0 is a JSON object with a
`systemMessage`. A notice carries `followup_message` with the same text too,
except a notice that names an install command, which carries `systemMessage`
alone for the reason above. Claude Code and Codex show `systemMessage`.
Cursor's native stop shows `followup_message`. Codex rejects plain text on a
Stop that exits 0, and Claude Code writes it to the debug log alone.

Each hook line runs `${CLAUDE_PLUGIN_ROOT}/bin/klin` when that file is
executable, and otherwise the `klin` that PATH resolves. Claude Code appends
every installed plugin's `bin/` to the end of PATH, for hooks as for the Bash
tool, so a binary an installer left in `~/.local/bin` would win over the
wrapper if the hooks called `klin` by name. Calling the wrapper by its path
makes the version the plugin pins the one Claude Code runs. Where `bin/` is
unavailable, which is the managed case of a plugin distributed through
organization settings, the hooks find `klin` on PATH from a route in 19.1, and
the skill says which command installs it. When neither is present the Stop
hook says so once and lets the turn end.

The hook lines resolve the binary before they run it, because a person may
install the plugin where no binary resolves yet. With neither the wrapper nor
a `klin` on PATH the session start, the prompt and the pre-tool events say
nothing and block nothing. The Stop hook names the install command in a
`systemMessage` on stdout, and only where a `klin.json` resolves at the
project root, so a tree that never opted in stays silent (ADR 0028). Nothing
blocks, so the turn ends at that stop and the line appears once. Cursor shows
no `systemMessage` at a stop and submits a `followup_message` as a prompt, so
the Cursor line's notice is written and not shown.

The wrapper reads two overrides, `KLIN_RELEASE_BASE_URL` and `KLIN_CACHE_DIR`.
They exist so a CLI test can fetch a release of its own over `file://` and
prove the two paths that a real release cannot: the first run that installs,
and the failure that installs nothing.

Installing the plugin is the whole install of the hooks for the host. It is
not an install of the `klin` command, and it is not the install for a
repository: no `init` runs, and the repository opts in through its own
`klin.json` (5.1). Once it has one, the first stop is gated.

This reverses ADR 0002. Its first reason, a version pin beside committed
baselines, went with ADR 0009. Its second reason is handled by the PATH
fallback above. A version difference between the wrapper's binary and a CI
binary is a NOTE per 5.2, not a failure.

### 19.3 The standalone route: `klin install`

The standalone route is the one documents lead with (19.0, ADR 0053, ADR 0055,
ADR 0056). On this route the binary comes from 19.1, and `klin install` is the
one command that installs and repairs the integration. It serves the person who wants the
`klin` command, a team that wants hooks committed and covered by CODEOWNERS,
and a host surface that loads no plugin. A plugin user may take it later for
the command, and the hooks it commits serve a teammate without the plugin. ADR
0046 records the command.

`klin install` does three things in one run: it opts the repository in, it
selects the hosts to serve, and it reconciles the explicit hook files klin
owns.

**The repository opt-in.** Inside a git repository, `klin install` writes the
`klin.json` marker at the **repository root** that `git rev-parse
--show-toplevel` names, and not at the directory the command was run from. So
a run inside `repo/apps/web` writes `repo/klin.json`. A marker that already
exists is a person's, and its content MUST be kept whole. Outside a repository
the project form is refused and names the `--user` form; `--user` outside a
repository opts no repository in and says so. `--user` MUST NOT write a
`klin.json` beside the home directory.

**Host selection.** A supported host is a candidate when `--host NAME` names
it, when the scope holds that host's own configuration directory, or when klin
can prove that host's native plugin is enabled for that scope. A marker
directory is evidence of the host and never of the install. Where several
hosts are provable, every one of them is reconciled unless `--host` narrows
the run. Where a repository holds no host's configuration directory and no
`--host` is given, every first-class host is reconciled, and the run says so
and names `--host`: a repository serves a team whose hosts klin cannot see,
and a hook file for a host nobody runs does nothing. A plugin the person's
home enables does not narrow this, so what the repository gets does not
depend on who runs the install (ADR 0056). Under `--user` a home that proves
no host is refused, and the refusal names the supported `--host` values. `--host` may be named again for a second host.

**A plugin beside the committed hooks.** A selected host whose native plugin
already supplies klin's hooks still receives its explicit entries and the
skill, and the run names the file that proves the plugin and says that on this
machine the committed copy yields on each event the plugin took first. The
plugin registers the same host events, so the host runs both copies, and 9.8
makes one of them take effect. A teammate without the plugin gets the
committed hooks, and a person who removes the plugin keeps them with no second
install. Each host's adapter knows where that host lists its enabled plugins.
Claude Code lists them under `enabledPlugins`
in its settings files: for a repository write klin reads the repository's, the
local ones beside them and the user's, and for a user write the user's alone,
because a plugin one repository enables gates that repository and not the
machine. Codex CLI lists them as `[plugins."klin@<marketplace>"]` tables in
`config.toml`, on unless the table says `enabled = false`, and klin reads the
repository's and the user's the same way. Cursor's documented local layout is
`.cursor/plugins/local/<name>`, and Cursor 3.20.21's observed marketplace
cache is `.cursor/plugins/cache/<marketplace>/<plugin>/<revision>`. klin
reads `.cursor-plugin/plugin.json` named `klin` at exactly those two depths,
under the project and the user's home, and a klin manifest anywhere else
under `plugins`, such as a marketplace's own source, is not an installed
plugin. Cursor records no enabled state klin can read, so the run says that
Cursor may load the copy it names. A repository write beside a user file that
already holds klin's entries is written the same way, and the run names that
file.

**Reconciliation.** The files klin writes are:

- Claude Code's `.claude/settings.json`, with `SessionStart`,
  `UserPromptSubmit`, `PreToolUse` and `Stop`. `PreToolUse` is the one entry
  that carries a matcher, the union of 19.2.
- Codex CLI's `.codex/hooks.json`, in the same nested shape and on the same
  event names, at turn scope.
- Cursor's `.cursor/hooks.json` at schema version 1, with `sessionStart`,
  `beforeSubmitPrompt`, `preToolUse`, `beforeShellExecution`,
  `beforeMCPExecution` and `stop`. `preToolUse` is the one entry that carries
  a matcher, `Write|Edit|Delete`. A shell or MCP event names no tool, so a
  matcher there MUST NOT be written: it would match nothing and gate nothing.

For each selected host the run MUST bring klin's own entries to the canonical
contract above, and not merely add what is missing:

- a command klin owns is one that runs the klin binary on `radius`, `guard`
  or `gate`. A command that runs another klin command, and a command that only
  mentions klin, is a person's. Claude Code and Codex nest several commands
  under one entry, so ownership is judged per command and never per entry: a
  person's command that shares an entry with klin's MUST survive, with its own
  text, when klin's command is taken out of that entry.
- a missing canonical entry is added;
- a klin-owned entry whose command, matcher or event is stale is replaced,
  including a matcher from before #214;
- a second klin-owned entry on one event is removed, because two copies run
  the lifecycle twice;
- a klin-owned entry on an event klin no longer writes is removed, and an
  event that removal empties goes with it;
- every command that is not klin's keeps its text and its order, and an entry
  left holding no command at all goes;
- one klin hook in the file MUST NOT be read as a complete install: a partial
  install is repaired.

This is how a person on the binary route receives a later fix without deleting
a hook file by hand.

Each line klin writes resolves `klin` on PATH before it runs it and ends the
hook when none resolves, the way the plugin's own lines do (19.2). After PATH
it looks in `~/.local/bin`, where the install script of 19.1 puts the binary,
because a host started from the terminal that ran the installer has no such
PATH yet (ADR 0056). A `klin` that PATH resolves still wins. A person
who never installed the binary, or who removed it, sees nothing rather than a
failed hook on every event. The one exception is the stop of a repository that
holds a `klin.json` at its Git root, which the line resolves with `git
rev-parse --show-toplevel` because a session may start below the root: there
the line names the install command in a `systemMessage` alone, so a teammate
who cloned the committed hooks learns what they are for. Cursor shows the
person no stop field that is not also a prompt, so on Cursor the notice is
written and not shown (`docs/cursor-compatibility.md`). A host whose klin
plugin is enabled gets the committed hooks and the skill all the same, and the
run says that the committed copy yields on this machine (above, 9.8). Cursor
runs the hooks in Claude Code's settings files by default, beside its own
(`docs/HOST_COMPATIBILITY.md`), so a repository that commits klin's hooks for
both hosts runs two copies in Cursor, and 9.8 makes one of them take effect
there too. Codex
skips a project hook file's hooks until the person trusts them through
`/hooks`, as it does a plugin's (19.2), so a document that gives the
standalone route for Codex names that step.

**Standalone skill.** The standalone route writes the exact text authored at `plugins/klin/skills/klin/SKILL.md`; the binary embeds that source so the plugin and standalone copies cannot drift.

At project scope the selected hosts receive:

- Claude Code: `.claude/skills/klin/SKILL.md`
- Codex and Cursor: `.agents/skills/klin/SKILL.md`

At user scope, `klin install --user` writes the corresponding paths under the person's home directory: `~/.claude/skills/klin/SKILL.md` for Claude Code and `~/.agents/skills/klin/SKILL.md` for Codex and Cursor. Codex and Cursor sharing a path produce one planned write and one output line. A native plugin that serves the selected host and scope carries the skill too, and klin writes the standalone copy all the same, so a teammate without the plugin has it.

Skill targets participate in the same preflight as hooks. A missing file is written, a byte-identical file is already current, and a different existing file is an explicit conflict that is never overwritten. A later binary may reconcile an older standalone file only when klin can prove it owns that file; without that proof, the different file is preserved and refused. The conflict is found before the marker or any host integration is written. Rerunning `klin install` is the reconciliation step after a binary update.

The standalone route copies the skill only. Slash commands and other host-specific command surfaces remain plugin-owned. User scope is local to one machine and does not reach a cloud or remote agent.

**Preflight and partial failure.** One run may touch several files. It MUST
resolve the repository root, the selected hosts, the plugins beside them, every
target path and every host file's shape before it writes anything, so a
deterministic error leaves every file as it was. Each owned file is written
whole, through a neighbour and a rename, so a run that dies partway leaves the
file it found. It follows a path that is a link, so a settings file kept in a
dotfiles tree stays a link, and it keeps the permissions the file had. Where a
filesystem failure still happens after the first write, the output MUST name
what was written and what was not, and the run MUST NOT print a plain success.

**Scope.** The default is project scope: the files are committed, a teammate
who clones gets the hooks, and CODEOWNERS SHOULD cover them. `--user` writes
the host's user-level file instead — `~/.claude/settings.json`,
`~/.codex/hooks.json`, `~/.cursor/hooks.json`. That write covers every
repository the person opens on that machine, it is not committed, it does not
travel with the repository, and this document MUST NOT claim it reaches a
cloud or remote agent. A person who chooses it accepts that an agent can
remove the lines. No flag is named `--global` (19.0).

**Output.** A run prints one line per component: the repository marker, and
each host klin knows with what happened to it — reconciled, already current,
or not requested — and, where a plugin or a user file also serves the host,
that the committed copy yields on this machine. The marker's own line says to commit
it, because the repository's opt-in travels with the repository at either
scope. One closing line follows, and it speaks of the host files alone: a run
that wrote none of them MUST NOT tell a person to commit hooks, and MUST say
the integration is already current. So a second run over a complete
installation writes no file and says exactly that. Raw host configuration is
not printed unless there is an error.

**Guard.** The hook files are not guarded (9.4), but `klin install` writes a
person's configuration and integration files, so the guard refuses the command
from an agent exactly as it refuses `klin init` and `klin turn reset`.

### 19.4 A custom harness

A harness klin does not maintain integrates through the harness protocol of
9.7: klin's own versioned event on stdin, klin's own decision on stdout, and
the same three commands. It supplies the binary from 19.1 and translates its
own lifecycle into that protocol. klin ships no adapter, no hook file and no
skill placement for such a harness, `klin install` connects none, and a
custom integration is not part of the compatibility promise that covers the
first-class plugins. The protocol is a
portability seam, not a second engine: a protocol event normalizes into the one
internal event of 9.1 and runs the same turn, guard and gate loop.

Claude Code, Codex CLI and Cursor are first-class and MUST NOT route through
the harness protocol in production. Each keeps its built-in adapter, its native
plugin, its compatibility tests and its own documentation. A custom harness
integration stays custom until a separate ticket promotes the harness: that
ticket proves the host's current official semantics, adds a built-in adapter or
native plugin, adds adversarial compatibility fixtures, defines install, update
and trust behavior, and moves the harness into the first-class matrix.
Speaking the harness protocol alone MUST NOT be described as first-class
support.

**The protocol directory and the guide.** `harness-protocol/` carries the two
schemas, one fixture per event kind and `reference-adapter.sh`, a minimal
reference of the protocol boundary that integrates no host;
`docs/HARNESS_INTEGRATION.md` carries the worksheet, the lifecycle mapping and
the conformance levels. The two MUST be enough to port a harness without
reading klin's Rust adapters. The reference adapter stays small and
dependency-light: it demonstrates the translation and is not a second
supported host runtime. `harness-protocol/` MUST NOT hold a copy of the skill.
klin's canonical skill is the one authored text of 19.3, and the guide points a
custom integration at it.

**Conformance levels.** A custom integration states its level, and states what
is missing rather than claiming equivalence with a native one.

- **Full** — session or prompt lifecycle, pre-tool interception with proven
  evidence, an end-of-turn hook, and a block that returns the report to the
  agent. One difference from a first-class host remains: klin's `ask` has no
  channel this protocol can prove, so the ambiguous class of 9.4 is refused
  (9.7).
- **Gate** — an end-of-turn check that reports a failure, missing one or more
  of the pre-tool and turn-feedback capabilities. The missing ones are named.
- **Manual / CI** — no reliable lifecycle hook. A person runs `klin gate`, and
  CI runs `klin gate --strict`. There is no same-turn feedback contract, and
  none is claimed.

### 19.5 CI

The Action installs the pinned version and runs `klin gate --strict` over
the workflow's own checkout, which needs `fetch-depth: 0` for the base commit
to resolve. An `args` input appends flags such as `--gate` names or
`--json`. The version comes from the `version` input, then the tag the
workflow pinned the Action at, then the latest release. The Action never reads
`klin.json`, which holds no version (ADR 0040). The Action runs the install
script under that release's tag URL, and that script verifies the checksum, so
a mismatch fails the job before any gate runs. A workflow without the Action
runs the same script and the same command.

### 19.6 Upgrades

A new klin version may change a measurement. Under the base commit model both
trees are measured by one binary, so an upgrade changes nothing about any
verdict except where a new check applies. A new derivable check runs on the
first stop after the upgrade, against a derived ceiling from the base tree,
so it is green on arrival. The plugin pins its own version and upgrades when
the plugin does, through `/plugin marketplace update` or the host's
auto-update. Every other route upgrades when the person asks. `klin update`
runs the `klin-update` beside the binary, or the one PATH resolves, which
installs the Latest release of 19.1 over the current one, and its exit code is
the updater's. Where no updater is found, `klin update` says so, names the
installer, and exits 2. ADR 0029 records that one tag names every route.
