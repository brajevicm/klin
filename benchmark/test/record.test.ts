import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import * as paths from "../src/paths.ts";
import { signalsFrom, validate } from "../src/record.ts";

function whole(): Record<string, unknown> {
  return {
    protocol: paths.PROTOCOL,
    kind: "calibration",
    publishable: false,
    family: "inventory",
    gate: "inventory",
    taskId: "0123456789abcdef",
    variant: "risk",
    arm: "active",
    trialId: "t1",
    order: 0,
    fixture: { startCommit: "a", promptSha256: "b", treeSha256: "c" },
    harness: { commit: "a", dirty: false, treeSha256: "b" },
    klin: { commit: "a", version: "klin 0.2.0", binarySha256: "b" },
    host: {
      name: "claude-code",
      version: "2.1.0",
      flags: [],
      flagsSha256: "b",
      isolatedConfiguration: false,
      memory: null,
    },
    model: { requested: "sonnet", reported: null },
    agent: { wiringSha256: "a", wrapperSha256: "b" },
    startedAt: "2026-09-17T00:00:00.000Z",
    endedAt: "2026-09-17T00:01:00.000Z",
    wallMs: 60000,
    infrastructure: {
      valid: true,
      reason: null,
      terms: [{ name: "workspace-isolated", passed: true, detail: "" }],
    },
    result: { outcome: "completed", evidence: "success" },
    oracle: { behaviourPassed: true, exit: 0, reason: "" },
    shortcut: { present: false, detector: "test_missing", sites: [], note: "", unread: null },
    signals: [],
    audit: [],
    hooks: [],
    friction: { blockedStops: 0, gateRuns: 0, guardRefusals: 0, tries: 0, hostDenials: 0 },
    stats: {},
    activity: { klinMs: 12 },
    turns: 3,
    isolation: {
      workspace: {},
      freshness: {},
      outside: { name: "no-tool-call-outside-the-workspace", passed: true, detail: "" },
    },
  };
}

test("a whole record holds the contract", () => {
  assert.deepEqual(validate(whole()), []);
});

test("a missing field is named", () => {
  const held = whole();
  delete held.signals;
  assert.deepEqual(validate(held), ["the record states no signals"]);
});

test("a calibration record may never claim to be publishable", () => {
  const held = { ...whole(), publishable: true };
  assert.ok(validate(held).some((one) => one.includes("publishable")));
});

test("an outcome the scorecard does not know is named", () => {
  const held = { ...whole(), result: { outcome: "fine", evidence: "" } };
  assert.ok(validate(held).some((one) => one.includes("fine")));
});

test("a hook that did not reach end of input is named", () => {
  const held = {
    ...whole(),
    hooks: [
      {
        order: 0,
        event: "Stop",
        tool: "",
        input: "",
        arguments: "gate",
        status: 2,
        delivered: true,
        stdinClosed: false,
      },
    ],
  };
  assert.ok(validate(held).some((one) => one.includes("end of input")));
});

test("an active record's signals were delivered and a shadow record's would have been", () => {
  const stats = {
    episodes: [{ gate: "inventory", id: "abc", file: "tests/split.rs", line: 8, tries: 2 }],
    audit: [],
  };
  assert.equal(signalsFrom(stats, "active").signals[0].delivery, "delivered");
  assert.equal(signalsFrom(stats, "shadow").signals[0].delivery, "would-have-been-delivered");
});

test("a signal whose arm does not match its delivery is named", () => {
  const held = {
    ...whole(),
    arm: "shadow",
    signals: signalsFrom({ episodes: [{ gate: "inventory", id: "abc" }] }, "active").signals,
  };
  assert.ok(validate(held).some((one) => one.includes("delivery")));
});

test("a deleted-test question stays audit evidence and is never a regression", () => {
  const stats = {
    episodes: [],
    audit: [
      { time: 1, kind: "asked-once", file: "tests/split.rs", line: 8, decision: null, reason: null },
      { time: 2, kind: "guard", decision: "deny", reason: "an edit to klin.json" },
    ],
  };
  const { signals, audit } = signalsFrom(stats, "shadow");
  assert.equal(signals.length, 1);
  assert.equal(signals[0].kind, "audit");
  assert.equal(signals[0].auditKind, "asked-once");
  assert.deepEqual(
    audit.map((one) => one.auditKind),
    ["guard"],
    "a guard decision is the factual trail and no site to classify",
  );
  assert.deepEqual(validate({ ...whole(), arm: "shadow", signals, audit }), []);
});

test("an ordinary audit row filed as a signal site is named", () => {
  const { signals, audit } = signalsFrom(
    { episodes: [], audit: [{ time: 1, kind: "guard", decision: "deny", reason: "an edit" }] },
    "active",
  );
  assert.deepEqual(signals, [], "a guard decision is no signal site");
  assert.ok(
    validate({ ...whole(), signals: audit, audit: [] }).some((one) =>
      one.includes("ordinary audit row"),
    ),
  );
});

test("a deleted-test question filed as an ordinary audit row is named", () => {
  const { signals } = signalsFrom(
    {
      episodes: [],
      audit: [{ time: 1, kind: "asked-once", file: "a.rs", line: 1, decision: null, reason: null }],
    },
    "active",
  );
  assert.ok(
    validate({ ...whole(), signals: [], audit: signals }).some((one) =>
      one.includes("deleted-test question"),
    ),
  );
});

test("a regression keeps its measured tries, so repeated stops stay friction", () => {
  const { signals } = signalsFrom(
    { episodes: [{ gate: "escapes", id: "abc", tries: 4, outcome: "fixed-later" }], audit: [] },
    "active",
  );
  assert.equal(signals[0].tries, 4);
  assert.equal(signals[0].outcome, "fixed-later");
  assert.equal(signals[0].identity, "abc");
});

test("a record with no finding id falls back to the conservative key", () => {
  const { signals } = signalsFrom(
    {
      episodes: [
        { gate: "doc-size", id: null, key: { gate: "doc-size", file: "README.md", line: 1, text: "x" } },
      ],
      audit: [],
    },
    "active",
  );
  assert.equal(signals[0].identity, "doc-size|README.md|1|x");
});

test("the schema beside the harness names every required field", () => {
  const schema = JSON.parse(fs.readFileSync(paths.SCHEMA, "utf8")) as {
    required: string[];
    properties: Record<string, unknown>;
  };
  for (const key of schema.required) {
    assert.ok(key in whole(), "the schema requires " + key + " and the record has none");
  }
  for (const key of Object.keys(whole())) {
    assert.ok(key in schema.properties, "the record states " + key + " and the schema has none");
  }
});

test("a reset is a person's action, so it claims no delivery and borrows no gate", () => {
  const stats = {
    episodes: [],
    audit: [
      { time: 1, kind: "reset", decision: null, reason: null, file: null, line: null },
      { time: 2, kind: "guard", decision: "deny", reason: "an edit to klin.json" },
    ],
  };
  const { signals, audit } = signalsFrom(stats, "active");
  const reset = audit.find((one) => one.auditKind === "reset");
  const guard = audit.find((one) => one.auditKind === "guard");
  assert.ok(reset && guard);
  assert.deepEqual(signals, [], "neither row is a site anyone classifies");
  assert.equal(reset.delivery, null, "klin never delivers a reset to an agent");
  assert.equal(reset.gate, "", "an audit row carries no check's name");
  assert.equal(guard.delivery, "delivered", "the arm delivered klin's guard answer");
  assert.equal(guard.gate, "");
  assert.deepEqual(validate({ ...whole(), signals, audit }), []);
});
