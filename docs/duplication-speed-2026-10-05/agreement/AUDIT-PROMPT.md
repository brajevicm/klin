# Prompt: blind audit of 100 duplicated-code pairs

Give this prompt and the file `audit-packet.json` to the auditing agent.

---

You are auditing pairs of identical code regions. Your labels will be compared
with labels from other reviewers, so work independently. Do not search for,
open or ask about any earlier labels, rules or thresholds for these pairs.

## Input

`audit-packet.json` is a JSON list of 100 pairs. Each pair has an `id` and two
`locations`. Each location has:

- `repo`, `path`, `language`, `start_line`, `end_line`: the matched region;
- `url`: a GitHub link to the region at a pinned commit;
- `context`: the matched lines plus about 3 lines before and after them.

The two regions of a pair contain the same code after comments and whitespace
are removed. Identifiers are kept as written.

## Task

Give each pair exactly one label:

- **copy**: repeated behavior that one shared helper could own without a
  contrived abstraction. Examples: a logic kernel, an algorithm, validation,
  error handling, state management, a cache or query policy. A difference in
  the code around the region does not excuse an identical behavior kernel.
- **boilerplate**: conventional local scaffolding. Examples: import or
  declaration preludes, standard form-field, modal or JSX markup,
  conventional setup, derive or declaration syntax. Extracting it would need a
  configurable abstraction and would not remove duplicated domain behavior.
- **required-shape**: an obligation of a type, protocol or API. Examples: a
  trait or interface implementation that each type must provide, a schema,
  route or API declaration, a typed adapter, an enum or match table that a type
  requires.
- **generated**: output of a code generator. Use this label only with
  evidence, such as a generator header in the file or a documented generation
  command. Cite the evidence. The shape of the code is not evidence.
- **distinct**: the two locations do not show two independent copies, for
  example two overlapping parts of one declaration.

## Method

1. Judge each pair on its own. Do not label a group of pairs at once.
2. Read the `context` first. When the context does not decide the label, open
   the `url` and read the surrounding code.
3. Ask: if a developer saw these two regions in a code review, would the right
   request be "extract this into one place"? If yes, the label is copy. If the
   repetition is expected by the language, framework or type system, the label
   is boilerplate or required-shape.
4. Write a short rationale that names what is repeated.

## Output

Return one JSON object, keyed by pair id, with every one of the 100 ids:

```json
{
  "<id>": {
    "label": "copy | boilerplate | required-shape | generated | distinct",
    "confidence": "high | medium | low",
    "rationale": "1 to 3 sentences specific to this pair"
  }
}
```

Before you return it, check that it has exactly 100 keys, that the keys
equal the ids in the packet, and that every label is one of the five values.
After the JSON, give the count for each label and list the pairs that you
labeled with low confidence.
