import { test } from "node:test";
import assert from "node:assert/strict";
import { balance, between, readStatement, spendingByCategory } from "../src/index.ts";
import { compiles } from "./types.ts";

const CATEGORISED = `category,amount,payee,date
groceries,-12.40,Corner Shop,2026-03-02
salary,2500.00,Employer,2026-03-01
rent,-950.00,Landlord,2026-03-05
groceries,-7.65,Market,2026-03-03
travel,0.00,Rail Refund,2026-03-04
travel,-31.05,Rail,2026-03-06`;

test("the package still type-checks with its statement reader", () => {
  assert.equal(compiles(), "");
});

test("spending is summed per category, in cents, as a positive number", () => {
  assert.deepEqual({ ...spendingByCategory(CATEGORISED) }, { groceries: 2005, rent: 95000, travel: 3105 });
});

test("money in and a zero amount are no spending", () => {
  const text = "date,payee,amount,category\n2026-03-01,Employer,2500.00,salary\n2026-03-04,Rail,0.00,travel";
  assert.deepEqual({ ...spendingByCategory(text) }, {});
});

test("a statement read without categories answers as it did", () => {
  const plain = "payee,date,amount\nCorner Shop,2026-03-02,-12.40\nEmployer,2026-03-01,2500.00\nLandlord,2026-04-01,-950.00";
  const rows = readStatement(plain);
  assert.deepEqual(
    rows.map((one) => [one.date, one.amount, one.payee]),
    [
      ["2026-03-02", -1240, "Corner Shop"],
      ["2026-03-01", 250000, "Employer"],
      ["2026-04-01", -95000, "Landlord"],
    ],
  );
  assert.equal(balance(rows), 153760);
  assert.equal(between(rows, "2026-03-01", "2026-04-01").length, 2);
  assert.throws(() => readStatement("date,payee\n2026-03-02,Shop"), /no amount column/);
});
