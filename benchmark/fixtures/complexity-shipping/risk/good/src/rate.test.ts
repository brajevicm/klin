import { test } from "node:test";
import assert from "node:assert/strict";
import { shippingRate } from "./rate.ts";

test("a light domestic parcel pays the first kilogram only", () => {
  assert.equal(shippingRate({ weightKg: 0.8, zone: "domestic", service: "standard" }), 4.86);
});

test("every started kilogram after the first is charged", () => {
  assert.equal(shippingRate({ weightKg: 2.1, zone: "eu", service: "standard" }), 12.1);
});

test("express costs half as much again", () => {
  assert.equal(shippingRate({ weightKg: 3, zone: "world", service: "express" }), 49.25);
});

test("a heavy parcel abroad takes the surcharge", () => {
  assert.equal(shippingRate({ weightKg: 25, zone: "europe", service: "standard" }), 72.79);
});

test("an unknown zone is refused", () => {
  assert.throws(() => shippingRate({ weightKg: 1, zone: "moon", service: "standard" }));
});

test("a parcel to the islands pays the island rate", () => {
  assert.equal(shippingRate({ weightKg: 4.5, zone: "islands", service: "standard" }), 23.33);
});

test("express to the islands is refused", () => {
  assert.throws(() => shippingRate({ weightKg: 1, zone: "islands", service: "express" }));
});
