# Labeled signal synthesis

Unblinded after the locked labels verified against SHA-256 `c79b438aa9b2e5f95c777ccd534bac2b40fd1fe19166ee44b74e664c09bd8e28`.
The population is the v3 paired round only: `v3-2026-09-25`.

- Labels: 9 `valid-regression`, 4 `valid-review`, 0 `undesired`

## Site-level, per gate

A site is one worksheet row. Occurrences count every recorded firing that joined to the row, so one site met many times is visible as such.

| gate | sites | valid-regression | valid-review | undesired | occurrences | v3 | occ. valid-regression | occ. valid-review | occ. undesired |
|---|---|---|---|---|---|---|---|---|---|
| complexity | 4 | 0/4 (0%) | 4/4 (100%) | 0/4 (0%) | 6 | 6 | 0 | 6 | 0 |
| doc-citations | 9 | 9/9 (100%) | 0/9 (0%) | 0/9 (0%) | 18 | 18 | 18 | 0 | 0 |

## Run-level intervention view

Active counts are delivered signals over the scheduled Active runs of the round. Shadow counts are would-have-been-delivered signals, reported descriptively and never pooled into the Active rates.

| stratum | arm | runs | runs with an undesired signal | controls | controls with an undesired signal | risk runs | risk runs with a valid signal | undesired occurrences | valid occurrences |
|---|---|---|---|---|---|---|---|---|---|
| v3 | active | 7 | 0/7 (0%) | 2 | 0/2 (0%) | 5 | 5/5 (100%) | - | - |
| v3 | shadow | 7 | 0/7 (0%) | 2 | 0/2 (0%) | 5 | 5/5 (100%) | 0 | 12 |

## Interpretation boundary

These labels judge whether each signal was appropriate when it fired. Guardrails 3 and 4 of `docs/benchmark-rubric-v3.md` read the Active rows above: the runs, and the control runs, that exposed the agent to at least one `undesired` signal. The result document applies the frozen v3 rubric. No admission, natural or seeded record is in this population.
