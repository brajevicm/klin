# Study-v4 amendment

Study v1 is preserved at `docs/phenotype-study-2026-10-04/`.
Study v2 is preserved at `docs/phenotype-study-2026-10-04-v2/`.
Study v3 is preserved at `docs/phenotype-study-2026-10-04-v3/`.

Study v4 keeps the exact study-v3 task population, 36-row run plan, model/host
bindings, project checks, phenotype definitions, decision rules, and klin 0.4.2
runtime:

- study commit: `138dc8d0a927c60df289bd485627f472488cf2ba`
- klin version: `0.4.2`
- release asset SHA-256: `6bca96b0f90bad16bac92c35d3a739ec02f5f915580d92161e35445773468b03`
- executable SHA-256: `e92d2cf1393b7da4d06eb762cd71b6e2787f365455a7c55cee9be2a3f974cd6e`

Before any #459 controlled outcome was inspected, execution review found that
the preregistered text referred to literal `klin __agent ready` execution.
Issue #452 is design-only and explicitly says not to implement the command; the
frozen 0.4.2 CLI and current main contain no such command.

Study v4 therefore freezes an external coordinator boundary that implements the
same readiness semantics for the experiment: measure after the initial task
attempt, surface exact frozen feedback only in Active, resume the same session,
and keep the same measurement hidden in Shadow.

The run ordering is unchanged from v3. All final study-v4 evidence uses
`study_version = 4`.
