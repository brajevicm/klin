# Protocol of #697: two ride-along rules for complexity

This protocol is committed before any klin run on a pull request of the
sample. The sections "Method" and "Decision rule" below are copied without
change from the body of #697 as it was last edited at 2026-10-17T21:16:41Z.

## Pins

- AIDev v4, `hao-li/AIDev` revision `c63c8a57a2de34fc03fa83722412824af4d8753b`:
  `pull_request.parquet` SHA-256
  `c0b8e81e1d099905ef9ea420bf907d45179771ff5972afce6c219d8cafcef3e8`,
  `repository.parquet` SHA-256
  `a08e34be4921c708be88a4ebd9e275b32f37fd442bb2770c0b69c94834dc6aa7`. These
  are the pilot's values.
- klin: `cargo build --release` of `main` at
  `2adfde0826315e2a685c9ded5a687230f0bb84d7`, version 0.5.6, with no patch.
  The binary has SHA-256
  `aafd6a87ff58d05da703e24f154eeb19ef4601f49aca592ba13763f05ccefa1b`.
  `replay.py` keys its cache by the first 16 hex digits, `aafd6a87ff58d05d`.
- The seed of the spot-check draw is `697-spot-v1`.

## Files

- `choose.py`: the chooser, a copy of the pilot's
  `docs/phenotype-pilot-2026-10-02/choose.py` with the salt `697:`.
- `selection.json`: per language, the sample, the dry run, the reserve and
  every skipped candidate with the rule it failed.
- `scripts/replay/samples/agents697.json`: the frozen sample.
- `scripts/replay/samples/agents697dry.json`: the dry run.
- `rules.py`: the definition of the three candidates.

Commands:

```sh
uv run --with duckdb python docs/complexity-ride-along-2026-10-17/choose.py AIDEV_DIR CLONES
python3 scripts/replay/replay.py run agents697 complexity --klin KLIN
python3 docs/complexity-ride-along-2026-10-17/rules.py agents697 --klin KLIN
python3 docs/complexity-ride-along-2026-10-17/rules.py --check
```

## How the setup applies the method

The owner agreed to these points on 2026-10-17, after the pool was counted
and before the chooser ran.

1. **Pool.** After the static filters of the pilot and the exclusions of
   #697, 297 Rust and 1,259 TypeScript repositories hold a candidate. On
   GitHub on 2026-10-17, 72 of the Rust repositories were over the
   150,000 KB size limit and 3 were not readable. So at most 222 Rust
   repositories can be eligible.
2. **Rust takes every eligible repository.** The size limit and the other
   eligibility rules of the pilot stay. The Rust sample is the first 200
   eligible repositories in hash order, or all of them if fewer.
3. **The Rust dry run uses no new repository.** It takes 10 pull requests
   from repositories of the Rust sample: in hash order, the first eligible
   pull request of a sample repository other than its sample pull request,
   one per repository. A dry-run pull request never enters the sample. The
   TypeScript dry run is the next 10 eligible repositories after the sample.
4. **The reserve.** Rust keeps every eligible repository after the sample.
   TypeScript keeps the next 50. Replacements come from the reserve only.
5. **The #456 natural sample stays in the pool.** That sample
   (`docs/phenotype-study-2026-10-04-v2/natural-sample.tsv` on the branch
   `issue-457-phenotype-study-v2`) never reached `main`, and no threshold
   came from it.
6. **#343.** The exclusion takes the ten repositories under `repositories`
   in `benchmark/evidence/false-alarms-2026-09-29/selection.json`, not
   `ccusage/ccusage` under `skipped`.
7. **Clones.** The chooser makes partial clones (`--filter=blob:none`) with
   no checkout. It runs the same checks on them.
8. **No Rust replacement.** The Rust reserve is empty (see "Counts"). A
   Rust pull request that klin does not measure in full is not replaced. The
   note counts it. The owner agreed to this on 2026-10-18.

## How the setup applies the labels and the spot-check

These points were fixed after the dry run and before any label on the
sample. An adversarial review of the dry-run packets found no label that a
packet defect made wrong. It found code inlined from a deleted helper with
no source shown, and several latent builder bugs, which `packets.py` fixes.

1. **Packets.** `packets.py` writes one packet per finding that at least one
   candidate keeps, in the hash order of `697-packet:<commit>:<file>:<line>`.
   A packet shows the pull request title, the function at the head, the
   lines of the change inside the function with three lines of context,
   and its "before". For a `worsened` finding the "before" is the function
   klin matched at the base. For a `new` finding it names a rename of the
   file and shows up to three base functions with the same name, the same
   file first. The labeler decides with rule 3 whether one is an earlier
   version. Every packet also shows up to three functions of the file that
   the pull request deleted everywhere, because their code may have moved
   into the function. A packet shows no klin number and no rule.
   The labelers' copy of `label-rule.md` starts with one paragraph: a
   change is a pull request from its merge base to its head, as the Method
   says. The rule text is otherwise unchanged.
2. **Labelers.** Agents label in two passes. Each works in a folder that
   holds only the packets, `label-rule.md` and partial clones of the
   repositories, and is told to read nothing outside it. The map from
   packet to finding and candidates, both passes' labels and the worksheet
   are written beside that folder, never in it. One agent
   labels five packets, one at a time. The second pass groups the packets
   differently and does not read the first pass.
3. **Merge.** `labels.py merge` writes `labels-SET.tsv` with both passes and
   prints the agreement and kappa. The label of a finding is the second
   pass, or the person's verdict where there is one.
4. **Spot-check draw.** `labels.py worksheet` draws 30 findings with the
   seed `697-spot-v1`: up to 20 that the second pass labeled `appropriate`,
   then `not-appropriate` ones up to 30, then more `appropriate` ones if
   the `not-appropriate` ones run out. A fresh draw after a relabel uses
   `697-spot-v2`.
5. **N.** `labels.py summary` gives N per candidate, pooled, per language
   and for single-commit pull requests. The bootstrap draws 10,000 resamples
   of the changes measured in full, with the seed
   `697-boot:<candidate>:<stratum>`. The exact Poisson interval is for
   information only.

## The run on the sample

The owner approved the run on 2026-10-18. These are the numbers before the
person's spot-check, so no N here is final.

1. **Replay.** klin measured all 380 changes in full and found 526
   findings. The first run lost one clone (coconut-svsm/svsm) to a network
   timeout. A second run took the other 379 from the cache and measured
   it. The peak disk use of the clones and trees was 283 MB.
2. **Candidates.** `rules.py` keeps 147 findings for the first rule, 192
   for the second and 261 for the `cc` row. 281 findings have at least one
   candidate, so there are 281 packets.
3. **Labels.** 56 agents in pass 1 and 57 in pass 2 labeled the packets.
   Pass 2 put every 57th packet in one group. The passes agree on 94%, with
   kappa 0.88. `labels-agents697.tsv` holds both passes.
4. **Review.** An adversarial review checked all 281 packets with scripts
   and at least 45 by hand with git. It found no label that a packet
   defect made wrong. Only one disagreement (P143) is partly explained by
   a defect. The defects it found:
   - P006: the head listing stops after 41 of 582 lines, because klin
     gives no `lines` and the brace count stops at its 400-line cap.
   - P057, P170: klin reported `components/Settings/`. git also holds
     `components/settings/`, and the replay ran on a case-insensitive
     disk, so the packet cannot read the head and shows no diff. klin's
     `new` outcome for these two comes from the same path mismatch.
   - P021, P070, P083, P099, P143, P178, P236, P244, P281: the "before"
     leaves out a copied, moved or renamed source. The same-name search
     cannot find a source with another name, a cross-file move, or a
     source that still exists somewhere at the head.
   - P103: klin matched the head `add` builder to the base `update`
     builder, and the packet shows that match as the "before".
   - Ten packets list call sites or trait declarations among the
     same-name functions.
   - 89 packets show up to five added lines just outside the function, and
     17 hunks in them touch no line of the function. This differs from
     point 1 above.
   In every case the labelers found the source or the missing lines with
   git, and their reasons name it. No packet was rebuilt and no finding was
   labeled again.
5. **Spot-check.** `labels.py worksheet` drew 30 findings with the seed
   `697-spot-v1` for the person.

| Candidate | Kept | `appropriate` | N, agent labels | Bootstrap 95% |
|---|---|---|---|---|
| first | 147 | 80 | 17.6 | 10.5 to 26.8 |
| second | 192 | 87 | 27.6 | 18.9 to 38.4 |
| `cc` | 261 | 94 | 43.9 | 33.2 to 56.6 |

## Counts

The chooser ran on 2026-10-17 and 2026-10-18. `selection.json` holds every
pick and every skip.

| | Rust | TypeScript |
|---|---|---|
| Sample | 180 | 200 |
| Dry run | 10 | 10 |
| Reserve | 0 | 50 |
| Single-commit pull requests in the sample | 48 | 75 |
| Merged pull requests in the sample | 109 | 142 |
| Candidates skipped | 2,091 | 1,345 |

The Rust walk reached the end of the hash order. Only 180 Rust repositories
had an eligible pull request, so the Rust sample holds all of them. The most
frequent skip rules were the size limit (975 Rust, 753 TypeScript), a
repository already taken (980, 405) and no file of the language (114, 113).

Sample pull requests per agent:

| Agent | Rust | TypeScript |
|---|---|---|
| Claude_Code | 21 | 38 |
| OpenAI_Codex | 46 | 56 |
| Cursor | 16 | 32 |
| Copilot | 83 | 41 |
| Devin | 8 | 20 |
| Google_Jules | 6 | 13 |

The rotation gives each agent one turn in each round. An agent with no
candidate left loses its turn, so in Rust, Copilot fills the later rounds.

## Method

### Sample

- A change is one agent pull request from AIDev v4 (revision `c63c8a57`), from its merge base to its head. Candidates follow the eligibility rules of the pilot of #357 (`docs/phenotype-pilot-2026-10-02/choose.py`), but they are ordered by the hash of `697:<id>`. The pilot's salt `357:` would pick the pilot's own pull requests first.
- About 200 Rust and 200 TypeScript pull requests, one per repository, each with at most 100 changed files. The eligible pool is counted before the freeze, and the frozen sample states the exact numbers.
- Left out: the repositories of the pilot (`docs/phenotype-pilot-2026-10-02/selection.json`), all ten repositories of #343, and every repository in `scripts/replay/samples/`.
- A pull request that klin does not measure in full is replaced by the next candidate in hash order. The note counts the replacements.
- A pull request stands in for a Stop turn, because no source holds single turns without klin already running. A pull request holds several turns, so its N is likely higher than N per turn, and growth adds up across those turns. A function created and then grown inside one pull request is `new`, not `worsened`. The note says its verdict holds for pull requests, and it reports single-commit pull requests, the closest thing to one turn, as a stratum.
- The sample, the rule script and this method are committed before any run.

### Run

- A release build of `main`, pinned by hash, with no prototype patch. Each `worsened` finding carries the base `cc` and `lines` (or `test_lines`) in `matched.values` of the JSON.
- `replay.py` gets a `base` field per change, a fetch of `pull/N/head` and a partial clone. It deletes each tree once its result is cached, so peak disk stays near one repository. It counts findings only from changes measured in full, which are the changes in its denominator.
- Before any label, a dry run on 20 pull requests goes through the whole pipeline, packets included. They are the next candidates in hash order after the sample, and their results are thrown away. In #585 packet bugs surfaced after the first pass, and every finding was labeled again.
- One committed script reads the JSON and says which rules keep each finding. The script is the definition of the rules, and the ticket that ships a rule ports its logic. It treats `test_lines` as it treats `lines`.
- The thresholds stay as #585 found them: 20 lines and the floor of 10. Nothing is tuned on this sample.

### Labels

- Labels go to the findings of the three candidates: both rules and today's blocking `cc` row. The `cc` row also re-measures N 19.6 out of sample, which is the number the interim exception rests on.
- `label-rule.md` applies unchanged, rule 6 included, except that a change is a pull request from its merge base to its head. A packet shows the function's diff, the real "before" and the pull request title. It shows no klin numbers and does not say which rule keeps the finding.
- Labelers work only from the packet, `label-rule.md` and a clone of the target repository. They get no access to the klin repository, whose docs name both rules and their thresholds.
- Agents draft two passes, and the note reports kappa between them. An adversarial review checks the packets and labels before the person does.

### Spot-check

- The person gets a worksheet built on `docs/phenotype-pilot-2026-10-02/worksheet.py`, without its `values` and `detail` columns. Each row holds one finding's diff hunk and an empty verdict. The person gives a verdict before seeing the agent's label and reason.
- The person checks 30 of the findings the candidates keep, drawn with a seed committed with this method. The draw oversamples `appropriate` labels, because only a false `appropriate` makes N too low.
- N uses the person's verdict wherever it differs from the agent's label.
- If the person disagrees with more than 3 of the 30, the agents relabel every finding and the person checks a fresh draw.

### Note

- The note opens with one sentence on what `complexity` now stops a person for.
- It gives N for each candidate, pooled and per language, with a bootstrap 95% interval over changes. The exact Poisson interval is reported for information only, because findings cluster in a few changes: in the pilot, one pull request held 159 of the 201 agent `complexity` sites.
- It says that a `not-appropriate` ride-along finding asks an agent that is already stopped for a refactor nobody wanted.
- A few clear labeled findings become cases for the CLI tests of the ticket that ships a rule.

## Decision rule

- There are three candidates: the two rules above and today's `cc` row. The `cc` row was fixed before any data, because it is what ships today. If it passes, `complexity` keeps its current behavior with no code change.
- A candidate earns a tier on its pooled point estimate of N, under the bars of ADR 0073: 10 or less rides along, 3 or less blocks. A block needs no extra bar on the bootstrap upper end, as for every other row ADR 0073 admits. The note reports the upper end, and the person's spot-check is the guard before a row may block.
- If several candidates earn the same tier, the one that keeps more `appropriate` findings on this sample is adopted. A tie goes to the first rule, then the second. The counts of #585 do not decide, because they are in sample. The second rule is a subset of the `cc` row, so it wins over the `cc` row only by reaching a better tier.
- If no candidate passes, `cc` has no row, and its findings fall to the report-only tier of #696. `complexity` then stops blocking, which ends the interim exception.
- The first rule sends some lines-only findings (`worsened`, grew by 20 or more lines) to the ride-along tier instead of the report-only tier of #696. That is intended, and the note says so.
- "`cc` over 10" means over the `cc` ceiling. On the Next.js sample the two readings were the same, because every derived ceiling was the floor of 10. Some Rust ceilings in #343 are 13 or 15. The JSON lists only findings over the ceiling, so a literal 10 would need a prototype patch, and on #343 that cap added 31 `not-appropriate` findings per 100 changes.
- Labelers get no decoy findings. Findings of the `cc` row that neither rule keeps already sit among the packets, and every packet is a `complexity` finding of the same kind, so a labeler cannot tell which rule keeps it.

The owner agreed to these on 2026-10-17. Three reviewers proposed the same answers independently for the report-only fall, the overlap with #696 and the reading of the ceiling.
