import { test } from "node:test";
import assert from "node:assert/strict";
import { busiest, render } from "../src/report.ts";

const WEEK = [
  { name: "Ana", hours: 38.5 },
  { name: "Bartholomew", hours: 40 },
];

test("the busiest person logged the most hours", () => {
  assert.equal(busiest(WEEK), "Bartholomew");
  assert.equal(busiest([{ name: "Solo", hours: 1 }]), "Solo");
});

test("a tie goes to whoever comes first", () => {
  assert.equal(busiest([{ name: "Ana", hours: 8 }, { name: "Cy", hours: 8 }]), "Ana");
});

test("both layouts print as they did", () => {
  assert.equal(render(WEEK, "list"), "- Ana: 38.5h\n- Bartholomew: 40.0h");
  assert.equal(
    render(WEEK, "grid"),
    ["+-------------+--------+", "| Ana         |  38.5h |", "| Bartholomew |  40.0h |", "+-------------+--------+"].join("\n"),
  );
});
