The note keeper prints one name per line and a caller wants machine-readable
output.

Give `list` a `--json` argument. `list --json` answers with a JSON array of
the names, as one line, with no spaces: `["a","b"]`, and `[]` when the keeper
holds nothing. A name holding a double quote or a backslash is escaped the way
JSON escapes it. `list` with no argument answers as it does today.

Cover the new behaviour with tests beside the ones already there.
