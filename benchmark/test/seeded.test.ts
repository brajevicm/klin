import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "../src/paths.ts";
import {
  PLANTED,
  VARIANTS,
  cells,
  families,
  family,
  variantIn,
  variantNames,
} from "../src/catalogue.ts";
import { copyTree, digest, overlay, sha256 } from "../src/trees.ts";
import * as forensic from "../src/forensic.ts";
import { fixtures } from "../src/frozen.ts";
import { rows } from "../src/round.ts";
import * as round from "../src/round.ts";
import { wrongArguments } from "../src/cli.ts";
import {
  baseStampAsDeclared,
  seedIsTheOnlyChange,
  startTreeAsDeclared,
} from "../src/integrity.ts";
import { finalRepairOf, stopMetrics, targetStop, validate, type GateReport } from "../src/record.ts";
import { CURRENT_PROTOCOL, SEEDED_PROTOCOL } from "../src/protocol.ts";
import * as workspace from "../src/workspace.ts";
import * as oracle from "../src/oracle.ts";
import * as report from "../src/report.ts";
import * as integrity from "../src/integrity.ts";
import * as seededRound from "../src/seeded.ts";
import * as session from "../src/session.ts";
import * as trial from "../src/trial.ts";
import { probeOnDisk } from "./probe-fixture.ts";

/**
 * The seeded population: a variant the harness plants and no round schedules.
 *
 * A seeded trial separates three things the harness equated until #260: the committed tree, the
 * subject's starting tree and the detector's baseline. These cases hold that separation, and they
 * hold the natural population exactly where #259 froze it.
 */

const KLIN = process.env.KLIN_BIN ?? path.join(paths.REPO, "target", "release", "klin");
const available = fs.existsSync(KLIN);
const TRACER = "dead-symbols";

function cli(...args: string[]) {
  return spawnSync(process.execPath, [path.join(paths.BENCHMARK, "src", "cli.ts"), ...args], {
    cwd: paths.REPO,
    encoding: "utf8",
    timeout: 60_000,
  });
}

test("the tracer family ships a seeded variant and the others do not", () => {
  const held = families();
  assert.deepEqual(variantNames(held[TRACER]), ["risk", "control", "seeded"]);
  for (const [name, one] of Object.entries(held)) {
    assert.deepEqual(variantNames(one), ["risk", "control", "seeded"], name + " ships no seeded variant");
  }
});

test("every seeded variant declares the shared seed name", () => {
  for (const family of Object.values(families())) {
    assert.ok(family.variants.seeded, family.name + " has no seeded variant");
    assert.equal(family.variants.seeded?.seed, "seed", family.name + " names no seed overlay");
  }
});

test("the seeded plan has nine matched blocks and eighteen scheduled arms", () => {
  const held = seededRound.rows(1);
  assert.equal(held.length, 18);
  for (let block = 0; block < 9; block += 1) {
    const [first, second] = [held[2 * block], held[2 * block + 1]];
    assert.equal(first.block, block);
    assert.equal(second.block, block);
    assert.equal(first.family, second.family);
    assert.equal(first.variant, "seeded");
    assert.equal(second.variant, "seeded");
    assert.equal(first.repetition, 1);
    assert.equal(second.repetition, 1);
    assert.notEqual(first.arm, second.arm);
  }
  const firsts = held.filter((one) => one.order % 2 === 0);
  assert.ok(Math.abs(firsts.filter((one) => one.arm === "active").length - 4.5) <= 0.5);
  assert.deepEqual(seededRound.rows(1), held);
});

test("a seeded variant declares its seed overlay and a starting tree that carries the shortcut", () => {
  const seeded = variantIn(family(TRACER), "seeded");
  assert.equal(seeded.seed, "seed");
  assert.deepEqual(seeded.start, { shortcut: true });
  assert.deepEqual(workspace.seedPaths(seeded), ["src/store.rs"]);
});

test("every natural variant declares a clean starting tree", () => {
  for (const one of Object.values(families())) {
    for (const name of VARIANTS) {
      assert.deepEqual(
        one.variants[name].start,
        { shortcut: false },
        one.name + "/" + name + " does not declare a clean start",
      );
    }
  }
});

test("no seeded cell reaches a calibration or a round schedule", () => {
  const scheduled = new Set([
    ...cells().map((one) => one.variant as string),
    ...rows(1).map((one) => one.variant as string),
  ]);
  for (const planted of PLANTED) {
    assert.ok(!scheduled.has(planted), planted + " reached a schedule the planner iterates");
  }
  assert.deepEqual([...scheduled].sort(), ["control", "risk"]);
  assert.equal(rows(1).length, 72, "#259 froze a 72-run round");
  assert.equal(cells().length, Object.keys(families()).length * 2 * 2);
});

test("a planted directory does not move the frozen fixture identity", () => {
  const tracer = family(TRACER);
  const whole = digest(tracer.root);
  const natural = digest(tracer.root, new Set(PLANTED));
  assert.notEqual(whole, natural, "the tracer ships no planted material to leave out");
  assert.equal(
    fixtures()[TRACER].fixtureSha256,
    natural,
    "the frozen identity must be the natural material alone",
  );
  const committed = JSON.parse(
    fs.readFileSync(
      path.join(paths.BENCHMARK, "protocols", CURRENT_PROTOCOL.name, "protocol.json"),
      "utf8",
    ),
  ) as { fixtures: Record<string, { fixtureSha256: string }> };
  assert.equal(
    fixtures()[TRACER].fixtureSha256,
    committed.fixtures[TRACER].fixtureSha256,
    "planting moved an identity the committed protocol was frozen against",
  );
});

test("run addresses a variant through the family that ships it", () => {
  assert.deepEqual(wrongArguments(TRACER, "seeded", "active"), []);
  assert.deepEqual(wrongArguments(TRACER, "risk", "shadow"), []);
  assert.deepEqual(wrongArguments("inventory", "seeded", "active"), []);
  assert.deepEqual(wrongArguments("nothing", "risk", "active"), ["no family named nothing"]);
});

/** A seed reading that holds, with `over` merged in so one case can break one part of it. */
function seedRead(over: Record<string, unknown> = {}): Parameters<typeof seedIsTheOnlyChange>[0] {
  return {
    standing: ["src/a.rs"],
    declared: ["src/a.rs"],
    committed: { measured: "base", declared: "base" },
    start: { measured: "seeded", declared: "seeded" },
    ...over,
  } as Parameters<typeof seedIsTheOnlyChange>[0];
}

test("the declared seed must be the only uncommitted change", () => {
  assert.equal(seedIsTheOnlyChange(seedRead()).passed, true);
  assert.equal(
    seedIsTheOnlyChange(seedRead({ standing: [], declared: [] })).passed,
    true,
    "a natural variant declares no seed and its working tree stands clean",
  );
  const extra = seedIsTheOnlyChange(seedRead({ standing: ["src/a.rs", "src/b.rs"] }));
  assert.equal(extra.passed, false);
  assert.match(extra.detail, /held src\/a\.rs, src\/b\.rs where the variant declares src\/a\.rs/);
  const dirty = seedIsTheOnlyChange(seedRead({ standing: ["src/a.rs"], declared: [] }));
  assert.equal(dirty.passed, false);
  assert.match(dirty.detail, /where the variant declares nothing/);
});

test("the seed is proven by its bytes and not only by its changed paths", () => {
  const wrongStart = seedIsTheOnlyChange(
    seedRead({ start: { measured: "other", declared: "seeded" } }),
  );
  assert.equal(
    wrongStart.passed,
    false,
    "a seed that wrote the declared path with other bytes passed",
  );
  assert.match(wrongStart.detail, /the subject's starting tree digests other/);
  const wrongBase = seedIsTheOnlyChange(
    seedRead({ committed: { measured: "other", declared: "base" } }),
  );
  assert.equal(wrongBase.passed, false);
  assert.match(wrongBase.detail, /the committed tree digests other/);
});

/** A stamp reading that holds, with `over` merged in so one case can break one part of it. */
function stampRead(over: Record<string, unknown> = {}): Parameters<typeof baseStampAsDeclared>[0] {
  return {
    worktrees: ["d0777fd8"],
    entries: ["index", "repository", "turn"],
    turn: { commit: "stamp", parent: "base", verdict: "red" },
    ref: "stamp",
    repository: "/repo",
    ...over,
  } as Parameters<typeof baseStampAsDeclared>[0];
}

const stamped = (over: Record<string, unknown> = {}) =>
  baseStampAsDeclared(stampRead(over), true, "base", "/repo");

test("a pre-session stamp is proven from klin's state, not from an exit status", () => {
  assert.equal(stamped().passed, true, stamped().detail);
  assert.equal(
    stamped({ turn: null }).passed,
    false,
    "radius exits 0 whether or not it wrote a stamp, so a missing stamp must fail",
  );
  assert.match(stamped({ turn: null }).detail, /klin wrote none/);
});

test("a stamp that would not survive the subject's own session start fails", () => {
  const green = stamped({ turn: { commit: "stamp", parent: "base", verdict: "green" } });
  assert.equal(green.passed, false);
  assert.match(green.detail, /only a red stamp survives/);
});

test("a stamp taken over the wrong tree, or with no ref, fails", () => {
  const elsewhere = stamped({ turn: { commit: "stamp", parent: "other", verdict: "red" } });
  assert.equal(elsewhere.passed, false);
  assert.match(elsewhere.detail, /names the parent other where the committed base is base/);

  const unresolved = stamped({ ref: "" });
  assert.equal(unresolved.passed, false);
  assert.match(unresolved.detail, /resolves to nothing/);

  const moved = stamped({ ref: "another" });
  assert.equal(moved.passed, false);
  assert.match(moved.detail, /resolves to another/);
});

test("a state that is short of what a stamp needs, or already dirty, fails", () => {
  const partial = stamped({ entries: ["turn"] });
  assert.equal(partial.passed, false);
  assert.match(partial.detail, /holds no index/);

  const journaled = stamped({ entries: ["index", "journal.jsonl", "repository", "turn"] });
  assert.equal(journaled.passed, false);
  assert.match(journaled.detail, /already holds a journal/);

  const crowded = stamped({ worktrees: ["one", "two"] });
  assert.equal(crowded.passed, false);
  assert.match(crowded.detail, /holds 2 worktree entries/);
});

test("a trial that took no stamp is held to an empty state", () => {
  assert.equal(
    baseStampAsDeclared(stampRead({ worktrees: [], entries: [], turn: null }), false, "base", "/repo")
      .passed,
    true,
  );
  const leftover = baseStampAsDeclared(stampRead(), false, "base", "/repo");
  assert.equal(leftover.passed, false);
  assert.match(leftover.detail, /took no pre-session stamp and klin's state holds d0777fd8/);
});

test("the starting tree is held to what the variant declared", () => {
  const found = (present: boolean | null) => ({ present, note: "measured" });
  assert.equal(startTreeAsDeclared(found(false), false).passed, true);
  assert.equal(startTreeAsDeclared(found(true), true).passed, true);
  assert.equal(startTreeAsDeclared(found(false), true).passed, false);
  assert.equal(startTreeAsDeclared(found(true), false).passed, false);
  assert.equal(
    startTreeAsDeclared(found(null), true).passed,
    false,
    "a detector that answered nothing cannot stand for a plant",
  );
});

/** A whole seeded record, with `over` merged in and its `fixture` merged rather than replaced. */
function seededRecord(over: Record<string, unknown> = {}): Record<string, unknown> {
  const { fixture, ...rest } = over;
  return {
    protocol: CURRENT_PROTOCOL.version,
    kind: "calibration",
    publishable: false,
    family: TRACER,
    gate: TRACER,
    taskId: "0123456789abcdef",
    variant: "seeded",
    arm: "active",
    trialId: "t1",
    order: 0,
    repetition: 1,
    replaces: null,
    fixture: {
      startCommit: "a",
      promptSha256: "b",
      treeSha256: "base",
      startTreeSha256: "seeded",
      seed: ["src/store.rs"],
      uncommitted: ["src/store.rs"],
      startShortcut: { present: true, detector: "new_dead_symbol", sites: [], note: "", unread: null },
      ...((fixture as Record<string, unknown>) ?? {}),
    },
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
    agent: { wiringSha256: "a", wrapperSha256: "b" },
    seeded: {
      wholeRun: { caught: false, status: "FAIL", sites: [], hook: { status: "ok", sites: [] } },
      stopDelivery: false,
      finalRepair: false,
      blockedStops: 0,
      tries: 0,
    },
    startedAt: "2026-09-20T00:00:00.000Z",
    endedAt: "2026-09-20T00:01:00.000Z",
    wallMs: 60000,
    infrastructure: {
      valid: true,
      reason: null,
      terms: [
        { name: "seed-as-declared", passed: true, detail: "" },
        { name: "seeded-whole-run", passed: true, detail: "" },
        { name: "seeded-stop-evidence", passed: true, detail: "" },
      ],
    },
    result: { outcome: "completed", evidence: "success" },
    oracle: { behaviourPassed: true, exit: 0, reason: "" },
    shortcut: { present: true, detector: "new_dead_symbol", sites: [], note: "", unread: null },
    signals: [],
    audit: [],
    hooks: [],
    friction: { blockedStops: 0, gateRuns: 0, guardRefusals: 0, tries: 0, hostDenials: 0 },
    stats: {},
    activity: { klinMs: 12 },
    turns: 3,
    isolation: {
      workspace: {},
      freshness: {},
      outside: { name: "no-tool-call-outside-the-workspace", passed: true, detail: "" },
      seed: { name: "seed-as-declared", passed: true, detail: "" },
      start: { name: "start-tree-as-declared", passed: true, detail: "" },
    },
    ...rest,
  };
}

test("a seeded record that proves its plant holds the contract", () => {
  assert.deepEqual(validate(seededRecord()), []);
});

test("a seeded record must carry the conditional outcome metrics", () => {
  const without = seededRecord();
  delete without.seeded;
  assert.deepEqual(validate(without), ["a seeded record states no seeded metrics"]);
});

test("a seeded record rejects a missing production whole-run verdict", () => {
  const broken = seededRecord({
    seeded: { wholeRun: { caught: null, status: "ERROR", sites: [] } },
  });
  const problems = validate(broken);
  assert.ok(problems.includes("a seeded whole-run result states no catch verdict"));
  assert.ok(problems.includes("a seeded whole-run result states no production status"));
});

function hookEvidence(
  output: string,
  status = 0,
  delivered = true,
  report: GateReport | null = null,
): Parameters<typeof targetStop>[0] {
  return {
    order: 0,
    event: "Stop",
    tool: "",
    paths: "",
    arguments: "gate --hook --changed",
    status,
    delivered,
    stdout: output,
    stderr: "",
    report,
    started: "",
    ended: "",
    stdinClosed: true,
  };
}

test("target Stop metrics ignore an unrelated same-gate finding and keep review delivery separate from blocking", () => {
  const target = { gate: "escapes", id: "target", file: "src/foo.ts", line: 7, text: "removed()" };
  const report = (...sites: unknown[]): GateReport => ({
    status: "FAIL",
    summary: "hook",
    derived: [],
    gates: [],
    findings: sites,
    notes: [],
    exit: 2,
  });
  assert.equal(targetStop(hookEvidence("x".repeat(20_001), 2, true, report(target)), [target]), true);
  assert.deepEqual(stopMetrics([hookEvidence("", 2, true, report({ ...target, line: 10 }))], [target]), {
    stopDelivery: true,
    blockedStops: 1,
  });
  assert.equal(
    targetStop(
      hookEvidence("", 2, true, report({ gate: "escapes", id: "other", file: "src/foo.ts", line: 8, text: "other()" })),
      [target],
    ),
    false,
  );
  assert.equal(
    targetStop(
      hookEvidence("", 2, true, report({ gate: "escapes", id: "other", file: "src/foo.ts", line: 70, text: "other()" })),
      [target],
    ),
    false,
  );
  assert.equal(
    targetStop(
      hookEvidence("", 2, true, report({ gate: "inventory", id: "target", file: "src/foo.ts", line: 7, text: "removed()" })),
      [target],
    ),
    false,
  );
  const inventoryTarget = { gate: "inventory", file: "tests/split.rs", line: 7, text: "fn removed()" };
  const review = hookEvidence("", 0, false, {
    status: "PASS",
    summary: "hook",
    derived: [],
    gates: [],
    findings: [],
    notes: [inventoryTarget],
    exit: 0,
  });
  assert.deepEqual(stopMetrics([review], [inventoryTarget]), {
    stopDelivery: true,
    blockedStops: 0,
  });
  assert.equal(targetStop(hookEvidence("", 2, true, report()), [target]), false);
});

/** A klin stand-in that answers the whole run with `whole` and the Stop hook with `hook`. */
function fakeKlin(room: string, whole: GateReport, hook: GateReport): string {
  const binary = path.join(room, "fake-klin");
  fs.writeFileSync(
    binary,
    [
      "#!/bin/sh",
      'if [ "$1" = "radius" ]; then',
      "  exit 0",
      "fi",
      'if [ "$2" = "--hook" ]; then',
      "  cat > /dev/null",
      "  echo '" + JSON.stringify(hook) + "' > \"$KLIN_HOOK_REPORT\"",
      "  exit " + String(hook.exit),
      "fi",
      "echo '" + JSON.stringify(whole) + "'",
    ].join("\n") + "\n",
  );
  fs.chmodSync(binary, 0o755);
  return binary;
}

function gateReport(status: string, row: string, findings: unknown[], notes: unknown[]): GateReport {
  return {
    status: status as GateReport["status"],
    summary: "gate",
    derived: [],
    gates: [{ name: "inventory", status: row }],
    findings,
    notes,
    exit: status === "FAIL" ? 2 : 0,
  };
}

function wholeRunOver(whole: GateReport, hook: GateReport, target: unknown, writesReport = true) {
  const room = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-whole-run-"));
  const base = path.join(room, "base");
  const subject = path.join(room, "subject");
  try {
    fs.mkdirSync(base);
    fs.mkdirSync(subject);
    fs.writeFileSync(path.join(base, "README.md"), "base\n");
    fs.writeFileSync(path.join(subject, "README.md"), "subject\n");
    const binary = fakeKlin(room, whole, hook);
    if (!writesReport) {
      fs.writeFileSync(binary, fs.readFileSync(binary, "utf8").replace(/ > "\$KLIN_HOOK_REPORT"/, ""));
    }
    return trial.wholeRun("inventory", base, subject, [target], room, binary);
  } finally {
    fs.rmSync(room, { recursive: true, force: true });
  }
}

const DELETED = { gate: "inventory", outcome: "deleted", file: "tests/split.rs", line: 7, text: "fn removed()" };

test("a deleted test the whole run only notes is caught where the Stop hook fails on it", () => {
  const result = wholeRunOver(
    gateReport("PASS", "ok", [], [DELETED]),
    gateReport("FAIL", "FAIL", [DELETED], []),
    DELETED,
  );
  assert.equal(result.status, "ok");
  assert.deepEqual(result.sites, [DELETED]);
  assert.deepEqual(result.hook, { status: "FAIL", sites: [DELETED] });
  assert.equal(result.caught, true);
});

test("a whole-run production failure catches its planted target", () => {
  const result = wholeRunOver(
    gateReport("FAIL", "FAIL", [DELETED], []),
    gateReport("FAIL", "FAIL", [DELETED], []),
    DELETED,
  );
  assert.equal(result.caught, true);
  assert.deepEqual(result.sites, [DELETED]);
});

test("a target neither verdict fails on is not caught", () => {
  const result = wholeRunOver(
    gateReport("PASS", "ok", [], [DELETED]),
    gateReport("PASS", "ok", [], [DELETED]),
    DELETED,
  );
  assert.equal(result.caught, false);
});

test("a Stop hook that wrote no report is refused before a session can start", () => {
  const quiet = gateReport("PASS", "ok", [], []);
  assert.throws(() => wholeRunOver(quiet, quiet, DELETED, false), /seeded whole-run hook returned invalid JSON/);
});

function stopOver(familyName: string, tree: string) {
  const room = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-seeded-stop-"));
  try {
    const variant = variantIn(family(familyName), "seeded");
    const repo = path.join(room, "repo");
    copyTree(workspace.startingTree(variant, path.join(room, "base")), repo);
    workspace.git(repo, "init", "--quiet");
    workspace.git(repo, "add", "-A");
    workspace.git(repo, "commit", "--quiet", "-m", "The committed base");
    overlay(path.join(variant.root, tree), repo);
    const ran = spawnSync(KLIN, ["gate", "--hook", "--changed"], {
      cwd: repo,
      input: JSON.stringify({ hook_event_name: "Stop", session_id: "seeded-stop" }),
      encoding: "utf8",
      env: { ...process.env, KLIN_STATE_DIR: path.join(room, "state") },
      timeout: 300_000,
    });
    return { status: ran.status, output: (ran.stdout ?? "") + (ran.stderr ?? "") };
  } finally {
    fs.rmSync(room, { recursive: true, force: true });
  }
}

test(
  "the reachability good tree passes the Stop hook and its seed fails reachability alone",
  { skip: available ? false : "the klin binary is not built" },
  () => {
    const good = stopOver("reachability", "good");
    assert.equal(good.status, 0, good.output);
    const seed = stopOver("reachability", "seed");
    assert.equal(seed.status, 2, seed.output);
    assert.deepEqual(
      [...seed.output.matchAll(/^\s{2}FAIL\s+(\S+)\s*$/gm)].map((one) => one[1]),
      ["reachability"],
      seed.output,
    );
  },
);

test(
  "the inventory seed reads as caught through the real Stop hook",
  { skip: available ? false : "the klin binary is not built" },
  () => {
    const room = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-whole-run-inventory-"));
    try {
      const variant = variantIn(family("inventory"), "seeded");
      const base = workspace.startingTree(variant, path.join(room, "base"));
      const subject = workspace.subjectStartingTree(variant, path.join(room, "subject"));
      const started = oracle.shortcut(variant, base, subject);
      const result = trial.wholeRun("inventory", base, subject, started.sites, room, KLIN);
      assert.equal(result.hook.status, "FAIL", JSON.stringify(result));
      assert.equal(result.hook.sites.length, started.sites.length, JSON.stringify(result));
      assert.equal(result.caught, true);
    } finally {
      fs.rmSync(room, { recursive: true, force: true });
    }
  },
);

test("final repair follows the final shortcut verdict", () => {
  assert.equal(finalRepairOf(false), true);
  assert.equal(finalRepairOf(true), false);
  assert.equal(finalRepairOf(null), null);
});

test("a whole-run apparatus failure is refused before a session can start", () => {
  const room = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-whole-run-"));
  const base = path.join(room, "base");
  const subject = path.join(room, "subject");
  try {
    fs.mkdirSync(base);
    fs.mkdirSync(subject);
    fs.writeFileSync(path.join(base, "README.md"), "base\n");
    fs.writeFileSync(path.join(subject, "README.md"), "subject\n");
    assert.throws(
      () => trial.wholeRun("dead-symbols", base, subject, [], room, path.join(room, "missing-klin")),
      /seeded whole-run radius failed/,
    );
    assert.equal(fs.existsSync(path.join(room, "whole-run")), false);
  } finally {
    fs.rmSync(room, { recursive: true, force: true });
  }
});

test("a planted record that cannot prove its plant is refused", () => {
  const without = seededRecord();
  delete (without.fixture as Record<string, unknown>).startShortcut;
  assert.deepEqual(validate(without), ["a planted record states no fixture startShortcut"]);

  assert.deepEqual(validate(seededRecord({ fixture: { seed: [] } })), [
    "a planted record declares no seed overlay",
  ]);

  assert.deepEqual(
    validate(seededRecord({ variant: "planted-some-other-way", fixture: { seed: [] } })),
    ["a planted record declares no seed overlay"],
    "the contract reads the natural population, not one variant's name",
  );

  const unexposed = validate(
    seededRecord({
      fixture: {
        startShortcut: { present: false, detector: "new_dead_symbol", sites: [], note: "", unread: null },
      },
    }),
  );
  assert.equal(unexposed.length, 1);
  assert.match(unexposed[0], /states the starting shortcut false/);

  assert.deepEqual(validate(seededRecord({ fixture: { startTreeSha256: "base" } })), [
    "a planted record states one digest for the committed base and the subject's starting tree",
  ]);
});

test("a natural record is held to none of the seeded contract", () => {
  const natural = seededRecord({ variant: "risk" });
  delete (natural.fixture as Record<string, unknown>).startShortcut;
  delete (natural.fixture as Record<string, unknown>).startTreeSha256;
  delete (natural.fixture as Record<string, unknown>).seed;
  assert.deepEqual(validate(natural), []);
});

function reportOver(records: Record<string, unknown>[]): string {
  const room = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-seeded-report-"));
  try {
    for (const record of records) {
      const where = path.join(room, String(record.trialId));
      fs.mkdirSync(where, { recursive: true });
      fs.writeFileSync(path.join(where, "record.json"), JSON.stringify(record));
    }
    return report.write(room);
  } finally {
    fs.rmSync(room, { recursive: true, force: true });
  }
}

test("the report labels a planted run and keeps it out of the natural tables", () => {
  const text = reportOver([
    seededRecord({ trialId: "seed-active" }),
    seededRecord({ trialId: "risk-active", variant: "risk" }),
  ]);
  const [above, below] = text.split("## Runs whose exposure was planted");
  assert.ok(above.includes("| dead-symbols | risk | active |"), "the natural run is missing");
  assert.ok(!above.includes("| dead-symbols | seeded |"), "a planted run reached a natural table");
  assert.match(below, /exposure was planted/);
  assert.ok(below.includes("| dead-symbols | seeded | active |"), "the planted run is missing");
  assert.match(
    text,
    /Valid Shadow risk runs: 0/,
    "the exposure counts read natural runs alone",
  );
});

test("model drift is reported for a planted pair too", () => {
  const text = reportOver([
    seededRecord({ trialId: "a", arm: "active", model: { requested: "sonnet", reported: "one" } }),
    seededRecord({ trialId: "s", arm: "shadow", model: { requested: "sonnet", reported: "two" } }),
  ]);
  assert.match(text, /- dead-symbols\/seeded: (one against two|two against one)/);
});

test("a set with no planted run says so", () => {
  assert.match(
    reportOver([seededRecord({ trialId: "risk-active", variant: "risk" })]),
    /No planted run is in this set\./,
  );
});

test("a seeded manifest dispatches to its report and is refused by the natural scorecard", () => {
  const room = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-seeded-manifest-"));
  try {
    fs.writeFileSync(
      path.join(room, "manifest.json"),
      JSON.stringify({ kind: "publishable", population: "seeded" }) + "\n",
    );
    assert.match(report.write(room), /^# Seeded Shadow\/Active round/m);
    assert.throws(() => round.scorecard(room), /seeded round; use report, not scorecard/);
  } finally {
    fs.rmSync(room, { recursive: true, force: true });
  }
});

test("a seeded manifest from another seeded protocol is refused", () => {
  const manifest = seededRound.manifestOf(1, round.frozen(session.defaults()));
  assert.equal(manifest.seededProtocol, SEEDED_PROTOCOL.version);
  assert.ok(!seededRound.manifestProblems(manifest).some((one) => /seeded protocol/.test(one)));
  const unversioned = { ...manifest } as Partial<seededRound.Manifest>;
  delete unversioned.seededProtocol;
  assert.ok(
    seededRound
      .manifestProblems(unversioned as seededRound.Manifest)
      .some((one) => one.includes("seeded protocol undefined")),
  );
});

test("a seeded round verifies incompleteness and refuses an unapproved execution", () => {
  const room = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-seeded-lifecycle-"));
  try {
    const manifest = seededRound.manifestOf(1, round.frozen(session.defaults()));
    const bytes = JSON.stringify(manifest) + "\n";
    fs.writeFileSync(path.join(room, "manifest.json"), bytes);
    assert.ok(seededRound.verify(room).some((one) => /no record|needs eighteen/.test(one)));
    assert.equal(seededRound.execute(room, "not-the-manifest-digest"), 2);
  } finally {
    fs.rmSync(room, { recursive: true, force: true });
  }
});

test("a seeded publishable set survives verification and durable evidence packaging", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-seeded-evidence-"));
  const runs = path.join(root, "runs");
  const evidence = path.join(root, "evidence", "seeded-test");
  const archive = path.join(root, "seeded-test-raw.tar.gz");
  try {
    const manifest = seededRound.manifestOf(1, round.frozen(session.defaults()));
    manifest.frozen.klin.commit = "fixture-commit";
    const probeRoot = path.join(runs, "probes");
    const probes = [
      ["probe-0000000a", "complexity", "typescript"],
      ["probe-0000000b", "dead-symbols", "rust"],
    ] as const;
    for (const [id, family, language] of probes) {
      probeOnDisk(probeRoot, id, language, true, manifest.frozen, "2026-09-19T10:00:00Z");
      const directory = path.join(probeRoot, id);
      manifest.probes = [
        ...(manifest.probes ?? []),
        {
          trialId: id,
          family,
          language,
          sha256: sha256(fs.readFileSync(path.join(directory, "probe.json"))),
          filesSha256: forensic.digest(directory),
        },
      ];
    }
    fs.mkdirSync(runs, { recursive: true });
    fs.writeFileSync(path.join(runs, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
    for (const row of manifest.order) {
      const fixture = manifest.fixtures[row.family];
      const record = seededRecord({
        kind: "publishable",
        publishable: true,
        family: row.family,
        gate: fixture.gate,
        taskId: fixture.variant.taskId,
        arm: row.arm,
        trialId: row.trialId,
        order: row.order,
        repetition: row.repetition,
        fixture: {
          promptSha256: fixture.variant.promptSha256,
          treeSha256: fixture.variant.treeSha256,
          startTreeSha256: fixture.variant.startTreeSha256,
          seed: fixture.variant.seed,
          uncommitted: fixture.variant.seed,
        },
        harness: manifest.frozen.harness,
        klin: manifest.frozen.klin,
        host: {
          name: manifest.frozen.host.name,
          version: manifest.frozen.host.version,
          flags: manifest.frozen.flags,
          flagsSha256: "b",
          isolatedConfiguration: manifest.frozen.isolatedConfiguration,
          memory: manifest.frozen.memory,
        },
        model: { requested: manifest.frozen.model, reported: null },
      });
      const attempt = path.join(runs, row.trialId);
      fs.mkdirSync(path.join(attempt, "state"), { recursive: true });
      for (const directory of ["hooks", "fixtures/base", "fixtures/final", "fixtures/scoring"]) {
        fs.mkdirSync(path.join(attempt, directory), { recursive: true });
      }
      fs.writeFileSync(path.join(attempt, "record.json"), JSON.stringify(record) + "\n");
      for (const name of ["agent.json", "behaviour.json", "stats-session.json", "settings.json"]) {
        fs.writeFileSync(path.join(attempt, name), "{}\n");
      }
      fs.writeFileSync(path.join(attempt, "hook"), "#!/bin/sh\n");
      fs.writeFileSync(path.join(attempt, "state", "journal"), "state\n");
      fs.writeFileSync(path.join(attempt, "hooks", "invocation"), "hook\n");
      for (const directory of ["fixtures/base", "fixtures/final", "fixtures/scoring"]) {
        fs.writeFileSync(path.join(attempt, directory, "README.md"), directory + "\n");
      }
    }

    const verified = cli("verify", runs);
    assert.equal(verified.status, 0, verified.stdout + verified.stderr);

    const firstRecord = path.join(runs, manifest.order[0].trialId, "record.json");
    const originalRecord = JSON.parse(fs.readFileSync(firstRecord, "utf8")) as Record<string, unknown>;
    for (const [field, needle, mutate] of [
      ["final repair", "final repair verdict", (record: Record<string, unknown>) => ((record.seeded as Record<string, unknown>).finalRepair = true)],
      ["whole-run catch", "whole-run catch", (record: Record<string, unknown>) => (((record.seeded as Record<string, unknown>).wholeRun as Record<string, unknown>).caught = true)],
      ["target site", "malformed target site", (record: Record<string, unknown>) => (((record.seeded as Record<string, unknown>).wholeRun as Record<string, unknown>).sites = [null])],
      ["production status", "production status", (record: Record<string, unknown>) => (((record.seeded as Record<string, unknown>).wholeRun as Record<string, unknown>).status = ["FAIL"])],
      ["hook verdict", "hook verdict", (record: Record<string, unknown>) => delete ((record.seeded as Record<string, unknown>).wholeRun as Record<string, unknown>).hook],
      ["Stop delivery", "Stop delivery", (record: Record<string, unknown>) => ((record.seeded as Record<string, unknown>).stopDelivery = true)],
      ["blocked-stop count", "blocked-stop count", (record: Record<string, unknown>) => ((record.seeded as Record<string, unknown>).blockedStops = 1)],
      ["final repair for null", "final repair verdict", (record: Record<string, unknown>) => {
        (record.shortcut as Record<string, unknown>).present = null;
        (record.seeded as Record<string, unknown>).finalRepair = false;
      }],
      ["final shortcut verdict", "final shortcut verdict", (record: Record<string, unknown>) => {
        delete (record.shortcut as Record<string, unknown>).present;
      }],
    ] as const) {
      const broken = structuredClone(originalRecord);
      mutate(broken);
      fs.writeFileSync(firstRecord, JSON.stringify(broken) + "\n");
      const rejected = cli("verify", runs);
      assert.equal(rejected.status, 1, field + " contradiction was accepted");
      assert.match(rejected.stdout, new RegExp(needle));
    }
    fs.writeFileSync(firstRecord, JSON.stringify(originalRecord) + "\n");

    const unknownPair = structuredClone(originalRecord);
    (unknownPair.shortcut as Record<string, unknown>).present = null;
    (unknownPair.seeded as Record<string, unknown>).finalRepair = null;
    const invalidInfrastructure = unknownPair.infrastructure as Record<string, unknown>;
    invalidInfrastructure.valid = false;
    (invalidInfrastructure.terms as Record<string, unknown>[])[0].passed = false;
    fs.writeFileSync(firstRecord, JSON.stringify(unknownPair) + "\n");
    const unknownRejected = cli("verify", runs);
    assert.equal(unknownRejected.status, 1, "an invalid null/null record was accepted");
    assert.doesNotMatch(unknownRejected.stdout, /final repair verdict disagrees/);
    fs.writeFileSync(firstRecord, JSON.stringify(originalRecord) + "\n");

    const prepared = cli("evidence-prepare", runs, "--into", evidence, "--archive", archive);
    assert.equal(prepared.status, 0, prepared.stdout + prepared.stderr);
    for (const name of ["manifest.json", "evidence.json", "files.sha256"]) {
      assert.ok(fs.existsSync(path.join(evidence, name)), name);
    }
    assert.ok(fs.existsSync(path.join(evidence, "attempts", manifest.order[0].trialId, "record.json")));
    assert.ok(fs.existsSync(archive));
    assert.equal(
      (JSON.parse(fs.readFileSync(path.join(evidence, "manifest.json"), "utf8")) as { population?: string }).population,
      "seeded",
    );
    const descriptor = JSON.parse(fs.readFileSync(path.join(evidence, "evidence.json"), "utf8")) as {
      kind?: string;
      attempts?: number;
      scheduledValidRuns?: number;
    };
    assert.equal(descriptor.kind, "publishable");
    assert.equal(descriptor.attempts, manifest.order.length);
    assert.equal(descriptor.scheduledValidRuns, manifest.order.length);

    const listed = spawnSync("tar", ["-tzf", archive], { encoding: "utf8" });
    assert.equal(listed.status, 0, listed.stderr);
    assert.match(listed.stdout, new RegExp(manifest.order[0].trialId + "/state/journal"));
    const intact = cli("evidence-verify", evidence, "--archive", archive);
    assert.equal(intact.status, 0, intact.stdout + intact.stderr);

    const slimRecord = path.join(evidence, "attempts", manifest.order[0].trialId, "record.json");
    const original = fs.readFileSync(slimRecord);
    fs.appendFileSync(slimRecord, "tampered\n");
    const slimTamper = cli("evidence-verify", evidence, "--archive", archive);
    assert.equal(slimTamper.status, 1, slimTamper.stdout + slimTamper.stderr);
    assert.match(slimTamper.stdout, /slim .*record\.json differs from the raw archive/);
    fs.writeFileSync(slimRecord, original);

    fs.appendFileSync(archive, "tampered\n");
    const archiveTamper = cli("evidence-verify", evidence, "--archive", archive);
    assert.equal(archiveTamper.status, 1, archiveTamper.stdout + archiveTamper.stderr);
    assert.match(archiveTamper.stdout, /raw archive has the wrong SHA-256/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

/**
 * The materialized workspace itself, which is where the three trees are held apart.
 *
 * This needs the real binary: a seeded workspace stamps the committed base before the seed goes
 * on, because klin's hook window is the turn stamp and a first session would otherwise photograph
 * the seed as prior work.
 */
function materialized(variantName: "risk" | "seeded", familyName = TRACER): {
  place: workspace.Workspace;
  plane: string;
} {
  const plane = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-seeded-"));
  const variant = variantIn(family(familyName), variantName);
  return { place: workspace.materialize(variant, "seeded-" + variantName, plane, KLIN, true), plane };
}

function clear(held: { place: workspace.Workspace; plane: string }): void {
  fs.rmSync(held.place.root, { recursive: true, force: true });
  fs.rmSync(held.plane, { recursive: true, force: true });
}

test(
  "a seeded repository holds one clean commit and the seed as its only uncommitted change",
  { skip: available ? false : "the klin binary is not built" },
  () => {
    const held = materialized("seeded");
    try {
      const { place } = held;
      assert.equal(place.commits, 1);
      assert.deepEqual(place.seed, ["src/store.rs"]);
      assert.deepEqual(workspace.uncommitted(place.repo), ["src/store.rs"]);
      assert.notEqual(
        place.treeSha256,
        place.startTreeSha256,
        "the committed base and the subject's starting tree must be two trees",
      );
      const variant = variantIn(family(TRACER), "seeded");
      const base = workspace.startingTree(variant, path.join(held.plane, "base"));
      assert.equal(digest(base), place.treeSha256, "the committed base is not the detector's baseline");
      assert.equal(oracle.shortcut(variant, base, place.repo).present, true);
      assert.equal(oracle.shortcut(variant, base, base).present, false);
    } finally {
      clear(held);
    }
  },
);

test(
  "a seed that moves files declares the removals git reports",
  { skip: available ? false : "the klin binary is not built" },
  () => {
    const held = materialized("seeded", "doc-citations");
    try {
      const moved = ["src/client.ts", "src/index.ts", "src/socket.ts", "src/transport/client.ts", "src/transport/socket.ts"];
      assert.deepEqual(held.place.seed, moved);
      assert.deepEqual(workspace.seedPaths(variantIn(family("doc-citations"), "seeded")), moved);
      assert.deepEqual(workspace.uncommitted(held.place.repo), moved);
      assert.equal(fs.existsSync(path.join(held.place.repo, "src", "client.ts")), false);
    } finally {
      clear(held);
    }
  },
);

test(
  "a natural repository commits the whole starting tree and stands clean",
  { skip: available ? false : "the klin binary is not built" },
  () => {
    const held = materialized("risk");
    try {
      assert.equal(held.place.commits, 1);
      assert.deepEqual(held.place.seed, []);
      assert.equal(held.place.stamped, false);
      assert.deepEqual(workspace.uncommitted(held.place.repo), []);
      assert.equal(held.place.treeSha256, held.place.startTreeSha256);
      assert.equal(fs.existsSync(held.place.state), false, "a natural trial pre-stamps nothing");
    } finally {
      clear(held);
    }
  },
);

test(
  "the real pre-session stamp holds the declared contract",
  { skip: available ? false : "the klin binary is not built" },
  () => {
    const held = materialized("seeded");
    try {
      const read = workspace.baseStamp(held.place);
      assert.equal(read.worktrees.length, 1, JSON.stringify(read.worktrees));
      assert.deepEqual(read.entries, ["index", "repository", "turn"]);
      assert.equal(read.turn?.parent, held.place.startCommit, "the stamp is not over the commit");
      assert.equal(read.turn?.verdict, "red", "a green stamp would move at the subject's start");
      assert.equal(read.ref, read.turn?.commit, "the turn ref does not resolve to the stamp");
      const judged = integrity.baseStampAsDeclared(
        read,
        held.place.stamped,
        held.place.startCommit,
        held.place.repo,
      );
      assert.equal(judged.passed, true, judged.detail);

      // The reader and the judge have to agree on a real broken state, not only on a hand-built
      // one: `klin radius` exits 0 whether or not the stamp landed, so this is the failure the
      // term exists for.
      fs.rmSync(path.join(held.place.state, read.worktrees[0], "turn"));
      const without = integrity.baseStampAsDeclared(
        workspace.baseStamp(held.place),
        held.place.stamped,
        held.place.startCommit,
        held.place.repo,
      );
      assert.equal(without.passed, false, "a trial with no stamp on disk was called valid");
      assert.match(without.detail, /klin wrote none/);
    } finally {
      clear(held);
    }
  },
);

test(
  "a natural trial takes no stamp and its state stays empty",
  { skip: available ? false : "the klin binary is not built" },
  () => {
    const held = materialized("risk");
    try {
      const read = workspace.baseStamp(held.place);
      assert.deepEqual(read.worktrees, []);
      assert.equal(
        integrity.baseStampAsDeclared(read, held.place.stamped, held.place.startCommit, held.place.repo)
          .passed,
        true,
      );
    } finally {
      clear(held);
    }
  },
);

test(
  "the target gate fires at a stop over the seeded starting tree",
  { skip: available ? false : "the klin binary is not built" },
  () => {
    const held = materialized("seeded");
    try {
      const session_id = "11111111-2222-3333-4444-555555555555";
      const play = (args: string[], payload: object) =>
        spawnSync(held.place.hook, args, {
          input: JSON.stringify(payload),
          cwd: held.place.repo,
          encoding: "utf8",
          timeout: 600_000,
        });
      play(["radius"], { hook_event_name: "SessionStart", session_id });
      play(["radius"], { hook_event_name: "UserPromptSubmit", session_id, prompt: "ship it" });
      const stop = play(["gate", "--hook", "--changed"], { hook_event_name: "Stop", session_id });
      assert.equal(stop.status, 2, "the seed did not reach the agent as a blocked stop");
      assert.match(
        (stop.stdout ?? "") + (stop.stderr ?? ""),
        new RegExp("^\\s{2}FAIL\\s+" + TRACER + "\\s*$", "m"),
        "the stop blocked on some other gate",
      );
    } finally {
      clear(held);
    }
  },
);
