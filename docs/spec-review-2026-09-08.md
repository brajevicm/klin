# Review of docs/SPEC.md, 2026-09-08

A review of `docs/SPEC.md` draft v0 against its own stated purpose: one
optional configuration file, no baseline a person maintains, green on the day
it arrives, and tighter from then on.

This review reads the spec on its own terms. It does not consult
`docs/adr/`. It cites spec sections, not decisions, so every finding stands or
falls on the text of the draft. It is a review, not a decision. Where a
finding needs a decision, the spec's own section 0 is the place to record it.

## 1. Does the tool make sense

Yes.

The premise holds. An agent optimizes for a green result, and the cheap routes
to green are legal code that a language linter passes. A silenced check, a
deleted test and a stub are all valid programs. A ratchet against a second
tree is the right instrument for that class of failure, because the judgement
needs two measurements and no linter holds two.

Section 8.1's four criteria are the strongest part of the document. They are
the reason `stubs` earns a place and `asset-path` does not. Keep them, and
apply them to every later proposal.

## 2. Is it well scoped

The core is well scoped. Sections 4, 6, 7 and 16 are a specification. A person
could build from them and get one answer.

Two parts are not well scoped.

### 2.1 Section 8's catalogue is too large to call core

Section 18 puts `duplication` and `sarif` with `run` in the core list, beside
items that take a day. Those two take weeks and each carries an unsolved
problem. A core list that mixes both sizes hides the real critical path.

### 2.2 Section 8.3 defers the hardest problem in the document

Section 8.3 marks the supply of dependencies to a bare `before` worktree as
implementation-defined. That supply is the whole difficulty of the linter
seam.

A symlink of today's `node_modules` into a worktree at an old commit runs a
new tool over old source. The result is then not a pure function of the
`before` commit, so it breaks the guarantee in section 12 and the caching
rule in section 4.3. An old commit may also fail to parse under today's linter
configuration, which turns a real `new` finding into a `held` one.

## 3. Major gaps

### G-A. The session boundary launders debt (6.2)

Section 6.2 states the green condition for one event only, a prompt
submitted. Session start writes the stamp with no condition. The stamp is a
tree of the working directory, including uncommitted and untracked files.

The consequence: an agent adds debt, takes a red stop, and the session ends.
The next session starts and stamps the dirty working tree. That debt is now in
`before`, so it reads as `held`. It also joins the distribution that derives
the ceiling in 5.4, which loosens the ceiling for everything that follows.

This defeats the rule that 6.2 exists to enforce. The HEAD fallback in the
same paragraph is safe, because HEAD is committed and holds none of the
uncommitted work. So two clauses in one paragraph choose `before` differently
for the same situation, and one of them leaks.

**Fix:** stamp HEAD on session start, or carry the previous red verdict across
the session boundary and keep the window closed.

### G-B. A person has no escape hatch (6.2, 9.5)

The window stays open until a person fixes the failures or accepts them. Only
a person writes an accepted entry, in a reviewed commit.

A person who abandons the work has neither route. The debt is real and it sits
in the tree, so the report is honest. But it now attaches to every later stop
in the session. The block-once policy of 9.3 prevents a livelock, so the run
degrades into a report that nobody acts on. A report that nobody acts on
trains both the agent and the person to ignore klin's output, which costs more
than the debt did.

**Fix:** give a person one command that moves the window and prints that a
person moved it. A reviewed config commit is the wrong instrument for
abandoned work.

### G-C. The survey cache never hits in the hook (6.6 against 13)

Section 6.6 keys the survey cache by the `before` commit. Under the turn
window the `before` commit is the turn stamp, which is a new dangling commit
on every turn.

So the first stop of every turn misses the cache and pays a whole-tree parse.
Section 13 grants 5 seconds for a warm cache and 30 seconds for a new base.
The hook is the path that needs the 5 seconds, and under this design it
almost never gets them.

**Fix:** derive values from the nearest committed tree, not from the stamp.
HEAD is out of the agent's reach inside one turn, which is the anti-gaming
property 6.6 actually needs. The cache then hits for every turn between two
commits.

### G-D. An absolute check contradicts the day-one-green MUST (5.1 against 8.2)

Section 5.1 says a run with no configuration MUST be green on any tree with a
base, because nothing in the working tree is worse than the same tree.

Section 8.2 marks `doc-citations`, `hallucinated-deps` and `inventory` as
absolute. An absolute check does not compare two trees, so the reason 5.1
gives does not apply to it. A repository that holds one stale document
citation is red on the day klin arrives, with no configuration, under `klin
gate`.

The MUST is false as written. Under `--changed` in the hook the failure hides,
because an untouched file is out of scope. That makes the contradiction
intermittent rather than absent, which is worse.

**Fix:** qualify the MUST to ratcheted checks, or derive an accepted set for
each absolute check from the base tree on the first run.

### G-E. "No source root, pass" reopens a closed hole (14 against 10)

The last row of section 14 says a survey that finds no source root passes.
Section 10 retires the unaccounted-gate failure, on the ground that every
derivable gate runs.

A CI job with the wrong working directory finds no root. So does a shallow
clone that resolves no base tree. Every gate then applies to nothing and the
run is green. That is a silent gate, which is the exact failure the retired
rule caught.

**Fix:** make a survey with no source root exit 2 under `--strict`.

### G-F. Deleting `.klin/` is the cheapest route to green (6.2, 7.4, 15.1)

The stamp holds the verdict that keeps the window open after a red stop.
Section 6.2 says the file is not guarded. Section 7.4 says the directory is
safe to delete. Section 15.1 admits that a determined agent can delete it.

Apply section 8.1's criterion 4 to the act itself. `rm -rf .klin` is a thing
an agent does when it wants green cheaply. It is the single cheapest such act
in the whole design, because it converts every open failure into `held` in one
command.

Guarding one state file carries none of the cost that guarding the
configuration carries. A deleted stamp needs no reviewed commit to restore.

**Fix:** deny a write to `.klin/turn` in the guard, and report a missing stamp
on a stop as a NOTE that names the deletion.

### G-G. Self-tightening is overclaimed (5.4)

Section 5.4 claims that the distribution never worsens and the percentile
never rises.

The ceiling is monotone, and that part is correct. New sites stay under the
ceiling and existing sites cannot rise, so the rounded 95th percentile never
rises above the ceiling it produced.

The claim about the percentile is wrong. New code between the old percentile
and the ceiling raises the percentile without raising the ceiling. The
statement needs to be about the ceiling, not the distribution.

The larger problem is the rate. On a tree of 5,000 functions, 20 clean new
functions move the 95th percentile by nothing. The floor of `cc 5` and `lines
25` then stops the fall for good. So mechanism 1 tightens in principle and
barely tightens in practice on the trees that need it most. Mechanism 3, the
dated schedule of 5.5, does the real work.

**Fix:** correct the claim, and stop presenting the derived ceiling as the
answer to "tighter from then on". Present the schedule as the tightening
mechanism and the derived ceiling as the day-one default.

### G-H. `git gc` can prune the turn stamp (6.5)

Section 6.5 makes the stamp a dangling commit from `git commit-tree`. Nothing
references it. A garbage collection inside a session prunes it, and the next
stop reports "no stamp, from HEAD" with no cause a person can see.

**Fix:** write `refs/klin/turn` and read the ref. One extra command, and the
failure stops being undiagnosable.

## 4. Smaller items

- `accepted_since(stamp)` appears in 16.1 and has no definition anywhere. It
  is one of two escapes from a held-open window, so it needs a rule. The
  configuration is guarded, so the rule can compare the accepted list against
  the stamp's version of the file.
- The `doc_size` rule in 5.4 is an editing artifact. One sentence holds two
  rules and a negation of the first.
- Exit codes carry two contracts. Section 4.9 gives exit 2 to a measurement
  failure. Section 9.1 gives exit 2 to a Stop block. Section 14 gives exit 1
  to an unreadable host event. State plainly that a hook-mode exit code is the
  host's protocol and not the verdict.
- `inventory` fires on a legitimate deletion. Removing a feature removes its
  tests. The only remedy in the draft is a person's reviewed commit, taken in
  the middle of a turn. Add a rule for the case where the code under test is
  gone too.
- `init` refuses to overwrite and has no `--force` (5.7). So a person who wants
  to re-pin after the tree improves has no path. State the path, even if it is
  "delete the file first".
- Section 5.1 says discovery walks up from the working directory. A monorepo
  with one configuration per package is not addressed, although 5.4 claims the
  root rule handles a monorepo. Say which one is true.

## 5. Confirmed live: the guard refuses ordinary reads

Section 9.4 and gap G1 of `docs/design-gaps-2026-09-08.md` are not
theoretical. The guard refused two read-only commands during this review,
before anything was written:

    find src tests -name '*.rs' | sort
    head -140 CONTEXT.md && cat klin.json

The first command names no guarded path. The token `*.rs` splits at the star,
the prefix is empty, and an empty prefix matches every guarded name. The
second command is a pair of reads.

Each refusal cost one turn and had no remedy. This is the first item in
section 18 and it should stay first.

## 6. What to cut from core

Move `duplication` and `sarif` with `run` out of the core list in section 18.

Ship the report-only linter seam first. Read one SARIF report from the `after`
tree, key each result by file, rule id and message, and scope it to the
window's changed files. A rule the agent added in a file the agent touched then
fails. A rule the base already held does not.

That version costs one tool run, needs no `before` worktree, needs no
dependency story, and fits the hook budget of section 13. It delivers most of
the value the goal names. Add the `before` run later, when a user asks for the
cases it catches.

## 7. Fix table

| # | Section | Fix |
|---|---|---|
| G-A | 6.2 | Stamp HEAD on session start, or carry the red verdict across the boundary |
| G-B | 6.2, 9.5 | One person-side command that moves the window and says who moved it |
| G-C | 6.6, 13 | Derive from the nearest commit, not the stamp, so the cache hits |
| G-D | 5.1, 8.2 | Qualify the MUST, or derive a first-run accepted set per absolute check |
| G-E | 10, 14 | No source root is exit 2 under `--strict` |
| G-F | 6.2, 7.4 | Guard `.klin/turn`, and report a missing stamp as a NOTE |
| G-G | 5.4 | Correct the claim, and name the dated schedule as the tightening mechanism |
| G-H | 6.5 | Write and read `refs/klin/turn` |
