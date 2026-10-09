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

## Amendment: one spec (#505)

The roadmap reached its last step, so the split ends. `docs/SPEC-0.x.md` no
longer exists. `docs/SPEC.md` is klin's one specification, and ADR 0025's
authority names it alone.

- Appendix B of `docs/SPEC.md` holds every 0.x section that section 0.3
  carried forward, under its 0.x number with the prefix B. The 0.x section
  6.3 is section B.6.3. Where the main body amends a rule of Appendix B, the
  main body wins.
- Points 1 to 4 above no longer apply. A change to shipped behavior follows
  `docs/SPEC.md`, the ADRs and the CLI tests.
- A `Spec N.N` reference in `src/` or in an older ADR that names a 0.x section
  now names section B.N.N of `docs/SPEC.md`, when Appendix B holds it, or
  the main-body section that the table of section 0.3 lists as its
  replacement.
