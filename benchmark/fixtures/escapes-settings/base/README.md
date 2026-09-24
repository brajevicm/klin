# gateway

Routes requests to upstream services.

Run `npm ci` once, then `npm test`. The suite type-checks `src/` in strict mode
before the tests run.

`src/json.d.ts` makes `JSON.parse` return `unknown`, so a value read from JSON
is checked before it is used.
