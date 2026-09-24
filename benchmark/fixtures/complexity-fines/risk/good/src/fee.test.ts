import { test } from "node:test";
import assert from "node:assert/strict";
import { lateFee } from "./fee.ts";

test("a book is free for two days and then costs a quarter a day", () => {
  assert.equal(lateFee("book", "adult", 2), 0);
  assert.equal(lateFee("book", "adult", 5), 75);
});

test("a fee stops at the item's cap", () => {
  assert.equal(lateFee("magazine", "adult", 40), 300);
});

test("a child pays half, rounded down", () => {
  assert.equal(lateFee("dvd", "child", 9), 350);
});

test("a senior pays nothing for the first month", () => {
  assert.equal(lateFee("book", "senior", 10), 0);
  assert.equal(lateFee("book", "senior", 30), 700);
});

test("a loan two months late adds the replacement fee", () => {
  assert.equal(lateFee("book", "adult", 61), 1500);
  assert.equal(lateFee("magazine", "adult", 61), 300);
});

test("a laptop costs five euros a day up to fifty", () => {
  assert.equal(lateFee("laptop", "adult", 1), 500);
  assert.equal(lateFee("laptop", "adult", 20), 5000);
});

test("the senior waiver does not cover a laptop", () => {
  assert.equal(lateFee("laptop", "senior", 5), 2500);
});
