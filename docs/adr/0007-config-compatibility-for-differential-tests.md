# Config compatibility for differential tests

> Superseded by ADR 0018. Every section uses one key vocabulary, and the
> differential test is retired.

The differential test writes one config body and drives both implementations
from it, changing only the baseline path. That is what makes it evidence: the
two tools read the same keys, so a disagreement in their output is a
disagreement about behaviour rather than about configuration.

Fourteen gates are still to be written. Each one either matches the reference keys,
and the differential test can reach it, or it does not, and that gate is
verified against fixtures alone.

So the default is to match, and a divergence needs an ADR of its own. Matching
costs nothing while a section is being written for the first time. It buys the
strongest evidence available that the port is correct, and it means every
divergence is a decision somebody recorded rather than a drift nobody noticed.

Two divergences already exist and are not in question. ADR 0001 excludes the
complexity tier, because its numbers are independently defined. The
conventions gate adds a `structural` key beside `pattern`, which is
additive, so existing regex rules still run and still compare.

## Consequences

The section name `crap` is kept. It is the published name of the metric,
Change Risk Anti-Patterns, so a reader who sees a score can look up what it
means, and the differential test can cover the gate. Renaming it would buy
decorum and cost both.

Where the reference key design is poor, the price of improving it is an ADR and a
gate the differential test cannot reach. That is the intended friction. It is
not a prohibition.
