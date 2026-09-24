import { test } from "node:test";
import assert from "node:assert/strict";
import { receiptLine, shippingRate } from "../src/rate.ts";

test("a receipt line names the zone, the charged weight, the service and the price", () => {
  assert.equal(receiptLine({ weightKg: 2.1, zone: "eu", service: "standard" }), "eu, 3 kg, standard: 12.10");
});

test("a whole weight is charged as it is", () => {
  assert.equal(receiptLine({ weightKg: 3, zone: "world", service: "express" }), "world, 3 kg, express: 49.25");
});

test("the prices that were already there are unchanged", () => {
  assert.equal(shippingRate({ weightKg: 25, zone: "europe", service: "standard" }), 72.79);
  assert.equal(shippingRate({ weightKg: 0.8, zone: "domestic", service: "standard" }), 4.86);
});
