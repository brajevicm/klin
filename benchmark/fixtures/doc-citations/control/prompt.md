The client retries with no wait between attempts.

Add `backoff(attempt)` to the wire layer and export it from the package entry.
It returns the milliseconds to wait before the attempt with that number.
Attempt 1 waits nothing. Attempt 2 waits 100. Each later attempt waits twice
the one before it, and no attempt waits more than 2000.

Cover the new behaviour with tests beside the ones already there.
