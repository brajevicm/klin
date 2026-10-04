# Natural phenotype measurement

This is the reproducible measurement workflow for issue #457. The population,
detectors, language scope and denominators are frozen in `protocol.md`,
`populations.tsv` and `phenotypes.tsv`.

Run from the repository root. `--cache` must name the same writable directory
for all commands; it holds the downloaded AIDev snapshot, GitHub response
cache, fetched repositories and temporary build trees. Selection checkpoints
let `materialize` resume after a network interruption.

```sh
uv run --with duckdb -- python docs/phenotype-study-2026-10-04/study.py materialize --cache /path/to/study-cache
python docs/phenotype-study-2026-10-04/study.py build-tools --cache /path/to/study-cache
python docs/phenotype-study-2026-10-04/study.py replay --cache /path/to/study-cache
python docs/phenotype-study-2026-10-04/study.py report
```

For an existing replay, `reconcile --cache /path/to/study-cache` refreshes the
registered-unit census and prevalence report from the cached changed-scope run
records without rerunning the frozen detectors.

materialize writes the frozen agent and matched-human sample, skipped-candidate
audit and input manifest. replay builds klin and the three frozen prototypes
from study_commit, runs the registered measurements and frozen hard-negative
corpora, and writes measurements.tsv, findings.tsv,
hard-negative-results.tsv, blind packet JSON files and
prevalence-descriptive.md. The natural and hard-negative findings share one
blind packet namespace. The packet source map and coordinator salt are kept
separately from the packet files. Ruff is resolved as version 0.16.10 by the
pinned invocation in the protocol.

Each replay tree gets an isolated `klin.json` with no `accepted` entries.
Automatic checks keep their sections absent so the frozen binary derives its
normal scope; the derived acyclic `layering` section is added for the registered
module-cycle measurement.

The prevalence report is descriptive. It does not add blind labels or make a
product disposition. Rows with incomplete coverage remain visible and are not
counted as clean. Change rates are withheld when the registered change-level
eligibility census is incomplete. Site rates are withheld when the change
census, eligible-unit census or measurement is incomplete. For a complete
change census, unresolved measurements are shown as an observed-to-possible
interval.
