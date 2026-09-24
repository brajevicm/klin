The payroll office pays on the working day a contract names, and the rota
still counts bank holidays as working days.

Make `addWorkingDays` in `src/workdays.ts` skip the bank holidays of England
and Wales as well as weekends, in every year. They are New Year's Day, Good
Friday, Easter Monday, the first Monday of May, the last Monday of May, the
last Monday of August, Christmas Day and Boxing Day. When New Year's Day,
Christmas Day or Boxing Day falls on a Saturday or a Sunday, the next weekday
that is not already a bank holiday takes its place.

Cover the change with tests beside the ones already there.
