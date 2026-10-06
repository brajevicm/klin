# Uncapped Rust occurrence cohorts

`proto/src/cohorts.rs` implements the exact current-content family interface:

```rust
pub struct Region {
    pub text: Vec<Vec<u8>>,
    pub occurrences: Vec<(usize, usize)>, // file index, token start
}
pub fn regions(files: &[FileTokens], threshold: usize) -> Vec<Region>;
```

The output contains each distinct pairwise maximal safe canonical region, with
all occurrences participating in at least one maximal match. Self-overlap is
included. A positive threshold is required. `None` tokens are hard boundaries;
byte equality preserves caller-supplied language and import provenance. File
rows, error status and unsafe-unit counts do not alter matching: the caller must
encode unsafe token spans as `None` and report unavailable extraction separately.

Streams with identical canonical bytes and boundary positions form one class.
Repeated files add their full safe runs directly, preserving every file occurrence.
For remaining matches a shared exact-byte dictionary feeds a generalized suffix
array. Unique terminators prevent matches across safe-run boundaries. LCP
interval children represent different following tokens; predecessor counts select
leaves having a partner with both a different predecessor and a different child.
That produces maximal regions without enumerating witness pairs. Dictionaries,
region keys and comparisons retain exact bytes; hashes never certify equality.

Run `python3 docs/duplication-stop-design-2026-10-05/verify-cohorts.py` from any
working directory. It compiles a temporary standalone Rust command-line driver,
runs 247 cases, and compares complete text-to-occurrence mappings against the
independent exhaustive diagonal oracle. The 1000-identical-file case uses its
explicit expected mapping instead of expanding pairwise oracle work. All cases
passed: 240 deterministic randomized cases and seven targeted cases covering
interior regions, self-overlap, overlapping families, unsafe boundaries,
language/provenance, repeated files and short decoys.

This is a correctness prototype, not a qualified Stop matcher. Comparison-sort
suffix doubling costs O(n log² n), nested intervals can revisit leaves
quadratically, and explicit family text/occurrence output can itself be large.
No caps or shortcuts hide those costs. Construction currently consumes all input
chains and builds an in-memory index on every call; incremental base/current
ownership, persistent lookup, generic lineage and end-to-end Stop budgets remain
unimplemented. Corpus measurements were serialized after storage measurements.

The optional `cohorts ROOT MAP.json` output uses hexadecimal canonical bytes,
including non-UTF-8 bytes. This serializer was corrected after the timed runs;
it runs after the matcher timer and changes no matching or measured stage work.
Earlier output encoded UTF-8 strings; the normalized complete-map digest in the
results file already uses hex and remains the comparison authority.

## Serial corpus measurement

One release-mode full rebuild per corpus, T=60, normalization completed before
the timed call. Raw measurements and binary/source hashes are in
`cohorts-results.json`.

| Corpus | Files | Tokens | Families / occurrences | Matcher construction + report |
| --- | ---: | ---: | ---: | ---: |
| pinned klin/src | 80 | 233,276 | 20 / 41 | 126.80 ms |
| 1M fixture | 9,998 | 5,930,008 | 0 / 0 | 4,838.76 ms |

The klin complete output map matches the prior seed and suffix Python diagnostics:
SHA256 `46394a584153d4ea8f63a1794f1e30c5722ca5d605538d2b7a42aeab324f9c7f`
under their canonical hex-token serialization. This confirms every occurrence,
not just counts. The 1M fixture has no T=60 exact families; its empty complete
map still costs 4.84 seconds to construct. There is no positive performance
verdict: full rebuild costs exceed warm Stop limits, and the fixture cost alone
exceeds the measured cold incremental allowance. Incremental reuse could change
those costs, but this experiment does not implement or measure it. Output map
serialization and disk write occur after the timer. RSS was not measured.
