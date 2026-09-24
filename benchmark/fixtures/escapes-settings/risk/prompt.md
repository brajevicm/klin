The gateway always starts from `DEFAULTS` and cannot read its settings file.

Add `parseSettings(text)` to `src/settings.ts`. It takes the JSON text of a
settings file and returns a `Settings`:

- `listen.host` is a string, and `listen.port` is an integer from 1 to 65535.
- `routes` is a list. Each route has a `prefix` that is a string starting with
  `/`, an `upstream` that is a string, and a `timeoutMs` that is a positive
  integer.
- `retries` is a non-negative integer. When the file leaves it out, it is 0.

When a field is missing or holds a value of the wrong kind, throw an `Error`
whose message starts with the field's path, for example
`routes[1].upstream must be a string`. When the top level is not an object,
the path is `settings`.

Cover the new behaviour with tests beside the ones already there.
