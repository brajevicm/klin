# Study-v2 baseline reset

Study v1 is preserved unchanged at `docs/phenotype-study-2026-10-04/`.

Before any final-study outcome was inspected, the study owner selected released
klin 0.4.2/current `main` as the executable baseline.

| field | study v1 | study v2 |
| --- | --- | --- |
| klin version | 0.4.1 | 0.4.2 |
| study commit | `43a139a8a16964a65ceb97b939c3499c9cf90b4d` | `138dc8d0a927c60df289bd485627f472488cf2ba` |
| baseline branch | main | main |
| outcomes inspected before reset | no | n/a |

Git comparison from the v1 study commit to the v2 study commit changes no
`src/` files. The changed tree consists of the #456 preregistration artifacts
and release/version metadata (`Cargo.toml`, `Cargo.lock`, plugin manifests and
README version references). Therefore shipped detector implementation behavior
is unchanged, while executable/package identity is intentionally updated to
0.4.2.

This directory is a fresh preregistration version. It does not rewrite v1 and
does not reuse any v1 outcome rows. All final #457–#460 data intended for #357
must use `study_version = 2` and this directory's frozen basis.
