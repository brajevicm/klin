Slow upstreams hold the relay's connections open for too long, and the
timeout cannot be set.

Add a `timeout_ms` setting to the settings file, read as `timeoutMs` in the
settings `loadSettings` answers. It defaults to 30000. A value that is not a
whole number of at least 1 is refused with the error
`the timeout must be a whole number of at least 1`.

Cover the new behaviour with tests beside the ones already there.
