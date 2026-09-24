import { test } from "node:test";
import assert from "node:assert/strict";
import { parseCsvLine } from "../src/csv.ts";

test("a wrapped field may hold a comma", () => {
  assert.deepEqual(parseCsvLine('a,"b,c",d'), ["a", "b,c", "d"]);
  assert.deepEqual(parseCsvLine('"1,2","3,4"'), ["1,2", "3,4"]);
});

test("two quotes inside a wrapped field stand for one", () => {
  assert.deepEqual(parseCsvLine('"he said ""no""",x'), ['he said "no"', "x"]);
  assert.deepEqual(parseCsvLine('""""'), ['"']);
});

test("plain rows are unchanged", () => {
  assert.deepEqual(parseCsvLine("a,b,c"), ["a", "b", "c"]);
  assert.deepEqual(parseCsvLine("a,,c"), ["a", "", "c"]);
  assert.deepEqual(parseCsvLine(""), [""]);
});

test("a trailing comma leaves an empty last field", () => {
  assert.deepEqual(parseCsvLine("a,b,"), ["a", "b", ""]);
  assert.deepEqual(parseCsvLine('"a",'), ["a", ""]);
  assert.deepEqual(parseCsvLine('a,""'), ["a", ""]);
});
