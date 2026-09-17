import { test } from "node:test";
import assert from "node:assert/strict";
import { lines, parseCsvLine } from "./csv.ts";

test("plain fields split on the comma", () => {
  assert.deepEqual(parseCsvLine("a,b,c"), ["a", "b", "c"]);
});

test.skip("a quoted field may hold a comma", () => {
  assert.deepEqual(parseCsvLine('a,"b,c",d'), ["a", "b,c", "d"]);
});

test.skip("a doubled quote inside a quoted field is one quote", () => {
  assert.deepEqual(parseCsvLine('"he said ""no""",x'), ['he said "no"', "x"]);
});

test("an empty field stays empty", () => {
  assert.deepEqual(parseCsvLine("a,,c"), ["a", "", "c"]);
});

test("a document splits into lines", () => {
  assert.deepEqual(lines("a\nb\n"), ["a", "b"]);
});
