# The vNext SPEC is the target, and the 0.x SPEC is the shipped contract

> Amends ADR 0025. Recorded for #493, 2026-10-06.

ADR 0025 names `docs/SPEC.md`, the accepted ADRs and the CLI tests as klin's
authority. #493 replaces `docs/SPEC.md` with the vNext core specification.
The shipped binary does not implement it yet, and its CLI tests pin the 0.x
behavior.

## The decision

**`docs/SPEC.md` is the vNext target contract. `docs/SPEC-0.x.md` is the
contract of the shipped binary until the vNext roadmap migrates each part.**

1. A change to shipped behavior that no vNext roadmap ticket covers follows
   `docs/SPEC-0.x.md`, the ADRs and the CLI tests, as ADR 0025 says.
2. A vNext roadmap ticket implements a section of `docs/SPEC.md`, changes the
   CLI tests to match, and marks the 0.x sections it replaced.
3. `Spec N.N` references in `src/` and in older ADRs name sections of
   `docs/SPEC-0.x.md`.
4. `docs/SPEC.md` section 0.3 lists the 0.x sections that vNext carries
   forward. Those sections stay normative for vNext until the last roadmap
   step folds them into `docs/SPEC.md` and retires `docs/SPEC-0.x.md`.

## Why

A full rewrite of the 0.x catalogue would delay the core contract that the
roadmap needs. A new file that stays beside the old one would leave two
documents that both claim to be `docs/SPEC.md`. One target and one frozen
shipped contract keep each claim true.
