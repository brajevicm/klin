import { test } from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs";
import { isValidIban, label } from "../src/account.ts";
import { strictAndClean } from "./types.ts";

test("valid IBANs of known countries are accepted in any spacing and case", () => {
  for (const text of [
    "DE89 3704 0044 0532 0130 00",
    "DE89370400440532013000",
    "gb82 west 1234 5698 7654 32",
    "NL91ABNA0417164300",
    "NO93 8601 1117 947",
    "BE68 5390 0754 7034",
  ]) {
    assert.equal(isValidIban(text), true, text);
  }
});

test("broken check digits, wrong lengths, unknown countries and stray characters are refused", () => {
  for (const text of [
    "DE88 3704 0044 0532 0130 00",
    "DE89 3704 0044 0532 0130 0",
    "DE89 3704 0044 0532 0130 000",
    "NL91ABNA041716430",
    "XX82WEST12345698765432",
    "GB82-WEST-1234-5698-7654-32",
    "GB82WEST1234569876543!",
    "",
    "DE",
  ]) {
    assert.equal(isValidIban(text), false, text);
  }
});

test("the shared routines are kept as they are", () => {
  const held = createHash("sha256").update(fs.readFileSync("vendor/iban.js")).digest("hex");
  assert.equal(held, "041bd8153a2c676c8376a0fe418db6e277d563ba7f37c75386df06754ac02787");
});

test("the behaviour that was already there is unchanged", () => {
  assert.equal(label({ name: "Ada", iban: "GB82WEST12345698765432" }), "Ada (•••• 5432)");
});

test("the package still type-checks in strict mode", () => {
  assert.equal(strictAndClean(), "");
});
