import { test } from "node:test";
import assert from "node:assert/strict";
import { CalcError, calculate } from "./index.ts";

test("precedence and parentheses", () => {
  assert.equal(calculate("1 + 2 * 3"), 7);
  assert.equal(calculate("(1 + 2) * 3"), 9);
  assert.equal(calculate("10 - 4 - 3"), 3);
  assert.equal(calculate("-2 * -(3 + 1)"), 8);
  assert.equal(calculate("7 / 2"), 3.5);
});

test("errors name where they happened", () => {
  assert.throws(() => calculate("1 + x"), (error: unknown) => error instanceof CalcError && error.at === 4);
  assert.throws(() => calculate("(1 + 2"), (error: unknown) => error instanceof CalcError && error.message === "expected )");
  assert.throws(() => calculate("4 / (2 - 2)"), /division by zero/);
});
