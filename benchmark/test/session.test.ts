import { test } from "node:test";
import assert from "node:assert/strict";
import { isGateReport, WHOLE_RUN_STATUSES } from "../src/record.ts";
import { wholeRunReport } from "../src/session.ts";

/** A `klin check` document whose one integration wrote no report: no finding, a hole, exit 3. */
function incomplete(): Record<string, unknown> {
  return {
    schema_version: 1,
    command: "check",
    judgement: "pass",
    measurement: "incomplete",
    execution: "ok",
    exit: 3,
    capabilities: [
      { name: "doc-size", state: "active", judgement: "pass", measurement: "complete", execution: "ok" },
      { name: "eslint", state: "active", judgement: "pass", measurement: "incomplete", execution: "ok" },
      { name: "escapes", state: "not-applicable", judgement: null, measurement: "incomplete", execution: "ok" },
    ],
    findings: [],
    notes: [],
    measurements: [],
    errors: [],
    diagnostics: { gates: [{ name: "doc-size", ms: 1 }, { name: "eslint", ms: 2 }] },
  };
}

test("an exit-3 check document is incomplete, never a pass, for the run and for each gate it names", () => {
  const report = wholeRunReport(incomplete()) as { status: string; gates: { name: string; status: string }[] };

  assert.equal(report.status, "INCOMPLETE");
  assert.deepEqual(
    report.gates.map((gate) => [gate.name, gate.status]),
    [["doc-size", "ok"], ["eslint", "INCOMPLETE"], ["escapes", "INCOMPLETE"]],
  );
});

test("a seeded trial reads an incomplete gate as no production verdict, so it cannot score a pass", () => {
  const report = wholeRunReport(incomplete());

  assert.ok(isGateReport(report));
  const eslint = (report as { gates: { name: string; status: string }[] }).gates.find((gate) => gate.name === "eslint");
  assert.ok(eslint && !WHOLE_RUN_STATUSES.includes(eslint.status));
});

test("an error outranks a failing finding, and a failing finding outranks a hole", () => {
  const failed = { ...incomplete(), judgement: "fail", exit: 1 };
  const erred = { ...failed, execution: "error", exit: 2 };

  assert.equal((wholeRunReport(failed) as { status: string }).status, "FAIL");
  assert.equal((wholeRunReport(erred) as { status: string }).status, "ERROR");
});
