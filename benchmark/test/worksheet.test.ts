import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { CURRENT_PROTOCOL } from "../src/protocol.ts";
import { digest, sha256 } from "../src/trees.ts";
import { prepare as prepareEvidence } from "../src/evidence.ts";
import { prepare, type EvidenceInput } from "../src/worksheet.ts";

function set(root: string, id: string, arm: string): EvidenceInput {
  const runs = path.join(root, id, "runs");
  const attempt = path.join(runs, id);
  const base = path.join(attempt, "fixtures", "base");
  const prompt = "Add the new behavior and cover it with a test.\n";
  fs.mkdirSync(path.join(base, "src"), { recursive: true });
  fs.mkdirSync(path.join(attempt, "fixtures", "final"), { recursive: true });
  fs.mkdirSync(path.join(attempt, "fixtures", "scoring"), { recursive: true });
  fs.mkdirSync(path.join(attempt, "state"), { recursive: true });
  fs.mkdirSync(path.join(attempt, "hooks", "0"), { recursive: true });
  fs.writeFileSync(path.join(base, "src", "main.ts"), "export function main() { return 1; }\n");
  fs.cpSync(base, path.join(attempt, "fixtures", "final"), { recursive: true });
  fs.cpSync(base, path.join(attempt, "fixtures", "scoring"), { recursive: true });
  fs.writeFileSync(
    path.join(attempt, "record.json"),
    JSON.stringify({
      protocol: CURRENT_PROTOCOL.version,
      kind: "publishable",
      publishable: true,
      family: "demo",
      gate: "complexity",
      taskId: "task",
      variant: "risk",
      arm,
      trialId: id,
      order: 0,
      repetition: 1,
      replaces: null,
      fixture: { treeSha256: digest(base), promptSha256: sha256(prompt) },
      infrastructure: { valid: true },
      signals: [{
        identity: "stable-site",
        kind: "regression",
        auditKind: null,
        gate: "complexity",
        label: "tangled function",
        file: "src/main.ts",
        line: 1,
        text: "export function main()",
        values: { cc: 9 },
        remedy: "Split the function.",
        outcome: "open",
        tries: 0,
        decision: null,
        reason: null,
        time: 1,
        delivery: arm === "active" ? "delivered" : "would-have-been-delivered",
      }],
    }, null, 2) + "\n",
  );
  for (const name of ["agent.json", "behaviour.json", "stats-session.json", "settings.json"]) {
    fs.writeFileSync(path.join(attempt, name), "{}\n");
  }
  fs.writeFileSync(path.join(attempt, "hook"), "#!/bin/sh\n");
  fs.writeFileSync(path.join(attempt, "state", "journal"), "state\n");
  fs.writeFileSync(
    path.join(attempt, "hooks", "0", "payload.json"),
    JSON.stringify({ hook_event_name: "UserPromptSubmit", prompt }) + "\n",
  );
  fs.writeFileSync(
    path.join(runs, "manifest.json"),
    JSON.stringify({
      protocol: CURRENT_PROTOCOL.version,
      kind: "publishable",
      publishable: true,
      order: [{ family: "demo", variant: "risk", arm, order: 0, repetition: 1, trialId: id }],
    }) + "\n",
  );
  const evidence = path.join(root, id, id === "v1" ? "publishable-2026-09-18" : "v2-2026-09-20");
  const archive = path.join(root, id + ".tar.gz");
  prepareEvidence(runs, evidence, archive);
  return { name: id === "v1" ? "v1" : "v2", directory: evidence, archive };
}

test("worksheet preparation is blinded, deduplicated, and repeatable", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-worksheet-"));
  try {
    const inputs = [set(root, "v1", "active"), set(root, "v2", "shadow")];
    assert.throws(
      () => prepare([{ ...inputs[1], name: "v1" }, inputs[1]], path.join(root, "wrong")),
      /is not the frozen evidence set publishable-2026-09-18/,
    );
    const into = path.join(root, "worksheet");
    const first = prepare(inputs, into);
    assert.deepEqual(first.counts, {
      includedRecords: 2,
      signalOccurrences: 2,
      worksheetRows: 1,
    });
    const worksheet = JSON.parse(fs.readFileSync(path.join(into, "worksheet.json"), "utf8")) as {
      rows: Record<string, unknown>[];
    };
    assert.equal(worksheet.rows[0].worksheetId, "S001");
    const visible = JSON.stringify(worksheet);
    for (const hidden of ["arm", "round", "trialId", "identity", "outcome", "delivery"]) {
      assert.equal(visible.includes('"' + hidden + '"'), false, hidden);
    }
    const markdown = fs.readFileSync(path.join(into, "worksheet.md"), "utf8");
    assert.match(markdown, /valid-regression.*valid-review.*undesired/s);
    for (const hidden of ["active", "shadow", "would-have-been-delivered", "open"]) {
      assert.equal(markdown.includes(hidden), false, hidden);
    }
    const joined = JSON.parse(fs.readFileSync(path.join(into, "join.sealed.json"), "utf8")) as {
      rows: { worksheetId: string; occurrences: unknown[] }[];
    };
    assert.deepEqual(joined.rows.map((row) => [row.worksheetId, row.occurrences.length]), [["S001", 2]]);
    const before = fs.readFileSync(path.join(into, "worksheet.json"), "utf8");
    prepare(inputs, into);
    assert.equal(fs.readFileSync(path.join(into, "worksheet.json"), "utf8"), before);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
