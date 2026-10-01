# Decisions on the `stubs` comment marker site, 2026-10-01

Research notes for #415. On 2026-10-01 the user kept all three
recommendations below, which is what ADR 0064 and spec 8.2.1 already state.

#415 keys every comment marker in one file as one `stubs` site and ratchets
its count. This note answers three questions the change left open:

1. Should a line that holds a marker and a code stub stay two sites?
2. Which line should a failure name first, when an edited marker and a new
   one are both unpaired?
3. Should `new_lines` stay a comma-joined string?

It reads branch `issue-415-stubs-marker-count` at `ef451b2c`, against `main`
at `2c2c1399`. Every "verified" claim below comes from the source at those
commits or from a run of a debug binary built from each, in throwaway
repositories with a base commit on `main` and the change in the working tree
of a branch. "Inferred" means reasoned from the source and not run.

## How the change works

- A row named in `Kind::keyed_by_row` records its match at the site
  `(file, row name)` and keeps each match's line and trimmed text as a mark
  (`src/markers.rs:148-163`). `stubs` names one such row, `comment marker`
  (`src/stubs.rs:23`, `src/stubs.rs:93`). Every other row, and every body
  shape, records at `(file, trimmed line text)` (`src/markers.rs:605-609`,
  `src/markers.rs:699-716`).
- `named()` pairs each current mark with one base mark of equal text, a
  multiset match, and names every current mark left over. The site's `line`
  becomes the first left over line, and `new_lines` lists them all as one
  string (`src/markers.rs:349-379`).
- The ratchet then judges the site on `count` alone (`src/stubs.rs:94-95`,
  `src/ratchet.rs:458-498`). `named()` changes what a failure prints, and
  never whether it fails.

## Decision 1: a marker and a code stub on one line

### What happens today

The three options differ only on a line that holds both a marker and a code
stub. Results of the two binaries on the same five edits, verified:

| Edit, base to after | `main` 2c2c1399 | branch ef451b2c |
|---|---|---|
| Typo fix in the comment of `todo!() // TODO wirte it` | FAIL, 1 new site, `not implemented x2` | FAIL, 1 new site, `not implemented` |
| Typo fix in the comment of `def save(key):  # TODO wirte it` over a `pass` body | FAIL, 1 new site, `comment marker x2` | FAIL, 1 new site, `pass body` |
| `todo!() // TODO handle errors` becomes `g() // TODO handle errors` | FAIL, 1 new site, `comment marker` | OK, held at the base |
| `todo!()` becomes `todo!() // TODO handle errors` | FAIL, 1 new site, `not implemented x2` | FAIL, 2 new sites, `comment marker` and `not implemented` |
| `g() // TODO handle errors` becomes `todo!() // TODO handle errors` | FAIL, 1 new site, `not implemented x2` | FAIL, 1 new site, `not implemented` |

Two facts follow from the table.

- A typo fix in a marker that shares a line with a code stub still fails on
  the branch. The code stub keeps the line text key, and that text includes
  the comment (`src/syntax/convention.rs:212-226` takes the declaration line
  for a body shape, `src/markers.rs:657-658` takes the trimmed line for a
  line pattern). So R035's false alarm survives on mixed lines under every
  option below, because none of them changes the code stub's key.
- On `main` the label shows one kind and hides the other. The Rust line read
  `not implemented x2` and the Python line read `comment marker x2`, which is
  the known limit of spec 8.2.1 (`docs/SPEC.md:1460-1461`). The branch names
  both kinds.

### The replay evidence

The #343 replay holds four `stubs` rows, and none has a marker and a code
stub on one line, verified from the worksheets:

- R035 is a lone `/// FIXME` doc comment whose typo was fixed
  (`benchmark/evidence/false-alarms-2026-09-29/worksheet.md:4109-4134`).
- R053 is a set of new `TODO` comments in new test files (`worksheet.md:6425`
  onward, label in `labels.json:210-213`).
- J063 and J075 are `todo!()` inside fixture strings
  (`journal-worksheet.md:2271-2290`, `journal-worksheet.md:2611-2630`).

So the replay says nothing about mixed lines. Any choice here rests on the
contract and on the probes above.

### Option A: two sites (current)

For:

- It is the only option that holds a marker the base already had when an
  agent implements the code stub beside it (row 3, verified). That edit is
  the work `stubs` asks for, and a failure there would punish it.
- Spec 8.2.1 states it (`docs/SPEC.md:1473-1475`), ADR 0064 lists it as a
  known cost (`docs/adr/0064-a-stubs-comment-marker-is-keyed-by-file-and-kind.md:45-46`),
  and `a_marker_and_a_body_shape_on_one_declaration_line_are_two_sites` pins
  it (`tests/stubs.rs:334`).
- A failure names both kinds, so the "label hides the second kind" limit no
  longer applies to such a line.
- One rule decides a site: a marker match always lands on the file's marker
  site. The count of that site depends on markers alone.

Against:

- #415 said "Nothing else changes". One mixed line now prints as two
  failures where `main` printed one (row 4, verified).
- ADR 0008 lists "one line carrying two escape kinds would become two
  entries" as a cost of putting the kind in the key
  (`docs/adr/0008-the-name-is-klin-the-architecture-is-not-the-klin-spec.md:33-34`).
  ADR 0064 answers that for markers through spec 4.4's allowance for a check
  that has no declaration line (`docs/SPEC.md:298-300`), and that ADR 0008
  objection was written about `escapes`, which keeps its key.

### Option B: fold the marker into the code stub's site

The marker on a code-stub line would count on the line text site, as on
`main`. Inferred effects, from the table and the code:

- Row 3 would fail again. At the base the marker sits on the code stub's
  site, and after the stub goes it lands on the marker site, where no base
  mark has its text. So implementing the stub reads as a new marker, which is
  the R035 class of false alarm (a marker the base held reads as new).
- Rows 1 and 2 fail as today, so folding fixes nothing there.
- The mixed-line label hides one kind again (`docs/SPEC.md:1460-1461`).
- A marker's site would depend on whether a code stub shares its line. The
  walk records line patterns in `tally()` before it reads body shapes
  (`src/markers.rs:583-588`), so folding a marker into a `pass body` site
  needs a second pass after the shapes. That is more code for a worse result.

### Option C: drop the marker on a line that holds a code stub

Inferred effects:

- Row 3 fails as in option B: the marker was counted nowhere at the base and
  counts on the marker site after.
- Row 4 fails once, through the code stub's changed line text, and the
  failure never mentions the marker.
- The marker site's count would no longer equal the number of markers in the
  file, which spec 8.2.1 says it is (`docs/SPEC.md:1464-1466`).

### Recommendation

Keep two sites. Verified: it is the only option of the three that passes
row 3, and none of the three changes rows 1 and 2. Inferred: folding or
dropping brings back an R035-class false alarm on a mixed line and adds code.

The mixed-line typo fix (rows 1 and 2) is a separate question. Fixing it
would mean keying a code stub by its line text without the trailing comment.
#415 ruled out changing the code-stub key ("an edit to such a line is an edit
to code"), and no replay row shows the case, so this note does not recommend
it.

## Decision 2: which line a failure names first

### What happens today

Verified with base `// TODO: one`, `// FIXME: hadnle it`, `fn f() {}` and
after `// TODO: one`, `// FIXME: handle it`, `fn f() {}`, `// TODO: new`:

```text
FAIL: 1 stub site(s) got worse
  src/lib.rs:2  comment marker x3, new on lines 2, 4
```

The JSON finding carries `"line": 2`, `"text": "comment marker"` and
`"new_lines": "2, 4"`, and its `matched` record carries the base count of 2.
Line 2 is the typo fix and line 4 is the new marker. The text report shows
the count rose by one, `x3` against `was comment marker x2`, so a reader can
tell that one of the two named lines is new. The `line` field alone points at
the edited one.

`named()` also runs on sites that held. A held site prints nothing and is
left out of the JSON `findings` list (`docs/SPEC.md:3750-3752`,
`src/ratchet.rs:824-848`), so the work is invisible there.

### Constraints

- AGENTS.md and ADR 0001 require measurements to be deterministic and
  self-consistent across the two trees one binary measures
  (`AGENTS.md:16-17`, `docs/adr/0001-complexity-over-tree-sitter.md:11-14`).
  Which line a failure names is presentation, and the verdict comes from the
  count. A naming rule still has to be deterministic, so that one change
  names the same line on every run.
- Spec 4.4 says the cross-file move pass "is an equality match on an
  unchanged body and not a similarity match" (`docs/SPEC.md:338-339`). That
  sentence governs which entry a finding matches. Naming lines is outside its
  scope. It does show that klin has kept similarity out of its matching. No
  similarity or edit distance code exists in `src/`, verified by search.

### Option A: keep the full set, first line first (current)

For: every line whose text the base file lacks is named, so the marker that
raised the count is always in the list (it follows from the multiset match in
`src/markers.rs:357-369`). It needs no heuristic.

Against: the `line` field, and so the location a harness jumps to, may be an
edited marker (verified above). ADR 0064 lists this as a known cost
(`docs/adr/0064-a-stubs-comment-marker-is-keyed-by-file-and-kind.md:41-44`).

### Option B: pair the leftovers by edit distance, name the rest

After the exact match, pair each leftover base line with its closest leftover
current line, and name only the current lines that stay unpaired. That names
exactly as many lines as the count rose.

For: in the probe above it would name line 4 only. Levenshtein distance from
`// FIXME: handle it` to `// FIXME: hadnle it` is 2, and from `// TODO: new`
it is 12 (computed with a scratch script).

Against: when the edit is larger than a typo, it can name the wrong line and
drop the new one from the list. With `// TODO old words` reworded to
`// TODO completely different` and `// TODO new` added, the new line is
closer to the old text (distance 8) than the reworded line is (distance 16),
so this option would name the reworded line and not name the new one
(computed with the same script). Option A names both. It would also be the
first similarity heuristic in klin.

### Option C: use the diff klin already reads

`src/hunks.rs` runs `git diff -U0 --diff-algorithm=histogram` and keeps only
the added side of each hunk (`src/hunks.rs:29-55`, `src/hunks.rs:121-133`).
A typo fix and a new line are both added lines, so the data as kept cannot
tell them apart. Only the `sarif` gate reads it (`src/sarif.rs:220`). Using
it would mean keeping the deleted side too and adding a git diff to the
`stubs` gate, which then depends on git's diff choices for its report. Not
recommended.

A diff-like alignment inside klin is possible without git: count, for each
leftover line, how many exactly paired markers precede it, and treat a group
where the base has as many leftovers as the current file as edits. A scratch
model of that rule named only the new line when an unchanged marker sat
between the edit and the new marker, and named both lines when the new marker
sat next to the edited one, which is the probe's case. It helps in a minority
of layouts and adds code.

### Option D: name lines only when the count rose

This changes nothing a person or a harness sees, because a held site is
neither printed nor recorded (see above). It saves a little work per held
site. It does not answer the question.

### Recommendation

Keep option A. Verified: it is the only option here that always names the
marker that raised the count, and the text report already shows how many of
the named lines are new. Inferred: option B trades that guarantee for a
better first line, and fails without a sign on a reworded marker. If the
first line matters to a harness, a cheaper change is to print the rise beside
the list, such as "1 new among lines 2, 4". That is a wording change to
`show()` (`src/markers.rs:268-283`) and keeps the full list.

### What other tools do

None of the tools read below pairs an edited finding with its old self by
similarity. Those that tolerate an in-place edit do it by position or by a key
that leaves the text out.

- Semgrep CLI, `--baseline-commit`: a match is removed as old when its
  `ci_unique_key` is in the baseline (`cli/src/semgrep/run_scan.py:405-450`).
  The key is the rule id, the path, the matched code text with whitespace and
  `nosem` comments removed, and an index among equal keys
  (`cli/src/semgrep/rule_match.py:207-250`, index at `rule_match.py:610-640`,
  commit `31729a1`). Inferred: a typo fix inside a `TODO` that a rule matches
  changes the text and reads as new, as on `main` here.
- Semgrep AppSec Platform, `match_based_id`: a hash of the rule pattern with
  metavariable values substituted, the path and the rule id, plus a per-file
  index (`src/osemgrep/reporting/Semgrep_hashing_functions.ml:45-56`,
  `:193-224`; `Cli_json_output.ml:379-416`; Semgrep's "Remove duplicate
  findings" page). Inferred: for a rule with no metavariables, every match in
  a file hashes the same and differs only by index, which is close to the
  file plus kind plus count key of ADR 0064.
- SonarQube 10.3: matching uses the rule and a whitespace-free hash of the
  issue's first line, and falls back to "the same rule, with the same message
  and with the same line number (but not necessarily with the same line
  hash)" ("How new issues are identified" on the 10.3 Issues page). Inferred:
  a typo fix that stays on its line matches when the rule's message does not
  quote the comment. klin keeps the line as a tie-breaker only (ADR 0008), so
  this fallback has no counterpart here.
- Betterer with ESLint `no-warning-comments`: ESLint reports the whole comment
  node (`lib/rules/no-warning-comments.js:188-195`). Betterer hashes the text
  of the reported span (`packages/betterer/src/test/file-test/file.ts:53-57`)
  and keeps an issue as existing only when its hash matches, at the same
  position or as a move (`differ.ts:80-135`). Any new issue makes the result
  worse (`constraint.ts:16-20`). Inferred: a typo fix in a `TODO` reads as one
  new issue and one fixed one, so the test gets worse.

## Decision 3: `new_lines` as a string

### Where `values` goes

Verified from the source and a `klin gate --gate stubs --json` run:

- Spec 4.5 types `values` as "metric name to number or string"
  (`docs/SPEC.md:361`). An array would need a spec change.
- The JSON report copies `values` into each failing finding as it is
  (`src/ratchet.rs:824-848`). The probe printed `"new_lines": "2, 4"`.
- The journal `stop` line is the 11.2 object (`docs/SPEC.md:3812-3813`), so
  it holds the same string.
- `klin stats` keeps `values` as opaque JSON (`src/stats.rs:163`,
  `src/stats.rs:1187`) and prints it only for a site with no text
  (`src/stats.rs:1014-1031`). A marker site has the text `comment marker`, so
  `stats` never prints it. `stats` follows a site by `id`
  (`src/stats.rs:110-120`), and the marker site's `id` hashes the gate, the
  file and `comment marker` (`src/ratchet.rs:874-885`), so it stays the same
  across marker edits.
- Accepted-entry validation checks only the ratcheted values
  (`src/ratchet.rs:172-196`). An entry that copies `new_lines` from a JSON
  finding is accepted, and `show()` then prints it back: the probe printed
  `was ? x2, new on lines 2, 4` and the same text in the NOTE for an entry
  that matched nothing. The `?` is older behaviour, an entry without the
  `stub` value. A wrong-looking line, and no wrong verdict.
- The only reader of the string is `show()`, which checks for a comma to
  pick "line" or "lines" (`src/markers.rs:275-278`).
- Two values already pack a list or a pair into a string: `layering` joins a
  cycle path with " → " (`src/layering.rs:761-763`), and `public-api` writes
  `origin` as `file:line` (`src/public_api.rs:241`).

No consumer inside klin parses `new_lines`.

### SARIF

klin writes no SARIF. Spec 11.3 records `--sarif PATH` as a design that did
not ship (`docs/SPEC.md:3789-3796`). `src/sarif.rs` reads other tools' SARIF
and takes only the first location of a result (`src/sarif.rs:379-395`).

If 11.3 ships, SARIF 2.1.0 (OASIS, errata 01) shapes the mapping:

- 3.27.12 `locations`: the array "SHALL NOT contain more than one element
  unless the condition indicated by the result, if any, can only be corrected
  by making a change at every location", and SHALL NOT hold occurrences that
  can be corrected independently. Deleting any one named marker brings the
  count back, so the named lines do not qualify for `locations`.
- 3.27.22 `relatedLocations`: "location objects ... each of which represents
  a location relevant to understanding the result". The named lines fit here,
  with the site's `line` as the single `locations` entry.
- 3.8.1 property bags: a `properties` value "MAY be of any JSON type,
  including ... arrays", so an array can travel there too.
- 3.27.17 `partialFingerprints` and Appendix B: a producer may supply a
  string that "contributes to the stable, unique identity" of a result, and a
  result management system "SHOULD NOT include an absolute line number" in a
  fingerprint. 3.27.16 says a direct producer "SHOULD NOT populate"
  `fingerprints`. klin's `id` is that kind of value: it leaves the line out and
  stays put across marker edits.
- GitHub code scanning reads `partialFingerprints` to spot the same result
  across runs, and "only uses the `primaryLocationLineHash`". If the field is
  missing, `upload-sarif` computes it from the source file. It reads only the
  first `locations` entry, and links `relatedLocations` only when the message
  embeds them ("SARIF support for code scanning", "Fingerprint generation"
  and "result object"). `primaryLocationLineHash` is a rolling hash of the
  first 100 non-space, non-tab characters from the start of the line, with an
  occurrence count (`codeql-action/src/fingerprints.ts:32-44`, `:67-76`,
  commit `61817fa`). Inferred: GitHub would key a klin marker result by the
  text at the site's `line`, which moves to whichever line `named()` puts
  first, so an alert could close and reopen as markers change. klin's `id`
  would have no effect there, because GitHub ignores every partial
  fingerprint except `primaryLocationLineHash`.

A SARIF writer would have to split the string to build `relatedLocations`.
That writer does not exist.

### Recommendation

Keep the string. Verified: no consumer in klin parses it, spec 4.5 allows it,
and two other checks already pack structure into a string value. Inferred: if
11.3 ships, the named lines belong in `relatedLocations` (3.27.22), and the
writer can read them from `Finding` directly or split the string. Decide the
shape with that writer, when it exists. A change to an array now would amend
4.5 and `show()` with no reader to benefit.

## Sources

Repository, at `ef451b2c` unless noted:

- `src/markers.rs`, `src/stubs.rs`, `src/ratchet.rs`, `src/stats.rs`,
  `src/sarif.rs`, `src/hunks.rs`, `src/syntax/convention.rs`,
  `src/layering.rs`, `src/public_api.rs`
- `docs/SPEC.md` sections 4.4, 4.5, 8.2, 8.2.1, 11.2, 11.3, 11.4
- ADR 0001, 0008, 0009, 0064 in `docs/adr/`
- `tests/stubs.rs`
- `docs/false-alarms-2026-09-29.md` and
  `benchmark/evidence/false-alarms-2026-09-29/` (`worksheet.md`,
  `journal-worksheet.md`, `labels.json`)
- Issue #415, `gh issue view 415 --repo brajevicm/klin`

External, primary sources only:

- OASIS, SARIF 2.1.0 Errata 01, sections 3.8.1, 3.27.12, 3.27.16, 3.27.17,
  3.27.22 and Appendix B:
  https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/sarif-v2.1.0-errata01-os-complete.html
- GitHub Docs, "SARIF support for code scanning":
  https://docs.github.com/en/code-security/code-scanning/integrating-with-code-scanning/sarif-support-for-code-scanning
- `github/codeql-action` at `61817fadad823920a8924645dd5f67985d6a1d94`,
  `src/fingerprints.ts`:
  https://github.com/github/codeql-action/blob/61817fadad823920a8924645dd5f67985d6a1d94/src/fingerprints.ts
- Semgrep Docs, "Remove duplicate findings":
  https://docs.semgrep.dev/semgrep-code/remove-duplicates
- `semgrep/semgrep` at `31729a1719c6e76ac8d41ec8fb9ab191621ee10f`:
  `cli/src/semgrep/rule_match.py`, `cli/src/semgrep/run_scan.py`,
  `src/osemgrep/reporting/Semgrep_hashing_functions.ml`,
  `src/osemgrep/reporting/Cli_json_output.ml`, under
  https://github.com/semgrep/semgrep/tree/31729a1719c6e76ac8d41ec8fb9ab191621ee10f
- SonarQube Server 10.3 docs, "Issues", section "How new issues are
  identified": https://docs.sonarsource.com/sonarqube-server/10.3/user-guide/issues
  The current pages fetched on 2026-10-01
  (`/sonarqube-server/latest/user-guide/issues/introduction`) do not hold
  this passage. This note did not look for where it moved.
- `phenomnomnominal/betterer` at `69a83c14975b6ef2e4e42736e686ffba0f28895f`,
  `packages/betterer/src/test/file-test/differ.ts`, `file.ts`,
  `constraint.ts`, `packages/eslint/src/eslint.ts`:
  https://github.com/phenomnomnominal/betterer/tree/69a83c14975b6ef2e4e42736e686ffba0f28895f
- `eslint/eslint` at `67eb586e2eda290813b329d4f40e605ad696bc4f`,
  `lib/rules/no-warning-comments.js`:
  https://github.com/eslint/eslint/blob/67eb586e2eda290813b329d4f40e605ad696bc4f/lib/rules/no-warning-comments.js
