import { test } from "node:test";
import assert from "node:assert/strict";
import { shippingRate } from "../src/rate.ts";

test("a light parcel to the islands pays the first kilogram only", () => {
  assert.equal(shippingRate({ weightKg: 1, zone: "islands", service: "standard" }), 12.96);
});

test("every started kilogram to the islands after the first is charged", () => {
  assert.equal(shippingRate({ weightKg: 4.5, zone: "islands", service: "standard" }), 23.33);
});

test("a heavy parcel to the islands takes the surcharge", () => {
  assert.equal(shippingRate({ weightKg: 22, zone: "islands", service: "standard" }), 83.59);
});

test("express to the islands is refused", () => {
  assert.throws(() => shippingRate({ weightKg: 1, zone: "islands", service: "express" }));
});

test("the zones that were already there are unchanged", () => {
  assert.equal(shippingRate({ weightKg: 0.8, zone: "domestic", service: "standard" }), 4.86);
  assert.equal(shippingRate({ weightKg: 2.1, zone: "eu", service: "standard" }), 12.1);
  assert.equal(shippingRate({ weightKg: 3, zone: "world", service: "express" }), 49.25);
  assert.equal(shippingRate({ weightKg: 25, zone: "europe", service: "standard" }), 72.79);
  assert.equal(shippingRate({ weightKg: 25, zone: "domestic", service: "express" }), 22.84);
  assert.throws(() => shippingRate({ weightKg: 1, zone: "moon", service: "standard" }));
  assert.throws(() => shippingRate({ weightKg: 31, zone: "eu", service: "standard" }));
});
