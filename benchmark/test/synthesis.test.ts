import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { sha256 } from "../src/trees.ts";
import { synthesize } from "../src/synthesis.ts";

function signal(gate: string, outcome: string, delivery: string): Record<string, unknown> {
  return {
    identity: gate + "-site",
    kind: "regression",
    auditKind: null,
    gate,
    label: gate,
    file: "src/main.ts",
    line: 1,
    text: "x",
    values: null,
    remedy: null,
    outcome,
    tries: 1,
    decision: null,
    reason: null,
    time: 1,
    delivery,
  };
}

function occurrence(round: string, trialId: string, arm: string, variant: string, gate: string): Record<string, unknown> {
  return {
    round,
    trialId,
    family: "demo",
    variant,
    arm,
    order: 0,
    repetition: 1,
    replaces: null,
    taskId: "task",
    promptSha256: "p",
    gate: "demo",
    signal: signal(gate, arm === "active" ? "fixed-next" : "open", arm === "active" ? "delivered" : "would-have-been-delivered"),
  };
}

function manifest(root: string, name: string, rows: { arm: string; variant: string; trialId: string }[]): string {
  const directory = path.join(root, name);
  fs.mkdirSync(directory, { recursive: true });
  const file = path.join(directory, "manifest.json");
  fs.writeFileSync(file, JSON.stringify({ kind: "publishable", publishable: true, order: rows.map((row, order) => ({ ...row, family: "demo", order, repetition: 1 })) }) + "\n");
  return directory;
}

function labeling(root: string): { directory: string; v1: string; v2: string; hash: string } {
  const v1 = manifest(root, "v1", [
    { arm: "active", variant: "risk", trialId: "a-risk" },
    { arm: "active", variant: "control", trialId: "a-control" },
    { arm: "shadow", variant: "risk", trialId: "s-risk" },
    { arm: "shadow", variant: "control", trialId: "s-control" },
  ]);
  const v2 = manifest(root, "v2", [
    { arm: "active", variant: "risk", trialId: "b-risk" },
    { arm: "shadow", variant: "risk", trialId: "t-risk" },
  ]);
  const directory = path.join(root, "labeling");
  fs.mkdirSync(directory);
  const join = {
    version: 1,
    status: "sealed-before-human-labeling",
    counts: { includedRecords: 6, signalOccurrences: 5, worksheetRows: 3 },
    evidence: [
      { round: "v1", setId: "v1-set", manifestSha256: sha256(fs.readFileSync(path.join(v1, "manifest.json"))) },
      { round: "v2", setId: "v2-set", manifestSha256: sha256(fs.readFileSync(path.join(v2, "manifest.json"))) },
    ],
    rows: [
      { worksheetId: "S001", occurrences: [occurrence("v1", "a-risk", "active", "risk", "complexity"), occurrence("v1", "s-risk", "shadow", "risk", "complexity")] },
      { worksheetId: "S002", occurrences: [occurrence("v1", "a-control", "active", "control", "escapes"), occurrence("v2", "b-risk", "active", "risk", "escapes")] },
      { worksheetId: "S003", occurrences: [occurrence("v2", "t-risk", "shadow", "risk", "inventory")] },
    ],
  };
  fs.writeFileSync(path.join(directory, "join.sealed.json"), JSON.stringify(join, null, 2) + "\n");
  const labels = JSON.stringify({ S001: "valid-regression", S002: "undesired", S003: "valid-review" }) + "\n";
  fs.writeFileSync(path.join(directory, "labels.locked.json"), labels);
  const hash = sha256(labels);
  fs.writeFileSync(path.join(directory, "labels.locked.sha256"), hash + "  labels.locked.json\n");
  return { directory, v1, v2, hash };
}

test("the synthesis verifies the locked labels before it joins and reports both views per stratum", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-synthesis-"));
  try {
    const made = labeling(root);
    assert.throws(() => synthesize(made.directory, { v1: made.v1, v2: made.v2 }, "0".repeat(64)), /recorded SHA-256/);
    const read = synthesize(made.directory, { v1: made.v1, v2: made.v2 }, made.hash);
    assert.equal(read.labelsSha256, made.hash);
    assert.deepEqual(read.sites.map((row) => [row.gate, row.sites, row.undesired, row.occurrences.v1, row.occurrences.v2]), [
      ["complexity", 1, 0, 2, 0],
      ["escapes", 1, 1, 1, 1],
      ["inventory", 1, 0, 0, 1],
    ]);
    assert.deepEqual(read.runs.v1, {
      active: { runs: 2, undesired: 1, controls: 1, controlsUndesired: 1, risk: 1, riskValid: 1 },
      shadow: { runs: 2, undesired: 0, controls: 1, controlsUndesired: 0, risk: 1, riskValid: 1, occurrences: { undesired: 0, valid: 1 } },
    });
    assert.deepEqual(read.runs.v2, {
      active: { runs: 1, undesired: 1, controls: 0, controlsUndesired: 0, risk: 1, riskValid: 0 },
      shadow: { runs: 1, undesired: 0, controls: 0, controlsUndesired: 0, risk: 1, riskValid: 1, occurrences: { undesired: 0, valid: 1 } },
    });
    const markdown = fs.readFileSync(path.join(made.directory, "synthesis.md"), "utf8");
    assert.match(markdown, /challenge-adequacy/);
    assert.match(markdown, /seeded/);
    assert.match(markdown, /\| escapes \| 1 \|/);
    const edited = path.join(made.directory, "labels.locked.json");
    fs.writeFileSync(edited, fs.readFileSync(edited, "utf8").replace("undesired", "valid-review"));
    assert.throws(() => synthesize(made.directory, { v1: made.v1, v2: made.v2 }, made.hash), /recorded SHA-256/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("a v3 synthesis stays sealed until a v3 lock is recorded and reads the v3 round alone", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-synthesis-v3-"));
  try {
    const v3 = manifest(root, "v3", [
      { arm: "active", variant: "risk", trialId: "a-risk" },
      { arm: "active", variant: "control", trialId: "a-control" },
      { arm: "shadow", variant: "risk", trialId: "s-risk" },
    ]);
    const directory = path.join(root, "labeling");
    fs.mkdirSync(directory);
    const join = {
      evidence: [{ round: "v3", setId: "v3-set", manifestSha256: sha256(fs.readFileSync(path.join(v3, "manifest.json"))) }],
      rows: [
        { worksheetId: "S001", occurrences: [occurrence("v3", "a-risk", "active", "risk", "complexity"), occurrence("v3", "s-risk", "shadow", "risk", "complexity")] },
        { worksheetId: "S002", occurrences: [occurrence("v3", "a-control", "active", "control", "doc-citations")] },
      ],
    };
    fs.writeFileSync(path.join(directory, "join.sealed.json"), JSON.stringify(join) + "\n");
    const labels = JSON.stringify({ S001: "valid-regression", S002: "undesired" }) + "\n";
    fs.writeFileSync(path.join(directory, "labels.locked.json"), labels);
    fs.writeFileSync(path.join(directory, "labels.locked.sha256"), sha256(labels) + "  labels.locked.json\n");
    assert.throws(() => synthesize(directory, { v3 }), /no locked v3 label hash is recorded/);
    assert.throws(() => synthesize(directory, { v1: v3, v2: v3 }, sha256(labels)), /sealed join names the rounds v3/);
    const read = synthesize(directory, { v3 }, sha256(labels));
    assert.deepEqual(read.runs.v3?.active, { runs: 2, undesired: 1, controls: 1, controlsUndesired: 1, risk: 1, riskValid: 1 });
    assert.deepEqual(read.sites.map((row) => [row.gate, row.occurrences.v3, row.occurrences.v1]), [["complexity", 2, undefined], ["doc-citations", 1, undefined]]);
    assert.match(fs.readFileSync(path.join(directory, "synthesis.md"), "utf8"), /v3 paired round only: `v3-set`/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
