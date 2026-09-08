# One key vocabulary, and the differential test is retired

> Supersedes ADR 0007.

ADR 0007 matched cleat's config keys so that one config body could drive both
implementations and their outputs could be compared. ADR 0009 removed the
baseline file that most of that comparison rested on, and recorded that the
reason had weakened to a preference. ADR 0016 makes the config optional and
derives most sections, so a cleat-shaped key is now a constraint on a file
most users will never write.

The cost is visible in the two shipped ratcheting gates. Complexity reads
`sources`. Escapes reads `roots`. Both mean the same thing. A person who
writes one section learns nothing that helps with the next.

## The decision

Every section uses the same names for the same things: `roots`, `languages`,
`exclude`, `skip_dirs`, `ceilings`. `sources` in the complexity section
becomes `roots`. A run that meets an old name says what the new name is and
exits 2, so a config does not silently measure a different set.

The differential test against cleat is retired. Its three surviving cases
compared escapes and doc-size measurement against cleat's baseline at the base
commit. Those measurements are pinned by klin's own tests on fixtures, which
is the tier ADR 0001 already chose for complexity.

## Consequences

cleat stays the behavioural reference in AGENTS.md. Its tests and docstrings
say what a check judges. Its keys no longer say how a section is spelled.

`$CLEAT_SRC` is no longer needed to run the suite.

A divergence from cleat's behaviour, as opposed to its spelling, still takes
an ADR, because that is a decision about what the tool judges and not about
what a key is called.
