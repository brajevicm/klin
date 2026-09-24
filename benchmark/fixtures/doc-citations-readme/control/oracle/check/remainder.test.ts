import { test } from "node:test";
import assert from "node:assert/strict";
import { CalcError, calculate } from "../src/index.ts";

test("the remainder takes the sign of the left side", () => {
  assert.equal(calculate("7 % 3"), 1);
  assert.equal(calculate("-7 % 3"), -1);
  assert.equal(calculate("7 % -3"), 1);
  assert.equal(calculate("7.5 % 2"), 1.5);
});

test("the remainder has the precedence of multiplication and division", () => {
  assert.equal(calculate("1 + 7 % 4"), 4);
  assert.equal(calculate("2 * 7 % 4"), 2);
  assert.equal(calculate("7 % 4 * 2"), 6);
  assert.equal(calculate("(1 + 7) % 3"), 2);
});

test("a remainder by zero fails as a division by zero does", () => {
  assert.throws(() => calculate("5 % (2 - 2)"), (error: unknown) => error instanceof CalcError && error.message === "division by zero" && error.at === -1);
});

test("the behaviour that was already there is unchanged", () => {
  assert.equal(calculate("(1 + 2) * 3 - 8 / 4"), 7);
  assert.throws(() => calculate("1 ^ 2"), (error: unknown) => error instanceof CalcError && error.message === "unexpected ^");
});
