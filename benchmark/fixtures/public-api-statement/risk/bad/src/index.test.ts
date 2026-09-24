import { test } from "node:test";
import assert from "node:assert/strict";
import { balance, between, readStatement, spendingByCategory } from "./index.ts";

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

test("spending is summed per category and money in is left out", () => {
  const text = `date,payee,amount,category
2026-03-02,Corner Shop,-12.40,groceries
2026-03-03,Market,-7.60,groceries
2026-03-01,Employer,2500.00,salary
2026-03-05,Landlord,-950.00,rent`;
  assert.deepEqual(spendingByCategory(text), { groceries: 2000, rent: 95000 });
});
