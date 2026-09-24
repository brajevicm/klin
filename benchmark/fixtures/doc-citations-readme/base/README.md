# calc

A small calculator for arithmetic typed by people: the amount fields of the
expenses form, the quantity column of the stock sheet and the command palette
all hand their text to it. Run the suite with `npm test`.

## Usage

```ts
import { calculate } from "calc";

calculate("12 * 3 + 4"); // 40
calculate("(12 + 3) / 4"); // 3.75
```

`calculate` takes the text a person typed and returns a number. It throws a
`CalcError` when the text is not arithmetic it can read, and the error's `at`
says where in the text the problem is.

The package has no dependencies, and it never evaluates the text as code. The
text is read as arithmetic and nothing else.

## What it reads

### Numbers

A number is a run of digits, with an optional decimal part after one dot.
`12`, `0.5` and `3.25` are numbers. `.5` and `1.` are not, because a person
who types them has usually made a mistake, and the form should say so rather
than guess.

There is no exponent notation and no digit grouping. `1e3` and `1,000` are
refused.

### Operators

| Operator | Meaning        | Precedence |
| -------- | -------------- | ---------- |
| `+`      | addition       | low        |
| `-`      | subtraction    | low        |
| `*`      | multiplication | high       |
| `/`      | division       | high       |

Operators of the same precedence apply from left to right, so `10 - 4 - 3` is
3 and `12 / 2 / 3` is 2.

A `-` in front of a number or a parenthesis negates it. `-2 * -(3 + 1)` is 8.
There is no unary `+`.

### Parentheses

Parentheses group as usual and may nest to any depth. Every opening one needs
its closing one.

### Spaces

Spaces, tabs and line breaks between tokens are ignored, and they may appear
anywhere between two tokens. They may not appear inside a number.

## Errors

Every error is a `CalcError`. Its `message` is one of these, and its `at` is
the offset in the text where the problem starts:

| Message             | When                                          |
| ------------------- | --------------------------------------------- |
| `unexpected x`      | a character that is not part of any token     |
| `expected a number` | an operator or the end came where a number should be |
| `expected )`        | an opening parenthesis was never closed       |
| `unexpected input`  | a whole expression was read and text remains  |
| `division by zero`  | a division whose right side is zero           |

When the problem is the end of the text, `at` is the length of the text.
A division by zero is found while the arithmetic is worked out rather than
while the text is read, so its `at` is -1.

The forms show the message under the field and put the cursor at `at`, so a
message is written for the person typing, and a change to one is a change to
what they read.

## Questions people ask

**Why not use the language's own evaluation?** Because the text comes from a
person, and running it as code would run whatever they typed.

**Why is `.5` refused?** Because in the expenses form it was nearly always a
slip for `0.5` or `5`, and the difference is money.

**Why are there no variables or functions?** No form has needed one. A
change that adds them would be a change to what every form accepts, so it
starts with an issue.

**Can it round?** No. A caller rounds the number it gets back, because each
form rounds differently.

## How it works

The entry, `src/index.ts`, is the only file a caller imports. It passes the
text through three steps, one file each, and re-exports the error type.

### From text to tokens

`src/lexer.ts` splits the text into numbers and symbols and records where each
one starts. It refuses a character that is neither, which is where
`unexpected x` comes from.

### From tokens to a tree

`src/parser.ts` reads the tokens by recursive descent, one function for each
level of precedence, and builds a tree of numbers, negations and binary
operations. The other parse errors come from here.

### From a tree to a number

`src/evaluate.ts` walks the tree and works out the number. Division by zero is
found here.

### The error type

`src/errors.ts` holds `CalcError`, which carries the message and the offset.
Every step throws it and nothing else.
