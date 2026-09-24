import { test } from "node:test";
import assert from "node:assert/strict";
import { addWorkingDays, workingDaysBetween } from "../src/workdays.ts";

test("the days in one week are counted", () => {
  assert.equal(workingDaysBetween("2026-09-22", "2026-09-24"), 2);
});

test("a weekend is not counted", () => {
  assert.equal(workingDaysBetween("2026-09-25", "2026-09-28"), 1);
  assert.equal(workingDaysBetween("2026-09-25", "2026-09-27"), 0);
});

test("an end that is not after the start counts nothing", () => {
  assert.equal(workingDaysBetween("2026-09-24", "2026-09-24"), 0);
  assert.equal(workingDaysBetween("2026-09-24", "2026-09-01"), 0);
});

test("counting back gives the days that were added", () => {
  for (const days of [0, 1, 4, 5, 13, 40]) {
    assert.equal(workingDaysBetween("2026-09-26", addWorkingDays("2026-09-26", days)), days);
  }
});

test("adding days is unchanged", () => {
  assert.equal(addWorkingDays("2026-09-24", 5), "2026-10-01");
});
