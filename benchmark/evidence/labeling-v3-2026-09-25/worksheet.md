# Blinded signal worksheet

This worksheet is prepared for human labeling. It contains no arm, round, repetition, trial, delivery, or post-signal outcome fields.
Label each row exactly one of `valid-regression`, `valid-review`, or `undesired`.

- Included records: 14
- Signal occurrences: 24
- Distinct worksheet rows: 13

## S001

- Signal category: regression
- Gate: complexity
- Site: `src/lib.rs:11` — `pub fn read_rows(name: &str, text: &str) -> Result<Vec<Vec<String>>, Refusal> {`
- Measured values: `{"cc":11,"lines":27}`

### Task intent

> Customers of the German office upload exports from their accounting package,
> and the upload form refuses them.
>
> Those files end in `.ssv`. Their fields are separated by semicolons, and each
> field is trimmed the same way as a comma field. A decimal comma inside a field
> stays as it is, so `1,50 ; Kaffee` reads as the two fields `1,50` and
> `Kaffee`. Blank lines and comment lines hold no row, as in every other file.
>
> Add the new suffix to `read_rows` in `src/lib.rs` and cover it with tests.
>

### Signal-time context

Signal-time tree excerpt from `src/lib.rs`:

```text
0009 |
0010 | /// The rows of one uploaded file, each a list of its fields, read by the file's name.
0011 | pub fn read_rows(name: &str, text: &str) -> Result<Vec<Vec<String>>, Refusal> {
0012 |     let lower = name.to_ascii_lowercase();
0013 |     let mut rows = Vec::new();
0014 |     for line in text.lines() {
0015 |         if line.trim().is_empty() || line.starts_with('#') {
```

Signal-time measurement:

> src/lib.rs:11  cc 11, 27 lines, was cc 10, 25 lines  pub fn read_rows(name: &str, text: &str) -> Result<Vec<Vec<String>>, R  — matched the base site at src/lib.rs:11, ceiling cc 8, lines 60

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S002

- Signal category: regression
- Gate: complexity
- Site: `src/quote.ts:18` — `export function computeQuote(`
- Measured values: `{"cc":11,"lines":35}`

### Task intent

> The sales desk has a new customer class.
>
> A `student` customer takes 20 percent off. It stacks with the order-size rule
> and with the regional rule the same way the trade and wholesale classes do,
> and the 30 percent ceiling still holds afterwards. Tax is worked out on the
> discounted amount, as it is today.
>
> Add the class to `src/quote.ts` and cover it with tests.
>

### Signal-time context

Signal-time tree excerpt from `src/quote.ts`:

```text
0016 |
0017 | /** The quote for one line of an order. */
0018 | export function computeQuote(
0019 |   unitPrice: number,
0020 |   units: number,
0021 |   customer: Customer,
0022 |   region: string,
```

Signal-time measurement:

> src/quote.ts:18  cc 11, 35 lines, was cc 10, 33 lines  export function computeQuote(  — matched the base site at src/quote.ts:18, ceiling cc 8, lines 60

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S003

- Signal category: regression
- Gate: complexity
- Site: `src/rate.ts:20` — `export function shippingRate(parcel: Parcel): number {`
- Measured values: `{"cc":12,"lines":36}`

### Task intent

> The parcel counter now sends to the Channel Islands and the Canaries.
>
> Add a zone called `islands` to `src/rate.ts`. The first kilogram costs 12
> euros and each started kilogram after it costs 2.40. The heavy-parcel
> surcharge and the fuel surcharge apply the same way as for every zone outside
> the country. The carrier offers no express service to the islands, so an
> express parcel to that zone is refused like an unknown zone.
>
> Cover the new zone with tests beside the ones already there.
>

### Signal-time context

Signal-time tree excerpt from `src/rate.ts`:

```text
0018 |
0019 | /** The price of sending one parcel, in euros. */
0020 | export function shippingRate(parcel: Parcel): number {
0021 |   if (!(parcel.weightKg > 0) || parcel.weightKg > MAX_KG) {
0022 |     throw new Error("a parcel weighs more than 0 and at most " + MAX_KG + " kg");
0023 |   }
0024 |   let first = 0;
```

Signal-time measurement:

> src/rate.ts:20  cc 12, 36 lines, was cc 10, 30 lines  export function shippingRate(parcel: Parcel): number {  — matched the base site at src/rate.ts:20, ceiling cc 8, lines 60

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S004

- Signal category: regression
- Gate: complexity
- Site: `src/rate.ts:20` — `export function shippingRate(parcel: Parcel): number {`
- Measured values: `{"cc":13,"lines":36}`

### Task intent

> The parcel counter now sends to the Channel Islands and the Canaries.
>
> Add a zone called `islands` to `src/rate.ts`. The first kilogram costs 12
> euros and each started kilogram after it costs 2.40. The heavy-parcel
> surcharge and the fuel surcharge apply the same way as for every zone outside
> the country. The carrier offers no express service to the islands, so an
> express parcel to that zone is refused like an unknown zone.
>
> Cover the new zone with tests beside the ones already there.
>

### Signal-time context

Signal-time tree excerpt from `src/rate.ts`:

```text
0018 |
0019 | /** The price of sending one parcel, in euros. */
0020 | export function shippingRate(parcel: Parcel): number {
0021 |   if (!(parcel.weightKg > 0) || parcel.weightKg > MAX_KG) {
0022 |     throw new Error("a parcel weighs more than 0 and at most " + MAX_KG + " kg");
0023 |   }
0024 |   let first = 0;
```

Signal-time measurement:

> src/rate.ts:20  cc 13, 36 lines, was cc 10, 30 lines  export function shippingRate(parcel: Parcel): number {  — matched the base site at src/rate.ts:20, ceiling cc 8, lines 60

### Proposed remedy or review question

> Reduce the function's responsibility or decision complexity. Split at coherent behavior boundaries, not into arbitrary helpers that only get under the gate. Accepting new debt is a policy decision for a person, in the config, in a reviewed commit.

## S005

- Signal category: regression
- Gate: doc-citations
- Site: `CONTRIBUTING.md:98` — `src/totals.ts`
- Measured values: `{"count":1,"resolution":"not under the roots — likely src/money/totals.ts"}`

### Task intent

> The money code should sit in one folder.
>
> Move the rounding, currency, tax, discount and totals code into `src/money/`,
> keeping each file's name. `src/index.ts` stays the one entry a caller
> imports, and nothing a caller can see may change.
>

### Signal-time context

Signal-time tree excerpt from `CONTRIBUTING.md`:

```text
0096 | - The printed form of an invoice is built in `src/invoice.ts`.
0097 | - Line totals, their tax and the invoice total are added up in
0098 |   `src/totals.ts`.
0099 | - Tax bands and their rates are in `src/tax.ts`. A new band starts there, and
0100 |   it needs a review from finance.
0101 | - Percentage discounts are applied in `src/discount.ts`.
0102 | - Half-even rounding is `src/rounding.ts`. Every rounded amount goes through
```

Signal-time measurement:

> CONTRIBUTING.md:98  not under the roots — likely src/money/totals.ts  src/totals.ts  — nothing matched

### Proposed remedy or review question

> Point the citation at where the file is now (a bare filename resolves when exactly one file under the roots has that name), or delete the sentence that cites it.

## S006

- Signal category: regression
- Gate: doc-citations
- Site: `CONTRIBUTING.md:99` — `src/tax.ts`
- Measured values: `{"count":1,"resolution":"not under the roots — likely src/money/tax.ts"}`

### Task intent

> The money code should sit in one folder.
>
> Move the rounding, currency, tax, discount and totals code into `src/money/`,
> keeping each file's name. `src/index.ts` stays the one entry a caller
> imports, and nothing a caller can see may change.
>

### Signal-time context

Signal-time tree excerpt from `CONTRIBUTING.md`:

```text
0097 | - Line totals, their tax and the invoice total are added up in
0098 |   `src/totals.ts`.
0099 | - Tax bands and their rates are in `src/tax.ts`. A new band starts there, and
0100 |   it needs a review from finance.
0101 | - Percentage discounts are applied in `src/discount.ts`.
0102 | - Half-even rounding is `src/rounding.ts`. Every rounded amount goes through
0103 |   it.
```

Signal-time measurement:

> CONTRIBUTING.md:99  not under the roots — likely src/money/tax.ts  src/tax.ts  — nothing matched

### Proposed remedy or review question

> Point the citation at where the file is now (a bare filename resolves when exactly one file under the roots has that name), or delete the sentence that cites it.

## S007

- Signal category: regression
- Gate: doc-citations
- Site: `CONTRIBUTING.md:101` — `src/discount.ts`
- Measured values: `{"count":1,"resolution":"not under the roots — likely src/money/discount.ts"}`

### Task intent

> The money code should sit in one folder.
>
> Move the rounding, currency, tax, discount and totals code into `src/money/`,
> keeping each file's name. `src/index.ts` stays the one entry a caller
> imports, and nothing a caller can see may change.
>

### Signal-time context

Signal-time tree excerpt from `CONTRIBUTING.md`:

```text
0099 | - Tax bands and their rates are in `src/tax.ts`. A new band starts there, and
0100 |   it needs a review from finance.
0101 | - Percentage discounts are applied in `src/discount.ts`.
0102 | - Half-even rounding is `src/rounding.ts`. Every rounded amount goes through
0103 |   it.
0104 | - Currency symbols and the printed form of an amount are in
0105 |   `src/currency.ts`.
```

Signal-time measurement:

> CONTRIBUTING.md:101  not under the roots — likely src/money/discount.ts  src/discount.ts  — nothing matched

### Proposed remedy or review question

> Point the citation at where the file is now (a bare filename resolves when exactly one file under the roots has that name), or delete the sentence that cites it.

## S008

- Signal category: regression
- Gate: doc-citations
- Site: `CONTRIBUTING.md:102` — `src/rounding.ts`
- Measured values: `{"count":1,"resolution":"not under the roots — likely src/money/rounding.ts"}`

### Task intent

> The money code should sit in one folder.
>
> Move the rounding, currency, tax, discount and totals code into `src/money/`,
> keeping each file's name. `src/index.ts` stays the one entry a caller
> imports, and nothing a caller can see may change.
>

### Signal-time context

Signal-time tree excerpt from `CONTRIBUTING.md`:

```text
0100 |   it needs a review from finance.
0101 | - Percentage discounts are applied in `src/discount.ts`.
0102 | - Half-even rounding is `src/rounding.ts`. Every rounded amount goes through
0103 |   it.
0104 | - Currency symbols and the printed form of an amount are in
0105 |   `src/currency.ts`.
0106 |
```

Signal-time measurement:

> CONTRIBUTING.md:102  not under the roots — likely src/money/rounding.ts  src/rounding.ts  — nothing matched

### Proposed remedy or review question

> Point the citation at where the file is now (a bare filename resolves when exactly one file under the roots has that name), or delete the sentence that cites it.

## S009

- Signal category: regression
- Gate: doc-citations
- Site: `README.md:103` — `src/lexer.ts`
- Measured values: `{"count":1,"resolution":"not under the roots — likely src/engine/lexer.ts"}`

### Task intent

> The evaluation engine should sit in its own folder.
>
> Move the lexer, the parser, the evaluator and the error type into
> `src/engine/`, keeping each file's name. `src/index.ts` stays the one entry a
> caller imports, and nothing a caller can see may change.
>

### Signal-time context

Signal-time tree excerpt from `README.md`:

```text
0101 | ### From text to tokens
0102 |
0103 | `src/lexer.ts` splits the text into numbers and symbols and records where each
0104 | one starts. It refuses a character that is neither, which is where
0105 | `unexpected x` comes from.
0106 |
0107 | ### From tokens to a tree
```

Signal-time measurement:

> README.md:103  not under the roots — likely src/engine/lexer.ts  src/lexer.ts  — nothing matched

### Proposed remedy or review question

> Point the citation at where the file is now (a bare filename resolves when exactly one file under the roots has that name), or delete the sentence that cites it.

## S010

- Signal category: regression
- Gate: doc-citations
- Site: `CONTRIBUTING.md:105` — `src/currency.ts`
- Measured values: `{"count":1,"resolution":"not under the roots — likely src/money/currency.ts"}`

### Task intent

> The money code should sit in one folder.
>
> Move the rounding, currency, tax, discount and totals code into `src/money/`,
> keeping each file's name. `src/index.ts` stays the one entry a caller
> imports, and nothing a caller can see may change.
>

### Signal-time context

Signal-time tree excerpt from `CONTRIBUTING.md`:

```text
0103 |   it.
0104 | - Currency symbols and the printed form of an amount are in
0105 |   `src/currency.ts`.
0106 |
```

Signal-time measurement:

> CONTRIBUTING.md:105  not under the roots — likely src/money/currency.ts  src/currency.ts  — nothing matched

### Proposed remedy or review question

> Point the citation at where the file is now (a bare filename resolves when exactly one file under the roots has that name), or delete the sentence that cites it.

## S011

- Signal category: regression
- Gate: doc-citations
- Site: `README.md:109` — `src/parser.ts`
- Measured values: `{"count":1,"resolution":"not under the roots — likely src/engine/parser.ts"}`

### Task intent

> The evaluation engine should sit in its own folder.
>
> Move the lexer, the parser, the evaluator and the error type into
> `src/engine/`, keeping each file's name. `src/index.ts` stays the one entry a
> caller imports, and nothing a caller can see may change.
>

### Signal-time context

Signal-time tree excerpt from `README.md`:

```text
0107 | ### From tokens to a tree
0108 |
0109 | `src/parser.ts` reads the tokens by recursive descent, one function for each
0110 | level of precedence, and builds a tree of numbers, negations and binary
0111 | operations. The other parse errors come from here.
0112 |
0113 | ### From a tree to a number
```

Signal-time measurement:

> README.md:109  not under the roots — likely src/engine/parser.ts  src/parser.ts  — nothing matched

### Proposed remedy or review question

> Point the citation at where the file is now (a bare filename resolves when exactly one file under the roots has that name), or delete the sentence that cites it.

## S012

- Signal category: regression
- Gate: doc-citations
- Site: `README.md:115` — `src/evaluate.ts`
- Measured values: `{"count":1,"resolution":"not under the roots — likely src/engine/evaluate.ts"}`

### Task intent

> The evaluation engine should sit in its own folder.
>
> Move the lexer, the parser, the evaluator and the error type into
> `src/engine/`, keeping each file's name. `src/index.ts` stays the one entry a
> caller imports, and nothing a caller can see may change.
>

### Signal-time context

Signal-time tree excerpt from `README.md`:

```text
0113 | ### From a tree to a number
0114 |
0115 | `src/evaluate.ts` walks the tree and works out the number. Division by zero is
0116 | found here.
0117 |
0118 | ### The error type
0119 |
```

Signal-time measurement:

> README.md:115  not under the roots — likely src/engine/evaluate.ts  src/evaluate.ts  — nothing matched

### Proposed remedy or review question

> Point the citation at where the file is now (a bare filename resolves when exactly one file under the roots has that name), or delete the sentence that cites it.

## S013

- Signal category: regression
- Gate: doc-citations
- Site: `README.md:120` — `src/errors.ts`
- Measured values: `{"count":1,"resolution":"not under the roots — likely src/engine/errors.ts"}`

### Task intent

> The evaluation engine should sit in its own folder.
>
> Move the lexer, the parser, the evaluator and the error type into
> `src/engine/`, keeping each file's name. `src/index.ts` stays the one entry a
> caller imports, and nothing a caller can see may change.
>

### Signal-time context

Signal-time tree excerpt from `README.md`:

```text
0118 | ### The error type
0119 |
0120 | `src/errors.ts` holds `CalcError`, which carries the message and the offset.
0121 | Every step throws it and nothing else.
0122 |
```

Signal-time measurement:

> README.md:120  not under the roots — likely src/engine/errors.ts  src/errors.ts  — nothing matched

### Proposed remedy or review question

> Point the citation at where the file is now (a bare filename resolves when exactly one file under the roots has that name), or delete the sentence that cites it.
