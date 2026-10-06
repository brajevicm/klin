# Blind check-only calibration audit

`blind.jsonl.gz` retains every pair's pinned source-context snippets without T
or token length. `blind-0/1/2.json.gz` retain the reviewed family-grouped shards.
`packet-provenance.json` records their decompressed hashes. `mapping.json` joins
pair IDs to lengths, complete-content family IDs, language and source locations;
contexts live in the blind archive rather than being duplicated in the mapping.
`labels.json` contains all 1,923 source-review labels/rationales/citations.
`review-0/1/2.md` describe the three independent blind source-review assignments.
These are model annotations, not human labels or developer-intent evidence.

The rule was committed as `13655f91` before labels. The owner authorized this
check-only calibration after Stop failure. Every eligible production file in the
three pinned roots was surveyed, including generated files and docs build code;
there is no specified generated-code exclusion. Reviewers did not read mapping
or threshold/length fields before labeling. `mixed` retains #480's definition:
some family members are a copy and at least one is independent. Family judgments
can differ by member pair; the all-copy rule rejects any family containing a
non-copy pair. Counts are correlated, not independent precision trials.

```sh
# Generate fresh unlabeled packets from prepared pinned roots (do not overwrite
# these reviewed artifacts until their provenance has been preserved).
python3 docs/duplication-speed-2026-10-05/calibrate.py collect /tmp/klin490-corpus /tmp/new-calibration
# Reproduce selection from the reviewed labels and mapping.
python3 docs/duplication-speed-2026-10-05/calibrate.py summarize docs/duplication-speed-2026-10-05/calibration
```

No T qualifies. `summary.json` records non-copy/total at T=60/80/100/150 per language
and contributing repositories; sensitivity around a selected T is inapplicable.
All generated labels removed still leave non-copy counterexamples at every T.
No original #480 Stop-first acceptance criterion or product threshold is claimed.
