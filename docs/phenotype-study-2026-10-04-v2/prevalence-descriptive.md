# Descriptive natural prevalence

The natural-agent and matched-human arms are shown separately. Human controls are changes not attributed to an agent by the preregistered metadata exclusions; this is not proof of human-only authorship.
Rate differences are descriptive and do not establish an AI-specific effect.

## Materialized sample

| Language | Agent changes | Matched human changes | Unmatched agent changes |
| --- | ---: | ---: | ---: |
| Rust | 40 | 27 | 13 |
| TypeScript | 40 | 31 | 9 |
| Python | 40 | 34 | 6 |

Frozen detector replay only. These are descriptive measurements, not valid-regression labels or product dispositions.
Incomplete rows remain visible and are not counted as clean. Unknown change eligibility is neither eligible nor ineligible. When the change census is complete, an affected-change interval includes observed positives as its lower bound and unresolved eligible changes as its upper bound. Change rates are withheld when change-level eligibility is unresolved. Site rates are withheld when change eligibility, any eligible-unit census, or any eligible measurement is incomplete.

| Phenotype | Population | Affected / eligible changes | Findings / eligible units | Change eligibility known / sampled changes | Unit census known / possible eligible changes | Fully measured / eligible changes | Measured / eligible units | State counts |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| shipped-complexity | natural-agent | unavailable (0 observed; eligibility unresolved for 57) | unavailable | 63 / 120 | 52 / 109 | unavailable | unavailable | complete=40, partial=12 |
| shipped-complexity | matched-human | unavailable (0 observed; eligibility unresolved for 41) | unavailable | 51 / 92 | 45 / 86 | unavailable | unavailable | complete=35, partial=10 |
| shipped-escapes | natural-agent | 36 / 120 | 383 / 49112 | 120 / 120 | 120 / 120 | 120 / 120 | 49112 / 49112 | complete=120 |
| shipped-escapes | matched-human | 21 / 92 | 82 / 21911 | 92 / 92 | 92 / 92 | 92 / 92 | 21911 / 21911 | complete=92 |
| shipped-stubs | natural-agent | 6–120 / 120 | unavailable | 120 / 120 | 0 / 120 | 0 / 120 | unavailable | partial=120 |
| shipped-stubs | matched-human | 4–92 / 92 | unavailable | 92 / 92 | 0 / 92 | 0 / 92 | unavailable | partial=92 |
| shipped-test-deletion | natural-agent | unavailable (0 observed; eligibility unresolved for 98) | unavailable | 22 / 120 | 5 / 104 | unavailable | unavailable | partial=6 |
| shipped-test-deletion | matched-human | unavailable (0 observed; eligibility unresolved for 78) | unavailable | 14 / 92 | 1 / 79 | unavailable | unavailable | partial=1 |
| shipped-test-skip | natural-agent | unavailable (36 observed; eligibility unresolved for 84) | unavailable | 36 / 120 | 0 / 120 | unavailable | unavailable | partial=36 |
| shipped-test-skip | matched-human | unavailable (21 observed; eligibility unresolved for 71) | unavailable | 21 / 92 | 0 / 92 | unavailable | unavailable | partial=21 |
| shipped-dead-symbols | natural-agent | unavailable (3 observed; eligibility unresolved for 77) | unavailable | 43 / 120 | 0 / 80 | unavailable | unavailable | partial=3 |
| shipped-dead-symbols | matched-human | unavailable (0 observed; eligibility unresolved for 58) | unavailable | 34 / 92 | 0 / 58 | unavailable | unavailable | — |
| shipped-reachability | natural-agent | unavailable (0 observed; eligibility unresolved for 80) | unavailable | 40 / 120 | 0 / 80 | unavailable | unavailable | — |
| shipped-reachability | matched-human | unavailable (0 observed; eligibility unresolved for 58) | unavailable | 34 / 92 | 0 / 58 | unavailable | unavailable | — |
| shipped-lockfile | natural-agent | 2 / 22 | unavailable | 120 / 120 | 20 / 22 | 20 / 22 | unavailable | complete=20, partial=2 |
| shipped-lockfile | matched-human | 1 / 14 | unavailable | 92 / 92 | 13 / 14 | 13 / 14 | unavailable | complete=13, partial=1 |
| shipped-module-cycle | natural-agent | unavailable (2 observed; eligibility unresolved for 78) | unavailable | 42 / 120 | 0 / 80 | unavailable | unavailable | partial=2 |
| shipped-module-cycle | matched-human | unavailable (1 observed; eligibility unresolved for 57) | unavailable | 35 / 92 | 0 / 58 | unavailable | unavailable | partial=1 |
| shipped-public-api | natural-agent | unavailable (6 observed; eligibility unresolved for 74) | unavailable | 46 / 120 | 0 / 80 | unavailable | unavailable | partial=6 |
| shipped-public-api | matched-human | unavailable (3 observed; eligibility unresolved for 55) | unavailable | 37 / 92 | 0 / 58 | unavailable | unavailable | partial=3 |
| test-all-checks-removed | natural-agent | unavailable (0 observed; eligibility unresolved for 80) | unavailable | 40 / 120 | 0 / 80 | unavailable | unavailable | — |
| test-all-checks-removed | matched-human | unavailable (0 observed; eligibility unresolved for 58) | unavailable | 34 / 92 | 0 / 58 | unavailable | unavailable | — |
| test-weakened-rust | natural-agent | unavailable (0 observed; eligibility unresolved for 40) | unavailable | 80 / 120 | 0 / 40 | unavailable | unavailable | — |
| test-weakened-rust | matched-human | unavailable (0 observed; eligibility unresolved for 27) | unavailable | 65 / 92 | 0 / 27 | unavailable | unavailable | — |
| test-disabled | natural-agent | unavailable (0 observed; eligibility unresolved for 80) | unavailable | 40 / 120 | 0 / 80 | unavailable | unavailable | — |
| test-disabled | matched-human | unavailable (0 observed; eligibility unresolved for 58) | unavailable | 34 / 92 | 0 / 58 | unavailable | unavailable | — |
| test-expected-mirrors-production | natural-agent | unavailable (0 observed; eligibility unresolved for 80) | unavailable | 40 / 120 | 0 / 80 | unavailable | unavailable | — |
| test-expected-mirrors-production | matched-human | unavailable (0 observed; eligibility unresolved for 58) | unavailable | 34 / 92 | 0 / 58 | unavailable | unavailable | — |
| test-new-unchecked | natural-agent | unavailable (3 observed; eligibility unresolved for 77) | unavailable | 43 / 120 | 0 / 80 | unavailable | unavailable | partial=3 |
| test-new-unchecked | matched-human | unavailable (0 observed; eligibility unresolved for 58) | unavailable | 34 / 92 | 0 / 58 | unavailable | unavailable | — |
| design-family-bypass | natural-agent | unavailable (1 observed; eligibility unresolved for 79) | unavailable | 41 / 120 | 0 / 80 | unavailable | unavailable | partial=1 |
| design-family-bypass | matched-human | unavailable (0 observed; eligibility unresolved for 58) | unavailable | 34 / 92 | 0 / 58 | unavailable | unavailable | — |
| design-registration-bypass | natural-agent | unavailable (1 observed; eligibility unresolved for 79) | unavailable | 41 / 120 | 0 / 80 | unavailable | unavailable | partial=1 |
| design-registration-bypass | matched-human | unavailable (0 observed; eligibility unresolved for 58) | unavailable | 34 / 92 | 0 / 58 | unavailable | unavailable | — |
| design-wrapper-bypass | natural-agent | unavailable (2 observed; eligibility unresolved for 78) | unavailable | 42 / 120 | 0 / 80 | unavailable | unavailable | partial=2 |
| design-wrapper-bypass | matched-human | unavailable (0 observed; eligibility unresolved for 58) | unavailable | 34 / 92 | 0 / 58 | unavailable | unavailable | — |
| design-component-cycle | natural-agent | unavailable (2 observed; eligibility unresolved for 38) | unavailable | 82 / 120 | 0 / 40 | unavailable | unavailable | partial=2 |
| design-component-cycle | matched-human | unavailable (0 observed; eligibility unresolved for 31) | unavailable | 61 / 92 | 0 / 31 | unavailable | unavailable | — |
| unfinished-ellipsis-body | natural-agent | unavailable (0 observed; eligibility unresolved for 40) | unavailable | 80 / 120 | 0 / 40 | unavailable | unavailable | — |
| unfinished-ellipsis-body | matched-human | unavailable (0 observed; eligibility unresolved for 34) | unavailable | 58 / 92 | 0 / 34 | unavailable | unavailable | — |
| unfinished-throw-body | natural-agent | unavailable (0 observed; eligibility unresolved for 40) | unavailable | 80 / 120 | 0 / 40 | unavailable | unavailable | — |
| unfinished-throw-body | matched-human | unavailable (1 observed; eligibility unresolved for 33) | unavailable | 59 / 92 | 0 / 34 | unavailable | unavailable | partial=1 |
| error-empty-handler | natural-agent | unavailable (1 observed; eligibility unresolved for 39) | unavailable | 81 / 120 | 0 / 40 | unavailable | unavailable | partial=1 |
| error-empty-handler | matched-human | unavailable (1 observed; eligibility unresolved for 33) | unavailable | 59 / 92 | 0 / 34 | unavailable | unavailable | partial=1 |
| error-default-handler | natural-agent | unavailable (0 observed; eligibility unresolved for 40) | unavailable | 80 / 120 | 0 / 40 | unavailable | unavailable | — |
| error-default-handler | matched-human | unavailable (0 observed; eligibility unresolved for 34) | unavailable | 58 / 92 | 0 / 34 | unavailable | unavailable | — |
| error-broad-handler | natural-agent | unavailable (6 observed; eligibility unresolved for 34) | unavailable | 86 / 120 | 0 / 40 | unavailable | unavailable | partial=6 |
| error-broad-handler | matched-human | unavailable (3 observed; eligibility unresolved for 31) | unavailable | 61 / 92 | 0 / 34 | unavailable | unavailable | partial=3 |
| python-uv-lock | natural-agent | unavailable (0 observed; eligibility unresolved for 4) | unavailable | 116 / 120 | 0 / 4 | unavailable | unavailable | — |
| python-uv-lock | matched-human | unavailable (0 observed; eligibility unresolved for 5) | unavailable | 87 / 92 | 0 / 5 | unavailable | unavailable | — |
| cargo-lock-entry-shape | natural-agent | unavailable (0 observed; eligibility unresolved for 13) | unavailable | 107 / 120 | 0 / 13 | unavailable | unavailable | — |
| cargo-lock-entry-shape | matched-human | unavailable (0 observed; eligibility unresolved for 9) | unavailable | 83 / 92 | 0 / 9 | unavailable | unavailable | — |
| new-direct-dependency | natural-agent | 14–27 / 27 | unavailable | 120 / 120 | 0 / 27 | 0 / 27 | unavailable | partial=27 |
| new-direct-dependency | matched-human | 11–24 / 24 | unavailable | 92 / 92 | 0 / 24 | 0 / 24 | unavailable | partial=24 |
| ruff-python-injection | natural-agent | 4–5 / 40 | unavailable | 120 / 120 | 40 / 40 | 39 / 40 | unavailable | complete=39, unsupported=1 |
| ruff-python-injection | matched-human | 6 / 34 | 12 / 162 | 92 / 92 | 34 / 34 | 34 / 34 | 162 / 162 | complete=34 |
| ruff-python-swallowed | natural-agent | 18–19 / 40 | unavailable | 120 / 120 | 40 / 40 | 39 / 40 | unavailable | complete=39, unsupported=1 |
| ruff-python-swallowed | matched-human | 19 / 34 | 486 / 162 | 92 / 92 | 34 / 34 | 34 / 34 | 162 / 162 | complete=34 |

All counts can be recalculated from measurements.tsv and findings.tsv.
