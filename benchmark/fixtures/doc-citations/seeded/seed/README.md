# transport

The wire layer. Run the suite with `npm test`.

## Layout

The package entry is `src/index.ts`, and it is the only file a caller imports.

Request sending lives in `src/client.ts`. It sends one request and retries
while the server says it is busy.

Frame building lives in `src/socket.ts`. A frame carries a kind and a
payload.

The delay between two attempts is worked out in `src/retry.ts`, which the
rewrite left behind and which the client now computes inline.

Further details live in `src/absent.ts`.
