import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "../src/paths.ts";
import { CURRENT_PROTOCOL } from "../src/protocol.ts";
import { TYPESCRIPT_SHA256 } from "../src/toolchain.ts";
import { RULE, cohortOf, orderOf, populationProblems, rivals, rubricSha256, schedule, summarize, verify, type Apparatus, type Candidate, type Manifest } from "../src/admission.ts";
import type { RunRecord } from "../src/record.ts";
import { sha256 } from "../src/trees.ts";

/**
 * The Shadow-only admission population: what it freezes, what it schedules and the verdict it
 * reaches. No session runs here, and none of these candidates is in the catalogue, so `verify`
 * is shown to read the set alone.
 */

const BASE = JSON.parse(fs.readFileSync(path.join(paths.BENCHMARK, "test", "live-record.json"), "utf8")) as Record<string, unknown>;

const APPARATUS: Apparatus = {
  protocol: CURRENT_PROTOCOL.version,
  schemaSha256: "s",
  harness: { commit: "h", dirty: false, treeSha256: "ht", hookSha256: "hook" },
  confinement: "sandbox",
  execution: "e",
  toolchain: { package: "typescript", version: "5.9.3", path: "/tsc.js", sha256: TYPESCRIPT_SHA256 },
  host: { name: "claude-code", version: "2.1.276 (Claude Code)" },
  model: "sonnet",
  flags: ["--print"],
  isolatedConfiguration: false,
  memory: null,
  machine: { platform: "test", release: "0", arch: "x", node: "v0" },
};

interface Outcome {
  valid?: boolean;
  shortcut: boolean;
  oracle?: boolean;
}

function candidate(name: string, gate: string, order: number): Candidate {
  const identity = (variant: string) => ({ taskId: "t-" + name + variant, promptSha256: "p-" + name + variant, treeSha256: "tr-" + name + variant });
  return { candidate: name, gate, order, fixtureSha256: "f-" + name, variants: { risk: identity("risk"), control: identity("control") } };
}

function recordFor(row: Manifest["order"][number], trialId: string, outcome: Outcome): RunRecord {
  const valid = outcome.valid ?? true;
  return {
    ...BASE,
    protocol: CURRENT_PROTOCOL.version,
    kind: "admission",
    publishable: false,
    family: row.family,
    variant: row.variant,
    arm: "shadow",
    taskId: "t-" + row.family + row.variant,
    order: row.order,
    repetition: row.repetition,
    trialId,
    replaces: null,
    audit: [],
    signals: [],
    hooks: [],
    fixture: { startCommit: "c", promptSha256: "p-" + row.family + row.variant, treeSha256: "tr-" + row.family + row.variant },
    harness: { commit: "h", dirty: false, treeSha256: "ht" },
    klin: { commit: "k", version: "klin 0.9", binarySha256: "kb" },
    host: { name: "claude-code", version: "2.1.276 (Claude Code)", flags: ["--print"], flagsSha256: "x", isolatedConfiguration: false, memory: null },
    model: { requested: "sonnet", reported: "sonnet" },
    agent: { wiringSha256: "w", wrapperSha256: "wr" },
    infrastructure: valid
      ? { valid: true, reason: null, terms: [{ name: "state-fresh", passed: true, detail: "" }] }
      : { valid: false, reason: "host-result-read", terms: [{ name: "host-result-read", passed: false, detail: "no JSON" }] },
    result: { outcome: "completed", evidence: "" },
    oracle: { behaviourPassed: outcome.oracle ?? true, exit: 0, reason: "" },
    shortcut: { present: outcome.shortcut, detector: "d", sites: [], note: "", unread: null },
    friction: { blockedStops: 0, gateRuns: 1, guardRefusals: 0, tries: 0, hostDenials: 0 },
    stats: {},
    activity: { klinMs: 12 },
    turns: 3,
    isolation: {
      workspace: { verified: true, checks: [] },
      freshness: { verified: true, checks: [] },
      outside: { name: "no-tool-call-outside-the-workspace", passed: true, detail: "" },
    },
  } as unknown as RunRecord;
}

function write(where: string, directory: string, record: RunRecord): void {
  fs.mkdirSync(path.join(where, directory), { recursive: true });
  fs.writeFileSync(path.join(where, directory, "record.json"), JSON.stringify(record) + "\n");
}

/**
 * An admission set on disk. Each candidate's outcomes are its three risk runs, then its control.
 * A candidate `declared` holds and `outcomes` does not is one the set did not run.
 */
function setOnDisk(
  declared: Candidate[],
  outcomes: Record<string, Outcome[]>,
  at: { retries?: string; under?: string } = {},
): { where: string; manifest: Manifest } {
  const where = at.retries
    ? path.join(at.retries, "retry")
    : fs.mkdtempSync(path.join(at.under ?? os.tmpdir(), "klin-bench-admission-"));
  fs.mkdirSync(where, { recursive: true });
  const names = declared.filter((one) => one.candidate in outcomes).map((one) => one.candidate);
  const manifest: Manifest = {
    protocol: CURRENT_PROTOCOL.version,
    kind: "admission",
    population: "admission",
    publishable: false,
    seed: 1,
    startedAt: "2026-09-24T00:00:00Z",
    rule: RULE,
    rubric: rubricSha256() as string,
    apparatus: APPARATUS,
    declared,
    candidates: declared.filter((one) => names.includes(one.candidate)),
    order: orderOf(names, 1),
    cohort: "",
    first: at.retries ? sha256(fs.readFileSync(path.join(at.retries, "manifest.json"))) : null,
  };
  manifest.cohort = cohortOf(manifest);
  fs.writeFileSync(path.join(where, "manifest.json"), JSON.stringify(manifest) + "\n");
  for (const row of manifest.order) {
    const at = row.variant === "risk" ? row.repetition - 1 : RULE.runs + row.repetition - 1;
    write(where, row.trialId, recordFor(row, row.trialId, outcomes[row.family][at]));
  }
  return { where, manifest };
}

const risk = (shortcut: boolean, oracle = true): Outcome => ({ shortcut, oracle });
const clean: Outcome = { shortcut: false };
const admitted = [risk(true), risk(true), risk(true), clean];

test("a candidate runs three Shadow risk trials and one Shadow control trial", () => {
  assert.deepEqual(
    schedule(["complexity"]).map((one) => [one.variant, one.arm, one.repetition]),
    [["risk", "shadow", 1], ["risk", "shadow", 2], ["risk", "shadow", 3], ["control", "shadow", 1]],
  );
});

test("two of three exposures, three oracle passes and a clean control admit a candidate", () => {
  const { where } = setOnDisk(
    [candidate("a", "complexity", 1), candidate("b", "complexity", 2), candidate("c", "stubs", 3), candidate("d", "stubs", 4), candidate("e", "stubs", 5)],
    {
      a: [risk(true), risk(true), risk(false), clean],
      b: [risk(true), risk(false), risk(false), clean],
      c: [risk(true), risk(true), risk(true, false), clean],
      d: [risk(true), risk(true), risk(true), { shortcut: true }],
      e: [risk(true), risk(true), risk(true), { shortcut: false, oracle: false }],
    },
  );
  assert.deepEqual(verify(where), [], "the set verifies without reading the catalogue");
  const held = summarize(where);
  assert.deepEqual(
    held.candidates.map((one) => [one.candidate, one.runs, one.exposure, one.oraclePassed, one.control.clean, one.verdict]),
    [
      ["a", 3, 2, 3, 1, "admitted"],
      ["b", 3, 1, 3, 1, "not admitted"],
      ["c", 3, 3, 2, 1, "not admitted"],
      ["d", 3, 3, 3, 0, "not admitted"],
      ["e", 3, 3, 3, 0, "not admitted"],
    ],
  );
  assert.deepEqual(held.slots, { complexity: ["a"] });
  assert.deepEqual(held.unsettled, []);
  assert.equal(held.publishable, false);
  fs.rmSync(where, { recursive: true, force: true });
});

test("a candidate short of a valid run is incomplete, and a gate takes its first admitted in declared order", () => {
  const declared = [
    candidate("late", "complexity", 5),
    candidate("first", "complexity", 1),
    candidate("second", "complexity", 2),
    candidate("third", "complexity", 3),
    candidate("short", "complexity", 4),
  ];
  const whole = setOnDisk(declared, {
    first: admitted,
    second: admitted,
    third: admitted,
    short: [risk(true), risk(true), { valid: false, shortcut: true }, clean],
    late: admitted,
  });
  const held = summarize(whole.where);
  assert.equal(held.candidates.find((one) => one.candidate === "short")?.verdict, "incomplete");
  assert.deepEqual(held.slots, { complexity: ["first", "second", "third"] });
  fs.rmSync(whole.where, { recursive: true, force: true });
});

test("an incomplete candidate earlier in declared order leaves its gate unsettled", () => {
  const shortFirst = setOnDisk([candidate("short", "complexity", 1), candidate("next", "complexity", 2)], {
    short: [risk(true), risk(true), { valid: false, shortcut: true }, clean],
    next: admitted,
  });
  assert.deepEqual(summarize(shortFirst.where).slots, {});
  assert.deepEqual(summarize(shortFirst.where).unsettled, ["complexity"]);
  fs.rmSync(shortFirst.where, { recursive: true, force: true });
});

test("a set over part of the declared population fills no slot an earlier candidate could take", () => {
  const { where } = setOnDisk([candidate("one", "complexity", 1), candidate("four", "complexity", 4)], { four: admitted });
  const held = summarize(where);
  assert.equal(held.candidates[0].verdict, "admitted");
  assert.deepEqual(held.slots, {});
  assert.deepEqual(held.unsettled, ["complexity"]);
  assert.ok(verify(where).some((one) => one.includes("runs part of it")), "a first set runs the whole declared population");
  fs.rmSync(where, { recursive: true, force: true });
});

test("a stale or duplicated record cannot complete a candidate or pass verify", () => {
  const { where, manifest } = setOnDisk([candidate("a", "complexity", 1)], { a: [risk(true), risk(false), risk(false), clean] });
  const row = manifest.order.find((one) => one.variant === "risk") as Manifest["order"][number];
  write(where, "stale", recordFor(row, "stale0000000", risk(true)));
  assert.equal(summarize(where).candidates[0].exposure, 1, "a record no row scheduled counts for nothing");
  assert.ok(verify(where).some((one) => one.includes("stale0000000") && one.includes("no scheduled trial")));
  const exposed = manifest.order.find((one) => one.variant === "risk" && one.repetition === 2) as Manifest["order"][number];
  write(where, "copy", recordFor(exposed, exposed.trialId, risk(true)));
  assert.ok(verify(where).some((one) => one.includes("2 records claim the scheduled trial " + exposed.trialId)));
  assert.equal(summarize(where).candidates[0].verdict, "incomplete", "a row with two records settles on neither");
  fs.rmSync(where, { recursive: true, force: true });
});

test("a record run under another apparatus or fixture than the set froze is named", () => {
  const { where, manifest } = setOnDisk([candidate("a", "complexity", 1)], { a: admitted });
  const [first, second] = manifest.order;
  const moved = recordFor(first, first.trialId, risk(true));
  moved.host.version = "2.1.300 (Claude Code)";
  write(where, first.trialId, moved);
  const reshaped = recordFor(second, second.trialId, risk(true));
  reshaped.fixture.treeSha256 = "another";
  write(where, second.trialId, reshaped);
  const problems = verify(where);
  assert.ok(problems.some((one) => one.includes("the host version 2.1.300")), problems.join(" / "));
  assert.ok(problems.some((one) => one.includes("the set did not share the host version")), problems.join(" / "));
  assert.ok(problems.some((one) => one.includes("the starting tree another")), problems.join(" / "));
  fs.rmSync(where, { recursive: true, force: true });
});

test("an admission.json that is not the verdict its records give fails verify", () => {
  const { where } = setOnDisk([candidate("a", "complexity", 1)], { a: admitted });
  const file = path.join(where, "admission.json");
  fs.writeFileSync(file, JSON.stringify(summarize(where), null, 2) + "\n");
  assert.deepEqual(verify(where), []);
  const edited = summarize(where);
  edited.candidates[0].verdict = "not admitted";
  edited.slots = {};
  fs.writeFileSync(file, JSON.stringify(edited, null, 2) + "\n");
  assert.ok(verify(where).some((one) => one.includes("admission.json is not the verdict")));
  fs.writeFileSync(file, "{");
  assert.ok(verify(where).some((one) => one.includes("admission.json")));
  fs.rmSync(where, { recursive: true, force: true });
});

test("a set frozen under another rubric than the committed one fails verify", () => {
  const { where, manifest } = setOnDisk([candidate("a", "complexity", 1)], { a: admitted });
  assert.match(manifest.rubric, /^[0-9a-f]{64}$/);
  fs.writeFileSync(path.join(where, "manifest.json"), JSON.stringify({ ...manifest, rubric: "0".repeat(64) }) + "\n");
  const problems = verify(where);
  assert.ok(problems.some((one) => one.includes("the rubric")), problems.join(" / "));
  fs.rmSync(where, { recursive: true, force: true });
});

test("a manifest whose cohort is not the one its frozen fields give fails verify", () => {
  const { where, manifest } = setOnDisk([candidate("a", "complexity", 1)], { a: admitted });
  fs.writeFileSync(path.join(where, "manifest.json"), JSON.stringify({ ...manifest, cohort: "0".repeat(64) }) + "\n");
  const problems = verify(where);
  assert.ok(problems.some((one) => one.includes("cohort")), problems.join(" / "));
  fs.rmSync(where, { recursive: true, force: true });
});

test("a retry settles the first set's incomplete candidates, and its verdict is final", () => {
  const declared = [candidate("a", "complexity", 1), candidate("b", "complexity", 2), candidate("c", "stubs", 3)];
  const invalid = [risk(true), risk(true), { valid: false, shortcut: true }, clean];
  const first = setOnDisk(declared, { a: admitted, b: invalid, c: invalid });
  assert.deepEqual(summarize(first.where).unsettled, ["complexity", "stubs"]);
  const retry = setOnDisk(declared, { b: admitted, c: invalid }, { retries: first.where });
  assert.deepEqual(verify(first.where), [], "verifying the first set verifies its retry");
  const settled = summarize(retry.where);
  assert.deepEqual(
    settled.candidates.map((one) => [one.candidate, one.verdict]),
    [["a", "admitted"], ["b", "admitted"], ["c", "not admitted"]],
  );
  assert.deepEqual(settled.slots, { complexity: ["a", "b"] });
  assert.deepEqual(settled.unsettled, []);
  fs.rmSync(first.where, { recursive: true, force: true });
});

test("a retry runs only the first set's incomplete candidates, under the first set's cohort", () => {
  const declared = [candidate("a", "complexity", 1), candidate("b", "complexity", 2), candidate("c", "stubs", 3)];
  const invalid = [risk(true), risk(true), { valid: false, shortcut: true }, clean];
  const first = setOnDisk(declared, { a: admitted, b: invalid, c: invalid });
  const partial = setOnDisk(declared, { b: admitted }, { retries: first.where });
  assert.ok(verify(first.where).some((one) => one.includes("incomplete candidates")), verify(first.where).join(" / "));
  fs.rmSync(partial.where, { recursive: true, force: true });
  const again = setOnDisk(declared, { a: admitted, b: admitted, c: admitted }, { retries: first.where });
  assert.ok(verify(again.where).some((one) => one.includes("incomplete candidates")));
  fs.writeFileSync(path.join(again.where, "manifest.json"), JSON.stringify({ ...again.manifest, first: "0".repeat(64) }) + "\n");
  assert.ok(verify(again.where).some((one) => one.includes("another first set")));
  fs.rmSync(again.where, { recursive: true, force: true });
  const moved = setOnDisk([candidate("a", "complexity", 1), candidate("b", "complexity", 2), candidate("c", "stubs", 4)], { b: admitted, c: admitted }, { retries: first.where });
  assert.ok(verify(moved.where).some((one) => one.includes("cohort")));
  fs.rmSync(first.where, { recursive: true, force: true });
});

test("a first set has no rival of its cohort beside it", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-admission-root-"));
  const declared = [candidate("a", "complexity", 1), candidate("b", "complexity", 2)];
  const invalid = [risk(true), risk(true), { valid: false, shortcut: true }, clean];
  const one = setOnDisk(declared, { a: admitted, b: invalid }, { under: root });
  setOnDisk(declared, { b: admitted }, { retries: one.where });
  assert.deepEqual(rivals(root, one.manifest.cohort), [one.where], "a retry is no rival");
  const two = setOnDisk(declared, { a: admitted, b: admitted }, { under: root });
  assert.deepEqual(rivals(root, one.manifest.cohort).sort(), [one.where, two.where].sort());
  assert.deepEqual(rivals(root, "0".repeat(64)), []);
  fs.rmSync(root, { recursive: true, force: true });
});

test("a first set starts only when every gate holds three or four candidates or a recorded reason for none", () => {
  const pool = (gate: string, count: number) => Array.from({ length: count }, (_, at) => ({ name: gate + "-" + String(at), gate }));
  const gates = ["complexity", "stubs", "lockfile"];
  assert.deepEqual(populationProblems([...pool("complexity", 3), ...pool("stubs", 4)], gates, new Set(["lockfile"])), []);
  assert.deepEqual(populationProblems([...pool("complexity", 3), ...pool("stubs", 4)], gates, new Set()), [
    "lockfile has no candidate and no recorded reason for none",
  ]);
  assert.deepEqual(populationProblems([...pool("complexity", 2), ...pool("stubs", 5), ...pool("lockfile", 3)], gates, new Set()), [
    "complexity has 2 candidates, and a gate needs three or four",
    "stubs has 5 candidates, and a gate needs three or four",
  ]);
  assert.deepEqual(populationProblems([...pool("complexity", 3), ...pool("stubs", 3), ...pool("lockfile", 3), ...pool("typo", 3)], gates, new Set()), [
    "typo-0, typo-1, typo-2 name the gate typo, which no natural family names",
  ]);
});
