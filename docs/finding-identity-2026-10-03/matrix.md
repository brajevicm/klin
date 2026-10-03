| case | gate | desired | base whole | base changed | proto whole | proto changed |
|---|---|---|---|---|---|---|
| comment-above | complexity | held | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| sibling-insert | complexity | a held, b new | FAIL, held 1; new src/a.rs:1 2 | FAIL, held 1; new src/a.rs:1 2 | FAIL, held 1; new src/a.rs:1 2 | FAIL, held 1; new src/a.rs:1 2 |
| sibling-reorder | complexity | held | ok, held 2 | ok, held 2 | ok, held 2 | ok, held 2 |
| body-edit | complexity | held | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| signature-only | complexity | held | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| signature-and-body | complexity | held | FAIL, held 0; new src/a.rs:1 2 | FAIL, held 0; new src/a.rs:1 2 | ok, held 1 | ok, held 1 |
| visibility-added | complexity | held | FAIL, held 0; new src/a.rs:1 2 | FAIL, held 0; new src/a.rs:1 2 | ok, held 1 | ok, held 1 |
| parameter-rename | complexity | held | FAIL, held 0; new src/a.rs:1 2 | FAIL, held 0; new src/a.rs:1 2 | ok, held 1 | ok, held 1 |
| parameter-rename-ts | complexity | held | FAIL, held 0; new src/a.ts:1 2 | FAIL, held 0; new src/a.ts:1 2 | ok, held 1 | ok, held 1 |
| rename-same-body | complexity | held (body-hash proof) | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| rename-and-body | complexity | new | FAIL, held 0; new src/a.rs:1 2 | FAIL, held 0; new src/a.rs:1 2 | FAIL, held 0; new src/a.rs:1 2 | FAIL, held 0; new src/a.rs:1 2 |
| file-move | complexity | held | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| file-move-and-signature | complexity | new: the key holds the file, and the body changed | FAIL, held 0; new src/b.rs:1 2 | FAIL, held 0; new src/b.rs:1 2 | FAIL, held 0; new src/b.rs:1 2 | FAIL, held 0; new src/b.rs:1 2 |
| nested-signature | complexity | held | FAIL, held 1; new src/a.rs:2 2 | FAIL, held 1; new src/a.rs:2 2 | ok, held 2 | ok, held 2 |
| nested-same-name-two-parents | complexity | q > inner worsened 2 -> 4 | FAIL, held 3; worsened src/a.rs:8 1 (was 1) | FAIL, held 3; worsened src/a.rs:8 1 (was 1) | FAIL, held 2; worsened src/a.rs:8 1 (was 1); worsened src/a.rs:9 4 (was 2) | FAIL, held 2; worsened src/a.rs:8 1 (was 1); worsened src/a.rs:9 4 (was 2) |
| two-owners-swap-rust | complexity | B::run worsened 3 -> 5 | ok, held 2 | ok, held 2 | FAIL, held 1; worsened src/a.rs:11 5 (was 3) | FAIL, held 1; worsened src/a.rs:11 5 (was 3) |
| two-owners-misnamed-rust | complexity | B::run worsened 3 -> 5, A held | FAIL, held 1; worsened src/a.rs:4 4 (was 3) | FAIL, held 1; worsened src/a.rs:4 4 (was 3) | FAIL, held 1; worsened src/a.rs:12 5 (was 3) | FAIL, held 1; worsened src/a.rs:12 5 (was 3) |
| two-traits-swap-rust | complexity | A as Q::run worsened 3 -> 5 | ok, held 2 | ok, held 2 | FAIL, held 1; worsened src/a.rs:10 5 (was 3) | FAIL, held 1; worsened src/a.rs:10 5 (was 3) |
| two-owners-swap-python | complexity | B.__init__ worsened 3 -> 5 | ok, held 2 | ok, held 2 | FAIL, held 1; worsened src/a.py:9 5 (was 3) | FAIL, held 1; worsened src/a.py:9 5 (was 3) |
| two-owners-swap-ts | complexity | B.run worsened 3 -> 5 | ok, held 2 | ok, held 2 | FAIL, held 1; worsened src/a.ts:11 5 (was 3) | FAIL, held 1; worsened src/a.ts:11 5 (was 3) |
| owner-rename-same-body | complexity | held (body-hash proof) | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| owner-rename-and-body | complexity | today's outcome: neither key persists, so the text pairs them | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| method-moved-to-other-owner-and-edited | complexity | today's outcome: neither key persists, so the text pairs them | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| twin-crosses-ceiling | complexity | B::run new (it crossed the ceiling) | ok, held 1 | ok, held 1 | FAIL, held 0; new src/a.rs:9 5 | FAIL, held 0; new src/a.rs:9 5 |
| static-and-instance-ts | complexity | both held | FAIL, held 0; new src/a.ts:2 2; new src/a.ts:6 2 | FAIL, held 0; new src/a.ts:2 2; new src/a.ts:6 2 | ok, held 2 | ok, held 2 |
| getter-setter-ts | complexity | both held | FAIL, held 1; new src/a.ts:7 2 | FAIL, held 1; new src/a.ts:7 2 | ok, held 2 | ok, held 2 |
| property-setter-python | complexity | both held (ambiguous: today's outcome) | FAIL, held 1; new src/a.py:9 2 | FAIL, held 1; new src/a.py:9 2 | FAIL, held 1; new src/a.py:9 2 | FAIL, held 1; new src/a.py:9 2 |
| duplicate-key-signature | complexity | today's outcome (ambiguous) | FAIL, held 1; new src/a.rs:3 2 | FAIL, held 1; new src/a.rs:3 2 | FAIL, held 1; new src/a.rs:3 2 | FAIL, held 1; new src/a.rs:3 2 |
| anonymous-callback-ts | complexity | today's outcome (ambiguous) | FAIL, held 0; new src/a.ts:1 2 | FAIL, held 0; new src/a.ts:1 2 | FAIL, held 0; new src/a.ts:1 2 | FAIL, held 0; new src/a.ts:1 2 |
| computed-member-ts | complexity | today's outcome (ambiguous) | FAIL, held 0; new src/a.ts:3 2 | FAIL, held 0; new src/a.ts:3 2 | FAIL, held 0; new src/a.ts:3 2 | FAIL, held 0; new src/a.ts:3 2 |
| bound-arrow-ts | complexity | held | FAIL, held 0; new src/a.ts:1 2 | FAIL, held 0; new src/a.ts:1 2 | ok, held 1 | ok, held 1 |
| nested-under-callback-ts | complexity | today's outcome (ambiguous ancestry) | FAIL, held 1; new src/a.ts:2 2 | FAIL, held 1; new src/a.ts:2 2 | FAIL, held 1; new src/a.ts:2 2 | FAIL, held 1; new src/a.ts:2 2 |
| delete-add-replacement | complexity | b new | FAIL, held 0; new src/a.rs:1 3 | FAIL, held 0; new src/a.rs:1 3 | FAIL, held 0; new src/a.rs:1 3 | FAIL, held 0; new src/a.rs:1 3 |
| same-name-new-body | complexity | held (same site, values decide) | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| added-duplicate-occurrence | complexity | one held, one new | FAIL, held 1; new src/a.rs:8 2 | FAIL, held 1; new src/a.rs:8 2 | FAIL, held 1; new src/a.rs:8 2 | FAIL, held 1; new src/a.rs:8 2 |
| added-duplicate-occurrence-python | complexity | one held, one new | FAIL, held 1; new src/a.py:7 2 | FAIL, held 1; new src/a.py:7 2 | FAIL, held 1; new src/a.py:7 2 | FAIL, held 1; new src/a.py:7 2 |
| accepted-param-rename | complexity | new, entry stale (a person edits the entry) | FAIL, held 0, 1 note; new src/a.rs:1 2 | FAIL, held 0, 1 note; new src/a.rs:1 2 | FAIL, held 0, 1 note; new src/a.rs:1 2 | FAIL, held 0, 1 note; new src/a.rs:1 2 |
| accepted-and-base-param-rename | complexity | held, entry stale under --strict | FAIL, held 0, 1 note; new src/a.rs:1 2 | FAIL, held 0, 1 note; new src/a.rs:1 2 | FAIL, held 1, 1 note | ok, held 1, 1 note |
| accepted-and-base-unchanged | complexity | held by the accepted entry | ok, held 1, accepted 1 | ok, held 1, accepted 1 | ok, held 1, accepted 1 | ok, held 1, accepted 1 |
| dead-param-rename | dead-symbols | held | FAIL, held 0; new src/lib.rs:2 1 | FAIL, held 0; new src/lib.rs:2 1 | FAIL, held 0; new src/lib.rs:2 1 | FAIL, held 0; new src/lib.rs:2 1 |
| dead-signature-only | dead-symbols | held | FAIL, held 0; new src/lib.rs:2 1 | FAIL, held 0; new src/lib.rs:2 1 | FAIL, held 0; new src/lib.rs:2 1 | FAIL, held 0; new src/lib.rs:2 1 |
| dead-comment-above | dead-symbols | held | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
