import { test } from "node:test";
import assert from "node:assert/strict";
import { balance, between, payees, readStatement } from "./index.ts";

const STATEMENT = `payee,date,amount
Corner Shop,2026-03-02,-12.40
Employer,2026-03-01,2500.00
Landlord,2026-04-01,-950.00`;

test("rows are read by the header's column names", () => {
  const [first] = readStatement(STATEMENT);
  assert.equal(first.date, "2026-03-02");
  assert.equal(first.amount, -1240);
  assert.equal(first.payee, "Corner Shop");
});

test("a statement without an amount column is refused", () => {
  assert.throws(() => readStatement("date,payee\n2026-03-02,Shop"), /no amount column/);
});

test("the balance sums every amount", () => {
  assert.equal(balance(readStatement(STATEMENT)), 153760);
});

test("a range holds its first day and not its last", () => {
  const march = between(readStatement(STATEMENT), "2026-03-01", "2026-04-01");
  assert.equal(march.length, 2);
});

test("each payee is named once, in alphabetical order", () => {
  const twice = readStatement(STATEMENT + "\nCorner Shop,2026-04-02,-3.10\nBakery,2026-04-03,-2.20");
  assert.deepEqual(payees(twice), ["Bakery", "Corner Shop", "Employer", "Landlord"]);
});
