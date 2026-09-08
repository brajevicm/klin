# Design gaps, 2026-09-08

An evaluation of the tree, the 15 ADRs and the 40 open issues, against the
goal: one config file, no baseline a person maintains, green on day one,
tighter from then on, hooks first and CI optional. `docs/SPEC.md` holds the
design that closes these gaps. This file names the gaps and triages the
tickets.

## What already holds

ADR 0009 removed the baseline file. A run compares the working tree against
the base commit, and the tool writes nothing. That is most of what the goal
asks for, and it is done. `init` writes every section it can infer. The guard,
the block-once Stop hook and the build key are in place and tested through
the binary. The vocabulary in `CONTEXT.md` is consistent and the ADRs record
every decision with its reason. Keep all of that.

## The gaps

### G1. The guard refuses ordinary reads

The guard refused four read-only commands in this session before any file was
touched. Two causes, both in `src/guard.rs`. A token that holds `*` is split
at the star, and for `*.rs` the prefix is empty, so every glob matches every
guarded name. The segment splitter ignores quoting. Issue #90 records both.

The cost is one lost turn per refusal. The design fix is larger than the bug
fix. The guard has two classes of case:

- A clear write: an edit tool aimed at a guarded path, a redirect onto one,
  a whole-tree restore, `init --add`. Deny.
- An ambiguous mention: a command outside the reader list that names a
  guarded path. Ask the person, do not deny.

Claude Code's PreToolUse hook accepts a JSON decision of `allow`, `deny` or
`ask`. The ambiguous class should return `ask`. See SPEC section 9.4.

### G2. Nothing tightens

The ceilings in `klin.json` are constants: `cc 8`, `lines 60`. `init` derives
a document ceiling from the document, and #92 derives the radius values from
history, but the complexity ceilings come from a `const` in `src/init.rs`.
Only #87 proposes a declining ceiling, and only for one gate.

A ratchet against the base prevents regression. It never lowers a ceiling.
So the tree today can hold its debt forever and stay green. The property that
makes tightening safe is not written down anywhere:

> A ceiling change never fails a site the base holds. A site over the
> ceiling in both trees, with no value risen, is held.

So a ceiling can fall without turning CI red. Only new code meets the lower
ceiling. The spec goes further than `init` writing a derived number once. A
ceiling the config does not pin is derived from the `before` tree at run time,
printed, and cached by commit. Existing sites cannot rise and new sites must
be under the ceiling, so the distribution never worsens and its 95th
percentile never rises. It falls whenever new code is better than the old
code. That is tightening with no person and no calendar. A person who wants a
number or a schedule pins one. This reverses ADR 0005's rule that a missing
key is an error. See SPEC sections 5.4 and 7.3.

### G2b. The config is a step a person has to take

Today a tree with no `klin.json` is exit 2. `init` writes one, and every gate
that cannot be inferred stays absent. The goal says users manage nothing. The
spec makes the file optional. Every value it could hold is derived and
printed, and the file exists to override. `init` becomes a way to pin what the
run would derive, so a person can review it. See SPEC section 5.

### G3. A local-only repository measures nothing after a commit

For a person with no remote and no CI, on the default branch, the merge-base
with `main` is HEAD. A dirty tree compares against HEAD, which is right. The
moment the agent commits, the tree is clean, the base is HEAD, and ADR 0013
lets the run pass with no measurement. The agent that commits each turn is
gated on nothing.

The merge-base is the wrong window for a turn, not only in this case. On the
default branch with a remote it measures unpushed commits and then nothing.
On a long branch it re-judges every file the branch touched at every stop, so
the hook slows as the branch grows. ADR 0013 exists only to patch the first
of these.

The turn stamp from ADR 0014 is the right window for the hook, always. The
hook compares the working tree against the tree as it stood when the window
opened. A commit inside the turn moves nothing. To stop debt leaking across
turns, the stamp moves forward only after a green stop or an acceptance.
Debt an agent leaves behind stays new until fixed or accepted. `klin gate` by
hand and CI keep the merge-base. This reverses ADR 0009 for the hook only.
See SPEC sections 6.1 and 6.2.

### G4. The linter integration has no design

The stated goal is to run beside eslint, tsc, clippy, ruff. The only ticket
that touches this is #47, `sarif`, which is framed as a cleat port and
blocked behind the registry rework. It also carries the unsolved problem that
no report exists at the base commit.

klin already materializes the base tree in a worktree. So an external tool
can run in both trees, and its results become a ratchet with site identity
like every other check. A `sarif` entry that carries a `run` command is
measured in both trees. An entry with only a `report` is differential. This
turns the sarif gate from a port into the seam the goal names. The cost is
running the tool twice, and a base worktree without installed dependencies.
See SPEC section 8.3.

### G5. The gate list is cleat's list, not an agent's failure list

Twelve open tickets are gate ports. They were chosen because cleat has them.
The question the goal asks is different: which deterministic check catches
what an agent does wrong, and names a remedy that adds or fixes code rather
than deleting work. SPEC section 8 applies four criteria and sorts the
catalogue into three tiers. One check that is missing entirely is `stubs`:
a placeholder body, an elided block, a `todo!()`, a `NotImplementedError`,
a `throw new Error("not implemented")`. That is the most agent-specific
failure there is, and no linter or cleat check names it.

### G6. No host adapter

`src/gate.rs` reads `stop_hook_active` straight from Claude Code's JSON. The
guard reads `tool_input.command` and `tool_input.file_path` the same way. The
README claims Cursor and Codex CLI support, and #67 and #68 are open. A `Host`
that maps each host's event to one internal shape is small and unblocks both.
See SPEC section 9.1.

### G7. Two windows, no model

`CONTEXT.md` defines Base and Turn, and no sentence relates them. ADR 0009
chooses the base, ADR 0014 adds the turn stamp for radius only. The spec
names one concept, the Window, with three kinds: branch, push and turn. Every
run compares two trees under one window, and every command says which.

### G8. Conformance without CI is not stated

The README says only CI is authoritative. The goal says CI is optional. Both
are true and the spec has to say what holds in each case. Without CI klin is
a feedback loop: every failure reaches the agent once per turn, and nothing
stops a person or an agent from overriding it. With CI, a protected branch
and CODEOWNERS, klin is a control. SPEC section 15 names the two levels.

### G9. Decisions worth reopening

The spec's section 0 lists seven ADR decisions it reverses. Three carry the
most weight and deserve a decision before any ticket below starts:

- ADR 0005, derive at run time instead of writing at `init`. Removes the
  setup step and gives self-tightening ceilings.
- ADR 0009 for the hook, the turn window instead of the merge-base. Removes
  ADR 0013 from the hook path and makes the hook's cost proportional to the
  turn.
- ADR 0007, one key vocabulary instead of cleat's keys. `sources` in one
  section and `roots` in the next is a cost with no remaining benefit, since
  ADR 0009 already emptied the differential test.

### G10. Smaller items

- No configuration reference exists. #18 has been open since the start.
- No machine-readable schema for `--json` output is written down.
- No performance budget is stated for the hook. Every hook run parses two
  trees. SPEC section 13 states one.
- `.gitignore` still names `.klin-build-blocked`. ADR 0015 moves state into
  `.klin/`. The spec moves it again, into the git directory or under
  `KLIN_STATE_DIR`, where no ignore line is needed. #89 should take that
  target instead, and still land before #91 and #92.
- ADR 0014, ADR 0015 and the change to `CONTEXT.md` are not committed.
- The accepted list rots by design: `--strict` fails on an entry that
  matches nothing, and a person deletes the line. That is the right shape.
  Say so in the reference and move on.

## Ticket triage

Open tickets, sorted by what to do with them.

**Do first, in this order.**

| Ticket | Why now |
|---|---|
| #90 guard false refusals | Every refusal costs a turn. Add the `ask` decision from G1. |
| #89 state directory | Retarget from `.klin/` to the git directory with a `KLIN_STATE_DIR` override, per SPEC 7.4. Blocks #91 and #92. Small. |
| #62 release pipeline | Every install route in SPEC 19 depends on it. |
| #66 the plugin | With the optional config and a `bin/klin` wrapper, it becomes the whole install for Claude Code. |
| #91 radius report | Gives the turn stamp that G3 needs. Open it on session start too. |
| #92 radius values from history | The first derived value outside documents. |
| new: the hook reads the turn window and writes the verdict | G3. The stamp moves only after a green stop. |
| new: survey at run time, cached by commit | G2b. Config optional, every derived value printed. |
| new: derived complexity ceilings | G2. Percentile of the `before` tree, with a floor and a minimum sample. |
| new: pinned dated ceilings on every gate | G2. Absorbs #87. |
| new: one key vocabulary, retire the differential test | G9. |
| new: host adapter | G6. Unblocks #67 and #68. |

**Build next, the tier 1 checks.**

| Ticket | Note |
|---|---|
| #47 sarif | Reframe per G4: `run` in both trees. Move ahead of the registry work if the registry is not needed for one fixed section. |
| #58 hallucinated-deps | Offline, lockfile only. Small. |
| #45 + #69 inventory over tests | The deleted test is the cheapest route to green. |
| #48 duplication | Copy instead of reuse is an agent habit. Built-in finder only. |
| new: stubs | G5. |
| #40 doc-citations | Already shipped. Close it. |

**Keep, later.**

#42 conventions, #46 public-api, #49 reference extractor, #52 dead-symbols,
#51 reachability, #53 coverage reader, #54 changed-coverage, #55 crap, #70
postflight, #65 sarif output, #60 hotspots, #62 release, #64 npm, #66 plugin,
#63 version pin, #18 config reference, #41 and #86 test-hygiene.

**Close or defer with a reason.**

| Ticket | Reason |
|---|---|
| #43 guard-suites | Reads a project-specific preflight script. Not language-agnostic. |
| #44 manifests | Judges a generated Xcode project. One stack. |
| #50 layering | cleat's own STRATEGY.md says the compiler's module graph is the check. Defer until the extractor exists and a user asks. |
| #57 db-migration-safety | Deterministic only for raw SQL. ORM migrations need a per-ORM reader. Defer. |
| #59 asset-path-verification | The ticket itself expects false positives. Defer. |
| #88 klin's own hygiene ceilings | Person-only. Fine as is, but not a design item. |
