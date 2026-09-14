# A derived ceiling reads the commit's scope

The compact source policy of #179 left one ambiguity: when `complexity` derives
its ceilings, whether `in` and `except` select only the functions judged or
also the functions sampled. Sampling the whole repository lets code the gate
does not judge loosen its yardstick. Sampling today's scope makes an
uncommitted policy edit move a number that section 4.3 says is a fact of one
commit, and a cache keyed only by that commit becomes order-dependent.

## The decision

The sample is the supported functions selected by the compact complexity scope
recorded in the derivation commit's `klin.json`. Today's scope selects what the
gate judges in both trees and never enters the sample. A scope edit therefore
changes judgment immediately and changes the ceiling only after the derivation
commit advances. The derived line names the recorded scope, and a NOTE names a
difference between it and today's scope. The two are compared in one canonical
form, so reordering or repeating a path is no difference. That NOTE and the
fallback NOTE below carry the `derivation` outcome, which the stop hook tells
even when nothing blocks, so a lagging ceiling is never silent.

The cache remains keyed by binary version, commit and section. Its entry keeps
the sample, recorded scope and any fallback note, so a warm run neither reads
the historical config nor parses the sample again.

An absent file, section, `in` or `except` is the ordinary whole-repository
scope. This also makes a pre-compact section containing only retired fields a
whole-repository sample. A present policy that cannot be read as compact scope
falls back to the whole repository with a NOTE. A valid recorded scope that
selects no supported function is not a fallback: it derives the floors and
prints the zero-function sample.

## Consequences

The ceiling is a pure function of the derivation commit and binary. A person
can write an exclusion once without maintaining numbers, excluded code does
not normalize the code still judged, and no working-tree edit recalibrates the
yardstick inside a window. Scoped cold runs pay one historical config read and
one parse; every later stop at that derivation commit reads the existing cache.

Measured on 2026-09-14, macos/aarch64, median of five, with the 0.1.1 binary of
`b3f52a5` beside this change. `whole` is the fixture with no `complexity`
section, and `rust` is `KLIN_PERF_SCOPE=rust`. Each cell is the run's total, and
the complexity gate's own `ms` follows in brackets:

| Row | before | after, whole | after, rust |
|---|---|---|---|
| 2k warm hook | 1,153 (90) | 1,528 (92) | 1,444 (41) |
| 2k cold survey | 1,828 (478) | 1,871 (478) | 1,618 (235) |
| 2k strict | 1,156 (179) | 1,231 (176) | 1,158 (88) |
| 10k warm hook | 4,087 (447) | 6,177 (462) | 6,056 (230) |
| 10k cold survey | 13,646 (5,106) | 14,368 (5,154) | 12,357 (2,729) |
| 10k strict | 5,036 (914) | 5,712 (947) | 5,465 (484) |
| 300k warm hook | 9,937 (1,488) | 17,552 (1,587) | 16,709 (694) |
| 300k cold survey | 24,932 (8,355) | 31,341 (8,510) | 25,705 (3,992) |
| 300k strict | 12,955 (2,933) | 19,190 (3,111) | 16,905 (1,393) |

The whole-repository sample costs what it cost before, and a narrower recorded
scope samples and measures less. The totals grew because `reachability` now runs
as an Automatic check on this fixture, where the earlier binary needed a section
a person wrote. Its own `ms` accounts for most of the growth: 2,002 ms in the
10k warm hook and 6,512 ms in the 300k warm hook. Without it, the 300k warm hook
went from 9,937 to 11,040 ms. A warm cache is read and not written again. The
first measurement of this change wrote an empty `in` and `except` that the next
run could not read back, so every stop sampled again. A CLI test now holds the
cache read-back for every scope shape.

Generated-file markers are not inferred here. Excluding them changes a
percentile in a direction the marker alone cannot predict and needs its own
decision. An additional informational percentile over today's scope is also
deferred: the recorded rule and scope already explain the enforced number, and
recomputing a second sample would spend hook time without changing a verdict.
