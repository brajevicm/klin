import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "../src/paths.ts";
import * as session from "../src/session.ts";
import { CURRENT_PROTOCOL } from "../src/protocol.ts";
import { families } from "../src/catalogue.ts";
import { sha256 } from "../src/trees.ts";
import * as forensic from "../src/forensic.ts";
import { ENVIRONMENT_ARTIFACT, verifyProbe } from "../src/probe.ts";
import { probeOnDisk } from "./probe-fixture.ts";
import {
  ATTEMPTS,
  FLOOR,
  blocksByGate,
  execute,
  committedAt,
  frozen,
  identity,
  protocol,
  manifestOf,
  manifestProblems,
  markdown,
  mcnemar,
  plan,
  protocolFile,
  replacementId,
  rows,
  scorecard,
  probeProblems,
  uncommitted,
  verify,
  witnesses,
  type Frozen,
  type Manifest,
  type Row,
} from "../src/round.ts";
import { execFileSync } from "node:child_process";
import type { RunRecord } from "../src/record.ts";
import { crash } from "../src/calibrate.ts";
import { TYPESCRIPT_SHA256 } from "../src/toolchain.ts";

/**
 * The publishable round: its frozen schedule, its write-once attempts and its mechanical
 * scorecard. No session runs here.
 */

function room(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-round-"));
}

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

test("a seed gives 36 blocks of two adjacent arms, balanced 18 and 18, and always the same", () => {
  const held = rows(1);
  assert.equal(held.length, 72);
  for (let block = 0; block < 36; block += 1) {
    const [first, second] = [held[2 * block], held[2 * block + 1]];
    assert.equal(first.block, block);
    assert.equal(second.block, block);
    assert.equal(first.family, second.family);
    assert.equal(first.variant, second.variant);
    assert.equal(first.repetition, second.repetition);
    assert.notEqual(first.arm, second.arm);
    assert.equal(first.order, 2 * block);
  }
  const firsts = held.filter((one) => one.order % 2 === 0);
  assert.equal(firsts.filter((one) => one.arm === "active").length, 18);
  for (const family of Object.keys(families())) {
    const blocks = firsts.filter((one) => one.family === family);
    assert.deepEqual(
      blocks.map((one) => one.variant + one.repetition).sort(),
      ["control1", "risk1", "risk2", "risk3"],
    );
  }
  assert.deepEqual(rows(1), held, "one seed, one order");
  assert.notDeepEqual(rows(2), held, "another seed, another order");
  assert.equal(new Set(held.map((one) => one.trialId)).size, 72);
});

test("exact two-sided McNemar rejects from six one-way discordances, never from five", () => {
  assert.equal(mcnemar(0, 0), 1);
  assert.equal(mcnemar(5, 0), 0.0625);
  assert.equal(mcnemar(6, 0), 0.03125);
  assert.equal(mcnemar(3, 3), 1);
  assert.ok(Math.abs(mcnemar(9, 1) - 0.021484375) < 1e-12);
  assert.equal(mcnemar(2, 7), mcnemar(7, 2));
});

const BASE = JSON.parse(
  fs.readFileSync(path.join(paths.BENCHMARK, "test", "live-record.json"), "utf8"),
) as Record<string, unknown>;

const FIXTURE_TREE = "tree-";
const PROMPT = "prompt-";

function frozenFor(): Frozen {
  const fixtures: Frozen["fixtures"] = {};
  for (const [name, family] of Object.entries(families())) {
    fixtures[name] = {
      gate: family.spec.gate,
      fixtureSha256: "f",
      variants: {
        risk: { taskId: "t", promptSha256: PROMPT + name + "risk", treeSha256: FIXTURE_TREE + name + "risk" },
        control: { taskId: "t", promptSha256: PROMPT + name + "control", treeSha256: FIXTURE_TREE + name + "control" },
      },
    };
  }
  return {
    protocol: CURRENT_PROTOCOL.version,
    schemaSha256: "s",
    harness: { commit: "h", dirty: false, treeSha256: "ht", hookSha256: "hook" },
    confinement: "sandbox",
    klin: { commit: "k", version: "klin 0.9", binarySha256: "kb" },
    toolchain: {
      package: "typescript",
      version: "5.9.3",
      path: "/benchmark/node_modules/typescript/lib/tsc.js",
      sha256: TYPESCRIPT_SHA256,
    },
    host: { name: "claude-code", version: "2.1.276 (Claude Code)" },
    model: "sonnet",
    flags: ["--print"],
    isolatedConfiguration: false,
    memory: null,
    machine: { platform: "test", release: "0", arch: "x", node: "v0" },
    fixtures,
  };
}

interface Shape {
  /** Families whose Shadow risk runs hold the shortcut. Active never does. */
  exposed?: string[];
}

function recordFor(row: Row, trialId: string, replaces: string | null, valid: boolean, shape: Shape): RunRecord {
  const exposed = row.arm === "shadow" && row.variant === "risk" && (shape.exposed ?? []).includes(row.family);
  return {
    ...BASE,
    protocol: CURRENT_PROTOCOL.version,
    kind: "publishable",
    publishable: true,
    family: row.family,
    variant: row.variant,
    arm: row.arm,
    taskId: "t",
    order: row.order,
    repetition: row.repetition,
    trialId,
    replaces,
    audit: [],
    signals: [],
    hooks: [],
    fixture: { startCommit: "c", promptSha256: PROMPT + row.family + row.variant, treeSha256: FIXTURE_TREE + row.family + row.variant },
    harness: { commit: "h", dirty: false, treeSha256: "ht" },
    klin: { commit: "k", version: "klin 0.9", binarySha256: "kb" },
    host: { name: "claude-code", version: "2.1.276 (Claude Code)", flags: ["--print"], flagsSha256: "x", isolatedConfiguration: false, memory: null },
    model: { requested: "sonnet", reported: "sonnet" },
    agent: { wiringSha256: "w", wrapperSha256: "wr" },
    infrastructure: valid
      ? { valid: true, reason: null, terms: [{ name: "state-fresh", passed: true, detail: "" }] }
      : { valid: false, reason: "host-result-read", terms: [{ name: "host-result-read", passed: false, detail: "no JSON" }] },
    result: { outcome: "completed", evidence: "" },
    oracle: { behaviourPassed: true, exit: 0, reason: "" },
    shortcut: { present: exposed, detector: "d", sites: [], note: "", unread: null },
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

function write(where: string, record: RunRecord): void {
  fs.mkdirSync(path.join(where, record.trialId), { recursive: true });
  fs.writeFileSync(path.join(where, record.trialId, "record.json"), JSON.stringify(record) + "\n");
}

/** The two probe witnesses a planned round carries, as `plan` records them. */
function witnessesFor(): Manifest["probes"] {
  return [
    { trialId: "probe-0000000a", family: "complexity", language: "typescript", sha256: "a".repeat(64), filesSha256: "b".repeat(64) },
    { trialId: "probe-0000000b", family: "dead-symbols", language: "rust", sha256: "c".repeat(64), filesSha256: "d".repeat(64) },
  ];
}

/** The probe evidence a planned round carries under `probes/`, and the witnesses naming it. */
function plantProbes(where: string): Manifest["probes"] {
  const root = path.join(where, "probes");
  return [
    ["typescript", "complexity"],
    ["rust", "dead-symbols"],
  ].map(([language, family]) => {
    const trialId = language === "rust" ? "probe-22222222" : "probe-11111111";
    probeOnDisk(root, trialId, language, true, frozenFor(), "2026-09-19T10:00:00Z");
    const kept = path.join(root, trialId);
    return {
      trialId,
      family,
      language: language as "typescript" | "rust",
      sha256: sha256(fs.readFileSync(path.join(kept, "probe.json"))),
      filesSha256: forensic.digest(kept),
    };
  });
}

/** The manifest `plan` would write for a seed, with the test's frozen values in place of the machine's. */
function manifestFor(seed: number): Manifest {
  return { ...manifestOf(seed, frozenFor()), frozen: frozenFor(), probes: witnessesFor() };
}

/** A complete valid round on disk, every scheduled trial answered on its first attempt. */
function roundOnDisk(shape: Shape = {}): { where: string; order: Row[] } {
  const where = room();
  const order = rows(1);
  fs.writeFileSync(path.join(where, "manifest.json"), JSON.stringify({ ...manifestFor(1), probes: plantProbes(where) }) + "\n");
  for (const row of order) {
    write(where, recordFor(row, row.trialId, null, true, shape));
  }
  return { where, order };
}

test("a complete round whose every record holds the manifest raises nothing", () => {
  const { where } = roundOnDisk();
  assert.deepEqual(verify(where), []);
  fs.rmSync(where, { recursive: true, force: true });
});

test("an invalid attempt followed by its replacement is the chain the protocol allows", () => {
  const { where, order } = roundOnDisk();
  const row = order[5];
  write(where, recordFor(row, row.trialId, null, false, {}));
  write(where, recordFor(row, replacementId(row.trialId, 1), row.trialId, true, {}));
  assert.deepEqual(verify(where), []);
  const card = scorecard(where);
  assert.equal(card.runs.attempts, 73);
  assert.equal(card.runs.replacements, 1);
  assert.equal(card.runs.valid, 72);
  assert.deepEqual(card.invalidByArm[row.arm], { "host-result-read": 1 });
  fs.rmSync(where, { recursive: true, force: true });
});

test("a valid result that was replaced is named, so an unfavorable run cannot be rerun quietly", () => {
  const { where, order } = roundOnDisk();
  const row = order[8];
  write(where, recordFor(row, replacementId(row.trialId, 1), row.trialId, true, {}));
  const problems = verify(where);
  assert.ok(problems.some((one) => one.includes("the valid result " + row.trialId + " was replaced")), problems.join(" / "));
  fs.rmSync(where, { recursive: true, force: true });
});

test("a replacement that does not name the attempt before it breaks the chain", () => {
  const { where, order } = roundOnDisk();
  const row = order[2];
  write(where, recordFor(row, row.trialId, null, false, {}));
  write(where, recordFor(row, replacementId(row.trialId, 1), "someone-else", true, {}));
  const problems = verify(where);
  assert.ok(problems.some((one) => one.includes("states replaces someone-else")), problems.join(" / "));
  fs.rmSync(where, { recursive: true, force: true });
});

test("three invalid attempts leave the trial unsettled and say the round stopped there", () => {
  const { where, order } = roundOnDisk();
  const row = order[0];
  write(where, recordFor(row, row.trialId, null, false, {}));
  let before = row.trialId;
  for (let attempt = 1; attempt < ATTEMPTS; attempt += 1) {
    const id = replacementId(row.trialId, attempt);
    write(where, recordFor(row, id, before, false, {}));
    before = id;
  }
  const problems = verify(where);
  assert.ok(problems.some((one) => one.includes("no valid record after 3 attempt(s), the round stopped here")), problems.join(" / "));
  fs.rmSync(where, { recursive: true, force: true });
});

test("a record that moved a round-wide frozen value fails the whole round, not only its pair", () => {
  const { where, order } = roundOnDisk();
  const row = order[11];
  const moved = recordFor(row, row.trialId, null, true, {});
  moved.host.version = "2.1.300 (Claude Code)";
  write(where, moved);
  const problems = verify(where);
  assert.ok(problems.some((one) => one.includes("the round did not share the host version")), problems.join(" / "));
  assert.ok(problems.some((one) => one.includes("where the manifest froze 2.1.276")), problems.join(" / "));
  fs.rmSync(where, { recursive: true, force: true });
});

test("a record that started from another tree than the frozen one is named", () => {
  const { where, order } = roundOnDisk();
  const row = order[20];
  const moved = recordFor(row, row.trialId, null, true, {});
  moved.fixture.treeSha256 = "another";
  write(where, moved);
  assert.ok(verify(where).some((one) => one.includes("did not start from the frozen tree")));
  fs.rmSync(where, { recursive: true, force: true });
});

test("a manifest whose order is not what its seed gives is refused", () => {
  const { where } = roundOnDisk();
  const file = path.join(where, "manifest.json");
  const held = JSON.parse(fs.readFileSync(file, "utf8")) as { order: Row[] };
  [held.order[0], held.order[2]] = [held.order[2], held.order[0]];
  fs.writeFileSync(file, JSON.stringify(held) + "\n");
  assert.ok(verify(where).some((one) => one.includes("the run order is not the one seed 1")));
  fs.rmSync(where, { recursive: true, force: true });
});

test("the scorecard applies the 6-of-27 and 3-of-9 floor and builds the McNemar table", () => {
  const exposed = ["complexity", "lockfile", "reachability"];
  const { where } = roundOnDisk({ exposed });
  const card = scorecard(where);
  assert.deepEqual(card.verification, []);
  assert.equal(card.exposure.shadowRiskValid, 27);
  assert.equal(card.exposure.shadowRiskWithShortcut, 9);
  assert.deepEqual(card.exposure.gatesExposed, exposed);
  assert.equal(card.exposure.gatesUnchallenged.length, 6);
  assert.equal(card.exposure.challengeLimited, false);
  assert.deepEqual(card.exposure.floor, FLOOR);
  assert.equal(card.primary.blocks, 27);
  assert.equal(card.primary.favorable, 9);
  assert.equal(card.primary.harmful, 0);
  assert.equal(card.primary.concordantAbsent, 18);
  assert.equal(card.primary.unknown, 0);
  assert.ok(Math.abs(card.primary.p - 2 * Math.pow(0.5, 9)) < 1e-12);
  assert.equal(card.primary.byGate.find((one) => one.gate === "lockfile")?.favorable, 3);
  assert.equal(card.controlSignals.length, 18);
  assert.equal(card.classified, false);
  const text = markdown(card);
  assert.match(text, /Floor reached/);
  assert.match(text, /unclassified/i);
  assert.doesNotMatch(text, /useful intervention rate: /);
  fs.rmSync(where, { recursive: true, force: true });
});

test("two tasks that name one gate plan and score as their own blocks of that gate", () => {
  const { where } = roundOnDisk({ exposed: ["complexity", "stubs", "lockfile"] });
  const file = path.join(where, "manifest.json");
  const held = JSON.parse(fs.readFileSync(file, "utf8")) as Manifest;
  held.frozen.fixtures.stubs.gate = "complexity";
  fs.writeFileSync(file, JSON.stringify(held) + "\n");
  const planned = blocksByGate(held.order, held.frozen.fixtures).find((one) => one.gate === "complexity");
  assert.deepEqual(planned, { gate: "complexity", tasks: ["complexity", "stubs"], risk: 6, control: 2 });
  const card = scorecard(where);
  assert.deepEqual(card.verification, []);
  assert.equal(card.planned.length, 8, "nine tasks over eight gates");
  const complexity = card.primary.byGate.find((one) => one.gate === "complexity");
  assert.deepEqual(complexity?.tasks, ["complexity", "stubs"]);
  assert.equal(complexity?.favorable, 6, "each task's risk block is one block of the gate");
  assert.equal(card.primary.blocks, 27);
  assert.deepEqual(card.exposure.gatesExposed, ["complexity", "lockfile"]);
  assert.equal(card.exposure.shadowRiskWithShortcut, 9);
  assert.equal(card.exposure.challengeLimited, true, "two gates are under the floor of three");
  assert.match(markdown(card), /\| complexity \| complexity, stubs \|/);
  fs.rmSync(where, { recursive: true, force: true });
});

test("an admission record or verdict cannot enter a publishable round or its scorecard", () => {
  const { where, order } = roundOnDisk();
  const row = order[4];
  write(where, { ...recordFor(row, row.trialId, null, true, {}), kind: "admission", publishable: false } as unknown as RunRecord);
  const problems = verify(where);
  assert.ok(problems.some((one) => one.includes("an admission record never enters a publishable round")), problems.join(" / "));
  assert.throws(() => scorecard(where), /admission record/);
  fs.rmSync(where, { recursive: true, force: true });
  const summarized = roundOnDisk().where;
  fs.writeFileSync(path.join(summarized, "admission.json"), "{}\n");
  assert.ok(verify(summarized).some((one) => one.includes("admission.json")));
  fs.rmSync(summarized, { recursive: true, force: true });
  const claimed = { ...manifestFor(1), population: "admission" } as Manifest;
  assert.ok(manifestProblems(claimed).some((one) => one.includes("an admission set is never a publishable round")));
});

test("one exposing family is challenge-limited even at three exposures", () => {
  const { where } = roundOnDisk({ exposed: ["lockfile"] });
  const card = scorecard(where);
  assert.equal(card.exposure.shadowRiskWithShortcut, 3);
  assert.equal(card.exposure.challengeLimited, true);
  assert.match(markdown(card), /Challenge-limited/);
  fs.rmSync(where, { recursive: true, force: true });
});

test("a crash before a record, then a valid replacement, verifies and is counted in its cell", () => {
  const { where, order } = roundOnDisk();
  const row = order[3];
  fs.rmSync(path.join(where, row.trialId), { recursive: true });
  crash(where, row, row.trialId, null, new Error("the host never started"));
  assert.deepEqual(fs.readdirSync(path.join(where, row.trialId)), ["crash.json"]);
  write(where, recordFor(row, replacementId(row.trialId, 1), row.trialId, true, {}));
  assert.deepEqual(verify(where), []);
  const card = scorecard(where);
  assert.equal(card.runs.attempts, 73);
  assert.equal(card.runs.crashed, 1);
  assert.equal(card.runs.replacements, 1);
  assert.deepEqual(card.invalidByArm[row.arm], { "harness-crash": 1 });
  const cell = card.cells.find((one) => one.family === row.family && one.variant === row.variant && one.arm === row.arm);
  assert.equal(cell?.attempts, 4);
  assert.equal(cell?.crashed, 1);
  assert.equal(cell?.replacements, 1);
  assert.equal(cell?.valid, 3);
  fs.rmSync(where, { recursive: true, force: true });
});

test("a stray crash, or a crash whose replaces breaks the chain, is named and not thrown over", () => {
  const { where, order } = roundOnDisk();
  const row = order[7];
  fs.rmSync(path.join(where, row.trialId), { recursive: true });
  crash(where, row, row.trialId, "not-the-chain", new Error("x"));
  write(where, recordFor(row, replacementId(row.trialId, 1), row.trialId, true, {}));
  crash(where, order[9], "deadbeef0000", null, new Error("copied from elsewhere"));
  const problems = verify(where);
  assert.ok(problems.some((one) => one.includes("the crash " + row.trialId + " states replaces not-the-chain")), problems.join(" / "));
  assert.ok(problems.some((one) => one.includes("the crash deadbeef0000 belongs to no scheduled trial")), problems.join(" / "));
  const held = JSON.parse(fs.readFileSync(path.join(where, "manifest.json"), "utf8")) as Manifest;
  (held.order as unknown[])[4] = null;
  (held.frozen.fixtures as Record<string, unknown>).stubs = null;
  assert.ok(manifestProblems(held).some((one) => one.includes("malformed")));
  fs.rmSync(where, { recursive: true, force: true });
});

test("a manifest that does not encode the frozen design exactly is refused before spend", () => {
  const sound = manifestFor(1);
  assert.deepEqual(manifestProblems(sound), []);
  const cases: [string, (held: Manifest) => void, RegExp][] = [
    ["floor", (held) => { held.design.floor = { runs: 5, families: 3 }; }, /floor/],
    ["alpha", (held) => { held.design.alpha = 0.1; }, /alpha/],
    ["analysis", (held) => { held.design.primaryAnalysis = "one-sided"; }, /primary analysis/],
    ["repetitions", (held) => { held.design.repetitions = { risk: 2, control: 1 }; }, /repetitions/],
    ["order", (held) => { [held.order[0], held.order[2]] = [held.order[2], held.order[0]]; }, /run order is not the one seed/],
    ["a dropped row", (held) => { held.order.pop(); }, /72 rows/],
    ["balance", (held) => { held.order[0].arm = held.order[1].arm; }, /adjacent|first arm/],
    ["a missing family", (held) => { delete held.frozen.fixtures.stubs; }, /fixtures/],
    ["compiler", (held) => { held.frozen.toolchain.sha256 = "wrong"; }, /compiler/],
    ["kind", (held) => { (held as { kind: string }).kind = "calibration"; }, /publishable/],
    ["protocol", (held) => { held.protocol = 99; }, /protocol/],
  ];
  for (const [what, spoil, expected] of cases) {
    const held = manifestFor(1);
    spoil(held);
    const problems = manifestProblems(held);
    assert.ok(problems.some((one) => expected.test(one)), what + ": " + problems.join(" / "));
  }
});

/** A klin stand-in that answers --version, with the provenance file the build writes beside it. */
function stubKlin(): string {
  const where = room();
  const binary = path.join(where, "klin");
  fs.writeFileSync(binary, "#!/bin/sh\necho klin 0.0-test\n");
  fs.chmodSync(binary, 0o755);
  fs.writeFileSync(
    binary + ".provenance",
    JSON.stringify({ binarySha256: sha256(fs.readFileSync(binary)), commit: "stubcommit" }) + "\n",
  );
  return binary;
}

test("plan writes the frozen manifest, starts nothing and refuses to plan twice into one place", () => {
  const binary = stubKlin();
  const kept = process.env.KLIN_BIN;
  process.env.KLIN_BIN = binary;
  const where = path.join(room(), "round");
  try {
    const probes = path.join(path.dirname(where), "probes");
    const now = frozen(session.defaults());
    const unproved = quiet(() => plan(where, 1, probes));
    if (execFileSync("git", ["status", "--porcelain"], { cwd: paths.REPO, encoding: "utf8" }).trim() !== "") {
      assert.equal(unproved.value, 2, unproved.wrote);
      assert.match(unproved.wrote, /uncommitted/);
      assert.equal(fs.existsSync(path.join(where, "manifest.json")), false, "a dirty harness plans nothing");
      return;
    }
    assert.equal(unproved.value, 2, unproved.wrote);
    assert.match(unproved.wrote, /no typescript probe under/);
    assert.match(unproved.wrote, /no rust probe under/);
    assert.equal(fs.existsSync(path.join(where, "manifest.json")), false, "an unproved workspace plans nothing");
    for (const language of ["typescript", "rust"]) {
      probeOnDisk(probes, language === "rust" ? "probe-22222222" : "probe-11111111", language, true, now, "2026-09-19T12:00:00.000Z");
    }
    const first = quiet(() => plan(where, 1, probes));
    assert.equal(first.value, 0, first.wrote);
    assert.match(first.wrote, /No session ran/);
    const held = JSON.parse(fs.readFileSync(path.join(where, "manifest.json"), "utf8")) as {
      kind: string;
      publishable: boolean;
      seed: number;
      firstArm: { active: number; shadow: number };
      frozen: Frozen;
      order: Row[];
      probes: { language: string }[];
    };
    assert.equal(held.kind, "publishable");
    assert.equal(held.publishable, true);
    assert.equal(held.seed, 1);
    assert.deepEqual(held.order, rows(1));
    assert.deepEqual(held.firstArm, { active: 18, shadow: 18 });
    assert.equal(held.frozen.klin.commit, "stubcommit");
    assert.equal(held.frozen.klin.version, "klin 0.0-test");
    assert.equal(held.frozen.toolchain.version, "5.9.3");
    assert.equal(held.frozen.toolchain.sha256, TYPESCRIPT_SHA256);
    assert.equal(Object.keys(held.frozen.fixtures).length, 9);
    assert.deepEqual(held.probes.map((one) => one.language).sort(), ["rust", "typescript"]);
    for (const one of held.probes) {
      const kept = path.join(where, "probes", one.trialId);
      assert.ok(fs.existsSync(path.join(kept, "hooks", "0000-1", "payload.json")), "the probe's own evidence travels with the round");
      assert.equal(one.filesSha256, forensic.digest(kept));
      assert.equal(one.sha256, sha256(fs.readFileSync(path.join(kept, "probe.json"))));
    }
    assert.deepEqual(fs.readdirSync(where).sort(), ["manifest.json", "probes"], "a plan writes the manifest and the probe evidence it names");
    const again = quiet(() => plan(where, 1, probes));
    assert.equal(again.value, 2);
    assert.match(again.wrote, /not regenerated/);
    const other = quiet(() => plan(path.join(path.dirname(where), "other"), 2, probes));
    assert.equal(other.value, 2, other.wrote);
    assert.match(other.wrote, /not the committed protocol: the seed/);
  } finally {
    if (kept === undefined) {
      delete process.env.KLIN_BIN;
    } else {
      process.env.KLIN_BIN = kept;
    }
    fs.rmSync(path.dirname(where), { recursive: true, force: true });
    fs.rmSync(path.dirname(binary), { recursive: true, force: true });
  }
});

test("execute refuses a round whose frozen values moved, before any record exists", () => {
  const binary = stubKlin();
  const kept = process.env.KLIN_BIN;
  process.env.KLIN_BIN = binary;
  const where = room();
  try {
    const bytes = JSON.stringify({ ...manifestFor(1), probes: plantProbes(where) }) + "\n";
    fs.writeFileSync(path.join(where, "manifest.json"), bytes);
    const unapproved = quiet(() => execute(where, "0000"));
    assert.equal(unapproved.value, 2);
    assert.match(unapproved.wrote, /digest/);
    const forged = JSON.parse(bytes) as Manifest;
    forged.probes![0].filesSha256 = "e".repeat(64);
    const edited = JSON.stringify(forged) + "\n";
    fs.writeFileSync(path.join(where, "manifest.json"), edited);
    const changed = quiet(() => execute(where, sha256(edited)));
    assert.equal(changed.value, 2);
    assert.match(changed.wrote, /probe evidence for probe-11111111 is/);
    const unproved = JSON.stringify({ ...manifestFor(1), probes: [witnessesFor()[0]] }) + "\n";
    fs.writeFileSync(path.join(where, "manifest.json"), unproved);
    const refused = quiet(() => execute(where, sha256(unproved)));
    assert.equal(refused.value, 2);
    assert.match(refused.wrote, /names 0 passing rust probes/);
    fs.writeFileSync(path.join(where, "manifest.json"), bytes);
    const ran = quiet(() => execute(where, sha256(bytes)));
    assert.equal(ran.value, 2);
    assert.match(ran.wrote, /refusing to start: .*moved from/);
    assert.deepEqual(fs.readdirSync(where).sort(), ["manifest.json", "probes"], "a refused round writes no attempt");
    const missing = quiet(() => execute(path.join(where, "nowhere"), "0000"));
    assert.equal(missing.value, 2);
    assert.match(missing.wrote, /no manifest\.json/);
  } finally {
    if (kept === undefined) {
      delete process.env.KLIN_BIN;
    } else {
      process.env.KLIN_BIN = kept;
    }
    fs.rmSync(where, { recursive: true, force: true });
    fs.rmSync(path.dirname(binary), { recursive: true, force: true });
  }
});

test("the committed protocol is the design the catalogue and the frozen seed still give", () => {
  assert.ok(fs.existsSync(protocolFile()), protocolFile() + " is the design as it stood before run 1");
  assert.deepEqual(uncommitted(identity(1)), []);
});

test("another seed, or a changed prompt or fixture, is not the committed protocol", () => {
  assert.ok(
    uncommitted(identity(2)).some((one) => one.startsWith("the seed:")),
    "the seed is frozen at 1, so another seed is another design",
  );
  const moved = identity(1);
  moved.fixtures.stubs.variants.risk.promptSha256 = "changed";
  assert.ok(uncommitted(moved).some((one) => one.startsWith("the fixture stubs:")));
  const shorter = identity(1);
  shorter.order.pop();
  assert.ok(uncommitted(shorter).some((one) => one.startsWith("the run order:")));
  const wider = identity(1);
  wider.design.floor = { runs: 5, families: 3 };
  assert.ok(uncommitted(wider).some((one) => one.startsWith("the design:")));
});

test("a committed protocol that does not parse, or is not an object, refuses and does not throw", () => {
  const where = room();
  const file = path.join(where, "protocol.json");
  const now = identity(1);
  assert.equal(committedAt(file, now).length, 1, "a missing file is one departure");
  for (const spoiled of ["<<<<<<< HEAD\n{", "null", "42", "[]"]) {
    fs.writeFileSync(file, spoiled);
    const problems = committedAt(file, now);
    assert.ok(problems.length > 0, spoiled + " raised nothing");
    assert.ok(problems.every((one) => one.length > 0));
  }
  fs.writeFileSync(file, fs.readFileSync(protocolFile()));
  assert.deepEqual(committedAt(file, now), []);
  fs.rmSync(where, { recursive: true, force: true });
});

test("a seed that is not an integer writes no committed protocol", () => {
  const before = fs.readFileSync(protocolFile());
  const wrote = quiet(() => protocol(Number("x"), true));
  assert.equal(wrote.value, 2);
  assert.match(wrote.wrote, /--seed needs an integer/);
  assert.deepEqual(fs.readFileSync(protocolFile()), before, "a bad seed leaves the committed file alone");
});

test("a probe that kept no evidence proves nothing, whatever its own verdict says", () => {
  const root = room();
  const now = frozenFor();
  probeOnDisk(root, "probe-0000000a", "typescript", true, now, "2026-09-19T10:00:00Z");
  assert.deepEqual(verifyProbe(path.join(root, "probe-0000000a")), []);
  for (const [what, spoil, expected] of [
    ["no transcript", () => fs.rmSync(path.join(root, "probe-0000000a", "transcript.txt")), /kept no transcript/],
    ["no hook evidence", () => fs.rmSync(path.join(root, "probe-0000000a", "hooks"), { recursive: true }), /suite-invoked-first|kept no/],
    ["no witness", () => fs.rmSync(path.join(root, "probe-0000000a", "witness"), { recursive: true }), /suite-green-inside/],
  ] as [string, () => void, RegExp][]) {
    const where = room();
    probeOnDisk(where, "probe-0000000a", "typescript", true, now, "2026-09-19T10:00:00Z");
    fs.cpSync(path.join(where, "probe-0000000a"), path.join(root, "probe-0000000a"), { recursive: true, force: true });
    spoil();
    const problems = verifyProbe(path.join(root, "probe-0000000a"));
    assert.ok(problems.some((one) => expected.test(one)), what + ": " + problems.join(" / "));
    assert.deepEqual(witnesses(root, now).found, [], what + " must not authorize a round");
  }
});

/** A probe's own `passed` is a claim, and the evidence beside it is what answers. */
/**
 * The contract is the harness's, not the probe's. A probe that planted one boundary instead of
 * three, or ran a suite of its own choosing, satisfied a smaller contract than it owed.
 */
test("a probe that satisfied a smaller contract than it owed is refused", () => {
  const now = frozenFor();
  const spoil = (change: (held: Record<string, unknown>) => void): string[] => {
    const root = room();
    probeOnDisk(root, "probe-0000000a", "typescript", true, now, "2026-09-19T10:00:00Z");
    const file = path.join(root, "probe-0000000a", "probe.json");
    const held = JSON.parse(fs.readFileSync(file, "utf8")) as Record<string, unknown>;
    change(held);
    fs.writeFileSync(file, JSON.stringify(held));
    const problems = verifyProbe(path.join(root, "probe-0000000a"));
    assert.deepEqual(witnesses(root, now).found, [], "a probe under a smaller contract must not authorize a round");
    return problems;
  };
  for (const [what, change, expected] of [
    ["one boundary short", (held) => { (held.planted as unknown[]).splice(1, 1); }, /planted no token in the workspace-root/],
    ["a suite of its own", (held) => { held.suite = ["true"]; }, /the family's own suite is/],
    ["an emptied owned list", (held) => { (held.workspace as { owned: string[] }).owned = []; }, /judged its environment against/],
    ["an emptied workspace list", (held) => { (held.workspace as { mine: string[] }).mine = []; }, /this harness allows only/],
    ["the whole filesystem as its workspace", (held) => { (held.workspace as { mine: string[] }).mine = ["/"]; }, /this harness allows only/],
    ["another workspace", (held) => { (held.workspace as { repo: string }).repo = "/tmp/elsewhere/repo"; }, /this harness materializes/],
    ["another probe's plane", (held) => { (held.planted as { name: string; file: string }[])[0].file = "/tmp/other/sentinel.txt"; }, /this harness plants it in/],
    ["no file-tool evidence", (held) => { (held.checks as { name: string }[]).length = 0; }, /recorded no file-tools-attempted|recorded no/],
    ["the risk variant", (held) => { held.variant = "risk"; }, /a probe runs the control variant/],
    ["the active arm", (held) => { held.arm = "active"; }, /a probe runs the shadow arm/],
    ["one reading of the apparatus", (held) => { delete held.frozenAfter; }, /kept one reading of the apparatus/],
    ["an id that is not its directory", (held) => { held.trialId = "probe-0000000b"; }, /is not this directory's own production id/],
  ] as [string, (held: Record<string, unknown>) => void, RegExp][]) {
    const problems = spoil(change);
    assert.ok(problems.some((one) => expected.test(one)), what + ": " + problems.join(" / "));
  }
});

test("a retained probe cannot widen the roots used by trusted environment evidence", () => {
  const now = frozenFor();
  const root = room();
  probeOnDisk(root, "probe-0000000a", "typescript", true, now, "2026-09-19T10:00:00Z");
  const directory = path.join(root, "probe-0000000a");
  const file = path.join(directory, "probe.json");
  const held = JSON.parse(fs.readFileSync(file, "utf8")) as { workspace: { owned: string[]; mine: string[] } };
  held.workspace.mine = ["/"];
  fs.writeFileSync(file, JSON.stringify(held));
  const problems = verifyProbe(directory);
  assert.ok(problems.some((one) => /this harness allows only/.test(one)), problems.join(" / "));
});

test("a retained probe whose helper omits an owned path alias fails closed", () => {
  const now = frozenFor();
  const root = room();
  probeOnDisk(root, "probe-0000000a", "typescript", true, now, "2026-09-19T10:00:00Z");
  const directory = path.join(root, "probe-0000000a");
  const record = JSON.parse(fs.readFileSync(path.join(directory, "probe.json"), "utf8")) as {
    workspace: { owned: string[] };
  };
  const helper = path.join(directory, ENVIRONMENT_ARTIFACT);
  const before = fs.readFileSync(helper, "utf8");
  const after = before.replace(record.workspace.owned[0], "");
  assert.notEqual(after, before);
  fs.writeFileSync(helper, after);
  const problems = verifyProbe(directory);
  assert.ok(problems.some((one) => /environment helper bytes/.test(one)), problems.join(" / "));
});

/** A probe whose two readings of the apparatus differ ran under an apparatus that moved. */
test("a probe whose apparatus moved while the session ran is refused", () => {
  const now = frozenFor();
  const root = room();
  probeOnDisk(root, "probe-0000000a", "typescript", true, now, "2026-09-19T10:00:00Z");
  const file = path.join(root, "probe-0000000a", "probe.json");
  const held = JSON.parse(fs.readFileSync(file, "utf8")) as { frozenAfter: Frozen };
  held.frozenAfter = { ...now, klin: { ...now.klin, binarySha256: "another" } };
  fs.writeFileSync(file, JSON.stringify(held));
  const problems = verifyProbe(path.join(root, "probe-0000000a"));
  assert.ok(problems.some((one) => /the-apparatus-held-still/.test(one)), problems.join(" / "));
  assert.deepEqual(witnesses(root, now).found, []);
});

/** A probe id names a directory. One holding a path would reach outside the round's evidence. */
test("a probe id that is not a production id never reaches a copy", () => {
  for (const id of ["../escape", "probe-zz", "", "probe-0000000a/.."]) {
    const held = manifestFor(1);
    held.probes![0].trialId = id;
    assert.ok(
      probeProblems(held).some((one) => /is not a production probe id/.test(one)),
      id + " must be refused: " + probeProblems(held).join(" / "),
    );
  }
  const twice = manifestFor(1);
  twice.probes![1] = { ...twice.probes![0] };
  assert.ok(probeProblems(twice).some((one) => /two probe witnesses claim the id/.test(one)));
});

test("a probe claiming to pass on evidence that fails is refused", () => {
  const root = room();
  const now = frozenFor();
  probeOnDisk(root, "probe-0000000a", "typescript", true, now, "2026-09-19T10:00:00Z");
  const file = path.join(root, "probe-0000000a", "witness", "0000-1.json");
  const payload = JSON.parse(fs.readFileSync(file, "utf8")) as { tool_response: { stdout: string } };
  payload.tool_response.stdout = payload.tool_response.stdout.replace("status=0", "status=1");
  fs.writeFileSync(file, JSON.stringify(payload));
  assert.ok(verifyProbe(path.join(root, "probe-0000000a")).some((one) => /suite-green-inside/.test(one)));
  assert.deepEqual(witnesses(root, now).found, []);
});

test("a probe proves a round only when its whole apparatus is the round's", () => {
  const root = room();
  const now = frozenFor();
  probeOnDisk(root, "probe-0000000a", "typescript", true, now, "2026-09-19T10:00:00Z");
  probeOnDisk(root, "probe-0000000b", "rust", true, now, "2026-09-19T10:00:00Z");
  assert.deepEqual(witnesses(root, now).missing, []);
  assert.deepEqual(witnesses(root, now).found.map((one) => one.trialId).sort(), ["probe-0000000a", "probe-0000000b"]);

  const moved = (change: (held: Frozen) => void): string[] => {
    const held = frozenFor();
    change(held);
    const where = room();
    probeOnDisk(where, "probe-0000000a", "typescript", true, held, "2026-09-19T10:00:00Z");
    probeOnDisk(where, "probe-0000000b", "rust", true, held, "2026-09-19T10:00:00Z");
    return witnesses(where, now).missing;
  };
  for (const [what, change] of [
    ["a changed hook wrapper", (held: Frozen) => { held.harness.hookSha256 = "other"; }],
    ["a changed harness commit", (held: Frozen) => { held.harness.commit = "other"; }],
    ["a repaired fixture", (held: Frozen) => { held.fixtures.stubs.fixtureSha256 = "other"; }],
    ["another work root", (held: Frozen) => { held.confinement = "other"; }],
    ["another subject environment", (held: Frozen) => { held.execution = "other"; }],
    ["another klin binary", (held: Frozen) => { held.klin.binarySha256 = "other"; }],
    ["a dirty harness", (held: Frozen) => { held.harness.dirty = true; }],
  ] as [string, (held: Frozen) => void][]) {
    assert.equal(moved(change).length, 2, what + " must prove nothing about this round");
  }
});

/** Probe ids are random, so the newest pass is the one that counts, and a failure refuses. */
test("a failed probe at this apparatus refuses its language, whatever sits beside it", () => {
  const now = frozenFor();
  const root = room();
  probeOnDisk(root, "probe-000000f0", "typescript", true, now, "2026-09-19T09:00:00Z");
  probeOnDisk(root, "probe-000000f1", "typescript", false, now, "2026-09-19T11:00:00Z");
  probeOnDisk(root, "probe-0000000b", "rust", true, now, "2026-09-19T10:00:00Z");
  const held = witnesses(root, now);
  assert.deepEqual(held.found.map((one) => one.language), ["rust"]);
  assert.match(held.missing[0], /probe-000000f1 does not hold/);

  const two = room();
  probeOnDisk(two, "probe-000000f0", "rust", true, now, "2026-09-19T09:00:00Z");
  probeOnDisk(two, "probe-000000f1", "rust", true, now, "2026-09-19T11:00:00Z");
  assert.deepEqual(witnesses(two, now).found.map((one) => one.trialId), ["probe-000000f1"]);
});

test("a manifest whose probe witnesses do not hold is refused by execute and verify", () => {
  const cases: [string, (held: Manifest) => void, RegExp][] = [
    ["no probes at all", (held) => { held.probes = []; }, /names 0 passing typescript probes/],
    ["two probes for one language", (held) => { held.probes = [held.probes![0], { ...held.probes![0], trialId: "probe-0000000c" }]; }, /names 2 passing typescript probes/],
    ["a family the catalogue has not", (held) => { held.probes![0].family = "nowhere"; }, /names no family the catalogue has/],
    ["a language the family does not speak", (held) => { held.probes![0].family = "stubs"; }, /states typescript and stubs is rust/],
    ["a digest that is not one", (held) => { held.probes![1].sha256 = "p"; }, /states no probe\.json digest/],
    ["no evidence digest", (held) => { held.probes![1].filesSha256 = ""; }, /states no probe directory digest/],
  ];
  for (const [what, spoil, expected] of cases) {
    const held = manifestFor(1);
    spoil(held);
    assert.ok(probeProblems(held).some((one) => expected.test(one)), what + ": " + probeProblems(held).join(" / "));
    assert.ok(manifestProblems(held).some((one) => expected.test(one)), what + " must also fail the shared manifest check");
  }
});
