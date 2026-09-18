import { test } from "node:test";
import assert from "node:assert/strict";
import { toCsvLine } from "../src/csv.ts";

test("a plain row needs no wrapping", () => {
  assert.equal(toCsvLine(["a", "b", "c"]), "a,b,c");
  assert.equal(toCsvLine(["a", "", "c"]), "a,,c");
});

test("a field holding a comma is wrapped", () => {
  assert.equal(toCsvLine(["a", "b,c"]), 'a,"b,c"');
});

test("a quote inside a wrapped field is written twice", () => {
  assert.equal(toCsvLine(['he said "no"']), '"he said ""no"""');
});

test("a field holding a newline is wrapped", () => {
  assert.equal(toCsvLine(["one\ntwo"]), '"one\ntwo"');
});
