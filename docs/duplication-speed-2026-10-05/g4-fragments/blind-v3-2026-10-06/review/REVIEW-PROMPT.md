# Independent blind review

Label every pair in `review-packet.json` independently. Do not search for, open, or ask about earlier labels, rule versions, thresholds, or results for these pairs. Do not discuss the packet with another reviewer before returning your labels.

Each pair has an id and two code locations. Each location includes its repository, path, language, line range, a link pinned to the introducing commit, and surrounding code. Start with the context. Open the link when it does not provide enough context.

Return exactly one label per pair:

- `copy`: repeated behavior that one shared helper could own without a contrived abstraction, such as an algorithm, validation, error handling, state management, caching, or query policy.
- `boilerplate`: conventional local scaffolding whose extraction would need configuration and would not remove duplicated domain behavior.
- `required-shape`: an obligation of a type, protocol, schema, route, API, adapter, enum, or other required declaration.
- `generated`: generated output, but only when a generator header or documented generation command supports that conclusion.
- `distinct`: the locations do not show two independent copies, such as overlapping regions of one declaration.
- `mixed`: inseparable substantive behavior and boilerplate or required-shape in the same region.

Judge each pair on its own. Use the context to name the repeated behavior in a short rationale. Do not infer that code is generated from its appearance alone. Use confidence `high`, `medium`, or `low`.

Return one JSON object keyed by all pair ids:

```json
{
  "<id>": {
    "label": "copy | boilerplate | required-shape | generated | distinct | mixed",
    "confidence": "high | medium | low",
    "rationale": "One to three sentences specific to this pair."
  }
}
```

Return raw JSON only, with no Markdown fences or text before or after it. Before returning, confirm that it has exactly 200 keys matching the packet ids and contains no other labels.
