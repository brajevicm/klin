# Large repositories have explicit performance budgets

Issue #182 turns the dense structural fixture from benchmark evidence into a
product contract. The optimizations in #176, #183 and #190--#193 are already
landed; this decision chooses the limits they must support.

## Decision

SPEC 13 defines these budgets for the deterministic `structural_300k` and
`structural_1m` rows:

| Workload | `structural_300k` | `structural_1m` |
| --- | ---: | ---: |
| Warm Stop hook, 20 changed files and a warm structural cache | 5 seconds | 5 seconds |
| Cold survey | 30 seconds | 60 seconds |
| Whole-tree `--strict` run | 20 seconds | 45 seconds |

The warm hook excludes the project's own build. The 100-file warm row remains
a scaling check and does not create another product budget. These are rounded
product requirements with operating headroom, not formulas derived from the
current medians. The existing 2,000-file budgets remain unchanged.

The release checklist reruns the dense rows on the controlled reference
machine. A median that misses a budget or rises by more than one third against
the preceding controlled row must be explained in the release notes before
release. Contributor tests continue to check deterministic fixture shape and
semantics only; they do not enforce machine-specific wall-clock or RSS values.

## Evidence

The authoritative decision run is the final #193 measurement: release build
`0.1.1` at benchmark commit
`aa2ca0a3bc92a5ee4b234e15bc87e8a5fa4f0856`, five iterations per row, on a
MacBook Pro 18,3 with an Apple M1 Pro, macOS 26.6.2 / Darwin 25.6.0 arm64. It measured
2,388 ms and 2,733 ms for the 20-file warm rows, 22,834 ms and 47,507 ms for
the cold surveys, and 12,168 ms and 30,724 ms for the strict runs at 300k and
1M respectively. The complete file/LoC counts, changed-file counters,
per-gate timings, RSS and cache-size evidence are in
`docs/structural-views-2026-09-15.md` and summarized in `RELEASE_NOTES.md`.

## Consequences

Large repositories have a visible latency promise for agent feedback and CI,
while the measurement process remains reproducible without making arbitrary
contributor hardware a timing oracle. A future release can exceed a budget
only with a documented decision, and a regression over one third remains
visible even when the absolute budget still passes. RSS and retained cache
size stay diagnostics because this decision owns wall-clock product budgets,
not a machine-independent memory contract.
