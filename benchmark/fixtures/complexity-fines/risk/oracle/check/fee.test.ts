import { test } from "node:test";
import assert from "node:assert/strict";
import { lateFee } from "../src/fee.ts";

test("a laptop costs five euros from the first day", () => {
  assert.equal(lateFee("laptop", "adult", 1), 500);
  assert.equal(lateFee("laptop", "adult", 3), 1500);
});

test("a laptop fee stops at fifty euros", () => {
  assert.equal(lateFee("laptop", "adult", 20), 5000);
});

test("a laptop two months late adds the replacement fee", () => {
  assert.equal(lateFee("laptop", "adult", 61), 5500);
});

test("a child pays half for a laptop", () => {
  assert.equal(lateFee("laptop", "child", 3), 750);
});

test("the senior waiver does not cover a laptop", () => {
  assert.equal(lateFee("laptop", "senior", 5), 2500);
});

test("the items that were already there are unchanged", () => {
  assert.equal(lateFee("book", "adult", 2), 0);
  assert.equal(lateFee("book", "adult", 5), 75);
  assert.equal(lateFee("dvd", "child", 9), 350);
  assert.equal(lateFee("magazine", "senior", 40), 300);
  assert.equal(lateFee("book", "senior", 10), 0);
  assert.equal(lateFee("book", "adult", 61), 1500);
  assert.equal(lateFee("dvd", "adult", 0), 0);
});
