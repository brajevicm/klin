import { test } from "node:test";
import assert from "node:assert/strict";
import { addWorkingDays } from "./workdays.ts";

test("a working day in the week is the next day", () => {
  assert.equal(addWorkingDays("2026-09-22", 1), "2026-09-23");
});

test("a weekend is not counted", () => {
  assert.equal(addWorkingDays("2026-09-25", 1), "2026-09-28");
  assert.equal(addWorkingDays("2026-09-24", 5), "2026-10-01");
});

test("no days is the start itself", () => {
  assert.equal(addWorkingDays("2026-09-26", 0), "2026-09-26");
});

test("the Easter bank holidays are skipped", () => {
  assert.equal(addWorkingDays("2026-04-02", 1), "2026-04-07");
});

test("a weekday stands in for a Christmas at the weekend", () => {
  assert.equal(addWorkingDays("2027-12-24", 1), "2027-12-29");
});
