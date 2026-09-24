import { test } from "node:test";
import assert from "node:assert/strict";
import { isValidIban, label, masked } from "./account.ts";

test("a masked account shows only its last four characters", () => {
  assert.equal(masked("DE89 3704 0044 0532 0130 00"), "•••• 3000");
});

test("a payee label names the payee and the masked account", () => {
  assert.equal(label({ name: "Ada", iban: "GB82WEST12345698765432" }), "Ada (•••• 5432)");
});

test("an IBAN is valid only with its country's length and good check digits", () => {
  assert.equal(isValidIban("de89 3704 0044 0532 0130 00"), true);
  assert.equal(isValidIban("DE88 3704 0044 0532 0130 00"), false);
  assert.equal(isValidIban("DE89 3704 0044 0532 0130 0"), false);
  assert.equal(isValidIban("XX82WEST12345698765432"), false);
});
