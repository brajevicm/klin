# Review shard 3: blind duplication labels

These are model judgments, not human labels. The reviewer read only the shard and the pinned atuin/documenso checkouts.

## Totals (593 pair ids, 333 families)

| label | pairs |
|---|---|
| boilerplate | 340 |
| copy | 202 |
| required-shape | 50 |
| distinct | 1 |
| generated | 0 |
| mixed | 0 |

## Generated evidence
None. No family carried a generator header or a documented generation command, so no pair is labelled generated.

## How labels were assigned
- boilerplate: import preludes for dialogs, forms and tables; shadcn form-field, dialog footer and DataTable skeleton JSX; table pagination setup; email template layout; webhook sample-payload literals; test fixture constants under packages/app-tests (test code that leaked through, labelled on its content).
- copy: repeated server logic, including prisma include/where policy, billing and permission checks, two-factor route handlers duplicated in two-factor.ts and email-password.ts, email render-and-send, the field sign/remove error handling across the document-signing-*-field components, kysely monthly-growth queries, seed helpers, the atuin axoasset load/compress error paths, the shell alias cache, the daemon connect path and the hub mutate response handling.
- required-shape: zod request/response schemas and `.pick` response shapes, Rust trait impls (Capability, MatchRenderer, from_row, harness session/resume methods), and tRPC route declarations.
- distinct: admin-document-jobs-table.tsx lines 102-115 and 105-118 overlap inside one skeleton block.

## Hard cases and uncertainty
- Signing-field hook setup (F8, F29): labelled boilerplate (mutation declarations). The handlers after it are labelled copy.
- Tri-state Select markup with the true/false/inherit mapping (F80, F138, F165, F211): labelled boilerplate, though the value mapping is a small behaviour.
- Embed schemas with the email transform (F117, F160) and the template settings form schema (F262): the first two are labelled required-shape and F262 copy. This is a borderline call.
- atuin daemon `from_settings` cfg pairs (F110, F266): labelled boilerplate. The larger connection block (F250) is labelled copy.
- Large page JSX (webhook form F157, branding page F279): labelled boilerplate. Someone could argue these are copies of a whole component.
- Seed and sample-data code: seed helpers are labelled copy, webhook sample payload literals boilerplate.
- Rationales are written per family from a template. Pairs in one family share the family label.
