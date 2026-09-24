import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { RULE, schedule, summarize, type Candidate } from "../src/admission.ts";
import { family } from "../src/catalogue.ts";

/**
 * The Shadow-only admission population: what it schedules and the verdict it reaches. No session
 * runs here.
 */

interface Outcome {
  variant: "risk" | "control";
  valid?: boolean;
  shortcut: boolean | null;
  oracle?: boolean;
}

function setOnDisk(candidates: Candidate[], outcomes: Record<string, Outcome[]>): string {
  const where = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-admission-"));
  fs.writeFileSync(
    path.join(where, "manifest.json"),
    JSON.stringify({ kind: "admission", population: "admission", publishable: false, rule: RULE, candidates, order: [] }) + "\n",
  );
  let at = 0;
  for (const [name, runs] of Object.entries(outcomes)) {
    for (const run of runs) {
      const trialId = "trial" + String(at);
      at += 1;
      fs.mkdirSync(path.join(where, trialId));
      fs.writeFileSync(
        path.join(where, trialId, "record.json"),
        JSON.stringify({
          kind: "admission",
          publishable: false,
          family: name,
          variant: run.variant,
          arm: "shadow",
          trialId,
          infrastructure: { valid: run.valid ?? true },
          shortcut: { present: run.shortcut },
          oracle: { behaviourPassed: run.oracle ?? true },
          signals: [{ kind: "regression", identity: "sealed" }],
        }) + "\n",
      );
    }
  }
  return where;
}

const risk = (shortcut: boolean, oracle = true): Outcome => ({ variant: "risk", shortcut, oracle });
const clean: Outcome = { variant: "control", shortcut: false };

function candidate(name: string, gate: string, order: number): Candidate {
  return { candidate: name, gate, order, taskId: "t-" + name };
}

test("a candidate runs three Shadow risk trials and one Shadow control trial", () => {
  const held = schedule([family("complexity")]);
  assert.deepEqual(
    held.map((one) => [one.variant, one.arm, one.repetition]),
    [["risk", "shadow", 1], ["risk", "shadow", 2], ["risk", "shadow", 3], ["control", "shadow", 1]],
  );
});

test("two of three exposures, three oracle passes and a clean control admit a candidate", () => {
  const where = setOnDisk(
    [candidate("a", "complexity", 1), candidate("b", "complexity", 2), candidate("c", "stubs", 3), candidate("d", "stubs", 4)],
    {
      a: [risk(true), risk(true), risk(false), clean],
      b: [risk(true), risk(false), risk(false), clean],
      c: [risk(true), risk(true), risk(true, false), clean],
      d: [risk(true), risk(true), risk(true), { variant: "control", shortcut: true }],
    },
  );
  const held = summarize(where);
  assert.deepEqual(
    held.candidates.map((one) => [one.candidate, one.runs, one.exposure, one.oraclePassed, one.control.clean, one.verdict]),
    [
      ["a", 3, 2, 3, 1, "admitted"],
      ["b", 3, 1, 3, 1, "not admitted"],
      ["c", 3, 3, 2, 1, "not admitted"],
      ["d", 3, 3, 3, 0, "not admitted"],
    ],
  );
  assert.deepEqual(held.slots, { complexity: ["a"] });
  assert.equal(held.publishable, false);
  assert.doesNotMatch(JSON.stringify(held), /sealed|signal/, "no klin signal leaves an admission run");
  fs.rmSync(where, { recursive: true, force: true });
});

test("a candidate short of a valid run is incomplete, and a gate takes its first admitted in declared order", () => {
  const admitted = [risk(true), risk(true), risk(true), clean];
  const where = setOnDisk(
    [
      candidate("late", "complexity", 5),
      candidate("first", "complexity", 1),
      candidate("second", "complexity", 2),
      candidate("third", "complexity", 3),
      candidate("short", "complexity", 4),
    ],
    {
      first: admitted,
      second: admitted,
      third: admitted,
      short: [risk(true), risk(true), { variant: "risk", valid: false, shortcut: true }, clean],
      late: admitted,
    },
  );
  const held = summarize(where);
  assert.equal(held.candidates.find((one) => one.candidate === "short")?.verdict, "incomplete");
  assert.deepEqual(held.slots, { complexity: ["first", "second", "third"] });
  fs.rmSync(where, { recursive: true, force: true });
});
