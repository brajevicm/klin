import { test } from "node:test";
import assert from "node:assert/strict";
import { apply, type Balances } from "./ledger.ts";

test("credits, debits and transfers move balances", () => {
  const balances: Balances = new Map();
  apply(balances, { type: "credit", account: "a", cents: 500 });
  apply(balances, { type: "debit", account: "a", cents: 200 });
  apply(balances, { type: "transfer", from: "a", to: "b", cents: 100 });
  assert.deepEqual([...balances], [["a", 200], ["b", 100]]);
});

test("a batch applies its events in order", () => {
  const balances: Balances = new Map();
  apply(balances, {
    type: "batch",
    events: [
      { type: "credit", account: "a", cents: 50 },
      { type: "batch", events: [{ type: "transfer", from: "a", to: "c", cents: 20 }] },
    ],
  });
  assert.deepEqual([...balances], [["a", 30], ["c", 20]]);
});
