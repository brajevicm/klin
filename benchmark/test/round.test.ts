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
import {
  ATTEMPTS,
  FLOOR,
  crash,
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
  uncommitted,
  verify,
  type Frozen,
  type Manifest,
  type Row,
} from "../src/round.ts";
import { execFileSync } from "node:child_process";
import type { RunRecord } from "../src/record.ts";
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
    harness: { commit: "h", dirty: false, treeSha256: "ht" },
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

/** The manifest `plan` would write for a seed, with the test's frozen values in place of the machine's. */
function manifestFor(seed: number): Manifest {
  return { ...manifestOf(seed, frozenFor()), frozen: frozenFor() };
}

/** A complete valid round on disk, every scheduled trial answered on its first attempt. */
function roundOnDisk(shape: Shape = {}): { where: string; order: Row[] } {
  const where = room();
  const order = rows(1);
  fs.writeFileSync(path.join(where, "manifest.json"), JSON.stringify(manifestFor(1)) + "\n");
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
    const unproved = quiet(() => plan(where, 1, probes));
    if (execFileSync("git", ["status", "--porcelain"], { cwd: paths.REPO, encoding: "utf8" }).trim() !== "") {
      assert.equal(unproved.value, 2, unproved.wrote);
      assert.match(unproved.wrote, /uncommitted/);
      assert.equal(fs.existsSync(path.join(where, "manifest.json")), false, "a dirty harness plans nothing");
      return;
    }
    assert.equal(unproved.value, 2, unproved.wrote);
    assert.match(unproved.wrote, /no passing typescript probe/);
    assert.match(unproved.wrote, /no passing rust probe/);
    assert.equal(fs.existsSync(path.join(where, "manifest.json")), false, "an unproved workspace plans nothing");
    const now = frozen(session.defaults());
    for (const language of ["typescript", "rust"]) {
      fs.mkdirSync(path.join(probes, language), { recursive: true });
      fs.writeFileSync(
        path.join(probes, language, "probe.json"),
        JSON.stringify({
          trialId: language,
          family: language,
          language,
          host: now.host.version,
          harness: { treeSha256: now.harness.treeSha256 },
          klin: { binarySha256: now.klin.binarySha256 },
          passed: true,
        }),
      );
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
    assert.equal(fs.readdirSync(where).length, 1, "the manifest is the only thing a plan writes");
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
    const bytes = JSON.stringify(manifestFor(1)) + "\n";
    fs.writeFileSync(path.join(where, "manifest.json"), bytes);
    const unapproved = quiet(() => execute(where, "0000"));
    assert.equal(unapproved.value, 2);
    assert.match(unapproved.wrote, /digest/);
    const unproved = quiet(() => execute(where, sha256(bytes)));
    assert.equal(unproved.value, 2);
    assert.match(unproved.wrote, /names no passing typescript or rust probe/);
    const proved = JSON.stringify({
      ...manifestFor(1),
      probes: ["typescript", "rust"].map((language) => ({ trialId: language, family: language, language, sha256: "p" })),
    }) + "\n";
    fs.writeFileSync(path.join(where, "manifest.json"), proved);
    const ran = quiet(() => execute(where, sha256(proved)));
    assert.equal(ran.value, 2);
    assert.match(ran.wrote, /refusing to start: .*moved from/);
    assert.deepEqual(fs.readdirSync(where), ["manifest.json"]);
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
