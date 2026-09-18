import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "../src/paths.ts";
import { families } from "../src/catalogue.ts";
import { sha256 } from "../src/trees.ts";
import {
  ATTEMPTS,
  FLOOR,
  execute,
  markdown,
  mcnemar,
  plan,
  replacementId,
  rows,
  scorecard,
  verify,
  type Frozen,
  type Row,
} from "../src/round.ts";
import type { RunRecord } from "../src/record.ts";

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
    protocol: paths.PROTOCOL,
    schemaSha256: "s",
    harness: { commit: "h", dirty: false, treeSha256: "ht" },
    klin: { commit: "k", version: "klin 0.9", binarySha256: "kb" },
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
    protocol: paths.PROTOCOL,
    kind: "publishable",
    publishable: true,
    family: row.family,
    variant: row.variant,
    arm: row.arm,
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

/** A complete valid round on disk, every scheduled trial answered on its first attempt. */
function roundOnDisk(shape: Shape = {}): { where: string; order: Row[] } {
  const where = room();
  const order = rows(1);
  fs.writeFileSync(
    path.join(where, "manifest.json"),
    JSON.stringify({ protocol: paths.PROTOCOL, kind: "publishable", publishable: true, seed: 1, frozen: frozenFor(), order }) + "\n",
  );
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
  assert.deepEqual(card.exposure.familiesExposed, exposed);
  assert.equal(card.exposure.familiesUnchallenged.length, 6);
  assert.equal(card.exposure.challengeLimited, false);
  assert.deepEqual(card.exposure.floor, FLOOR);
  assert.equal(card.primary.blocks, 27);
  assert.equal(card.primary.favorable, 9);
  assert.equal(card.primary.harmful, 0);
  assert.equal(card.primary.concordantAbsent, 18);
  assert.equal(card.primary.unknown, 0);
  assert.ok(Math.abs(card.primary.p - 2 * Math.pow(0.5, 9)) < 1e-12);
  assert.equal(card.primary.byFamily.find((one) => one.family === "lockfile")?.favorable, 3);
  assert.equal(card.controlSignals.length, 18);
  assert.equal(card.classified, false);
  const text = markdown(card);
  assert.match(text, /Floor reached/);
  assert.match(text, /unclassified/i);
  assert.doesNotMatch(text, /useful intervention rate: /);
  fs.rmSync(where, { recursive: true, force: true });
});

test("one exposing family is challenge-limited even at three exposures", () => {
  const { where } = roundOnDisk({ exposed: ["lockfile"] });
  const card = scorecard(where);
  assert.equal(card.exposure.shadowRiskWithShortcut, 3);
  assert.equal(card.exposure.challengeLimited, true);
  assert.match(markdown(card), /Challenge-limited/);
  fs.rmSync(where, { recursive: true, force: true });
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
    const first = quiet(() => plan(where, 7));
    assert.equal(first.value, 0, first.wrote);
    assert.match(first.wrote, /No session ran/);
    const held = JSON.parse(fs.readFileSync(path.join(where, "manifest.json"), "utf8")) as {
      kind: string;
      publishable: boolean;
      seed: number;
      firstArm: { active: number; shadow: number };
      frozen: Frozen;
      order: Row[];
    };
    assert.equal(held.kind, "publishable");
    assert.equal(held.publishable, true);
    assert.equal(held.seed, 7);
    assert.deepEqual(held.order, rows(7));
    assert.deepEqual(held.firstArm, { active: 18, shadow: 18 });
    assert.equal(held.frozen.klin.commit, "stubcommit");
    assert.equal(held.frozen.klin.version, "klin 0.0-test");
    assert.equal(Object.keys(held.frozen.fixtures).length, 9);
    assert.equal(fs.readdirSync(where).length, 1, "the manifest is the only thing a plan writes");
    const again = quiet(() => plan(where, 7));
    assert.equal(again.value, 2);
    assert.match(again.wrote, /not regenerated/);
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
    const manifest = { protocol: paths.PROTOCOL, kind: "publishable", publishable: true, seed: 1, frozen: frozenFor(), order: rows(1) };
    fs.writeFileSync(path.join(where, "manifest.json"), JSON.stringify(manifest) + "\n");
    const ran = quiet(() => execute(where));
    assert.equal(ran.value, 2);
    assert.match(ran.wrote, /refusing to start: .*moved from/);
    assert.deepEqual(fs.readdirSync(where), ["manifest.json"]);
    const missing = quiet(() => execute(path.join(where, "nowhere")));
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
