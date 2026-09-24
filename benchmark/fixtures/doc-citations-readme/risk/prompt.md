The evaluation engine should sit in its own folder.

Move the lexer, the parser, the evaluator and the error type into
`src/engine/`, keeping each file's name. `src/index.ts` stays the one entry a
caller imports, and nothing a caller can see may change.
