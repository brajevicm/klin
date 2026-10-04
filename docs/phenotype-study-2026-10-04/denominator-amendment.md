# Denominator amendment, 2026-10-04

The first replay used JSON gate counters as eligible-unit denominators. Those
counters include whole-tree declarations, files, surfaces, dependency edges,
or files of coverage; they do not consistently represent the units frozen in
`phenotypes.tsv`. Those site-rate cells are withdrawn.

This amendment changes only census and reporting. It does not change the frozen
population, detector, candidate, identity, or denominator definitions. The
replay recovers exact eligible-unit counts where the frozen run artifacts
support them: changed complexity functions when fully enumerated, changed
source-line events for escapes, before-tree test identities for deletion,
and Ruff's changed Python files. A row whose registered unit census cannot be
recovered is marked `partial`, has `eligible_units_known: false` in its
`values`, and records the reason in `holes`; its site rate is reported as
unavailable rather than as zero.

For affected changes with a complete change-level eligibility census, observed
findings establish the lower bound. Eligible changes whose detector outcome is
incomplete and has no observed finding remain unresolved and form the upper
bound. The report shows that interval instead of counting those changes as
clean. Missing site-unit counts do not make a detector outcome unresolved. When
the frozen artifacts do not resolve change-level eligibility, the affected-
change rate is unavailable and the number of unresolved sample changes is
shown.

The frozen binary and prototype outputs do not expose complete changed-unit
censuses for stubs, test-skip, dead symbols, reachability, lockfile, module-cycle,
public-api, new direct dependencies, the research test/design/unfinished/error
families, or lock-shape candidates.
For several of those phenotypes they also do not expose the registered
change-level eligibility census. The report withholds affected-change rates
where that eligibility cannot be established, and withholds all site rates
where the unit census is incomplete. This is a recorded study limitation; the
acceptance criterion that every phenotype have a reportable registered
eligibility denominator remains unmet for those families. Natural findings and
hard-negative calibration rows remain separate; no case labels or product
dispositions are added.
