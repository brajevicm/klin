# One key vocabulary, and the differential test is retired

> Supersedes ADR 0007.
> ADR 0025 supersedes the remaining external behavioural reference obligation.

ADR 0007 matched reference config keys so that one config body could drive both
implementations and their outputs could be compared. ADR 0009 removed the
baseline file that most of that comparison rested on, and recorded that the
reason had weakened to a preference. ADR 0016 makes the config optional and
derives most sections, so compatibility is now a constraint on a file
most users will never write.

The cost is visible in the two shipped ratcheting gates. Complexity reads
`sources`. Escapes reads `roots`. Both mean the same thing. A person who
writes one section learns nothing that helps with the next.

## The decision

Every section uses the same names for the same things: `roots`, `languages`,
`exclude`, `skip_dirs`, `ceilings`. `sources` in the complexity section
becomes `roots`. A run that meets an old name says what the new name is and
exits 2, so a config does not silently measure a different set.

The differential test is retired. Its three surviving cases
compared escapes and doc-size measurement against the reference baseline at the base
commit. Those measurements are pinned by klin's own tests on fixtures, which
is the tier ADR 0001 already chose for complexity.

## Consequences

The suite runs from this checkout alone. This decision originally retained an
external behavioural reference and required an ADR for behavioural divergence.
ADR 0025 retires that obligation; klin's specification, decisions and CLI tests
define what a check judges.
