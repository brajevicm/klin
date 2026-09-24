import { test } from "node:test";
import assert from "node:assert/strict";
import { balance, between, payees, readStatement } from "../src/index.ts";
import { compiles } from "./types.ts";

const STATEMENT = "payee,date,amount\nLandlord,2026-04-01,-950.00\nCorner Shop,2026-03-02,-12.40\nEmployer,2026-03-01,2500.00\nCorner Shop,2026-04-02,-3.10";

test("the package still type-checks with its statement reader", () => {
  assert.equal(compiles(), "");
});

test("each payee is named once, in alphabetical order", () => {
  assert.deepEqual(payees(readStatement(STATEMENT)), ["Corner Shop", "Employer", "Landlord"]);
  assert.deepEqual(payees([]), []);
});

test("the sums that were already there are unchanged", () => {
  const rows = readStatement(STATEMENT);
  assert.equal(balance(rows), 153450);
  assert.equal(between(rows, "2026-03-01", "2026-04-01").length, 2);
});
