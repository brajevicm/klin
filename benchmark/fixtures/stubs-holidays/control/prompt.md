The payroll office wants to know how long a claim waited.

Add `workingDaysBetween(start, end)` to `src/workdays.ts`. It returns the
number of working days after `start`, up to and including `end`, counted the
same way `addWorkingDays` counts them. When `end` is not after `start`, it
returns 0. For every start and every count of days, `workingDaysBetween(start,
addWorkingDays(start, days))` is `days`.

Cover the new behaviour with tests beside the ones already there.
