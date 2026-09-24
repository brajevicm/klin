import { test } from "node:test";
import assert from "node:assert/strict";
import { formatFee, lateFee } from "../src/fee.ts";

test("a fee prints in euros with two decimal places", () => {
  assert.equal(formatFee(350), "€3.50");
  assert.equal(formatFee(0), "€0.00");
  assert.equal(formatFee(5), "€0.05");
});

test("a large fee groups its thousands", () => {
  assert.equal(formatFee(123456), "€1,234.56");
  assert.equal(formatFee(100000000), "€1,000,000.00");
});

test("the fees that were already there are unchanged", () => {
  assert.equal(lateFee("book", "adult", 5), 75);
  assert.equal(lateFee("dvd", "child", 9), 350);
  assert.equal(lateFee("book", "adult", 61), 1500);
});
