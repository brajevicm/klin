| case | gate | desired | identity before | identity after | base whole | base changed | proto whole | proto changed |
|---|---|---|---|---|---|---|---|---|
| comment-above | complexity | held | 1 `a` | 3 `a` | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| sibling-insert | complexity | a held, b new | 1 `a` | 1 `b`; 5 `a` | FAIL, held 1; new src/a.rs:1 2 | FAIL, held 1; new src/a.rs:1 2 | FAIL, held 1; new src/a.rs:1 2 | FAIL, held 1; new src/a.rs:1 2 |
| sibling-reorder | complexity | held | 1 `a`; 5 `b` | 1 `b`; 6 `a` | ok, held 2 | ok, held 2 | ok, held 2 | ok, held 2 |
| body-edit | complexity | held | 1 `a` | 1 `a` | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| signature-only | complexity | held | 1 `a` | 1 `a` | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| signature-and-body | complexity | held | 1 `a` | 1 `a` | FAIL, held 0; new src/a.rs:1 2 | FAIL, held 0; new src/a.rs:1 2 | ok, held 1 | ok, held 1 |
| visibility-added | complexity | held | 1 `a` | 1 `a` | FAIL, held 0; new src/a.rs:1 2 | FAIL, held 0; new src/a.rs:1 2 | ok, held 1 | ok, held 1 |
| parameter-rename | complexity | held | 1 `a` | 1 `a` | FAIL, held 0; new src/a.rs:1 2 | FAIL, held 0; new src/a.rs:1 2 | ok, held 1 | ok, held 1 |
| parameter-rename-ts | complexity | held | 1 `a` | 1 `a` | FAIL, held 0; new src/a.ts:1 2 | FAIL, held 0; new src/a.ts:1 2 | ok, held 1 | ok, held 1 |
| parameter-rename-tsx | complexity | held | 1 `Row` | 1 `Row` | FAIL, held 0; new src/a.tsx:1 2 | FAIL, held 0; new src/a.tsx:1 2 | ok, held 1 | ok, held 1 |
| rename-same-body | complexity | held (body-hash proof) | 1 `a` | 1 `b` | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| rename-and-body | complexity | new | 1 `a` | 1 `b` | FAIL, held 0; new src/a.rs:1 2 | FAIL, held 0; new src/a.rs:1 2 | FAIL, held 0; new src/a.rs:1 2 | FAIL, held 0; new src/a.rs:1 2 |
| file-move | complexity | held | src/a.rs:1 `a` | src/b.rs:1 `a` | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| file-move-and-signature | complexity | new: the key holds the file, and the body changed | src/a.rs:1 `a` | src/b.rs:1 `a` | FAIL, held 0; new src/b.rs:1 2 | FAIL, held 0; new src/b.rs:1 2 | FAIL, held 0; new src/b.rs:1 2 | FAIL, held 0; new src/b.rs:1 2 |
| nested-signature | complexity | held | 1 `outer`; 2 `outer > inner` | 1 `outer`; 2 `outer > inner` | FAIL, held 1; new src/a.rs:2 2 | FAIL, held 1; new src/a.rs:2 2 | ok, held 2 | ok, held 2 |
| nested-same-name-two-parents | complexity | q > inner worsened 2 -> 4 | 1 `p`; 2 `p > inner`; 10 `q`; 11 `q > inner` | 1 `p`; 2 `p > inner`; 8 `q`; 9 `q > inner` | FAIL, held 3; worsened src/a.rs:8 1 (was 1) | FAIL, held 3; worsened src/a.rs:8 1 (was 1) | FAIL, held 2; worsened src/a.rs:8 1 (was 1); worsened src/a.rs:9 4 (was 2) | FAIL, held 2; worsened src/a.rs:8 1 (was 1); worsened src/a.rs:9 4 (was 2) |
| two-owners-swap-rust | complexity | B::run worsened 3 -> 5 | 4 `impl A > run`; 13 `impl B > run` | 4 `impl A > run`; 11 `impl B > run` | ok, held 2 | ok, held 2 | FAIL, held 1; worsened src/a.rs:11 5 (was 3) | FAIL, held 1; worsened src/a.rs:11 5 (was 3) |
| two-owners-misnamed-rust | complexity | B::run worsened 3 -> 5, A held | 4 `impl A > run`; 13 `impl B > run` | 4 `impl A > run`; 12 `impl B > run` | FAIL, held 1; worsened src/a.rs:4 4 (was 3) | FAIL, held 1; worsened src/a.rs:4 4 (was 3) | FAIL, held 1; worsened src/a.rs:12 5 (was 3) | FAIL, held 1; worsened src/a.rs:12 5 (was 3) |
| two-traits-swap-rust | complexity | A as Q::run worsened 3 -> 5 | 3 `impl A as P > run`; 12 `impl A as Q > run` | 3 `impl A as P > run`; 10 `impl A as Q > run` | ok, held 2 | ok, held 2 | FAIL, held 1; worsened src/a.rs:10 5 (was 3) | FAIL, held 1; worsened src/a.rs:10 5 (was 3) |
| two-owners-swap-python | complexity | B.__init__ worsened 3 -> 5 | 2 `A > __init__`; 13 `B > __init__` | 2 `A > __init__`; 9 `B > __init__` | ok, held 2 | ok, held 2 | FAIL, held 1; worsened src/a.py:9 5 (was 3) | FAIL, held 1; worsened src/a.py:9 5 (was 3) |
| two-owners-swap-ts | complexity | B.run worsened 3 -> 5 | 3 `A > run`; 13 `B > run` | 3 `A > run`; 11 `B > run` | ok, held 2 | ok, held 2 | FAIL, held 1; worsened src/a.ts:11 5 (was 3) | FAIL, held 1; worsened src/a.ts:11 5 (was 3) |
| owner-rename-same-body | complexity | held (body-hash proof) | 3 `impl A > run` | 3 `impl B > run` | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| owner-rename-and-body | complexity | today's outcome: neither key persists, so the text pairs them | 3 `impl A > run` | 3 `impl B > run` | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| method-moved-to-other-owner-and-edited | complexity | today's outcome: neither key persists, so the text pairs them | 4 `impl A > run` | 5 `impl B > run` | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| twin-crosses-ceiling | complexity | B::run new (it crossed the ceiling) | 4 `impl A > run`; 13 `impl B > run` | 4 `impl A > run`; 9 `impl B > run` | ok, held 1 | ok, held 1 | FAIL, held 0; new src/a.rs:9 5 | FAIL, held 0; new src/a.rs:9 5 |
| static-and-instance-ts | complexity | both held | 2 `A > static make`; 6 `A > make` | 2 `A > static make`; 6 `A > make` | FAIL, held 0; new src/a.ts:2 2; new src/a.ts:6 2 | FAIL, held 0; new src/a.ts:2 2; new src/a.ts:6 2 | ok, held 2 | ok, held 2 |
| getter-setter-ts | complexity | both held | 3 `A > get value`; 7 `A > set value` | 3 `A > get value`; 7 `A > set value` | FAIL, held 1; new src/a.ts:7 2 | FAIL, held 1; new src/a.ts:7 2 | ok, held 2 | ok, held 2 |
| property-setter-python | complexity | both held (ambiguous: today's outcome) | 3 `?duplicate`; 9 `?duplicate` | 3 `?duplicate`; 9 `?duplicate` | FAIL, held 1; new src/a.py:9 2 | FAIL, held 1; new src/a.py:9 2 | FAIL, held 1; new src/a.py:9 2 | FAIL, held 1; new src/a.py:9 2 |
| duplicate-key-signature | complexity | today's outcome (ambiguous) | 3 `?duplicate`; 8 `?duplicate` | 3 `?duplicate`; 8 `?duplicate` | FAIL, held 1; new src/a.rs:3 2 | FAIL, held 1; new src/a.rs:3 2 | FAIL, held 1; new src/a.rs:3 2 | FAIL, held 1; new src/a.rs:3 2 |
| anonymous-callback-ts | complexity | today's outcome (ambiguous) | 1 `?anonymous` | 1 `?anonymous` | FAIL, held 0; new src/a.ts:1 2 | FAIL, held 0; new src/a.ts:1 2 | FAIL, held 0; new src/a.ts:1 2 | FAIL, held 0; new src/a.ts:1 2 |
| computed-member-ts | complexity | today's outcome (ambiguous) | 3 `?computed` | 3 `?computed` | FAIL, held 0; new src/a.ts:3 2 | FAIL, held 0; new src/a.ts:3 2 | FAIL, held 0; new src/a.ts:3 2 | FAIL, held 0; new src/a.ts:3 2 |
| bound-arrow-ts | complexity | held | 1 `f` | 1 `f` | FAIL, held 0; new src/a.ts:1 2 | FAIL, held 0; new src/a.ts:1 2 | ok, held 1 | ok, held 1 |
| nested-under-callback-ts | complexity | today's outcome (ambiguous ancestry) | 1 `?anonymous`; 2 `?ancestry` | 1 `?anonymous`; 2 `?ancestry` | FAIL, held 1; new src/a.ts:2 2 | FAIL, held 1; new src/a.ts:2 2 | FAIL, held 1; new src/a.ts:2 2 | FAIL, held 1; new src/a.ts:2 2 |
| delete-add-replacement | complexity | b new | 1 `a` | 1 `b` | FAIL, held 0; new src/a.rs:1 3 | FAIL, held 0; new src/a.rs:1 3 | FAIL, held 0; new src/a.rs:1 3 | FAIL, held 0; new src/a.rs:1 3 |
| same-name-new-body | complexity | held (same site, values decide) | 1 `a` | 1 `a` | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
| added-duplicate-occurrence | complexity | one held, one new | 3 `impl F > new` | 3 `?duplicate`; 8 `?duplicate` | FAIL, held 1; new src/a.rs:8 2 | FAIL, held 1; new src/a.rs:8 2 | FAIL, held 1; new src/a.rs:8 2 | FAIL, held 1; new src/a.rs:8 2 |
| added-duplicate-occurrence-python | complexity | one held, one new | 1 `h` | 1 `?duplicate`; 7 `?duplicate` | FAIL, held 1; new src/a.py:7 2 | FAIL, held 1; new src/a.py:7 2 | FAIL, held 1; new src/a.py:7 2 | FAIL, held 1; new src/a.py:7 2 |
| accepted-param-rename | complexity | new, entry stale (a person edits the entry) | none | src/a.rs:1 `a` | FAIL, held 0, 1 note; new src/a.rs:1 2 | FAIL, held 0, 1 note; new src/a.rs:1 2 | FAIL, held 0, 1 note; new src/a.rs:1 2 | FAIL, held 0, 1 note; new src/a.rs:1 2 |
| accepted-and-base-param-rename | complexity | held, entry stale under --strict | 1 `a` | 1 `a` | FAIL, held 0, 1 note; new src/a.rs:1 2 | FAIL, held 0, 1 note; new src/a.rs:1 2 | FAIL, held 1, 1 note | ok, held 1, 1 note |
| accepted-and-base-unchanged | complexity | held by the accepted entry | 1 `a` | 2 `a` | ok, held 1, accepted 1 | ok, held 1, accepted 1 | ok, held 1, accepted 1 | ok, held 1, accepted 1 |
| dead-param-rename | dead-symbols | held | n/a | n/a | FAIL, held 0; new src/lib.rs:2 1 | FAIL, held 0; new src/lib.rs:2 1 | FAIL, held 0; new src/lib.rs:2 1 | FAIL, held 0; new src/lib.rs:2 1 |
| dead-signature-only | dead-symbols | held | n/a | n/a | FAIL, held 0; new src/lib.rs:2 1 | FAIL, held 0; new src/lib.rs:2 1 | FAIL, held 0; new src/lib.rs:2 1 | FAIL, held 0; new src/lib.rs:2 1 |
| dead-comment-above | dead-symbols | held | n/a | n/a | ok, held 1 | ok, held 1 | ok, held 1 | ok, held 1 |
