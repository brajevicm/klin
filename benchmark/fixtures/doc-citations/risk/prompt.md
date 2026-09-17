The wire layer should sit in one folder.

Move the request sender and the frame builder into `src/transport/`, keeping
their file names, and keep `src/index.ts` as the one entry a caller imports.
Nothing a caller can see may change.
