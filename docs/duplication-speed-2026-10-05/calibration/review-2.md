# Blind calibration review 2

These are agent-assigned review labels, not human labels or independent human precision evidence. I reviewed all 169 assigned families, all 506 span records, and every one of the 588 pair locations in `blind-2.json`. Every pair has a label, contextual rationale and pinned-source citations in `labels.json`.

The review used each family's complete first source-context span, then complete per-span diffs against it (unchanged text was read once), with each span's location and every pair's membership inspected. Additional pinned source was consulted for generation provenance and contextual boundaries. I did not read `mapping.json`, threshold values or token lengths. Families are presentation groups: the decision was checked against both locations for each pair. Within these assigned families the contextual classifications happened to be consistent; that is a review outcome, not an assumption that equality requires identical labels.

Source snapshots supplied for this review:

- klin: `c1805539c4d2743975915ab8452d731bac47e207`, under `src/`.
- GlareDB: `8001afa4cff0cecfec79f2cb292108da08aa4c35`.
- karakeep: `f8ae986692f82efe8c1f3940907aab553e4f5a49`.

The source trees are extracted snapshots without Git metadata; the supplied SHAs identify the citations, rather than a locally verified checkout HEAD.

## Totals

| Label | Pairs | Families |
|---|---:|---:|
| copy | 41 | 32 |
| boilerplate | 394 | 78 |
| required-shape | 82 | 29 |
| generated | 59 | 18 |
| distinct | 12 | 12 |
| mixed | 0 | 0 |

## Evidence and judgment boundaries

Generated labels require source evidence. The [Parquet format header](https://github.com/glaredb/glaredb/blob/8001afa4cff0cecfec79f2cb292108da08aa4c35/crates/ext_parquet/src/format.rs#L1-L3) explicitly identifies Thrift Compiler output. API sidebar labels are supported by the [gen-api command](https://github.com/karakeep-app/karakeep/blob/f8ae986692f82efe8c1f3940907aab553e4f5a49/docs/package.json#L16) and [OpenAPI output configuration](https://github.com/karakeep-app/karakeep/blob/f8ae986692f82efe8c1f3940907aab553e4f5a49/docs/docusaurus.config.ts#L60-L73); historical sidebars retain generated endpoint navigation in version snapshots. Generated does not mean any handwritten source merely located in a generator-related crate: the TPC-H table adapters were reviewed as handwritten adapter scaffolding.

Repeated imports, short adapter constructors and registrations are boilerplate. Their repetition does not imply a request to share an algorithm. In particular, the large numeric-function family already imports common UnaryInputNumericScalar machinery: [cos setup](https://github.com/glaredb/glaredb/blob/8001afa4cff0cecfec79f2cb292108da08aa4c35/crates/glaredb_core/src/functions/scalar/builtin/numeric/cos.rs#L1-L21). Stateless scalar bind adapters likewise precede different unary/binary operations.

Required-shape labels apply to interface signatures, schema/protocol field mappings, expression-free trait visitors and thin delegates to existing shared kernels. This category records why a reviewer would not require extraction for the matched span; it does not claim the language technically forbids any abstraction. Representative [expression-free LogicalNode adapters](https://github.com/glaredb/glaredb/blob/8001afa4cff0cecfec79f2cb292108da08aa4c35/crates/glaredb_core/src/logical/logical_join.rs#L241-L257) and [trim delegates](https://github.com/glaredb/glaredb/blob/8001afa4cff0cecfec79f2cb292108da08aa4c35/crates/glaredb_core/src/functions/scalar/builtin/string/trim.rs#L153-L167) already express the appropriate common implementation boundaries.

Distinct labels cover intentionally separate array representations and optimized all-valid/nullable paths where the equal portion is incidental to separate kernels. Representative [binary execution](https://github.com/glaredb/glaredb/blob/8001afa4cff0cecfec79f2cb292108da08aa4c35/crates/glaredb_core/src/arrays/executor/scalar/binary.rs#L131-L143) and [row matching](https://github.com/glaredb/glaredb/blob/8001afa4cff0cecfec79f2cb292108da08aa4c35/crates/glaredb_core/src/arrays/row/row_matcher.rs#L293-L310) require storage/lifetime context before proposing reuse.

Copy labels identify a concrete candidate for centralizing implementation, not proof of historical copying. Examples include [duplicated async read loop](https://github.com/glaredb/glaredb/blob/8001afa4cff0cecfec79f2cb292108da08aa4c35/crates/rayexec_io/src/future/read_into.rs#L58-L84), [S3 signing helpers](https://github.com/glaredb/glaredb/blob/8001afa4cff0cecfec79f2cb292108da08aa4c35/crates/rayexec_io/src/s3/credentials.rs#L154-L226), [list scan implementation](https://github.com/glaredb/glaredb/blob/8001afa4cff0cecfec79f2cb292108da08aa4c35/crates/glaredb_core/src/functions/table/builtin/list_entries.rs#L294-L377), [rule error presentation](https://github.com/karakeep-app/karakeep/blob/f8ae986692f82efe8c1f3940907aab553e4f5a49/apps/web/components/dashboard/rules/RuleEngineRuleEditor.tsx#L67-L95) and [list query refinements](https://github.com/karakeep-app/karakeep/blob/f8ae986692f82efe8c1f3940907aab553e4f5a49/packages/shared/types/lists.ts#L76-L91). Each corresponding pair cites both occurrences in the JSON.

The UI primitive pairs are labeled boilerplate, not generated: the [shadcn configuration](https://github.com/karakeep-app/karakeep/blob/f8ae986692f82efe8c1f3940907aab553e4f5a49/apps/web/components.json) and the inspected wrappers support conventional locally owned primitives, but do not establish automated generation of each file.

## Uncertainty

The labels apply the issue's actionable-review meaning of copy. They are judgment calls about extraction value, not forensic claims that one author copied another. Borderline cases include bitwise aggregate validity handling (copy), stateless arithmetic/bind wrappers (boilerplate), per-app UI primitives (boilerplate), vtable downcast closures (copy), and direct/flat COUNT loops (copy because the complete validity update can share a helper after validity acquisition). Optimized array executors are distinct where ownership/selection differences shape the loop. A maintainer could reasonably dispute these extraction boundaries; those disputes require contextual relabeling before treating this agent set as calibrated blocker precision. No pair was left unreviewed or assigned mixed merely to avoid making a contextual judgment.

The assigned set contains no mixed labels after review. That does not mean uncertainty is zero: the distinction between conventional adapters and actionable reuse is the principal uncertainty. These labels should not be presented as blind human labels or sufficient evidence for a zero-false-block product claim.
