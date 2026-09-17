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
    agent: { configSha256: "a", configurationDigest: "b" },
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
    hooks: [],
    friction: { blockedStops: 0, gateRuns: 0, guardRefusals: 0, tries: 0, hostDenials: 0 },
    stats: {},
    activity: { klinMs: 12 },
    turns: 3,
    isolation: { workspace: {}, freshness: {} },
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
    hooks: [{ order: 0, event: "Stop", arguments: "gate", status: 2, delivered: true, stdinClosed: false }],
  };
  assert.ok(validate(held).some((one) => one.includes("end of input")));
});

test("an active record's signals were delivered and a shadow record's would have been", () => {
  const stats = {
    episodes: [{ gate: "inventory", id: "abc", file: "tests/split.rs", line: 8, tries: 2 }],
    audit: [],
  };
  assert.equal(signalsFrom(stats, "active")[0].delivery, "delivered");
  assert.equal(signalsFrom(stats, "shadow")[0].delivery, "would-have-been-delivered");
});

test("a signal whose arm does not match its delivery is named", () => {
  const held = {
    ...whole(),
    arm: "shadow",
    signals: signalsFrom({ episodes: [{ gate: "inventory", id: "abc" }] }, "active"),
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
  const signals = signalsFrom(stats, "shadow");
  const asked = signals.filter((one) => one.auditKind === "asked-once");
  assert.equal(asked.length, 1);
  assert.equal(asked[0].kind, "audit");
  assert.equal(asked[0].auditKind, "asked-once");
  assert.deepEqual(validate({ ...whole(), arm: "shadow", signals }), []);
});

test("a regression keeps its measured tries, so repeated stops stay friction", () => {
  const signals = signalsFrom(
    { episodes: [{ gate: "escapes", id: "abc", tries: 4, outcome: "fixed-later" }], audit: [] },
    "active",
  );
  assert.equal(signals[0].tries, 4);
  assert.equal(signals[0].outcome, "fixed-later");
  assert.equal(signals[0].identity, "abc");
});

test("a record with no finding id falls back to the conservative key", () => {
  const signals = signalsFrom(
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
  const signals = signalsFrom(stats, "active");
  const reset = signals.find((one) => one.auditKind === "reset");
  const guard = signals.find((one) => one.auditKind === "guard");
  assert.ok(reset && guard);
  assert.equal(reset.delivery, null, "klin never delivers a reset to an agent");
  assert.equal(reset.gate, "", "an audit row carries no check's name");
  assert.equal(guard.delivery, "delivered", "the arm delivered klin's guard answer");
  assert.equal(guard.gate, "");
  assert.deepEqual(validate({ ...whole(), signals }), []);
});
