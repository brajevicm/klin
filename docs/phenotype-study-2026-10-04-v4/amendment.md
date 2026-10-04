# Amendment: simulate #452 readiness in the research coordinator

**Timing:** committed before any #459 controlled outcome was opened or inspected.

## Broken v3 rule

Study v3 said REVIEW delivery follows #452's hidden readiness semantics and
named `klin __agent ready`. Verification against the frozen 0.4.2 source and
current main found that no `__agent` command exists.

That is expected from #452 itself: the issue is a completed **design** decision
and its deliverable explicitly says not to implement the rename/protocol there.

Literal execution is therefore impossible on the frozen product binary.

## v4 execution rule

For controlled research only, the coordinator supplies one explicit readiness
boundary after the agent's initial task attempt:

1. preserve the initial candidate tree;
2. measure shipped and research candidates against the frozen base;
3. for Shadow, record findings and show nothing;
4. for Active, construct the exact preregistered feedback and resume the **same
   host session** with that feedback;
5. preserve the repaired final tree;
6. run independent task oracle and residual measurement on verifier copies.

This is a simulation of #452's intended hidden readiness semantics, not a claim
that klin 0.4.2 exposes or ships that command. Stop-placed shipped findings use
the same post-attempt intervention boundary in the controlled harness so both
arms remain identical except for feedback visibility.

No task, model, host version, arm order, detector definition, repair wording,
oracle, project check, decision threshold, or 36-row ordering changes.

## Schema correction

Because study v3 has only one repetition, v4 also narrows
`controlled_run.repetition` from the inherited `[1,2]` schema allowance to
the literal value `1`. This rejects malformed evidence; it does not change any
planned run.
