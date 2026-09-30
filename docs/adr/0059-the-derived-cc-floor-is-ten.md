# The derived `cc` floor is 10

> Amends spec 5.4. The percentile, the sample size and the `lines` floor of
> 25 stand. What changes is the `cc` floor, from 5 to 10.

A derived `complexity.cc` is the 95th percentile of `cc` over the derivation
commit's functions, or the floor, whichever is higher. Below 50 functions the
floor is the ceiling. The floor was 5.

The replay of #343 (`docs/false-alarms-2026-09-29.md`) labeled every finding
that the floor of 5 alone decided as not appropriate: R049, a new callback at
cc 7, and the short functions of R054 at cc 6 to 10. An agent drafted those
labels and agents reviewed them. The same record noted that a floor of 10
clears only one more not-appropriate row, R065, and left the decision to
#389. #389 decided it.

## The decision

**The derived `cc` floor is 10.** A derived `complexity.cc` is the 95th
percentile or 10, whichever is higher, and below 50 functions the ceiling is
10. A pinned `cc` still wins, whatever its value, so a person who wants 5
writes `"cc": 5`.

10 is the threshold McCabe proposed and NIST SP 500-235 (Watson and McCabe,
1996) records as the default limit. SlopCodeBench counts a function as
complex at CC > 10, the threshold it takes from Radon. This record does not
cite NASA for 10: NPR 7150.2 asks for 15 or lower on safety-critical code.

## Rejected

- **Keep 5.** The floor alone decided the noise in R049 and R054. A small
  tree has no percentile to protect it, so every function at cc 6 fails
  there, and no labeled row supports that.
- **14, 15 or 20.** No labeled row needs a floor above 10. A higher floor
  loosens every small tree with no evidence that the current one is wrong.

## Consequences

A tree whose percentile is below 10, or which has fewer than 50 functions,
now accepts a new function up to cc 10. A tree whose percentile is above 10
is judged as before. `klin init --pin` writes 10 where it wrote 5.
