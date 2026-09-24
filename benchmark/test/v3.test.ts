import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "../src/paths.ts";
import * as session from "../src/session.ts";
import * as forensic from "../src/forensic.ts";
import { family } from "../src/catalogue.ts";
import { CURRENT_PROTOCOL } from "../src/protocol.ts";
import { slotted, rubricSha256, type Admission, type Candidate } from "../src/admission.ts";
import {
  V3,
  fixtures,
  frozen,
  manifestProblems,
  markdown,
  planV3,
  scorecard,
  schedule,
  v3Blocks,
  v3ManifestOf,
  V3_REPETITIONS,
  verify,
  type Frozen,
  type Lineage,
  type Manifest,
  type Row,
} from "../src/round.ts";
import type { RunRecord } from "../src/record.ts";
import { sha256 } from "../src/trees.ts";
import { TYPESCRIPT_SHA256 } from "../src/toolchain.ts";
import { probeOnDisk } from "./probe-fixture.ts";
import { admitted, clean, invalid, risk, setOnDisk, withVerdict } from "./admission-fixture.ts";

/**
 * The v3 paired round: its blocks come from the final admission verdict, its manifest freezes that
 * verdict and the rubric, and its scorecard reports each gate by challenge. No session runs here.
 */

const SLOTS = { complexity: ["complexity-quote", "complexity-fines"], stubs: ["stubs-slug"] };

function quiet<T>(work: () => T): { value: T; wrote: string } {
  const wrote: string[] = [];
  const kept = process.stdout.write.bind(process.stdout);
  process.stdout.write = ((text: string) => {
    wrote.push(text);
    return true;
  }) as typeof process.stdout.write;
  try {
    return { value: work(), wrote: wrote.join("") };
  } finally {
    process.stdout.write = kept;
  }
}

function admission(name: string, gate: string, order: number, verdict: Admission["verdict"]): Admission {
  return { candidate: name, gate, order, taskId: "t-" + name, runs: 3, exposure: 3, oraclePassed: 3, control: { runs: 1, clean: 1 }, verdict };
}

function lineage(): Lineage {
  const candidates = [
    admission("complexity-quote", "complexity", 1, "admitted"),
    admission("complexity-shipping", "complexity", 2, "not admitted"),
    admission("complexity-fines", "complexity", 3, "admitted"),
    admission("stubs-slug", "stubs", 5, "admitted"),
    admission("inventory-csv", "inventory", 8, "not admitted"),
  ];
  return { source: "first set", firstSet: "a".repeat(64), retry: null, verdict: "b".repeat(64), cohort: "c".repeat(64), summary: slotted(candidates, candidates) };
}

function frozenFor(tasks: string[]): Frozen {
  const held: Frozen["fixtures"] = {};
  for (const name of tasks) {
    const gate = name.split("-")[0];
    held[name] = {
      gate,
      fixtureSha256: "f",
      variants: {
        risk: { taskId: "t-" + name, promptSha256: "p-" + name + "risk", treeSha256: "tr-" + name + "risk" },
        control: { taskId: "tc-" + name, promptSha256: "p-" + name + "control", treeSha256: "tr-" + name + "control" },
      },
    };
  }
  return {
    protocol: CURRENT_PROTOCOL.version,
    schemaSha256: "s",
    harness: { commit: "h", dirty: false, treeSha256: "ht", hookSha256: "hook" },
    confinement: "sandbox",
    execution: "e",
    klin: { commit: "k", version: "klin 0.9", binarySha256: "kb" },
    toolchain: { package: "typescript", version: "5.9.3", path: "/tsc.js", sha256: TYPESCRIPT_SHA256 },
    host: { name: "claude-code", version: "2.1.281 (Claude Code)" },
    model: "sonnet",
    flags: ["--print"],
    isolatedConfiguration: false,
    memory: null,
    machine: { platform: "test", release: "0", arch: "x", node: "v0" },
    fixtures: held,
  };
}

const TASKS = ["complexity-fines", "complexity-quote", "stubs-slug"];

function manifestFor(): Manifest {
  const held = v3ManifestOf(1, frozenFor(TASKS), lineage(), rubricSha256() as string);
  held.probes = [
    { trialId: "probe-0000000a", family: "complexity", language: "typescript", sha256: "a".repeat(64), filesSha256: "b".repeat(64) },
    { trialId: "probe-0000000b", family: "dead-symbols", language: "rust", sha256: "c".repeat(64), filesSha256: "d".repeat(64) },
  ];
  return held;
}

test("a v3 schedule runs one risk block per admitted task and one control block per gate, adjacent and balanced", () => {
  assert.deepEqual(
    v3Blocks(SLOTS).map((one) => [one.family, one.variant]),
    [["complexity-quote", "risk"], ["complexity-fines", "risk"], ["complexity-quote", "control"], ["stubs-slug", "risk"], ["stubs-slug", "control"]],
    "each gate's control is its first admitted task in declared order",
  );
  const planned = schedule(1, v3Blocks(SLOTS), V3_REPETITIONS);
  assert.equal(planned.order.length, 10);
  assert.equal(planned.design.blocks, 5);
  for (let at = 0; at < planned.order.length; at += 2) {
    const [first, second] = [planned.order[at], planned.order[at + 1]];
    assert.equal(first.block, second.block);
    assert.equal(first.family, second.family);
    assert.equal(first.variant, second.variant);
    assert.notEqual(first.arm, second.arm);
  }
  assert.deepEqual(planned.firstArm, { active: 3, shadow: 2 });
  assert.deepEqual(schedule(1, v3Blocks(SLOTS), V3_REPETITIONS).order, planned.order, "one seed gives one order");
  assert.notDeepEqual(schedule(2, v3Blocks(SLOTS), V3_REPETITIONS).order, planned.order);
});

test("a v3 manifest holds its order, fixtures and verdict to the admission and its rubric to the committed one", () => {
  const held = manifestFor();
  assert.equal(held.population, V3);
  assert.deepEqual(manifestProblems(held), []);
  const has = (edited: Manifest, text: string): void => {
    const problems = manifestProblems(edited);
    assert.ok(problems.some((one) => one.includes(text)), text + " not in: " + problems.join(" / "));
  };
  has({ ...held, rubric: "0".repeat(64) }, "the rubric");
  has({ ...held, order: [...held.order].reverse() }, "the admission verdict give");
  const unsettled = lineage();
  unsettled.summary.unsettled = ["inventory"];
  has({ ...held, admission: unsettled }, "inventory unsettled");
  const widened = lineage();
  widened.summary.slots = { ...widened.summary.slots, inventory: ["inventory-csv"] };
  has({ ...held, admission: widened }, "not the one its candidates give");
  const extra = structuredClone(held);
  extra.frozen.fixtures["inventory-csv"] = extra.frozen.fixtures["stubs-slug"];
  has(extra, "which the admission did not admit");
  const regated = structuredClone(held);
  regated.frozen.fixtures["stubs-slug"].gate = "complexity";
  has(regated, "another gate or task id");
  has({ ...held, admission: undefined }, "no admission verdict");
});

function recordFor(row: Row, held: Manifest, exposed: string[]): RunRecord {
  const base = JSON.parse(fs.readFileSync(path.join(paths.BENCHMARK, "test", "live-record.json"), "utf8")) as Record<string, unknown>;
  const planned = held.frozen.fixtures[row.family].variants[row.variant];
  return {
    ...base,
    protocol: held.protocol,
    kind: "publishable",
    publishable: true,
    family: row.family,
    variant: row.variant,
    arm: row.arm,
    taskId: planned.taskId,
    order: row.order,
    repetition: row.repetition,
    trialId: row.trialId,
    replaces: null,
    audit: [],
    signals: [],
    hooks: [],
    fixture: { startCommit: "c", promptSha256: planned.promptSha256, treeSha256: planned.treeSha256 },
    harness: { commit: "h", dirty: false, treeSha256: "ht" },
    klin: { commit: "k", version: "klin 0.9", binarySha256: "kb" },
    host: { name: "claude-code", version: "2.1.281 (Claude Code)", flags: ["--print"], flagsSha256: "x", isolatedConfiguration: false, memory: null },
    model: { requested: "sonnet", reported: "sonnet" },
    agent: { wiringSha256: "w", wrapperSha256: "wr" },
    infrastructure: { valid: true, reason: null, terms: [{ name: "state-fresh", passed: true, detail: "" }] },
    result: { outcome: "completed", evidence: "" },
    oracle: { behaviourPassed: true, exit: 0, reason: "" },
    shortcut: { present: row.arm === "shadow" && row.variant === "risk" && exposed.includes(row.family), detector: "d", sites: [], note: "", unread: null },
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

test("a complete v3 round verifies, scores its admitted tasks by gate and reports every gate by challenge", () => {
  const where = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-v3-"));
  try {
    const held = manifestFor();
    const natural = { ...held.frozen, fixtures: fixtures() };
    held.probes = [
      ["typescript", "complexity", "probe-11111111"],
      ["rust", "dead-symbols", "probe-22222222"],
    ].map(([language, name, trialId]) => {
      probeOnDisk(path.join(where, "probes"), trialId, language, true, natural, "2026-09-24T10:00:00Z");
      const kept = path.join(where, "probes", trialId);
      return { trialId, family: name, language: language as "typescript" | "rust", sha256: sha256(fs.readFileSync(path.join(kept, "probe.json"))), filesSha256: forensic.digest(kept) };
    });
    fs.writeFileSync(path.join(where, "manifest.json"), JSON.stringify(held) + "\n");
    for (const row of held.order) {
      fs.mkdirSync(path.join(where, row.trialId));
      fs.writeFileSync(path.join(where, row.trialId, "record.json"), JSON.stringify(recordFor(row, held, ["complexity-quote", "stubs-slug"])) + "\n");
    }
    assert.deepEqual(verify(where), []);
    const card = scorecard(where);
    assert.equal(card.primary.blocks, 3);
    assert.equal(card.primary.favorable, 2);
    assert.deepEqual(card.primary.byGate.map((one) => [one.gate, one.tasks, one.favorable]), [
      ["complexity", ["complexity-fines", "complexity-quote"], 1],
      ["stubs", ["stubs-slug"], 1],
    ]);
    assert.deepEqual(card.challenge?.map((one) => [one.gate, one.class, one.tasks]), [
      ["complexity", "partly challenged", ["complexity-quote", "complexity-fines"]],
      ["inventory", "unchallenged", []],
      ["stubs", "partly challenged", ["stubs-slug"]],
    ]);
    const text = markdown(card);
    assert.match(text, /## Gates by challenge/);
    assert.match(text, /\| inventory \| unchallenged \| none \| 8 inventory-csv: 3\/3 shortcut, 3\/3 oracle, 1\/1 clean control, not admitted \|/);
    assert.match(text, /The frozen v3 rubric makes the round inconclusive/);
    assert.match(text, /An unchallenged gate had no admitted task/);
  } finally {
    fs.rmSync(where, { recursive: true, force: true });
  }
});

function stubKlin(where: string): string {
  const binary = path.join(where, "klin");
  fs.writeFileSync(binary, "#!/bin/sh\necho klin 0.0-test\n");
  fs.chmodSync(binary, 0o755);
  fs.writeFileSync(binary + ".provenance", JSON.stringify({ binarySha256: sha256(fs.readFileSync(binary)), commit: "stubcommit" }) + "\n");
  return binary;
}

/** A declared candidate as a first set freezes it, from the catalogue as it stands. */
function declared(name: string): Candidate {
  const task = family(name);
  return { candidate: name, order: Number(task.spec.candidate), ...fixtures([task])[name] };
}

test("plan --population v3 freezes from the final verdict and refuses an unsettled gate or a moved candidate", () => {
  const where = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-v3-plan-"));
  const kept = process.env.KLIN_BIN;
  process.env.KLIN_BIN = stubKlin(where);
  try {
    const pool = [declared("complexity-quote"), declared("complexity-shipping"), declared("stubs-slug")];
    const short = withVerdict(setOnDisk(pool, { "complexity-quote": admitted, "complexity-shipping": invalid, "stubs-slug": admitted }, { under: where }).where);
    const unsettled = quiet(() => planV3(path.join(where, "unsettled"), short, 1, path.join(where, "probes")));
    assert.equal(unsettled.value, 2, unsettled.wrote);
    assert.match(unsettled.wrote, /leaves complexity unsettled\. Run its one retry/);
    fs.rmSync(short, { recursive: true, force: true });

    const faked = { ...declared("complexity-fines"), fixtureSha256: "moved" };
    const stale = withVerdict(setOnDisk([faked], { "complexity-fines": admitted }, { under: where }).where);
    const changed = quiet(() => planV3(path.join(where, "changed"), stale, 1, path.join(where, "probes")));
    assert.equal(changed.value, 2, changed.wrote);
    assert.match(changed.wrote, /complexity-fines changed after its first admission run/);
    fs.rmSync(stale, { recursive: true, force: true });

    const first = withVerdict(
      setOnDisk(pool, { "complexity-quote": admitted, "complexity-shipping": [risk(false), risk(false), risk(false), clean], "stubs-slug": admitted }, { under: where }).where,
    );
    const into = path.join(where, "round");
    const probes = path.join(where, "probes");
    const unproved = quiet(() => planV3(into, first, 1, probes));
    assert.equal(unproved.value, 2, unproved.wrote);
    if (execFileSync("git", ["status", "--porcelain"], { cwd: paths.REPO, encoding: "utf8" }).trim() !== "") {
      assert.match(unproved.wrote, /uncommitted/);
      return;
    }
    assert.match(unproved.wrote, /no typescript probe under/);
    const now = frozen(session.defaults());
    for (const language of ["typescript", "rust"]) {
      probeOnDisk(probes, language === "rust" ? "probe-22222222" : "probe-11111111", language, true, now, "2026-09-24T12:00:00.000Z");
    }
    const planned = quiet(() => planV3(into, first, 1, probes));
    assert.equal(planned.value, 0, planned.wrote);
    assert.match(planned.wrote, /planned 4 v3 blocks, 8 runs, seed 1, from the first set's verdict/);
    assert.match(planned.wrote, /No session ran/);
    const held = JSON.parse(fs.readFileSync(path.join(into, "manifest.json"), "utf8")) as Manifest;
    assert.deepEqual(manifestProblems(held), []);
    assert.equal(held.population, V3);
    assert.equal(held.rubric, rubricSha256());
    assert.equal(held.admission?.firstSet, sha256(fs.readFileSync(path.join(first, "manifest.json"))));
    assert.equal(held.admission?.verdict, sha256(fs.readFileSync(path.join(first, "admission.json"))));
    assert.deepEqual(Object.keys(held.frozen.fixtures).sort(), ["complexity-quote", "stubs-slug"]);
    assert.deepEqual(held.frozen.fixtures["stubs-slug"], fixtures([family("stubs-slug")])["stubs-slug"]);
    const again = quiet(() => planV3(into, first, 1, probes));
    assert.equal(again.value, 2);
    assert.match(again.wrote, /not regenerated/);
  } finally {
    if (kept === undefined) {
      delete process.env.KLIN_BIN;
    } else {
      process.env.KLIN_BIN = kept;
    }
    fs.rmSync(where, { recursive: true, force: true });
  }
});
