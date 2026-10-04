# Natural prevalence replay — study v2

This directory uses the v2 preregistration and the klin 0.4.2 baseline at
`138dc8d0a927c60df289bd485627f472488cf2ba`. The natural sample and selection
audit are the deterministic materialization described in `sample-provenance.md`.
No v1 measurement or finding rows are carried into v2.

Run with a persistent cache directory:

```sh
uv run --with duckdb -- python docs/phenotype-study-2026-10-04-v2/study.py materialize --cache /path/to/cache
python docs/phenotype-study-2026-10-04-v2/study.py build-tools --cache /path/to/cache
python docs/phenotype-study-2026-10-04-v2/study.py replay --cache /path/to/cache
python docs/phenotype-study-2026-10-04-v2/study.py reconcile --cache /path/to/cache
python docs/phenotype-study-2026-10-04-v2/study.py report
```

The script stores v2 replay/build output under `cache/study-v2/`; immutable
AIDev, GitHub, and bare-repository caches may be shared with the v1 run.
