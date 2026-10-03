# Issue #443: minified source-line amplification

The pilot reported a SIGKILL after 451 seconds with no output on
MontrealAI/AGI-Alpha-Agent-v0#3073. That historical kill's cause remains
unknown. This reproduction establishes a resource-heavy path in the same tree.

The head is `35bffe634035d44f92e764643ebaa33c19403605`; `main` points to
its merge base, `09a39cc21d4ce79781c54e29d19d7b1a76472603`.
The working tree has a `klin.json` containing `{}`. Run the release binary
from that tree with `klin gate --json`.

Before this change, the reproduction produced no output after nearly two
minutes and reached 1,766,224 KiB resident memory. It was terminated manually.
A separate repository holding only its 3,585,993-byte `plotly.min.js` file
and an empty base also exceeded a ten-second subprocess deadline.

Complexity retains a complete source line as each function's site text,
including in its survey sample. Thousands of functions sharing a minified
multi-megabyte line amplify that text before the runner can print a report.
The initial fix refused source lines exceeding 65,536 UTF-8 bytes before
parsing. That stopped this bundle but did not address multiplicative site
text retention on dense lines below the ceiling. The follow-up below fixes
that representation; the source-line ceiling remains defense in depth,
not a total memory guarantee.

After this change, `/usr/bin/time -l` measured the same release command at
4.65 seconds elapsed and 52,543,488 bytes maximum resident set size on an
Apple M1 Pro running macOS, on 2026-10-03. It returned exit 2 with valid JSON,
status `ERROR`, and a complexity error naming
`alpha_factory_v1/demos/alpha_agi_insight_v1/insight_browser_v1/lib/bundle.esm.min.js:1`,
its 117,874-byte line and the 65,536-byte ceiling. This satisfies the issue's
named-resource-error alternative; it does not measure the refused bundles.

The CLI tests cover the named error, a default `{}` gate's JSON report, and
the inclusive byte boundary with multibyte UTF-8 and CRLF. Timing and RSS are
reproduction evidence, not contributor test assertions (ADR 0042).

## Follow-up to the adversarial review

PR #449's review supplied a valid 60,000-byte JavaScript line,
`"()=>0;".repeat(10_000)`, below the new source-line ceiling. The initial
PR binary crossed a 512 MiB diagnostic stop condition: 657,600 KiB RSS was
observed before the probe killed it. That condition is a local measurement
harness, not a product or contributor-test budget.

Complexity now shares the text for identical source-row/holder-row pairs
using `Rc<str>` within each parsed file. It creates owned finding text only
for functions over their ceilings. The survey visits the same selected
functions and records only their cyclomatic complexity and length, without
constructing retained functions, site text or body hashes. Finding output
can still consume memory proportional to its text; this change does not
claim a total memory guarantee.

A release run of `complexity --strict` with one 10,000-function line in both
trees returned exit 0 in 0.27 seconds at 17,612,800 bytes peak RSS. The
three-line variant retained all 30,000 functions and returned exit 0 in
0.46 seconds at 43,728,896 bytes peak RSS. Each run sampled the base's full
function count and measured both trees; a trailing non-function declaration
made the working tree differ from its base. These are single diagnostic
measurements on the same machine, not timing test assertions.

Inventory now propagates parser errors through `convention::tests` instead
of collapsing them into a generic grammar rejection. Its CLI regression
checks an oversized test source through `gate --gate inventory --json`,
including the complete measured byte count and ceiling. The original
complexity test also asserts both measured and ceiling byte counts.

## Normal-workload comparison

The comparison uses release binaries from pre-PR commit `f1e273bf` and this
follow-up, on the same machine. The deterministic fixture has 2,000 source
files (1,000 Rust and 1,000 JavaScript), ten eight-line functions per file:
160,000 base lines and 20,000 base functions. Each function has one `if` and
an `else`, cyclomatic complexity 2, and a body length of 8. The base commits
those files and `{}`; the working tree adds one simple function in each of
20 Rust files. The command is `klin gate --gate complexity --json`, so the
survey samples the base and complexity measures both trees. All runs return
exit 0 and status `PASS`.

Five pairs alternate binary order, clearing `.git/klin` before each run.
The table is the second measurement series, after the full test suite had
finished; an earlier series overlapped the suite and had much wider timing
variation. This is a local normal-workload diagnostic, not a substitute for
ADR 0042's controlled dense benchmark rows.

| Pair | Before elapsed (s) | After elapsed (s) | Before peak RSS (bytes) | After peak RSS (bytes) |
| --- | ---: | ---: | ---: | ---: |
| 1 | 2.23 | 2.08 | 13,369,344 | 13,811,712 |
| 2 | 2.14 | 2.05 | 13,828,096 | 13,811,712 |
| 3 | 2.12 | 2.05 | 13,189,120 | 14,073,856 |
| 4 | 2.11 | 2.05 | 13,303,808 | 13,778,944 |
| 5 | 2.12 | 2.10 | 13,549,568 | 14,172,160 |
| Median | 2.12 | 2.05 | 13,369,344 | 13,811,712 |

Median complexity gate time is 1,670 ms before and 1,606 ms after. The
combined parser scan and representation changes show no latency increase
in this fixture. Median RSS increases by 442,368 bytes, consistent with the
small per-file sharing table and shared-string metadata on short-line
source. This comparison does not isolate the parser scan's individual cost.
