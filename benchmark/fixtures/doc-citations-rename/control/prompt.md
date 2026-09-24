Staff can sign in and cannot sign out.

Add `logout(service, token, now)` beside `login`, and export it from
`src/index.ts`. When the token reads as a user under the service's secret, it
writes `logout <id>` to the audit trail at `now` and returns `true`. Any other
token writes nothing and returns `false`.

Cover the new behaviour with tests beside the ones already there.
