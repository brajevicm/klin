# Shard 0 blind review

These are model judgments, not human labels. One reviewer read only the shard input and the pinned checkouts (atuin d5c6bee0, documenso cd0cc5fe).

## Totals (594 pair ids, 330 families)

| label | pairs |
|---|---|
| copy | 323 |
| boilerplate | 246 |
| required-shape | 25 |
| generated | 0 |
| distinct | 0 |
| mixed | 0 |

One family (the openpage-api stats routes: same CORS JSON response, cache headers and OPTIONS handler) accounts for 89 copy pairs, plus 5 more in a sibling family. Without those two families, copy is 229 pairs.

## Generated evidence

None. I found no generator headers or documented generation commands in any matched file. Vendored axoasset code (`vendor/axoasset`) is vendored, not generated, and I labeled it on content (boilerplate).

## How I applied the labels

- copy: repeated behaviour that one helper could own. Examples: signing-field error and remove handling, envelope access lookups, the field placement and resize handlers duplicated across template, document and embed editors, seed loops, email render-and-send, harness session scanning and checkpoint code, MessagePack record deserialisation, and the daemon client connect code.
- boilerplate: import preludes, react-hook-form field markup, dialog trigger and reset scaffolding, DataTable props and skeletons, paginated-query scaffolding, React Email layout, API example scripts, clap and derive declarations.
- required-shape: Zod request/response schema declarations, tRPC route declarations, and Rust trait impls (serde Deserialize, TryFrom, PermissibleToolCall, FtsQueryExt, harness locate/rehydrate).

## Hard cases and uncertainty

- Error-toast blocks (for example the admin user enable/disable/delete dialogs): the shape is shared but the messages differ per action. I called these boilerplate. The signing-field components were different: there the error kernel (UNAUTHORIZED rethrow, analytics capture, toast) is identical, so I called them copy.
- UI duplication beyond standard markup: the folder page body and breadcrumbs, the dragged-field preview, the tooltip portal, and the role-grouped recipient list. I labeled these copy because they duplicate an existing component (FolderGrid, the use-field-page-coords hook) or a specific widget. FieldItem prop wiring and the settings form sections stayed boilerplate. This is the line I'm least sure of.
- openpage-api routes: OPTIONS is a per-route obligation, but the duplicated JSON/CORS/cache response builder is behaviour, so I labeled the family copy. If you count it as required-shape, the copy total drops sharply.
- `csc/credential.ts`: an inline parameter type restates the exported `CscCredentialRow` type. I labeled it copy, though it's a type-level duplicate rather than behaviour.
- `rmp/decode.rs`: two different helpers share only their generic signature and where-clause. I labeled this boilerplate; distinct would also be defensible.
- Seed and sample-data files: I labeled the repeated seed creation loops copy. Repeated fixture literals (webhook sample data, app-tests field-meta constants) I labeled boilerplate.
- Shell-specific code in atuin (alias caching, ATUIN_NOBIND handling): labeled copy because the kernel is the same despite the platform differences.
