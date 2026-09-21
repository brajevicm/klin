import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { CURRENT_PROTOCOL } from "../src/protocol.ts";
import { digest, sha256 } from "../src/trees.ts";
import { prepare as prepareEvidence } from "../src/evidence.ts";
import { prepare, type EvidenceInput, type FrozenEvidence } from "../src/worksheet.ts";

interface SetOptions {
  newSource?: string;
  signalText?: string;
  signalLine?: number;
  tool?: string;
  stop?: boolean;
}

function set(root: string, id: string, arm: string, options: SetOptions = {}): EvidenceInput {
  const runs = path.join(root, id, "runs");
  const attempt = path.join(runs, id);
  const base = path.join(attempt, "fixtures", "base");
  const prompt = "Add the new behavior and cover it with a test.\n";
  const baseSource = "export function main() { return 1; }\n";
  const newSource = options.newSource ?? "export function main() { return 2; }\n";
  const signalText = options.signalText ?? "export function main()";
  const signalLine = options.signalLine ?? 1;
  fs.mkdirSync(path.join(base, "src"), { recursive: true });
  fs.mkdirSync(path.join(attempt, "fixtures", "final"), { recursive: true });
  fs.mkdirSync(path.join(attempt, "fixtures", "scoring"), { recursive: true });
  fs.mkdirSync(path.join(attempt, "state"), { recursive: true });
  fs.mkdirSync(path.join(attempt, "hooks", "0"), { recursive: true });
  fs.writeFileSync(path.join(base, "src", "main.ts"), baseSource);
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
        line: signalLine,
        text: signalText,
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
  fs.mkdirSync(path.join(attempt, "hooks", "1"), { recursive: true });
  fs.writeFileSync(
    path.join(attempt, "hooks", "1", "payload.json"),
    JSON.stringify({
      hook_event_name: "PreToolUse",
      tool_name: options.tool ?? "Edit",
      tool_input: options.tool === undefined
        ? { file_path: "/repo/src/main.ts", old_string: baseSource, new_string: newSource, replace_all: false }
        : {},
    }) + "\n",
  );
  if (options.stop !== false) {
    fs.mkdirSync(path.join(attempt, "hooks", "2"), { recursive: true });
    fs.writeFileSync(
      path.join(attempt, "hooks", "2", "payload.json"),
      JSON.stringify({ hook_event_name: "Stop" }) + "\n",
    );
    fs.writeFileSync(
      path.join(attempt, "hooks", "2", "stderr"),
      "src/main.ts:" + String(signalLine) + " cc 9, 1 lines, was cc 8, 1 lines " + signalText + "\n",
    );
  }
  fs.writeFileSync(
    path.join(runs, "manifest.json"),
    JSON.stringify({
      protocol: CURRENT_PROTOCOL.version,
      kind: "publishable",
      publishable: true,
      order: [{ family: "demo", variant: "risk", arm, order: 0, repetition: 1, trialId: id }],
    }) + "\n",
  );
  const directory = path.join(root, id, id === "v1" ? "publishable-2026-09-18" : "v2-2026-09-20");
  const archive = path.join(root, id + ".tar.gz");
  prepareEvidence(runs, directory, archive);
  return { name: id === "v1" ? "v1" : "v2", directory, archive };
}

function expectations(inputs: EvidenceInput[]): Record<"v1" | "v2", FrozenEvidence> {
  const result = {} as Record<"v1" | "v2", FrozenEvidence>;
  for (const input of inputs) {
    const descriptor = JSON.parse(fs.readFileSync(path.join(input.directory, "evidence.json"), "utf8")) as Record<string, unknown>;
    result[input.name] = {
      setId: String(descriptor.setId),
      recordProtocol: Number(descriptor.recordProtocol),
      runManifestSha256: String(descriptor.runManifestSha256),
      rawFilesManifestSha256: String(descriptor.rawFilesManifestSha256),
      archiveSha256: sha256(fs.readFileSync(input.archive)),
      archiveBytes: fs.statSync(input.archive).size,
    };
  }
  return result;
}

function prepareSynthetic(inputs: EvidenceInput[], into: string): ReturnType<typeof prepare> {
  return prepare(inputs, into, expectations(inputs));
}

function replacementSet(root: string, id: string, children: number): EvidenceInput {
  const input = set(root, id, "active");
  const source = path.join(root, id, "runs");
  const original = path.join(source, id);
  const recordFile = path.join(original, "record.json");
  const record = JSON.parse(fs.readFileSync(recordFile, "utf8")) as Record<string, unknown>;
  record.infrastructure = { valid: false };
  fs.writeFileSync(recordFile, JSON.stringify(record, null, 2) + "\n");
  for (let index = 0; index < children; index += 1) {
    const child = id + "-child-" + String(index + 1);
    const childDirectory = path.join(source, child);
    fs.cpSync(original, childDirectory, { recursive: true });
    const childRecord = { ...record, trialId: child, replaces: id, infrastructure: { valid: true } };
    fs.writeFileSync(path.join(childDirectory, "record.json"), JSON.stringify(childRecord, null, 2) + "\n");
  }
  prepareEvidence(source, input.directory, input.archive);
  return input;
}

test("worksheet preparation is blinded, deduplicated, and repeatable", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-worksheet-"));
  try {
    const inputs = [set(root, "v1", "active"), set(root, "v2", "shadow")];
    assert.throws(
      () => prepare([{ ...inputs[1], name: "v1" }, inputs[1]], path.join(root, "wrong")),
      /frozen identity/,
    );
    assert.throws(() => prepare(inputs, path.join(root, "impostor")), /frozen identity/);
    const into = path.join(root, "worksheet");
    const first = prepareSynthetic(inputs, into);
    assert.deepEqual(first.counts, {
      includedRecords: 2,
      signalOccurrences: 2,
      worksheetRows: 1,
    });
    assert.match(first.rows[0].context.excerpt, /return 2/);
    assert.match(first.rows[0].context.measurement, /cc 9.*was cc 8/);
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
    assert.match(markdown, /Signal-time measurement/);
    for (const hidden of ["active", "shadow", "would-have-been-delivered", "open"]) {
      assert.equal(markdown.includes(hidden), false, hidden);
    }
    const joined = JSON.parse(fs.readFileSync(path.join(into, "join.sealed.json"), "utf8")) as {
      rows: { worksheetId: string; occurrences: unknown[] }[];
    };
    assert.deepEqual(joined.rows.map((row) => [row.worksheetId, row.occurrences.length]), [["S001", 2]]);
    const before = fs.readFileSync(path.join(into, "worksheet.json"), "utf8");
    prepareSynthetic(inputs, into);
    assert.equal(fs.readFileSync(path.join(into, "worksheet.json"), "utf8"), before);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("worksheet uses signal-time edits and preserves the neutral comparison", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-worksheet-context-"));
  try {
    const added = "export function main() { return 2; }\nexport function added() {}\n";
    const inputs = [
      set(root, "v1", "active", { newSource: added, signalText: "export function added()", signalLine: 2 }),
      set(root, "v2", "shadow", { newSource: added, signalText: "export function added()", signalLine: 2 }),
    ];
    const result = prepareSynthetic(inputs, path.join(root, "worksheet"));
    assert.match(result.rows[0].context.excerpt, /export function added/);
    assert.match(result.rows[0].context.measurement, /cc 9.*was cc 8/);
    assert.equal(result.rows[0].context.note, null);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("materially different signal-time context is not deduplicated", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-worksheet-context-"));
  try {
    const inputs = [
      set(root, "v1", "active", { newSource: "export function main() { return 2; }\n" }),
      set(root, "v2", "shadow", { newSource: "export function main() { return 3; }\n" }),
    ];
    const result = prepareSynthetic(inputs, path.join(root, "worksheet"));
    assert.equal(result.counts.worksheetRows, 2);
    assert.notEqual(result.rows[0].context.excerpt, result.rows[1].context.excerpt);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("unreconstructable signal-time context fails closed", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-worksheet-context-"));
  try {
    const inputs = [set(root, "v1", "active", { tool: "Mystery" }), set(root, "v2", "shadow")];
    assert.throws(
      () => prepareSynthetic(inputs, path.join(root, "worksheet")),
      /cannot reconstruct forensic tool Mystery/,
    );
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("a scheduled invalid attempt without a replacement is rejected", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-worksheet-replacement-"));
  try {
    const inputs = [replacementSet(root, "v1", 0), set(root, "v2", "shadow")];
    assert.throws(
      () => prepareSynthetic(inputs, path.join(root, "worksheet")),
      /has no valid replacement for scheduled trial v1/,
    );
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("ambiguous replacement chains are rejected", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-worksheet-replacement-"));
  try {
    const inputs = [replacementSet(root, "v1", 2), set(root, "v2", "shadow")];
    assert.throws(
      () => prepareSynthetic(inputs, path.join(root, "worksheet")),
      /has ambiguous replacements for v1: v1-child-1, v1-child-2/,
    );
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
