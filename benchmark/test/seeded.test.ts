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
import { digest } from "../src/trees.ts";
import { fixtures } from "../src/frozen.ts";
import { rows } from "../src/round.ts";
import { wrongArguments } from "../src/cli.ts";
import { seedIsTheOnlyChange, startTreeAsDeclared } from "../src/integrity.ts";
import { validate } from "../src/record.ts";
import { CURRENT_PROTOCOL } from "../src/protocol.ts";
import * as workspace from "../src/workspace.ts";
import * as oracle from "../src/oracle.ts";
import * as report from "../src/report.ts";

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

test("the tracer family ships a seeded variant and the others do not", () => {
  const held = families();
  assert.deepEqual(variantNames(held[TRACER]), ["risk", "control", "seeded"]);
  for (const [name, one] of Object.entries(held).filter(([name]) => name !== TRACER)) {
    assert.deepEqual(variantNames(one), ["risk", "control"], name + " ships an unexpected variant");
  }
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
  const refused = wrongArguments("inventory", "seeded", "active");
  assert.equal(refused.length, 1);
  assert.match(refused[0], /inventory ships no variant named seeded, only risk, control/);
  assert.deepEqual(wrongArguments("nothing", "risk", "active"), ["no family named nothing"]);
});

test("the declared seed must be the only uncommitted change", () => {
  assert.equal(seedIsTheOnlyChange([], []).passed, true);
  assert.equal(seedIsTheOnlyChange(["src/a.rs"], ["src/a.rs"]).passed, true);
  const extra = seedIsTheOnlyChange(["src/a.rs", "src/b.rs"], ["src/a.rs"]);
  assert.equal(extra.passed, false);
  assert.match(extra.detail, /held src\/a\.rs, src\/b\.rs where the variant declares src\/a\.rs/);
  const dirty = seedIsTheOnlyChange(["src/a.rs"], []);
  assert.equal(dirty.passed, false);
  assert.match(dirty.detail, /where the variant declares nothing/);
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
    startedAt: "2026-09-20T00:00:00.000Z",
    endedAt: "2026-09-20T00:01:00.000Z",
    wallMs: 60000,
    infrastructure: {
      valid: true,
      reason: null,
      terms: [{ name: "seed-as-declared", passed: true, detail: "" }],
    },
    result: { outcome: "completed", evidence: "success" },
    oracle: { behaviourPassed: true, exit: 0, reason: "" },
    shortcut: { present: true, detector: "new_dead_symbol", sites: [], note: "", unread: null },
    signals: [],
    audit: [],
    hooks: [],
    friction: { blockedStops: 1, gateRuns: 1, guardRefusals: 0, tries: 0, hostDenials: 0 },
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

/**
 * The materialized workspace itself, which is where the three trees are held apart.
 *
 * This needs the real binary: a seeded workspace stamps the committed base before the seed goes
 * on, because klin's hook window is the turn stamp and a first session would otherwise photograph
 * the seed as prior work.
 */
function materialized(variantName: "risk" | "seeded"): {
  place: workspace.Workspace;
  plane: string;
} {
  const plane = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-seeded-"));
  const variant = variantIn(family(TRACER), variantName);
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
