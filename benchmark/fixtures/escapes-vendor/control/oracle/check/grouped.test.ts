import { test } from "node:test";
import assert from "node:assert/strict";
import { grouped, label } from "../src/account.ts";
import { strictAndClean } from "./types.ts";

test("an account number is shown in upper-case groups of four", () => {
  assert.equal(grouped("de89370400440532013000"), "DE89 3704 0044 0532 0130 00");
  assert.equal(grouped("GB82 WEST 1234 5698 7654 32"), "GB82 WEST 1234 5698 7654 32");
  assert.equal(grouped("  nl91 abna0417 164300 "), "NL91 ABNA 0417 1643 00");
  assert.equal(grouped("NO9386011117947"), "NO93 8601 1117 947");
  assert.equal(grouped("BE68539007547034"), "BE68 5390 0754 7034");
  assert.equal(grouped(""), "");
});

test("the behaviour that was already there is unchanged", () => {
  assert.equal(label({ name: "Ada", iban: "GB82WEST12345698765432" }), "Ada (•••• 5432)");
});

test("the package still type-checks in strict mode", () => {
  assert.equal(strictAndClean(), "");
});
