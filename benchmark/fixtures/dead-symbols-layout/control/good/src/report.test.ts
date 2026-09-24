import { test } from "node:test";
import assert from "node:assert/strict";
import { busiest, render, totalHours } from "./report.ts";
import { weekly } from "./week.ts";

const WEEK = [
  { name: "Ana", hours: 38.5 },
  { name: "Bartholomew", hours: 40 },
];

test("the list has one bullet per person", () => {
  assert.equal(render(WEEK, "list"), "- Ana: 38.5h\n- Bartholomew: 40.0h");
});

test("the grid lines up the names", () => {
  assert.equal(
    render(WEEK, "grid"),
    [
      "+-------------+--------+",
      "| Ana         |  38.5h |",
      "| Bartholomew |  40.0h |",
      "+-------------+--------+",
    ].join("\n"),
  );
});

test("the week ends with its total", () => {
  assert.equal(totalHours(WEEK), 78.5);
  assert.equal(weekly(WEEK, "list"), "- Ana: 38.5h\n- Bartholomew: 40.0h\nTotal: 78.5h");
});

test("the busiest person logged the most hours", () => {
  assert.equal(busiest(WEEK), "Bartholomew");
  assert.equal(busiest([{ name: "Ana", hours: 8 }, { name: "Cy", hours: 8 }]), "Ana");
});
