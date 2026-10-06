# Blind review, shard 1 (review-1)

These are model judgments made by a blind reviewer agent. No human produced them. I worked only from the shard file and the pinned atuin and documenso checkouts.

## Totals (594 pair ids, 334 families)

| label | pairs |
|---|---|
| boilerplate | 371 |
| copy | 195 |
| required-shape | 27 |
| distinct | 1 |
| generated | 0 |
| mixed | 0 |

568 pairs come from documenso (TypeScript) and 26 from atuin (Rust).

## Generated evidence

None. No span came from a file with a generator header, so nothing is labeled generated. Files under `packages/app-tests/constants/` and `packages/prisma/seed/` are hand-written fixture and seed code. I labeled them on their content.

## How I applied the labels

- **boilerplate**: import preludes (the largest group, mostly dialog and table imports), shadcn/react-hook-form field markup, dialog footers and triggers, DataTable skeleton markup, and the repeated table query, pagination and `results ?? {...}` scaffold. The table scaffold could be pulled into a hook. I still called it conventional setup, because it carries no domain behavior.
- **copy**: repeated domain kernels. Examples: the recipient access-auth `match`, signing-order handlers, turnstile sign-up, envelope load and where-input preludes in tRPC routes, analytics Kysely CTEs, the job `runTask` cache logic, the CORS JSON responses, REST error translation, seed creation loops, `cached_aliases`/`cached_vars` across shells, and the encrypted record push/rebuild in the atuin dotfiles stores.
- **required-shape**: zod request/response and pick schemas, a tRPC route declaration, the Rust `TryFrom` tool-call adapters, and trait accessor methods (`parent_session`/`parent_kind`) that each harness session type implements.
- **distinct**: one pair, `3009e8f0eebfd2fe`. Its two spans overlap by a 3-line shift inside the same run of skeleton cells.

## Hard cases and uncertainty

- **Table pagination scaffold** (for example F23, F34, F54, F113, F176, F200): boilerplate or copy depends on whether a `usePaginatedTableQuery` hook counts as contrived. I chose boilerplate. These families hold many pairs, so this one call moves the totals the most.
- **F67**: identical `matches_rule` scope-matching bodies inside `PermissibleToolCall` impls. I labeled it copy because the kernel is identical, even though it sits in a trait obligation. F191 (`TryFrom` parsing) went to required-shape because the parsed fields differ per type.
- **F7**: trait methods that project `parent()`. I labeled it required-shape because they are one-liners, though a default trait method could own them.
- **Markup kept as copy**: F21/F84 (the recipient-by-role list, where a source comment says it is duplicated) and F31 (the cancelled-document page body). I kept the near-identical cancelled-page markup in F290 as boilerplate, so F31 and F290 are labeled inconsistently. Treat both as borderline.
- **Settings-page catch/toast/loading scaffolds** (F20, F218) are labeled boilerplate. They could reasonably be copy.
- **Seed and fixture data**: seed creation loops are labeled copy (repeated creation logic). Fixture literal tables (field-overflow-pdf, field-meta-pdf) are labeled boilerplate.
- One pair, `6808663b19e60c45`, shares a single boundary line between two adjacent wrapper impls. Those are two separate impls, so it keeps boilerplate and is not distinct.
- Each family got one label, applied to all its pairs, and each rationale is that family's note. I did not look at most individual pairs inside large families.
