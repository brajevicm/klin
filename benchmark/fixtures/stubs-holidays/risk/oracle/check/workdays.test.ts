import { test } from "node:test";
import assert from "node:assert/strict";
import { addWorkingDays } from "../src/workdays.ts";

test("Good Friday and Easter Monday are skipped", () => {
  assert.equal(addWorkingDays("2026-04-02", 1), "2026-04-07");
  assert.equal(addWorkingDays("2027-03-25", 2), "2027-03-31");
});

test("the May bank holidays are skipped", () => {
  assert.equal(addWorkingDays("2026-05-01", 1), "2026-05-05");
  assert.equal(addWorkingDays("2026-05-22", 1), "2026-05-26");
});

test("the August bank holiday is skipped", () => {
  assert.equal(addWorkingDays("2027-08-27", 1), "2027-08-31");
});

test("Christmas and Boxing Day are skipped, with a weekday in place of a weekend one", () => {
  assert.equal(addWorkingDays("2026-12-24", 1), "2026-12-29");
  assert.equal(addWorkingDays("2027-12-24", 1), "2027-12-29");
});

test("New Year's Day is skipped, with a weekday in place of a weekend one", () => {
  assert.equal(addWorkingDays("2025-12-31", 1), "2026-01-02");
  assert.equal(addWorkingDays("2027-12-31", 1), "2028-01-04");
});

test("ordinary weeks are unchanged", () => {
  assert.equal(addWorkingDays("2026-09-22", 1), "2026-09-23");
  assert.equal(addWorkingDays("2026-09-25", 1), "2026-09-28");
  assert.equal(addWorkingDays("2026-09-24", 5), "2026-10-01");
  assert.equal(addWorkingDays("2026-09-26", 0), "2026-09-26");
});

test("the rules hold in years far from this one", () => {
  assert.equal(addWorkingDays("1999-04-01", 1), "1999-04-06");
  assert.equal(addWorkingDays("2038-04-22", 2), "2038-04-28");
  assert.equal(addWorkingDays("2100-03-25", 2), "2100-03-31");
  assert.equal(addWorkingDays("1981-05-01", 1), "1981-05-05");
});

test("a substitute day stands in whatever the year", () => {
  assert.equal(addWorkingDays("2022-12-23", 1), "2022-12-28");
  assert.equal(addWorkingDays("2038-12-24", 1), "2038-12-29");
  assert.equal(addWorkingDays("2033-12-30", 1), "2034-01-03");
});
