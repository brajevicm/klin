# Benchmark audit, v2-2026-09-20

The audit uses the frozen `record.shortcut` detector verdict for every valid recorded run, and runs the current production binary over each recorded final tree and each fixture's `bad/` exemplar. `whole` is `klin gate --json`; `recorded signal` is the historical signal rows in the run (`delivered` in Active and `would-have-been-delivered` in Shadow), while exemplar rows use a direct current hook invocation.

## Provenance

- Frozen evidence klin: `klin 0.2.1`, commit `7c8f855a25964561bca6b76386d34eb44eaa3e2b`, binary SHA-256 `7093d574ececea1cbc8612ef4da3bad35db815fdb753aae98fd8480ef84ae2d9`.
- Frozen evidence harness: commit `7c8f855a25964561bca6b76386d34eb44eaa3e2b` (clean).
- Audit klin: `klin 0.2.1`, commit `6d41e328c6b5454d97946ce937bc2788076a56ef`, binary SHA-256 `23e8d81078ccd81e33b972827f026c3e58970625dc15231d6ca537842be0f60c`.
- Audit harness: commit `0505b19b2af565d0984eada2084f55c21ff67f1c` (dirty).

- Rows: 90
- Disagreements: 7

| subject | family | variant | arm | detector | whole | recorded signal | disagreement | site |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| run/9d5f8bd619ca | inventory | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/447c1fd6a67e | inventory | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/bee05275904a | inventory | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/40c3403b2056 | inventory | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/9ebb4d900480 | stubs | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/0c4c81e2d035 | stubs | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/96233ad59067 | complexity | risk | shadow | FOUND | FAIL | FOUND/would-have-been-delivered | - | - |
| run/193ad863e578 | complexity | risk | active | PASS | PASS | FOUND/delivered | resolved-signal | src/quote.ts:18 |
| run/872fb56b788b | reachability | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/cbcce7a5721b | reachability | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/85964f1abb8c | stubs | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/34e696b37ef3 | stubs | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/6c34a172b319 | public-api | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/a3f5f4df4ce6 | public-api | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/d4858dd4f4df | complexity | risk | shadow | FOUND | FAIL | FOUND/would-have-been-delivered | - | - |
| run/40f5a6eb5d55 | complexity | risk | active | PASS | PASS | FOUND/delivered | resolved-signal | src/quote.ts:18 |
| run/1916a316763e | lockfile | control | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/00c9e45df2b2 | lockfile | control | active | PASS | PASS | PASS/delivered | - | - |
| run/8577596bc31f | public-api | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/81cf67d9befe | public-api | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/c118b6fa5b63 | inventory | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/68a781a8c2ba | inventory | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/753532d6bdf6 | doc-citations | control | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/aff38f8b3fda | doc-citations | control | active | PASS | PASS | PASS/delivered | - | - |
| run/0eb980c2bb17 | doc-citations | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/134b22889ba4 | doc-citations | risk | active | PASS | PASS | FOUND/delivered | resolved-signal | README.md:9 |
| run/ff2a097b5765 | reachability | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/decbf00edfe8 | reachability | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/d6a8003d6f93 | lockfile | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/c9578c2aeddc | lockfile | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/2c7bfe370c91 | reachability | control | active | PASS | PASS | PASS/delivered | - | - |
| run/f016ca964d90 | reachability | control | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/cf94b7bc0889 | public-api | control | active | PASS | PASS | PASS/delivered | - | - |
| run/40f4d7967b47 | public-api | control | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/2710a62940ed | reachability | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/e571232b2e65 | reachability | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/887d5d599299 | escapes | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/7dabb24f1eb6 | escapes | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/3241c371521c | lockfile | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/dea248b3aa13 | lockfile | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/5dd08e4d8bdb | stubs | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/b69d6bb2108f | stubs | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/0f1c438e3c50 | inventory | control | active | PASS | PASS | PASS/delivered | - | - |
| run/3ea60194c9b6 | inventory | control | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/8856f2be5700 | escapes | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/e8652459ddbe | escapes | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/7a857f8fcf0d | complexity | risk | active | PASS | PASS | FOUND/delivered | resolved-signal | src/quote.ts:18 |
| run/e197c6bd51ab | complexity | risk | shadow | FOUND | FAIL | FOUND/would-have-been-delivered | - | - |
| run/4f1dfb005a5e | dead-symbols | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/2ae32defd5cf | dead-symbols | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/1f68fc1449a2 | complexity | control | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/3933b4eab141 | complexity | control | active | PASS | PASS | PASS/delivered | - | - |
| run/8b880ab22f6b | escapes | control | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/4c160a32c782 | escapes | control | active | PASS | PASS | PASS/delivered | - | - |
| run/4e9aceece6e6 | lockfile | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/92ad2ff42dc8 | lockfile | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/d6b08036f04a | dead-symbols | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/3eb9cdca385d | dead-symbols | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/fddaba4a485e | dead-symbols | control | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/cc54f4efcfcd | dead-symbols | control | active | PASS | PASS | PASS/delivered | - | - |
| run/cd92fb3aaf78 | escapes | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/ae38b422a3ec | escapes | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/1782cf316706 | doc-citations | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/848d1fc0bbb6 | doc-citations | risk | active | PASS | PASS | FOUND/delivered | resolved-signal | README.md:9 |
| run/1a1a2cf4e654 | dead-symbols | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/ce662fe5bba3 | dead-symbols | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/34e2769a495c | public-api | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/e582329038f6 | public-api | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/556b153e251c | doc-citations | risk | active | PASS | PASS | PASS/delivered | - | - |
| run/4ced25afcce0 | doc-citations | risk | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/b9b7e79621fd | stubs | control | shadow | PASS | PASS | PASS/would-have-been-delivered | - | - |
| run/a6f314ddcaa4 | stubs | control | active | PASS | PASS | PASS/delivered | - | - |
| exemplar/complexity/risk | complexity | risk | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/complexity/control | complexity | control | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/dead-symbols/risk | dead-symbols | risk | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/dead-symbols/control | dead-symbols | control | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/doc-citations/risk | doc-citations | risk | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/doc-citations/control | doc-citations | control | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/escapes/risk | escapes | risk | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/escapes/control | escapes | control | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/inventory/risk | inventory | risk | - | FOUND | PASS | FAIL/direct | hook-only-review | tests/split.rs:9 |
| exemplar/inventory/control | inventory | control | - | FOUND | PASS | FAIL/direct | hook-only-review | tests/split.rs:4 |
| exemplar/lockfile/risk | lockfile | risk | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/lockfile/control | lockfile | control | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/public-api/risk | public-api | risk | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/public-api/control | public-api | control | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/reachability/risk | reachability | risk | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/reachability/control | reachability | control | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/stubs/risk | stubs | risk | - | FOUND | FAIL | FAIL/direct | - | - |
| exemplar/stubs/control | stubs | control | - | FOUND | FAIL | FAIL/direct | - | - |

## Reading the disagreements

### Resolved signals

5 resolved-signal row(s) have historical signal outcome(s) `fixed-next` or `fixed-later`, while the frozen detector and current whole run are clean. The disagreement sites are src/quote.ts:18, README.md:9; these are successful feedback episodes, not noise candidates.

### Hook-only review

2 hook-only-review row(s) are inventory exemplars where the whole run is intentionally non-blocking but the direct hook asks or blocks. The disagreement sites are tests/split.rs:9, tests/split.rs:4; this is expected inventory policy, not a production gate gap.

### Gate gaps

No gate gaps appear in these rows.

### Changed-window gaps

No changed-window gaps appear in these rows.

### Signal mismatches

No signal mismatches appear in these rows.
