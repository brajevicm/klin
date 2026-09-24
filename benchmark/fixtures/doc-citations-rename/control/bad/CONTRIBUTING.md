# Contributing to sessions

This package decides who may sign in to the staff portal, so a mistake here
is a security incident rather than a bug. Please read this guide before your
first change.

## Getting started

You need Node.js 22 or later. There is nothing to install and nothing to
build: `npm test` runs the TypeScript sources directly.

Run the suite before you change anything, and again before you push. A red
suite on `main` blocks every other team's release, so it is fixed before
anything else is merged.

## Making a change

Open an issue first for a change that alters who can sign in, how long a
session lasts or what is written to the audit trail. Those are policy, and
the security team owns the policy.

Keep each pull request to one purpose, and describe in it what a person
signing in will notice. When they will notice nothing, say so.

Write tests that name the behaviour, not the function. "a sixth attempt
within a minute is limited" says what the portal promises. "allow returns
false" says only what the code happens to do today.

Never put a real password, token or secret in a test, a fixture or a commit
message, not even an expired one.

## Review

Every change needs one approval from the identity team. The files below need
a second approval from the security team as well, whatever the change:

- `src/passwordHash.ts`, because a change to how a password is hashed can lock
  every person out, or let everyone in;
- `src/sessionToken.ts`, because a change to how a token is signed can make
  every issued token readable as someone else's.

A change to the attempt limits in `rateLimit.ts` needs the security team to
agree to the new numbers, in the issue, before the pull request is opened.

## Releasing

The identity team releases on Mondays. The audit trail format is read by the
compliance export, so a change to a line that `auditLog.ts` writes is released
only after the compliance team confirms that their export reads it.

## Layout

The entry is `src/index.ts`, and it is the only file a caller imports.
`src/login.ts` holds the sign-in flow itself. Each step it takes is its own
file: stored users in `src/userStore.ts`, password hashing in
`src/passwordHash.ts`, attempt limits in `src/rateLimit.ts`, signed tokens in
`src/sessionToken.ts`, and the audit trail in `src/auditLog.ts`. Signing out is
in `src/signOut.ts`.
