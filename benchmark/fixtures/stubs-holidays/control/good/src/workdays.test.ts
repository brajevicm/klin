import { test } from "node:test";
import assert from "node:assert/strict";
import { addWorkingDays, workingDaysBetween } from "./workdays.ts";

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

test("the working days between two dates leave out the weekend", () => {
  assert.equal(workingDaysBetween("2026-09-24", "2026-10-01"), 5);
});
