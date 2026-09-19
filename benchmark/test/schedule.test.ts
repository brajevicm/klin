import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { all, scheduled, trialId, verify } from "../src/calibrate.ts";
import { VARIANTS, ARMS } from "../src/catalogue.ts";
import * as paths from "../src/paths.ts";
import { CURRENT_PROTOCOL } from "../src/protocol.ts";
import type { RunRecord } from "../src/record.ts";

/**
 * The exact scheduled set, and nothing else, is what ran.
 *
 * These read the manifest against the catalogue, so a manifest that scheduled the wrong
 * experiment cannot vouch for itself. No session runs here.
 */

function room(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-schedule-"));
}

interface Row {
  family: string;
  variant: string;
  arm: string;
  order: number;
  trialId: string;
}

/** The canonical schedule for one family: both variants, both arms, in catalogue order. */
function rowsFor(family: string, from = 0): Row[] {
  const held: Row[] = [];
  for (const variant of VARIANTS) {
    for (const arm of ARMS) {
      const order = from + held.length;
      held.push({ family, variant, arm, order, trialId: trialId(family, variant, arm, order) });
    }
  }
  return held;
}

function recordsFor(rows: Row[]): RunRecord[] {
  return rows.map((row) => ({ ...row }) as unknown as RunRecord);
}

function complete(): { read: { selectedFamilies: string[]; order: Row[] }; held: RunRecord[] } {
  const order = rowsFor("inventory");
  return { read: { selectedFamilies: ["inventory"], order }, held: recordsFor(order) };
}

test("one Active and one Shadow in every expected cell raises nothing", () => {
  const { read, held } = complete();
  assert.deepEqual(scheduled(read, held), []);
  assert.equal(read.order.length, 4, "--only inventory is four cells");
});

test("two Actives and no Shadow in a cell fails", () => {
  const { read, held } = complete();
  held[1] = { ...held[1], arm: "active", trialId: "x1" } as RunRecord;
  const problems = scheduled(read, held);
  assert.ok(
    problems.some((one) => one.includes("one Active and one Shadow")),
    problems.join(" / "),
  );
});

test("two Shadows and no Active in a cell fails", () => {
  const { read, held } = complete();
  held[0] = { ...held[0], arm: "shadow", trialId: "x0" } as RunRecord;
  const problems = scheduled(read, held);
  assert.ok(
    problems.some((one) => one.includes("one Active and one Shadow")),
    problems.join(" / "),
  );
});

test("a cell with only one arm present fails", () => {
  const { read, held } = complete();
  const problems = scheduled(read, held.slice(1));
  assert.ok(
    problems.some((one) => one.includes("one Active and one Shadow")),
    problems.join(" / "),
  );
  assert.ok(problems.some((one) => one.includes("left no record")), problems.join(" / "));
});

test("a manifest that duplicates a cell fails", () => {
  const { read, held } = complete();
  read.order = [...read.order, read.order[0]];
  const problems = scheduled(read, held);
  assert.ok(problems.some((one) => one.includes("2 times")), problems.join(" / "));
});

test("a manifest that omits an expected cell fails", () => {
  const { read, held } = complete();
  read.order = read.order.slice(1);
  const problems = scheduled(read, held);
  assert.ok(problems.some((one) => one.includes("scheduled no")), problems.join(" / "));
});

test("a record no row scheduled fails", () => {
  const { read, held } = complete();
  const problems = scheduled(read, [
    ...held,
    { ...held[0], trialId: "unscheduled" } as RunRecord,
  ]);
  assert.ok(
    problems.some((one) => one.includes("belongs to no scheduled trial")),
    problems.join(" / "),
  );
});

test("a scheduled row with no record fails", () => {
  const { read, held } = complete();
  const problems = scheduled(read, held.slice(0, 3));
  assert.ok(problems.some((one) => one.includes("left no record")), problems.join(" / "));
});

test("a record whose arm, order or trial id disagrees with the manifest fails", () => {
  for (const [what, over] of [
    ["arm", { arm: "shadow" }],
    ["order", { order: 99 }],
    ["family", { family: "complexity" }],
    ["variant", { variant: "control" }],
  ] as [string, Partial<RunRecord>][]) {
    const { read, held } = complete();
    held[0] = { ...held[0], ...over } as RunRecord;
    const problems = scheduled(read, held);
    assert.ok(
      problems.some((one) => one.includes("the record states " + what)),
      what + ": " + problems.join(" / "),
    );
  }
});

test("a manifest trial id the schedule does not give fails", () => {
  const { read, held } = complete();
  read.order[0] = { ...read.order[0], trialId: "deadbeef0000" };
  const problems = scheduled(read, held);
  assert.ok(problems.some((one) => one.includes("the schedule gives")), problems.join(" / "));
});

test("a manifest naming a family the catalogue does not have fails", () => {
  const { read, held } = complete();
  read.selectedFamilies = ["inventory", "no-such-family"];
  const problems = scheduled(read, held);
  assert.ok(
    problems.some((one) => one.includes("the catalogue does not have")),
    problems.join(" / "),
  );
});

test("an unknown --only family is rejected before any trial starts", () => {
  const where = room();
  const wrote: string[] = [];
  const kept = process.stdout.write.bind(process.stdout);
  process.stdout.write = ((text: string) => {
    wrote.push(text);
    return true;
  }) as typeof process.stdout.write;
  let exit: number;
  try {
    exit = all({ into: where, only: ["no-such-family"], seed: 1 });
  } finally {
    process.stdout.write = kept;
  }
  assert.equal(exit, 2);
  assert.match(wrote.join(""), /no family named no-such-family/);
  assert.equal(fs.existsSync(path.join(where, "manifest.json")), false, "no set was started");
  fs.rmSync(where, { recursive: true, force: true });
});

/** The whole verifier over a complete four-record `--only inventory` set on disk. */
test("verify accepts the complete four-record inventory set", () => {
  const where = room();
  const base = JSON.parse(
    fs.readFileSync(path.join(paths.BENCHMARK, "test", "live-record.json"), "utf8"),
  ) as Record<string, unknown>;
  const order = rowsFor("inventory");
  for (const row of order) {
    const into = path.join(where, row.trialId);
    fs.mkdirSync(into, { recursive: true });
    fs.writeFileSync(
      path.join(into, "record.json"),
      JSON.stringify({ ...base, protocol: CURRENT_PROTOCOL.version, audit: [], ...row }) + "\n",
    );
  }
  fs.writeFileSync(
    path.join(where, "manifest.json"),
    JSON.stringify({ selectedFamilies: ["inventory"], order }) + "\n",
  );
  const problems = verify(where).filter(
    (one) =>
      one.includes("scheduled") ||
      one.includes("Active") ||
      one.includes("belongs to") ||
      one.includes("should hold"),
  );
  assert.deepEqual(problems, [], verify(where).join(" / "));
  fs.rmSync(where, { recursive: true, force: true });
});

test("a manifest that states no selected families fails", () => {
  const { held } = complete();
  assert.deepEqual(scheduled({ order: [] }, held), [
    "the calibration manifest states no selected families",
  ]);
});

/** An arbitrary group of records says nothing about a treatment, so nothing is read from it. */
test("a malformed pair is a schedule error and no frozen-variable difference", () => {
  const where = room();
  const base = JSON.parse(
    fs.readFileSync(path.join(paths.BENCHMARK, "test", "live-record.json"), "utf8"),
  ) as Record<string, unknown>;
  const order = rowsFor("inventory");
  for (const row of order) {
    const into = path.join(where, row.trialId);
    fs.mkdirSync(into, { recursive: true });
    fs.writeFileSync(
      path.join(into, "record.json"),
      JSON.stringify({
        ...base,
        protocol: CURRENT_PROTOCOL.version,
        audit: [],
        ...row,
        arm: "active",
        klin: { commit: "abc", version: "klin 0.2.0", binarySha256: row.trialId },
      }) + "\n",
    );
  }
  fs.writeFileSync(
    path.join(where, "manifest.json"),
    JSON.stringify({ selectedFamilies: ["inventory"], order }) + "\n",
  );
  const problems = verify(where);
  assert.ok(
    problems.some((one) => one.includes("one Active and one Shadow")),
    problems.join(" / "),
  );
  assert.deepEqual(
    problems.filter((one) => one.includes("did not share")),
    [],
    "the frozen variables of an unpaired cell are read only once it is a pair",
  );
  fs.rmSync(where, { recursive: true, force: true });
});
