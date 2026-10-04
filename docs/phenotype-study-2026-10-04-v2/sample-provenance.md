# Natural sample provenance

The sample is carried forward from the v1 materialization because the frozen v2 `populations.tsv`, `phenotypes.tsv`, AIDev revision, parquet hashes, and selection rules match the v1 inputs. The byte-identical [v1 selection audit](../phenotype-study-2026-10-04/selection-audit.tsv) records selection provenance without storing another 22,453-line copy. The preregistration protocol changes the executable baseline to klin 0.4.2; no measurement, finding, runtime, or prevalence row is carried forward.

The materialized sample has 40 natural-agent changes per language and 120 distinct agent repositories. It has 27 Rust, 31 TypeScript, and 34 Python matched-human changes; all 92 are distinct repositories and each is paired with a distinct agent change. The frozen AIDev revision is `c63c8a57a2de34fc03fa83722412824af4d8753b`; the parquet hashes are recorded in `study-inputs.json`.
