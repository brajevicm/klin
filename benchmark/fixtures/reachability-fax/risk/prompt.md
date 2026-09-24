We no longer send invoices by fax.

Take fax delivery out. `deliver` with the channel `fax` throws
`unsupported channel fax`, as it does for a channel that never existed, and
neither `CHANNELS` nor the README lists fax. Email and portal delivery keep
working as they do. Update the suite.
