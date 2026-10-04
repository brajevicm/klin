# Descriptive natural prevalence

The natural-agent and matched-human arms are shown separately. Human controls are changes not attributed to an agent by the preregistered metadata exclusions; this is not proof of human-only authorship.
Rate differences are descriptive and do not establish an AI-specific effect.

## Materialized sample

| Language | Agent changes | Matched human changes | Unmatched agent changes |
| --- | ---: | ---: | ---: |
| Rust | 40 | 27 | 13 |
| TypeScript | 40 | 31 | 9 |
| Python | 40 | 34 | 6 |

Frozen detector replay only. These are descriptive rates, not valid-regression labels or product dispositions.
Unsupported, partial, unavailable, invalid-syntax, local-resolution-incomplete and tool-error rows remain visible and do not count as clean measurements.

| Phenotype | Population | Affected / eligible changes | Findings / eligible units | Fully measured / eligible changes | Measured / eligible units | State counts |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| shipped-complexity | natural-agent | 0 / 53 | 0 / 1848 | 40 / 53 | 1514 / 1848 | complete=40, partial=13, unsupported=67 |
| shipped-complexity | matched-human | 0 / 45 | 0 / 1810 | 35 / 45 | 1325 / 1810 | complete=35, partial=10, unsupported=47 |
| shipped-escapes | natural-agent | 36 / 120 | 383 / 15492 | 120 / 120 | 15492 / 15492 | complete=120 |
| shipped-escapes | matched-human | 21 / 92 | 82 / 7324 | 92 / 92 | 7324 / 7324 | complete=92 |
| shipped-stubs | natural-agent | 6 / 120 | 9 / 15507 | 119 / 120 | 15478 / 15507 | complete=119, partial=1 |
| shipped-stubs | matched-human | 4 / 92 | 4 / 7339 | 92 / 92 | 7339 / 7339 | complete=92 |
| shipped-test-deletion | natural-agent | 0 / 26 | 0 / 37 | 0 / 26 | 31 / 37 | partial=26, unsupported=94 |
| shipped-test-deletion | matched-human | 0 / 23 | 0 / 36 | 0 / 23 | 32 / 36 | partial=23, unsupported=69 |
| shipped-test-skip | natural-agent | 36 / 120 | 383 / 15492 | 0 / 120 | 15492 / 15492 | partial=120 |
| shipped-test-skip | matched-human | 21 / 92 | 82 / 7324 | 0 / 92 | 7324 / 7324 | partial=92 |
| shipped-dead-symbols | natural-agent | 3 / 80 | 10 / 136504 | 60 / 80 | 60459 / 136504 | complete=60, partial=20, unsupported=40 |
| shipped-dead-symbols | matched-human | 0 / 58 | 0 / 125937 | 41 / 58 | 53363 / 125937 | complete=41, partial=17, unsupported=34 |
| shipped-reachability | natural-agent | 0 / 53 | 0 / 15906 | 53 / 53 | 15906 / 15906 | complete=53, unsupported=67 |
| shipped-reachability | matched-human | 0 / 39 | 0 / 13387 | 39 / 39 | 13387 / 13387 | complete=39, unsupported=53 |
| shipped-lockfile | natural-agent | 2 / 25 | 4 / 598 | 20 / 25 | 330 / 598 | complete=20, partial=5, unsupported=95 |
| shipped-lockfile | matched-human | 1 / 18 | 2 / 8813 | 10 / 18 | 8521 / 8813 | complete=10, partial=8, unsupported=74 |
| shipped-module-cycle | natural-agent | 2 / 79 | 10 / 85144 | 19 / 79 | 3501 / 85144 | complete=19, partial=60, unsupported=41 |
| shipped-module-cycle | matched-human | 1 / 57 | 8 / 78347 | 11 / 57 | 2115 / 78347 | complete=11, partial=46, unsupported=35 |
| shipped-public-api | natural-agent | 6 / 50 | 11 / 107551 | 5 / 50 | 346 / 107551 | complete=5, partial=45, unsupported=70 |
| shipped-public-api | matched-human | 3 / 37 | 12 / 101233 | 4 / 37 | 253 / 101233 | complete=4, partial=33, unsupported=55 |
| test-all-checks-removed | natural-agent | 0 / 80 | 0 / 30421 | 0 / 80 | 30421 / 30421 | partial=80, unsupported=40 |
| test-all-checks-removed | matched-human | 0 / 58 | 0 / 14074 | 0 / 58 | 14074 / 14074 | partial=58, unsupported=34 |
| test-weakened-rust | natural-agent | 0 / 40 | 0 / 8569 | 0 / 40 | 8569 / 8569 | partial=40, unsupported=80 |
| test-weakened-rust | matched-human | 0 / 27 | 0 / 4729 | 0 / 27 | 4729 / 4729 | partial=27, unsupported=65 |
| test-disabled | natural-agent | 0 / 80 | 0 / 30421 | 0 / 80 | 30421 / 30421 | partial=80, unsupported=40 |
| test-disabled | matched-human | 0 / 58 | 0 / 14074 | 0 / 58 | 14074 / 14074 | partial=58, unsupported=34 |
| test-expected-mirrors-production | natural-agent | 0 / 80 | 0 / 30421 | 0 / 80 | 30421 / 30421 | partial=80, unsupported=40 |
| test-expected-mirrors-production | matched-human | 0 / 58 | 0 / 14074 | 0 / 58 | 14074 / 14074 | partial=58, unsupported=34 |
| test-new-unchecked | natural-agent | 3 / 80 | 17 / 30421 | 0 / 80 | 30421 / 30421 | partial=80, unsupported=40 |
| test-new-unchecked | matched-human | 0 / 58 | 0 / 14074 | 0 / 58 | 14074 / 14074 | partial=58, unsupported=34 |
| design-family-bypass | natural-agent | 1 / 80 | 9 / 30421 | 0 / 80 | 30421 / 30421 | partial=80, unsupported=40 |
| design-family-bypass | matched-human | 0 / 58 | 0 / 14074 | 0 / 58 | 14074 / 14074 | partial=58, unsupported=34 |
| design-registration-bypass | natural-agent | 1 / 80 | 6 / 30421 | 0 / 80 | 30421 / 30421 | partial=80, unsupported=40 |
| design-registration-bypass | matched-human | 0 / 58 | 0 / 14074 | 0 / 58 | 14074 / 14074 | partial=58, unsupported=34 |
| design-wrapper-bypass | natural-agent | 2 / 80 | 2 / 30421 | 0 / 80 | 30421 / 30421 | partial=80, unsupported=40 |
| design-wrapper-bypass | matched-human | 0 / 58 | 0 / 14074 | 0 / 58 | 14074 / 14074 | partial=58, unsupported=34 |
| design-component-cycle | natural-agent | 2 / 40 | 10 / 21852 | 0 / 40 | 21142 / 21852 | local-resolution-incomplete=2, partial=38, unsupported=80 |
| design-component-cycle | matched-human | 0 / 31 | 0 / 9345 | 0 / 31 | 8195 / 9345 | local-resolution-incomplete=3, partial=28, unsupported=61 |
| unfinished-ellipsis-body | natural-agent | 0 / 40 | 0 / 25468 | 0 / 40 | 25468 / 25468 | partial=40, unsupported=80 |
| unfinished-ellipsis-body | matched-human | 0 / 34 | 0 / 16193 | 0 / 34 | 16193 / 16193 | partial=34, unsupported=58 |
| unfinished-throw-body | natural-agent | 0 / 40 | 0 / 25468 | 0 / 40 | 25468 / 25468 | partial=40, unsupported=80 |
| unfinished-throw-body | matched-human | 1 / 34 | 1 / 16193 | 0 / 34 | 16193 / 16193 | partial=34, unsupported=58 |
| error-empty-handler | natural-agent | 1 / 40 | 1 / 25468 | 0 / 40 | 25468 / 25468 | partial=40, unsupported=80 |
| error-empty-handler | matched-human | 1 / 34 | 1 / 16193 | 0 / 34 | 16193 / 16193 | partial=34, unsupported=58 |
| error-default-handler | natural-agent | 0 / 40 | 0 / 25468 | 0 / 40 | 25468 / 25468 | partial=40, unsupported=80 |
| error-default-handler | matched-human | 0 / 34 | 0 / 16193 | 0 / 34 | 16193 / 16193 | partial=34, unsupported=58 |
| error-broad-handler | natural-agent | 6 / 40 | 33 / 25468 | 0 / 40 | 25468 / 25468 | partial=40, unsupported=80 |
| error-broad-handler | matched-human | 3 / 34 | 16 / 16193 | 0 / 34 | 16193 / 16193 | partial=34, unsupported=58 |
| python-uv-lock | natural-agent | 0 / 4 | 0 / 11 | 0 / 4 | 5 / 11 | partial=4, unsupported=116 |
| python-uv-lock | matched-human | 0 / 5 | 0 / 274 | 0 / 5 | 236 / 274 | partial=5, unsupported=87 |
| cargo-lock-entry-shape | natural-agent | 0 / 13 | 0 / 3451 | 0 / 13 | 3451 / 3451 | partial=13, unsupported=107 |
| cargo-lock-entry-shape | matched-human | 0 / 9 | 0 / 1918 | 0 / 9 | 1918 / 1918 | partial=9, unsupported=83 |
| new-direct-dependency | natural-agent | 14 / 14 | 49 / 49 | 14 / 14 | 49 / 49 | complete=14, unsupported=106 |
| new-direct-dependency | matched-human | 11 / 11 | 41 / 41 | 11 / 11 | 41 / 41 | complete=11, unsupported=81 |
| ruff-python-injection | natural-agent | 4 / 39 | 15 / 286 | 39 / 39 | 286 / 286 | complete=39, unsupported=81 |
| ruff-python-injection | matched-human | 6 / 34 | 12 / 162 | 34 / 34 | 162 / 162 | complete=34, unsupported=58 |
| ruff-python-swallowed | natural-agent | 18 / 39 | 273 / 286 | 39 / 39 | 286 / 286 | complete=39, unsupported=81 |
| ruff-python-swallowed | matched-human | 19 / 34 | 486 / 162 | 34 / 34 | 162 / 162 | complete=34, unsupported=58 |

All counts can be recalculated from `measurements.tsv` and `findings.tsv`.
