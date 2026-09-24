The team now names files in kebab case.

Rename the camelCase files in `src/` to kebab case, so `userStore.ts` becomes
`user-store.ts`, and change nothing else about them. `src/index.ts` stays the
one entry a caller imports, and nothing a caller can see may change.
