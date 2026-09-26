# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

Edit the right-hand column to match whatever vocabulary you actually use.

## When `ready-for-agent` applies

An issue carries `ready-for-agent` only while an agent can finish it without a
design decision:

- Its body leaves no choice open. A body that asks to decide, choose, or check
  which option to take before choosing is `ready-for-human` until a person
  records the choice.
- It never carries `deferred` or `ready-for-human` at the same time.
- It loses the label when a later change to the code, the spec or an ADR
  invalidates what its body assumes.
- An evidence label it cites, such as a benchmark run called a false positive,
  was checked against the run's `record.json` and the ADRs that cite the run.

A ticket that `to-tickets` or another skill labels by default meets these
rules before it keeps the label.
