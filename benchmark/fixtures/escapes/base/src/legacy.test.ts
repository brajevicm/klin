import { test } from "node:test";
import assert from "node:assert/strict";
import { readRow } from "./legacy.ts";

test("the old reader splits on the pipe", () => {
  assert.deepEqual(readRow("a|b"), ["a", "b"]);
});

test.skip("the old reader will one day reject a number", () => {
  assert.deepEqual(readRow(12), []);
});
