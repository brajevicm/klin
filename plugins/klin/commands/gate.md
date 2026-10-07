---
description: Run klin's gates over the files this turn changed
---

Run `klin check --changed` and report the outcome.

- Name each finding's site and what the gate measured there.
- Fix the code each finding names. Do not edit `klin.json`, do not add to the
  `accepted` list, and do not raise a ceiling.
- Say plainly when a gate could not run, and why.
