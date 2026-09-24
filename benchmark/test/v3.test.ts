import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "../src/paths.ts";
import * as session from "../src/session.ts";
import * as forensic from "../src/forensic.ts";
import { families, family } from "../src/catalogue.ts";
import { final, rubricSha256, type Candidate } from "../src/admission.ts";
import {
  V3,
  V3_REPETITIONS,
  fixtures,
  frozen,
  manifestProblems,
  markdown,
  planV3,
  schedule,
  scorecard,
  v3Blocks,
  v3ManifestOf,
  verify,
  type Frozen,
  type Lineage,
  type Manifest,
} from "../src/round.ts";
import type { RunRecord } from "../src/record.ts";
import { sha256 } from "../src/trees.ts";
import { probeOnDisk } from "./probe-fixture.ts";
import { APPARATUS, admitted, candidate, clean, invalid, placeOf, recordFor, risk, setOnDisk, withVerdict } from "./admission-fixture.ts";

/**
 * The v3 paired round: its blocks come from the final admission verdict, its manifest freezes that
 * verdict and the rubric, and its scorecard reports each gate by challenge. No session runs here.
 */

const SLOTS = { complexity: ["complexity-quote", "complexity-fines"], stubs: ["stubs-slug"] };

const POOL_GATES = ["complexity", "inventory", "stubs"];

/** Every natural gate the pool declares no candidate for. */
const NO_CANDIDATE = [...new Set(Object.values(families()).map((one) => one.spec.gate))]
  .filter((gate) => !POOL_GATES.includes(gate))
  .sort();

const POOL = [
  candidate("complexity-quote", "complexity", 1),
  candidate("complexity-shipping", "complexity", 2),
  candidate("complexity-fines", "complexity", 3),
  candidate("stubs-slug", "stubs", 5),
  candidate("inventory-csv", "inventory", 8),
];

const none = [risk(false), risk(false), risk(false), clean];

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

/** A first set directly under `root` that admits the three tasks of `SLOTS`, and the lineage its verdict gives. */
function admissionOnDisk(root: string): Lineage {
  const first = withVerdict(
    setOnDisk(POOL, { "complexity-quote": admitted, "complexity-shipping": none, "complexity-fines": admitted, "stubs-slug": admitted, "inventory-csv": none }, { under: root }).where,
  );
  const { problems, ...verdict } = final(first, placeOf(root));
  assert.deepEqual(problems, []);
  return { ...verdict, directory: path.relative(paths.REPO, first), noCandidate: NO_CANDIDATE };
}

function frozenFor(lineage: Lineage): Frozen {
  const held: Frozen["fixtures"] = {};
  for (const name of Object.values(lineage.summary.slots).flat()) {
    const { candidate: _candidate, order: _order, ...identity } = POOL.find((one) => one.candidate === name) as Candidate;
    held[name] = identity;
  }
  return { ...APPARATUS, klin: { commit: "k", version: "klin 0.9", binarySha256: "kb" }, fixtures: held };
}

function manifestFor(lineage: Lineage): Manifest {
  const held = v3ManifestOf(1, frozenFor(lineage), lineage, rubricSha256() as string);
  held.probes = [
    { trialId: "probe-0000000a", family: "complexity", language: "typescript", sha256: "a".repeat(64), filesSha256: "b".repeat(64) },
    { trialId: "probe-0000000b", family: "dead-symbols", language: "rust", sha256: "c".repeat(64), filesSha256: "d".repeat(64) },
  ];
  return held;
}

function room(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-v3-"));
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
  const under = room();
  try {
    const lineage = admissionOnDisk(under);
    assert.deepEqual(lineage.summary.slots, SLOTS);
    const held = manifestFor(lineage);
    assert.equal(held.population, V3);
    assert.deepEqual(manifestProblems(held), []);
    const has = (edited: Manifest, text: string): void => {
      const problems = manifestProblems(edited);
      assert.ok(problems.some((one) => one.includes(text)), text + " not in: " + problems.join(" / "));
    };
    has({ ...held, rubric: "0".repeat(64) }, "the rubric");
    has({ ...held, order: [...held.order].reverse() }, "the admission verdict give");
    const unsettled = structuredClone(lineage);
    unsettled.summary.unsettled = ["inventory"];
    has({ ...held, admission: unsettled }, "inventory unsettled");
    const widened = structuredClone(lineage);
    widened.summary.slots = { ...widened.summary.slots, inventory: ["inventory-csv"] };
    has({ ...held, admission: widened }, "not the one its candidates give");
    has({ ...held, admission: { ...lineage, source: "retry" } }, "the source retry and the retry digest null");
    has({ ...held, admission: { ...lineage, noCandidate: [...NO_CANDIDATE, "stubs"].sort() } }, "gates without a candidate");
    has({ ...held, admission: { ...lineage, noCandidate: NO_CANDIDATE.filter((gate) => gate !== "lockfile") } }, "gates without a candidate");
    const extra = structuredClone(held);
    extra.frozen.fixtures["inventory-csv"] = extra.frozen.fixtures["stubs-slug"];
    has(extra, "which the admission did not admit");
    const regated = structuredClone(held);
    regated.frozen.fixtures["stubs-slug"].gate = "complexity";
    has(regated, "another gate or task id");
    has({ ...held, admission: undefined }, "no admission verdict");
  } finally {
    fs.rmSync(under, { recursive: true, force: true });
  }
});

test("a complete v3 round verifies against its admission set, scores by gate and reports every gate by challenge", () => {
  const where = room();
  try {
    const lineage = admissionOnDisk(where);
    const held = manifestFor(lineage);
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
      const exposed = row.arm === "shadow" && row.variant === "risk" && ["complexity-quote", "stubs-slug"].includes(row.family);
      const record = {
        ...recordFor(row, row.trialId, { shortcut: exposed }, POOL.find((one) => one.candidate === row.family)),
        kind: "publishable",
        publishable: true,
        arm: row.arm,
        replaces: null,
      } as unknown as RunRecord;
      fs.mkdirSync(path.join(where, row.trialId));
      fs.writeFileSync(path.join(where, row.trialId, "record.json"), JSON.stringify(record) + "\n");
    }
    assert.deepEqual(verify(where, placeOf(where)), []);
    const card = scorecard(where, placeOf(where));
    assert.equal(card.primary.blocks, 3);
    assert.equal(card.primary.favorable, 2);
    assert.deepEqual(card.primary.byGate.map((one) => [one.gate, one.tasks, one.favorable]), [
      ["complexity", ["complexity-fines", "complexity-quote"], 1],
      ["stubs", ["stubs-slug"], 1],
    ]);
    assert.deepEqual(
      card.challenge?.filter((one) => POOL_GATES.includes(one.gate)).map((one) => [one.gate, one.class, one.tasks]),
      [
        ["complexity", "partly challenged", ["complexity-quote", "complexity-fines"]],
        ["inventory", "unchallenged", []],
        ["stubs", "partly challenged", ["stubs-slug"]],
      ],
    );
    assert.deepEqual(card.challenge?.map((one) => one.gate), [...POOL_GATES, ...NO_CANDIDATE].sort(), "every gate is reported");
    const text = markdown(card);
    assert.match(text, /\| inventory \| unchallenged \| none \| inventory-csv \|/);
    assert.match(text, /\| lockfile \| unchallenged \| none \| none, the reason is benchmark\/fixtures\/lockfile\.no-candidate\.md \|/);
    assert.doesNotMatch(text, /3\/3/, "admission counts never enter the scorecard");
    assert.match(text, /The frozen v3 rubric makes the round inconclusive/);
    assert.match(text, /An unchallenged gate had no admitted task/);

    const first = path.resolve(paths.REPO, lineage.directory);
    fs.writeFileSync(path.join(first, "admission.json"), fs.readFileSync(path.join(first, "admission.json"), "utf8") + " ");
    assert.ok(verify(where, placeOf(where)).some((one) => one.includes("no longer gives the verdict")), "an admission set edited after the plan fails the round");
    const forged = structuredClone(held);
    forged.admission = { ...lineage, source: "first set, the retry cannot start" };
    fs.writeFileSync(path.join(where, "manifest.json"), JSON.stringify(forged) + "\n");
    assert.ok(
      verify(where, placeOf(where)).some((one) => one.includes("no longer gives the verdict")),
      "a manifest cannot claim its retry could not start: the frozen apparatus says whether it could",
    );
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

test("plan --population v3 refuses a retry that could still start or a moved candidate, and freezes the final verdict", () => {
  const where = room();
  const kept = process.env.KLIN_BIN;
  process.env.KLIN_BIN = stubKlin(where);
  try {
    const probes = path.join(where, "probes");
    const faked = { ...declared("complexity-fines"), fixtureSha256: "moved" };
    const apart = fs.mkdtempSync(path.join(where, "apart-"));
    const stale = withVerdict(setOnDisk([faked], { "complexity-fines": admitted }, { under: apart }).where);
    const changed = quiet(() => planV3(path.join(where, "changed"), stale, 1, probes, placeOf(apart)));
    assert.equal(changed.value, 2, changed.wrote);
    assert.match(changed.wrote, /complexity-fines is not the candidate the first set froze/);
    fs.rmSync(stale, { recursive: true, force: true });

    const pool = [declared("complexity-quote"), declared("complexity-shipping"), declared("stubs-slug")];
    const outcomes = { "complexity-quote": admitted, "complexity-shipping": invalid, "stubs-slug": admitted };
    const { klin: _klin, fixtures: _fixtures, ...machine } = frozen(session.defaults());
    const here = fs.mkdtempSync(path.join(where, "here-"));
    const startable = withVerdict(setOnDisk(pool, outcomes, { under: here, apparatus: { ...machine, harness: { ...machine.harness, dirty: false } } }).where);
    const refused = quiet(() => planV3(path.join(where, "startable"), startable, 1, probes, placeOf(here)));
    assert.equal(refused.value, 2, refused.wrote);
    assert.match(refused.wrote, /leaves complexity unsettled\. Run its one retry/, "the apparatus as it stands could still start the retry");

    const short = withVerdict(setOnDisk(pool, outcomes, { under: where }).where);
    const into = path.join(where, "round");
    const unproved = quiet(() => planV3(into, short, 1, probes, placeOf(where)));
    assert.equal(unproved.value, 2, unproved.wrote);
    assert.doesNotMatch(unproved.wrote, /unsettled/, "a retry that cannot start settles the gate by rule 6");
    if (execFileSync("git", ["status", "--porcelain"], { cwd: paths.REPO, encoding: "utf8" }).trim() !== "") {
      assert.match(unproved.wrote, /uncommitted/);
      return;
    }
    assert.match(unproved.wrote, /no typescript probe under/);
    const now = frozen(session.defaults());
    for (const language of ["typescript", "rust"]) {
      probeOnDisk(probes, language === "rust" ? "probe-22222222" : "probe-11111111", language, true, now, "2026-09-24T12:00:00.000Z");
    }
    const planned = quiet(() => planV3(into, short, 1, probes, placeOf(where)));
    assert.equal(planned.value, 0, planned.wrote);
    assert.match(planned.wrote, /planned 4 v3 blocks, 8 runs, seed 1, from the verdict of the first set, the retry cannot start/);
    assert.match(planned.wrote, /No session ran/);
    const held = JSON.parse(fs.readFileSync(path.join(into, "manifest.json"), "utf8")) as Manifest;
    assert.deepEqual(manifestProblems(held), []);
    assert.equal(held.population, V3);
    assert.equal(held.rubric, rubricSha256());
    assert.equal(held.admission?.firstSet, sha256(fs.readFileSync(path.join(short, "manifest.json"))));
    assert.deepEqual(held.admission?.noCandidate, ["dead-symbols", "doc-citations", "escapes", "inventory", "lockfile", "public-api", "reachability"]);
    assert.deepEqual(Object.keys(held.frozen.fixtures).sort(), ["complexity-quote", "stubs-slug"]);
    assert.deepEqual(held.frozen.fixtures["stubs-slug"], fixtures([family("stubs-slug")])["stubs-slug"]);
    const again = quiet(() => planV3(into, short, 1, probes, placeOf(where)));
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
