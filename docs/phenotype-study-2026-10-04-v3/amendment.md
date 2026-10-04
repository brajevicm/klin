# Amendment: one controlled repetition

**Timing:** committed before any #459 controlled outcome was opened or inspected.

Study v2 preregistered two repetitions for each task × agent-family pair. Study
v3 uses one repetition.

The purpose of the second repetition was stochastic replication. For #357's
product-admission decision, it doubles controlled-agent execution cost while the
primary causal unit remains the paired Active/Shadow comparison. Keeping one
pair for every task and both agent families preserves language/task diversity,
two independent model families, appeasement/harm observation, and the direct
counterfactual comparison.

Frozen quantities that do not change:

- 9 real tasks (3 Rust, 3 TypeScript, 3 Python);
- OpenAI Codex `gpt-6-luna` and Claude Code `claude-sonnet-5-5`;
- klin 0.4.2 runtime and executable hash;
- Active/Shadow semantics and deterministic arm ordering;
- candidate feedback construction and lifecycle placement;
- project checks, independent task oracles and residual measurement;
- phenotype registry, blind-label contract and #357 decision rules.

Resulting design: `9 × 2 × 1 × 2 = 36` runs, or 18 paired comparisons.
