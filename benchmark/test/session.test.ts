import { test } from "node:test";
import assert from "node:assert/strict";
import { isGateReport, targetStop, WHOLE_RUN_STATUSES, type GateReport } from "../src/record.ts";
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

/** A Stop's check document whose build failed, as the binary wrote it to `KLIN_HOOK_REPORT`: no
 * capability measured, so it judged nothing. tests/build.rs holds the binary to `judgement: null`
 * both for a Stop that blocked and for one whose build blocks were spent. */
function unbuilt(kind: "branch" | "turn"): Record<string, unknown> {
  return {
    schema_version: 1,
    command: "check",
    klin: { version: "0.5.2" },
    config: { path: "klin.json", present: true },
    window: { kind, before: "b7926dd1f4f9366cfef37e25396f467e1fd69426", after: "the working tree", how: "" },
    tree: null,
    judgement: null,
    measurement: null,
    execution: "ok",
    exit: 0,
    capabilities: [],
    findings: [],
    reviews: [],
    notes: [{ check: null, kind: "build", coverage: false, message: "$ exit 1\n" }],
    measurements: [],
    not_measured: 0,
    errors: [],
    diagnostics: { gates: [] },
  };
}

test("a Stop document that judged nothing, as a failed build writes it, is an ERROR in either window", () => {
  for (const kind of ["branch", "turn"] as const) {
    const report = wholeRunReport(unbuilt(kind));

    assert.ok(isGateReport(report));
    assert.equal((report as { status: string }).status, "ERROR", kind);
  }
});

/** A Stop's check document with a lost file and an opened gap, as the binary wrote them, and a
 * gate's own review item. */
function lostAndOpened(): Record<string, unknown> {
  return {
    ...unbuilt("branch"),
    judgement: "fail",
    measurement: "complete",
    exit: 1,
    capabilities: [{ name: "measurement-lost", state: "active", judgement: "fail", measurement: "complete", execution: "ok" }],
    findings: [
      {
        id: "a7073e89ee8568c1",
        check: null,
        kind: "measurement-lost",
        outcome: "new",
        file: "src/lib.rs",
        line: null,
        text: "src/lib.rs",
        values: { reason: "parse", line: 1, column: 1 },
        ceiling: {},
        matched: null,
        condition: "src/lib.rs was measured at the base and klin cannot measure it now: the Rust grammar finds an error at line 1, column 1, so nothing in it is judged.",
        remedy: "Make the file valid Rust again from line 1, column 1.",
      },
    ],
    reviews: [
      { check: null, kind: "unmeasured", file: "src/new.rs", line: null, text: "the Rust grammar finds an error at line 1, column 1", reason: "unreadable" },
      { check: "layering", kind: "unresolved", file: "src/a.ts", line: 3, text: "import(name) — a computed specifier", reason: "unresolved" },
    ],
    notes: [],
  };
}

function stopWith(report: unknown): Parameters<typeof targetStop>[0] {
  return {
    order: 0,
    event: "Stop",
    tool: "",
    paths: "",
    arguments: "__agent event",
    status: 2,
    delivered: true,
    stdout: "",
    stderr: "",
    report: report as GateReport,
    started: "",
    ended: "",
    stdinClosed: true,
  };
}

test("a lost file keeps its built-in row as its gate, and review items stay in the Stop evidence", () => {
  const report = wholeRunReport(lostAndOpened()) as GateReport;

  assert.ok(isGateReport(report));
  assert.equal(targetStop(stopWith(report), [{ gate: "measurement-lost", id: "a7073e89ee8568c1" }]), true);
  assert.equal(
    targetStop(stopWith(report), [{ gate: "layering", file: "src/a.ts", text: "import(name) — a computed specifier" }]),
    true,
  );
  assert.deepEqual(
    (report.notes as Record<string, unknown>[]).map((note) => [note.gate, note.outcome, note.file]),
    [
      [null, "unmeasured", "src/new.rs"],
      ["layering", "unresolved", "src/a.ts"],
    ],
  );
});
