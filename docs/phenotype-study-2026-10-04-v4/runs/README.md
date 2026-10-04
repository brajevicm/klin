# Controlled-run records

This directory stores one immutable JSON record per #459 controlled run.

File name:

```text
<run_id>.json
```

Each record must validate against `../schema.json#/$defs/controlled_run` and preserve:

- the exact bound model id and host version;
- `session_seed` or `not-exposed`;
- final patch/tree metadata;
- every measured finding event;
- exact Active feedback text and SHA-256;
- project-check/oracle evidence;
- wall time, turns, feedback bytes and human escalations.

Shadow findings are measured but never surfaced: `shown=false`, with null feedback text/hash and no evidence-packet id.

No result file should be committed before the corresponding fresh session and independent verifier run have actually completed.


## Finalization order

During execution, preserve raw run evidence immediately, but do not finalize a
schema-valid `runs/<run_id>.json` for an Active run until the shared blind
packet namespace has assigned its `evidence_packet_id`.

That namespace is shared with #457 and the frozen hard-negative replay. The
packet id is therefore a post-collection join key, not something the agent sees
during the run. Preserve the pre-repair finding snapshot and exact feedback text
at delivery time; add only the blind packet id later. Never regenerate the
finding from the repaired tree.
