# Blind review, shard 2

These labels are model judgments made by one reviewer. No human produced them.

593 pair ids in 333 families. Every id has exactly one label. A Python check confirmed that the key set matches the shard's pair ids and that every label is one of the six allowed values.

## Per-label pair totals

| label | pairs |
|---|---|
| boilerplate | 309 |
| copy | 265 |
| required-shape | 19 |
| generated | 0 |
| distinct | 0 |
| mixed | 0 |

## Generated evidence

None. No file in the shard has a generator header, and I found no documented generation command. Nothing is labeled generated.

## How the labels were applied

- **boilerplate**: import preludes, which make up most of the documenso dialog, table and form pairs. Also standard JSX such as skeleton rows, FormField blocks, dialog footers, DataTable props and email template scaffolding; the conventional per-table query, pagination handler and default results object; clap/derive argument structs; struct field lists; thin per-codec wrappers in vendor/axoasset; and leaked test fixture data in packages/app-tests/constants.
- **copy**: repeated behavior that one helper could own. Examples: signing-field error handling with analytics capture; billing and permission preambles in tRPC routes; presign token authentication; Kysely access and search filters; AI client streaming and error code; envelope access lookups; seed creation sequences; the atuin daemon connect code; the duplicated DangerLevel/ConfidenceLevel parsing; the harness rehydrate and session logic; and the shell alias caching.
- **required-shape**: Zod request, response and schema declarations, where each API or form must declare its fields.

## Hard cases and uncertainty

- The pagination and query scaffolding in tables (F85, F90, F181, F190, F197, F283, F294, F302) is small, identical behavior across about 40 tables. I labeled it boilerplate because it is per-component wiring. A reviewer could call it copy.
- The debounced search in F31 and F227 is labeled copy, while the plain pagination scaffolding is boilerplate. The line between the two is a judgment call.
- Duplicated JSX components (the attachment popovers in F86, F103 and F130, recipient lists in F32, folder grids in F115 and F300, and the PDF viewer card in F228) are labeled copy because they are whole repeated components, not standard markup. Smaller markup repeats such as dropdown items, F151 and F164 are labeled boilerplate.
- Seed scripts (packages/prisma/seed) are labeled copy on content. They are development-only code.
- The FieldType dispatch tables in F166 are labeled copy rather than required-shape, because the same table is duplicated in two views. One type alone does not require it to exist twice.
- F15, F26, F205 and similar same-file repeats do not overlap, so none of them is labeled distinct.
- Test constants in packages/app-tests/constants (F72, F223, F293, F320) leaked through. They are labeled boilerplate on content because they are fixture data.
