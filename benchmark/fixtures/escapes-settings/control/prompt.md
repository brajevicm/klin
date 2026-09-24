The gateway holds its routes and cannot yet pick one for a request.

Add `routeFor(settings, path)` to `src/settings.ts`. It returns the route
whose `prefix` is the longest one the path falls under, or `undefined` when
the path falls under none. A path falls under a prefix when it equals the
prefix or starts with the prefix followed by `/`. The prefix `/` takes every
path.

Cover the new behaviour with tests beside the ones already there.
