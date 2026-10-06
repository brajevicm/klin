# Region lineage requirements (#489)

Status: conservative research boundary, not a shipped ratchet or an accepted
replacement for #479. Design C's posting index cannot by itself supply these
certificates. This note answers the seven questions in #489 and identifies
which cases remain unresolved rather than inventing ancestry.

## Family and occurrence identity

A family requires the same **complete ordered normalized token sequence**,
language, length and measurement basis. Overlap is not equality. Do not take
the transitive union of overlapping matches: AB shared by one pair and BC
shared by another do not establish that either pair shares ABC. A fingerprint
is a compact equality hypothesis; blocking needs the separately selected
verification contract. Reporting can coalesce overlapping evidence, but must
not turn coalescing into additional or fewer certified origin bundles.

An occurrence is one physical interval in a file. Its family membership is
current content; its ancestry is a separate, optional certificate. Token
positions, diagonals, shared minimizers, line proximity and matching partner
identity do not certify ancestry across edits.

The narrow certificate available for future evaluation is a region exactly
aligned to one supported structural role (for example an entire function body)
inside a same-path uniquely identified enclosing declaration, using the same
admitted structural-identity version in both trees. The role must be unique on
both sides. This uses #479's same-file identity boundary; it does not introduce
name stripping or a below-T function detector. A body is eligible only when it
is an ordinary exact region of at least T tokens.

Arbitrary interior regions, changed edges, multiple overlapping regions in one
role, cross-file edits and competing descendants have no admitted certificate.
They stay unresolved. A shared 41-token minimizer is not a substitute for one.
This limits a possible ratchet, not the detector's ability to show candidates.

## Seven worked cases

| #489 question | Example | Required result |
|---|---|---|
| 1. Family identity | A and B share XY; B and C share YZ; X != Z | Keep XY and YZ separate. No invented XYZ family or pooled allowance. Overlapping reporting does not certify shared ancestry. |
| 2. Descendant identity | Same file and unique body role: A:H1 -> A:H2 | A certified role consumes one H1 slot and contributes H1 as an origin of H2. A token-offset shift alone is insufficient. An interior span with changed edges is unresolved. |
| 3. Extension | Old copied body S becomes S+E at the same two certified roles | Two H1 slots produce two certified descendants in H2: one inherited origin, zero regression. A third definitely-new S+E occurrence adds one regression. If E is outside the certified role, the extended-region ancestry is unresolved; the old family remains separately accountable. |
| 4. Split/merge | One legacy interval S produces two eligible fragments P,Q in new helpers | One old slot cannot certify two current occurrences. Reuse/splitting requires a separately admitted identity rule; leave fragment ancestry unresolved. Two independent certified body roles H1,H2 converging to H3 give two origins and one regression. A physical merge that destroys the roles is unresolved. |
| 5. Eligibility exit | A:H remains eligible, B:H becomes test/below-T, C:H is new | A proven B descendant consumes B's slot without joining the eligible group. C cannot spend B's allowance; one new regression. If B's ancestry is ambiguous, its slot is unresolved, never free. |
| 6. Stop computation | Changed body A has one match in a base cohort with 1,000 members | The ratchet needs cohort counts, touched slot certificates and provenance, not 1,000 pair edges. Existing positional postings prove neither cohort identity nor touched-slot states. Without the required persisted metadata, Stop is lineage-incomplete even if candidate matching is complete. |
| 7. Ambiguity | Removed A:H1 and edited B:H2 could both explain new C:H1 | Do not greedily spend A's slot. C is lineage-unknown until competing ancestry is resolved. If other certified origins already prove a regression, retain that lower-bound finding alongside INCOMPLETE. |

An unchanged occurrence is implicitly consumed. A same-fingerprint move may
conserve a free slot only after competing cross-fingerprint ancestry is ruled
out. A cross-file move plus edit is unresolved under the current identity
boundary. Fingerprint/basis changes require rederivation or INCOMPLETE.

## Required stored evidence

For an admitted family, retain the normalization/equality/identity versions,
language, verified content identity and token length, total base occurrence
count and deterministic representative. For changed paths, retain enough
per-occurrence structural role identity, range, family and eligibility state
to classify each old slot as consumed, free or unresolved. Eligible-only
fingerprints are insufficient: eligibility exits must retain their ancestry.

The current REG2 index stores k-gram keys and offsets plus capped-key flags.
It does not store complete family digests, counts by complete family, supported
role certificates or out-of-scope descendants. Therefore its measured cache
size is **candidate-index cost only**, not a lineage ratchet's cache cost.

The measured 1M index has about 390k selected fingerprints. A naive 16-byte
record per selected anchor would add about 6.2 MB; that is an illustrative
encoding cost, not a measured final lineage design. Persisting all T-token
windows is larger still. Existing structural declaration metadata might be
reused for the narrow body-role case, but content/group metadata still needs
an implementation and measurement. No claim that this fits the remaining
cache space is established.

## Disposition

One partner is sufficient for a candidate duplication witness, but not for
certifying origin bundles or conserving lineage slots. The safe current
placement is contextual check REVIEW. A Stop ratchet remains deferred until
arbitrary-region identity, overlapping-family accounting and a compact
persisted representation have an admitted contract and measured implementation.
This note does not close #489's implementation/feasibility questions.
