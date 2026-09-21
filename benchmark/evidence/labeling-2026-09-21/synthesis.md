# Labeled signal synthesis

Unblinded after the locked labels verified against SHA-256 `223bc66b760d18f51a6e4cc2196de5e4d03b2adaf10cc7b997aed24bd13c96dd`.
The population is the two natural rounds only: v1 `publishable-2026-09-18` and v2 `v2-2026-09-20`.

- Labels: 23 `valid-regression`, 10 `valid-review`, 5 `undesired`

## Site-level, per gate

A site is one worksheet row. Occurrences count every recorded firing that joined to the row, so one site met many times is visible as such.

| gate | sites | valid-regression | valid-review | undesired | occurrences | v1 | v2 | occ. valid-regression | occ. valid-review | occ. undesired |
|---|---|---|---|---|---|---|---|---|---|---|
| complexity | 8 | 8/8 (100%) | 0/8 (0%) | 0/8 (0%) | 19 | 11 | 8 | 19 | 0 | 0 |
| doc-citations | 2 | 2/2 (100%) | 0/2 (0%) | 0/2 (0%) | 4 | 0 | 4 | 4 | 0 | 0 |
| escapes | 5 | 0/5 (0%) | 0/5 (0%) | 5/5 (100%) | 5 | 4 | 1 | 0 | 0 | 5 |
| inventory | 4 | 0/4 (0%) | 4/4 (100%) | 0/4 (0%) | 24 | 12 | 12 | 0 | 24 | 0 |
| lockfile | 11 | 11/11 (100%) | 0/11 (0%) | 0/11 (0%) | 11 | 11 | 0 | 11 | 0 | 0 |
| public-api | 6 | 0/6 (0%) | 6/6 (100%) | 0/6 (0%) | 48 | 24 | 24 | 0 | 48 | 0 |
| reachability | 2 | 2/2 (100%) | 0/2 (0%) | 0/2 (0%) | 4 | 4 | 0 | 4 | 0 | 0 |

## Run-level intervention view

Active counts are delivered signals over the scheduled Active natural runs of that round. Shadow counts are would-have-been-delivered signals, reported descriptively and never pooled into the Active rates. v1 and v2 stay separate because their apparatus and fixtures differ.

| stratum | arm | runs | runs with an undesired signal | controls | controls with an undesired signal | risk runs | risk runs with a valid signal | undesired occurrences | valid occurrences |
|---|---|---|---|---|---|---|---|---|---|
| v1 | active | 36 | 2/36 (6%) | 9 | 0/9 (0%) | 27 | 12/27 (44%) | - | - |
| v1 | shadow | 36 | 1/36 (3%) | 9 | 0/9 (0%) | 27 | 10/27 (37%) | 1 | 29 |
| v2 | active | 36 | 0/36 (0%) | 9 | 0/9 (0%) | 27 | 10/27 (37%) | - | - |
| v2 | shadow | 36 | 1/36 (3%) | 9 | 0/9 (0%) | 27 | 6/27 (22%) | 1 | 21 |

## Interpretation boundary

These labels judge whether each signal was appropriate when it fired. They do not cure the challenge-adequacy shortfall of the natural experiment: the repaired v2 round exposed the target shortcut in 3 of 27 Shadow risk runs and one of nine families, below #115's frozen floor, and stays challenge-limited. No seeded (#260/#261) record is in this population or in any rate above; the seeded experiment is a separate conditional-mechanism analysis. This document states counts and turns them into no causal product conclusion beyond what #115's frozen rubric permits.
