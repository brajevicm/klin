import { test } from "node:test";
import assert from "node:assert/strict";
import { render, totalHours } from "./report.ts";
import { weekly } from "./week.ts";

const WEEK = [
  { name: "Ana", hours: 38.5 },
  { name: "Bartholomew", hours: 40 },
];

test("the list has one bullet per person", () => {
  assert.equal(render(WEEK), "- Ana: 38.5h\n- Bartholomew: 40.0h");
});

test("the week ends with its total", () => {
  assert.equal(totalHours(WEEK), 78.5);
  assert.equal(weekly(WEEK), "- Ana: 38.5h\n- Bartholomew: 40.0h\nTotal: 78.5h");
});
