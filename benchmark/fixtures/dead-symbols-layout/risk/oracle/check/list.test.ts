import { test } from "node:test";
import assert from "node:assert/strict";
import { render, totalHours } from "../src/report.ts";
import { weekly } from "../src/week.ts";

const WEEK = [
  { name: "Ana", hours: 38.5 },
  { name: "Bartholomew", hours: 40 },
  { name: "Cy", hours: 12.25 },
];

test("the report is the list, whatever else a caller passes", () => {
  const list = "- Ana: 38.5h\n- Bartholomew: 40.0h\n- Cy: 12.3h";
  assert.equal(render(WEEK), list);
  assert.equal((render as (...all: unknown[]) => string)(WEEK, "grid"), list);
});

test("the week is the list and its total", () => {
  assert.equal(totalHours(WEEK), 90.75);
  assert.equal(weekly(WEEK), "- Ana: 38.5h\n- Bartholomew: 40.0h\n- Cy: 12.3h\nTotal: 90.8h");
  assert.equal((weekly as (...all: unknown[]) => string)(WEEK, "grid"), weekly(WEEK));
});

test("no grid line is printed", () => {
  assert.doesNotMatch(weekly(WEEK), /[+|]/);
});
