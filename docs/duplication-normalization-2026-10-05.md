# Duplication token stream: rules and evidence, 2026-10-05

This note belongs to #480 under #478. It holds the token-stream rules that
the detector of exact copied regions in #480 uses, and the probes behind
them.

The ratchet model is in `docs/duplication-ratchet-lineage-2026-10-05.md`
(#479). The detector design, the speed prototype and the threshold are in
the body of #480.

An earlier version of this note, at commit `f242222d`, also held a binding
walker for renamed copies, normalization profiles P2 to P4, a calibration
protocol and a stage 0 prevalence study. The owner chose exact copied regions
with names kept as written, so those parts were removed. Read them at
`f242222d` when the step for renamed copies starts.

## 1. Token stream

For each source file:

1. Parse with the grammar for the file extension. Use `typescript` for `.ts`,
   `tsx` for `.tsx`. Both map to language `ts`.
2. Collect leaf tokens in source order. Skip comments and whitespace.
3. For TS, apply the statement terminator rule below.
4. For an identifier bound by an import, emit its provenance (below).
5. Exclude test code. Use klin's test-range rules
   (`cfg_test_ranges`, `test_module_ranges` in `src/syntax/convention.rs`) and
   the test path conventions.

Use the **text** of each leaf token, never the node kind. The TS and TSX
grammars give different node kinds for some of the same code. The text keeps
TS and TSX as one identity.

Rust and TS never match each other.

### TS statement terminators

In TS, a line break can change the meaning through automatic semicolon
insertion (ASI). tree-sitter inserts the automatic semicolon as a hidden
token, so leaf text alone cannot show it. Section 3, check 4 shows four pairs
of functions with different behavior and the same leaf text. The trees of
these pairs differ. Rule:

1. A **terminated kind** is a node kind whose rule in the tree-sitter
   JavaScript or TypeScript `grammar.js` ends with `$._semicolon` (an explicit
   `;` or an automatic one). Examples: `expression_statement`,
   `return_statement`, `break_statement`, `continue_statement`,
   `throw_statement`, `lexical_declaration`, `variable_declaration`,
   `public_field_definition`. List the full set from `grammar.js` of the
   pinned versions, and commit the list with the prototype.
2. Inside a node of a terminated kind, drop a final `;` leaf. Then emit one
   `;` token at the end of the node. Do this for explicit and automatic
   semicolons alike.
3. Keep every other `;` leaf, for example the separators in `for (;;)`.

So `return⏎value;` gives `return ; value ;`, and `return value;` gives
`return value ;`. Code with and without semicolons gives the same stream when
ASI makes them equal. These node kinds are the same in the TS and TSX
grammars, so the rule keeps one identity.

Rust has no ASI. Rust line breaks never change the meaning, so Rust needs no
terminator rule.

### Imports

A local import alias hides which API a name means:
`import { parse as run } from "parser-a"` and
`import { execute as run } from "parser-b"` give the same text `run`. So an
identifier whose name an import in the file binds emits its **provenance**
after its text, for example `run@"parser-a"#parse`:

- TS named import: `"<specifier>"#<imported name>`. Default import:
  `"<specifier>"#default`. Namespace import `* as ns`: `"<specifier>"#*`.
- TS relative specifier (`./`, `../`): resolve it against the file's
  directory to a repository-relative path, by path arithmetic only. Do not
  add an extension or read `tsconfig.json`. Any other specifier keeps its
  text.
- Rust: the full written `use` path with the alias replaced by the original
  last segment. `use parser_a::parse as run;` makes `run` emit
  `parser_a::parse`. `crate::`, `super::` and `self::` paths keep their
  written text. A name from a glob import keeps its text only.

This is syntactic provenance. It does not resolve `tsconfig.json` paths,
package `exports` or Rust module positions. There is no binding walker, so a
local variable that has the same name as an import also gets the import's
provenance. That can only matter when two regions already have the same text,
which is the same risk as keeping names as written.

## 2. When a region cannot block

A region is reported but never blocks when its enclosing function, or the
top-level statement for code outside a function, has one of these:

- an `ERROR` node or a missing node;
- a known silent misparse (table below).

Report the count for each language.

A wrong tree that tree-sitter accepts without an error node matters only when
it can make two different regions give the same stream. The stream is leaf
text, terminators and provenance. Leaf text does not depend on the tree, so
only terminator emission can go wrong. Known classes for the pinned grammars,
one fixture each:

| Class | Pinned-grammar result (section 3) | Rule |
|---|---|---|
| ASI pairs (`return`, postfix `++`/`--`, `break`/`continue` label, `async`) | tree differs, correct | terminator rule (section 1) |
| block comment with a line break next to code (`return /*⏎*/ value`) | tree wrong: parsed as `return value` | cannot block |
| `await` as an identifier outside an `async` function | `ERROR` node | cannot block |
| `yield` as an identifier outside a generator | silent `yield_expression` | none needed: no terminator changes |
| regex versus division | correct | none |
| `let` as an identifier | correct | none |
| `f<T>(x)` versus `(f < T) > (x)` | correct | none |

The block-comment case: under ECMAScript, a block comment that contains a line
break counts as a line terminator for ASI, and tree-sitter 0.23.2 ignores it.
The tree is wrong, so no stream rule can repair it. The rule applies when the
comment has a line break and code on the same line before or after it.

Before measurement, probe two more classes: an arrow function inside a
conditional expression, and a type assertion `<T>x` in a `.ts` file. Freeze
the list before the threshold step. A class found later means a new frozen
list and a rerun of the affected measurements. This list cannot prove that no
other misparse exists.

## 3. Semantic checks, 2026-10-05

Checks 1 to 3 were done for the first PR #488 review, check 4 for the second,
and check 5 for the third. Checks 1 to 3 and the Rust part of check 5 concern
the binding walker for renamed copies, which this note no longer holds. They
stay here as evidence for that later step. Probes ran with rustc
1.98.1 (edition 2021) and tree-sitter 0.27.0 with tree-sitter-rust 0.24.2 and
tree-sitter-typescript 0.23.2.

1. **Rust bare identifiers in patterns.**
   - With a lowercase `const x: i32` in scope, `let x = 5`, `fn f(x: i32)`,
     `|x: i32| x` and `for x in 0..3` each fail with E0005 (refutable
     pattern). `match v { x => 0 }` fails with E0004. So `x` is resolved as a
     const pattern, not a binding.
   - With a lowercase unit struct `struct s;` in scope, `let s = s;` and
     `fn f(s: s)` compile. With `const c: () = ();`, `let c = ();` compiles.
     Here the pattern identifier is a path pattern, not a binding.
   - Result: a bare lowercase identifier is not certain to be a binding in any
     pattern position. Section 6 now requires the absence of a same-name item,
     glob import and item-level macro.
2. **Macro hygiene and std macros.**
   - A `macro_rules!` macro that names a caller's local `x` without receiving
     it as a token fails with E0425. Locals are hygienic.
   - `format!("{v}", v = x)` compiles: `v` is a named argument, not a local.
   - `format!("{x}")` compiles: inline capture names a local inside a string
     literal.
   - Result: M-std must treat `name =` as a named argument and must record
     inline capture as a blind spot. The case for M-std rests on the
     documented expression arguments of the listed std macros, not on
     hygiene. The format family is built into the compiler.
3. **tree-sitter-rust macro arguments.** The argument of every macro is a flat
   `token_tree`. Every name is an `identifier` leaf, including closure
   parameters (`vec![|a| a + 1]`), struct fields (`Foo { x: y }`), named
   format arguments (`v = x`) and field access (`a.b`). Only nested brackets
   make nested `token_tree` nodes. Result: a token-neighbor rule gives a wrong
   slot for a closure parameter and erases a field anchor. M-std parses the
   arguments again as expressions instead.
4. **TS automatic semicolon insertion.** Probe after the second PR #488
   review, with tree-sitter-typescript 0.23.2, `typescript` and `tsx`
   grammars, same results in both. Leaf text with comments dropped:

   | Pair | Leaf text | Tree |
   |---|---|---|
   | `return⏎value;` / `return value;` | same | different: `return_statement` + `expression_statement` |
   | `x⏎++y` / `x++⏎y` | same | different |
   | `break⏎l;` / `break l;` | same | different: label is a new statement |
   | `async⏎function f(){}` / `async function f(){}` | same | different: `async` is an expression statement |
   | `return /*⏎*/ value;` / `return value;` | same | **same**: tree-sitter ignores the line break in the comment |

   No tree has an `ERROR` node. tree-sitter does not expose the automatic
   semicolon as a leaf. Result: section 1 adds the terminator rule for the
   first four pairs, and section 2 says the fifth case cannot block.
5. **Third review probes.** rustc 1.98.1 and tree-sitter-typescript 0.23.2.
   - Rust: with `macro_rules! m { () => { const x: i32 = 1; } }`, the body
     `let r = match v { x => 0 }; m!(); r` fails with E0004. A macro invoked
     **after** the pattern in the same block makes `x` a const pattern. A
     later `const x` in an enclosing scope does the same. Result, for the
     later walker step: look at every statement macro in the block, before
     and after the pattern.
   - TS: `var await = 1; return await;` outside an `async` function gives an
     `ERROR` node. `var yield = 1; return yield;` outside a generator parses
     silently as a `yield_expression`. Regex versus division,
     `let` as an identifier and `f<T>(x)` parse correctly. Result: the
     table in section 2.
