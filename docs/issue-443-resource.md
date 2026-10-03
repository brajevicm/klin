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
The shared parser now refuses a source line exceeding 65,536 UTF-8 bytes
before allocating a parse or extracting sites. The ceiling also covers
survey and tolerant readers; ordinary multiline source has no file-size cap.
This is a deterministic source-line ceiling, not a total memory guarantee.

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
