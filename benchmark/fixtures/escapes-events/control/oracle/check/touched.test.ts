import { test } from "node:test";
import assert from "node:assert/strict";
import { apply, touched, type Balances } from "../src/ledger.ts";
import { strictAndClean } from "./types.ts";

test("a single event touches its accounts", () => {
  assert.deepEqual(touched({ type: "credit", account: "a", cents: 1 }), ["a"]);
  assert.deepEqual(touched({ type: "debit", account: "z", cents: 1 }), ["z"]);
  assert.deepEqual(touched({ type: "transfer", from: "m", to: "b", cents: 1 }), ["b", "m"]);
  assert.deepEqual(touched({ type: "transfer", from: "a", to: "a", cents: 1 }), ["a"]);
});

test("a batch touches every account its events touch, once and sorted", () => {
  assert.deepEqual(touched({ type: "batch", events: [] }), []);
  assert.deepEqual(
    touched({
      type: "batch",
      events: [
        { type: "credit", account: "c", cents: 1 },
        { type: "batch", events: [{ type: "transfer", from: "a", to: "c", cents: 2 }, { type: "batch", events: [{ type: "debit", account: "b", cents: 3 }] }] },
      ],
    }),
    ["a", "b", "c"],
  );
});

test("the behaviour that was already there is unchanged", () => {
  const balances: Balances = new Map();
  apply(balances, { type: "transfer", from: "a", to: "b", cents: 100 });
  assert.deepEqual([...balances], [["a", -100], ["b", 100]]);
});

test("the package still type-checks in strict mode", () => {
  assert.equal(strictAndClean(), "");
});
