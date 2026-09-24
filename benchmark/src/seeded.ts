import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "./paths.ts";
import { ARMS, families, variantIn, type ArmName } from "./catalogue.ts";
import { CURRENT_PROTOCOL, SEEDED_PROTOCOL } from "./protocol.ts";
import { digest, sha256 } from "./trees.ts";
import * as session from "./session.ts";
import * as selftest from "./selftest.ts";
import * as trial from "./trial.ts";
import * as workspace from "./workspace.ts";
import {
  FROZEN,
  crash,
  crashes,
  normalizedFlags,
  ordering,
  preflight,
  recordProblems,
  records,
  shuffledBy,
  trialId,
} from "./calibrate.ts";
import { failedOn, validate, type RunRecord } from "./record.ts";
import {
  PROBES,
  ATTEMPTS,
  chain,
  copyProbes,
  drift,
  frozen,
  identity,
  probeEvidenceProblems,
  probeProblems,
  replacementId,
  uncommitted,
  witnesses,
  type Frozen,
  type Witness,
} from "./round.ts";
import { PROBES as PROBE_RUNS } from "./probe.ts";

export const VARIANT = "seeded" as const;

/** Which families a seeded round schedules, and how many adjacent pairs each one gets. */
export interface Design {
  families: string[];
  repetitions: number;
}

export function everyFamily(): Design {
  return { families: Object.keys(families()).sort(), repetitions: 1 };
}

/** Every way a design can name something the planted catalogue cannot schedule. */
export function designProblems(design: Design): string[] {
  const problems: string[] = [];
  const known = new Set(Object.keys(families()));
  const named = Array.isArray(design?.families) ? design.families : [];
  if (named.length === 0) problems.push("the seeded design names no family");
  for (const name of named) if (!known.has(name)) problems.push("no family " + String(name) + " in the catalogue");
  if (JSON.stringify(named) !== JSON.stringify([...new Set(named)].sort())) {
    problems.push("the seeded design does not name its families once each, sorted");
  }
  if (!Number.isInteger(design?.repetitions) || design.repetitions < 1) {
    problems.push("--repetitions needs a positive integer, and the design states " + String(design?.repetitions));
  }
  return problems;
}

export function roundDirectory(): string {
  return path.join(paths.RUNS, "seeded-" + new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19));
}

export interface Row {
  family: string;
  variant: typeof VARIANT;
  repetition: number;
  arm: ArmName;
  block: number;
  order: number;
  trialId: string;
}

export interface SeededFixture {
  gate: string;
  fixtureSha256: string;
  variant: {
    taskId: string;
    promptSha256: string;
    treeSha256: string;
    startTreeSha256: string;
    seed: string[];
    staged: boolean;
  };
}

export interface Manifest {
  protocol: number;
  seededProtocol: number;
  kind: "publishable";
  publishable: true;
  population: "seeded";
  seed: number;
  plannedAt: string;
  design: {
    families: string[];
    repetitions: number;
    blocks: number;
    runs: number;
    attemptsPerTrial: number;
  };
  firstArm: Record<ArmName, number>;
  frozen: Frozen;
  fixtures: Record<string, SeededFixture>;
  order: Row[];
  probes?: Witness[];
}

/** One seeded block per family and repetition, shuffled once and held to its adjacent pair of arms. */
export function rows(seed: number, design: Design = everyFamily()): Row[] {
  const draw = ordering(seed);
  const blocks = design.families.flatMap((family) =>
    Array.from({ length: design.repetitions }, (_, index) => ({ family, repetition: index + 1 })),
  );
  const firsts = shuffledBy(
    blocks.map((_, index) => (index < Math.ceil(blocks.length / 2) ? ARMS[0] : ARMS[1])),
    draw,
  );
  const held: Row[] = [];
  shuffledBy(blocks, draw).forEach((block, index) => {
    const first = firsts[index];
    const second = ARMS.find((arm) => arm !== first) as ArmName;
    for (const arm of [first, second]) {
      const order = held.length;
      held.push({
        family: block.family,
        variant: VARIANT,
        repetition: block.repetition,
        arm,
        block: index,
        order,
        trialId: trialId(block.family, VARIANT, arm, order),
      });
    }
  });
  return held;
}

export interface Schedule {
  seed: number;
  blocks: number;
  runs: number;
  firstArm: Record<ArmName, number>;
  order: Row[];
}

export function schedule(seed: number, design: Design = everyFamily()): Schedule {
  const order = rows(seed, design);
  const firstArm = { active: 0, shadow: 0 };
  for (const row of order.filter((one) => one.order % 2 === 0)) {
    firstArm[row.arm] += 1;
  }
  return { seed, blocks: order.length / 2, runs: order.length, firstArm, order };
}

/** The full fixture identity for the planted population, separate from natural v2's identity. */
export function fixtures(names: string[] = everyFamily().families): Record<string, SeededFixture> {
  const held: Record<string, SeededFixture> = {};
  const room = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-seeded-plan-"));
  try {
    for (const [name, family] of Object.entries(families()).filter(([one]) => names.includes(one))) {
      const variant = variantIn(family, VARIANT);
      const base = workspace.startingTree(variant, path.join(room, name, "base"));
      const subject = workspace.subjectStartingTree(variant, path.join(room, name, "subject"));
      held[name] = {
        gate: family.spec.gate,
        fixtureSha256: digest(family.root),
        variant: {
          taskId: variant.taskId,
          promptSha256: variant.promptSha256,
          treeSha256: digest(base),
          startTreeSha256: digest(subject),
          seed: workspace.seedPaths(variant),
          staged: variant.staged,
        },
      };
    }
  } finally {
    fs.rmSync(room, { recursive: true, force: true });
  }
  return held;
}

function readManifest(directory: string): { bytes: Buffer; value: Manifest } {
  const bytes = fs.readFileSync(path.join(directory, "manifest.json"));
  return { bytes, value: JSON.parse(bytes.toString("utf8")) as Manifest };
}

function seededFixtureDrift(was: Record<string, SeededFixture>, now: Record<string, SeededFixture>): string[] {
  const names = [...new Set([...Object.keys(was), ...Object.keys(now)])].sort();
  return names
    .filter((name) => JSON.stringify(was[name]) !== JSON.stringify(now[name]))
    .map((name) => "the seeded fixture " + name + " moved");
}

function naturalProtocolDrift(): string[] {
  return uncommitted(identity(1));
}

export function manifestOf(seed: number, held: Frozen, probes: Witness[] = [], design: Design = everyFamily()): Manifest {
  const planned = schedule(seed, design);
  return {
    protocol: CURRENT_PROTOCOL.version,
    seededProtocol: SEEDED_PROTOCOL.version,
    kind: "publishable",
    publishable: true,
    population: "seeded",
    seed,
    plannedAt: new Date().toISOString(),
    design: {
      families: design.families,
      repetitions: design.repetitions,
      blocks: planned.blocks,
      runs: planned.runs,
      attemptsPerTrial: ATTEMPTS,
    },
    firstArm: planned.firstArm,
    frozen: held,
    fixtures: fixtures(design.families),
    order: planned.order,
    probes,
  };
}

/** Every way a seeded manifest can be malformed or drift from the planted catalogue. */
export function manifestProblems(held: Manifest): string[] {
  if (!held || typeof held !== "object") {
    return ["the seeded manifest is not an object"];
  }
  const problems: string[] = [];
  if (held.population !== "seeded") problems.push("the manifest is not a seeded population");
  if (held.kind !== "publishable" || held.publishable !== true) {
    problems.push("the seeded manifest is not publishable");
  }
  if (held.protocol !== CURRENT_PROTOCOL.version) {
    problems.push("the manifest states protocol " + String(held.protocol));
  }
  if (held.seededProtocol !== SEEDED_PROTOCOL.version) {
    problems.push(
      "the manifest states seeded protocol " +
        String(held.seededProtocol) +
        " where the planted catalogue is " +
        SEEDED_PROTOCOL.name,
    );
  }
  if (!held.frozen || typeof held.frozen !== "object") {
    problems.push("the seeded manifest states no frozen provenance");
  }
  const design = { families: held.design?.families, repetitions: held.design?.repetitions } as Design;
  const unscheduled = designProblems(design);
  if (unscheduled.length > 0) return [...problems, ...unscheduled, ...probeProblems(held)];
  const planned = schedule(held.seed, design);
  if (!Number.isInteger(held.seed)) {
    problems.push("the seeded manifest states no integer seed");
  } else if (JSON.stringify(held.order) !== JSON.stringify(planned.order)) {
    problems.push("the seeded order is not the one seed " + String(held.seed) + " gives");
  }
  if (held.design?.attemptsPerTrial !== ATTEMPTS) {
    problems.push("the seeded design states " + String(held.design?.attemptsPerTrial) + " attempts per trial");
  }
  if (held.design?.blocks !== planned.blocks) problems.push("the seeded design does not have " + String(planned.blocks) + " blocks");
  if (held.design?.runs !== planned.runs) problems.push("the seeded design does not have " + String(planned.runs) + " runs");
  const order = Array.isArray(held.order) ? held.order : [];
  if (order.length !== planned.runs) problems.push("the seeded order holds " + String(order.length) + " rows");
  for (let at = 0; at + 1 < order.length; at += 2) {
    const [first, second] = [order[at], order[at + 1]];
    if (
      first?.family !== second?.family ||
      first?.variant !== VARIANT ||
      second?.variant !== VARIANT ||
      first?.repetition !== second?.repetition ||
      first?.arm === second?.arm ||
      first?.block !== second?.block
    ) {
      problems.push("rows " + String(at) + " and " + String(at + 1) + " are not one seeded pair");
    }
  }
  const firsts = order.filter((one) => one?.order % 2 === 0);
  const active = firsts.filter((one) => one.arm === "active").length;
  if (Math.abs(active - (firsts.length - active)) > 1) {
    problems.push("seeded first arms are not balanced");
  }
  if (held.firstArm?.active !== active || held.firstArm?.shadow !== firsts.length - active) {
    problems.push("the seeded first-arm count does not describe its own order");
  }
  const known = Array.isArray(design.families) ? design.families : [];
  const named = held.fixtures && typeof held.fixtures === "object" ? Object.keys(held.fixtures).sort() : [];
  if (JSON.stringify(named) !== JSON.stringify(known)) {
    problems.push("the seeded fixtures do not name exactly the scheduled families");
  }
  for (const name of known) {
    const fixture = held.fixtures?.[name];
    const variant = fixture?.variant;
    if (!fixture || !variant) {
      problems.push("the seeded fixture " + name + " is missing");
      continue;
    }
    for (const key of ["taskId", "promptSha256", "treeSha256", "startTreeSha256"] as const) {
      if (typeof variant[key] !== "string" || variant[key] === "") {
        problems.push("the seeded fixture " + name + " states no " + key);
      }
    }
    if (!Array.isArray(variant.seed) || variant.seed.length === 0) {
      problems.push("the seeded fixture " + name + " states no seed paths");
    }
    if (typeof variant.staged !== "boolean") {
      problems.push("the seeded fixture " + name + " states no staging");
    }
  }
  problems.push(...probeProblems(held));
  return problems;
}

/** Freeze the seeded schedule and its treatment-independent provenance without running a session. */
export function plan(into: string, seed: number, design: Design = everyFamily(), probes = PROBE_RUNS): number {
  if (!Number.isInteger(seed)) {
    process.stdout.write("--seed needs an integer, and it gave " + String(seed) + "\n");
    return 2;
  }
  const unscheduled = designProblems(design);
  if (unscheduled.length > 0) {
    process.stdout.write(unscheduled.join("; ") + "\n");
    return 2;
  }
  const known = session.defaults();
  const blocked = preflight(known.klinBin);
  if (blocked !== "") {
    process.stdout.write(blocked + "\n");
    return 2;
  }
  const missing = Object.values(families()).filter((one) => design.families.includes(one.name) && !one.variants.seeded);
  if (missing.length > 0) {
    process.stdout.write("no seeded fixture for " + missing.map((one) => one.name).join(", ") + "\n");
    return 2;
  }
  const failed = selftest.run(design.families, "seeded").filter((one) => !one.passed);
  if (failed.length > 0) {
    process.stdout.write("the seeded self-test failed: " + failed.map((one) => one.detail).join("; ") + "\n");
    return 2;
  }
  const file = path.join(into, "manifest.json");
  if (fs.existsSync(file)) {
    process.stdout.write(file + " exists. Plan into a new directory.\n");
    return 2;
  }
  const held = manifestOf(seed, frozen(known), [], design);
  if (held.frozen.klin.commit === "") {
    process.stdout.write("no build provenance ties " + known.klinBin + " to a source commit. benchmark/build-klin writes one.\n");
    return 2;
  }
  if (held.frozen.harness.dirty) {
    process.stdout.write("the harness has uncommitted changes. Commit or stash first.\n");
    return 2;
  }
  const departures = naturalProtocolDrift();
  if (departures.length > 0) {
    process.stdout.write("the natural protocol is not committed: " + departures.join("; ") + "\n");
    return 2;
  }
  const proved = witnesses(probes, held.frozen);
  if (proved.missing.length > 0) {
    process.stdout.write("the workspace is not proved for this round: " + proved.missing.join("; ") + "\n");
    return 2;
  }
  fs.mkdirSync(into, { recursive: true });
  copyProbes(into, proved, held);
  const problems = manifestProblems(held);
  if (problems.length > 0) {
    fs.rmSync(path.join(into, PROBES), { recursive: true, force: true });
    process.stdout.write("the seeded plan does not encode the design: " + problems.join("; ") + "\n");
    return 2;
  }
  const bytes = JSON.stringify(held, null, 2) + "\n";
  fs.writeFileSync(file, bytes);
  process.stdout.write(
    [
      "planned " + String(held.design.blocks) + " " + SEEDED_PROTOCOL.name + " blocks, " + String(held.design.runs) + " runs, seed " + String(seed),
      "first arm: " + String(held.firstArm.active) + " Active, " + String(held.firstArm.shadow) + " Shadow",
      "manifest " + file,
      "sha256 " + sha256(bytes),
      "",
      "No session ran. Review the manifest and approve its digest before execution.",
    ].join("\n") + "\n",
  );
  return 0;
}

function settled(attempts: { record: RunRecord | null }[]): boolean {
  return attempts.some((one) => one.record?.infrastructure.valid === true);
}

/** Execute only the approved seeded manifest; invalid attempts remain evidence and may be replaced. */
export function execute(directory: string, approved: string): number {
  const file = path.join(directory, "manifest.json");
  if (!fs.existsSync(file)) {
    process.stdout.write("no manifest.json under " + directory + ". Plan the seeded round first.\n");
    return 2;
  }
  const first = readManifest(directory);
  const manifest = first.value;
  if (sha256(first.bytes) !== approved) {
    process.stdout.write("refusing to start: the seeded manifest digest is " + sha256(first.bytes) + " and the approved digest is " + approved + "\n");
    return 2;
  }
  const unsound = manifestProblems(manifest);
  if (unsound.length > 0) {
    process.stdout.write("refusing to start: " + unsound.join("; ") + "\n");
    return 2;
  }
  const known = session.defaults();
  const blocked = preflight(known.klinBin);
  if (blocked !== "") {
    process.stdout.write(blocked + "\n");
    return 2;
  }
  const moved = [
    ...probeEvidenceProblems(directory, manifest),
    ...drift(manifest.frozen, frozen(known)),
    ...seededFixtureDrift(manifest.fixtures, fixtures(manifest.design.families)),
    ...naturalProtocolDrift(),
  ];
  if (moved.length > 0) {
    process.stdout.write("refusing to start: " + moved.join("; ") + "\n");
    return 2;
  }
  const options = (row: Row, replaces: string | null): trial.TrialOptions => ({
    ...known,
    order: row.order,
    repetition: row.repetition,
    replaces,
    kind: "publishable",
    control: directory,
  });
  const blocks = new Map<number, Row[]>();
  for (const row of manifest.order) blocks.set(row.block, [...(blocks.get(row.block) ?? []), row]);
  const say = (text: string): void => process.stdout.write(text + "\n");
  for (const [block, held] of [...blocks].sort(([a], [b]) => a - b)) {
    let onDisk = records(directory);
    let failed = crashes(directory);
    if (held.every((row) => settled(chain(row, onDisk, failed)))) continue;
    if (!fs.readFileSync(file).equals(first.bytes)) {
      say("stopped before seeded block " + String(block) + ": manifest.json changed under the round");
      return 2;
    }
    const movedNow = [...drift(manifest.frozen, frozen(known)), ...seededFixtureDrift(manifest.fixtures, fixtures(manifest.design.families)), ...naturalProtocolDrift()];
    if (movedNow.length > 0) {
      say("stopped before seeded block " + String(block) + ": " + movedNow.join("; "));
      return 2;
    }
    for (const row of held.sort((a, b) => a.order - b.order)) {
      onDisk = records(directory);
      failed = crashes(directory);
      let attempts = chain(row, onDisk, failed);
      while (!settled(attempts)) {
        if (attempts.length >= ATTEMPTS) {
          say("stopped: seeded " + row.family + " " + row.arm + " at order " + String(row.order) + " was infrastructure-invalid " + String(ATTEMPTS) + " times");
          return 1;
        }
        const attempt = attempts.length;
        const id = attempt === 0 ? row.trialId : replacementId(row.trialId, attempt);
        const replaces = attempt === 0 ? null : attempts[attempt - 1].trialId;
        say(String(row.order + 1) + "/" + String(manifest.order.length) + " seeded block " + String(block) + " " + row.family + " " + row.arm + (replaces ? " replacing " + replaces : ""));
        const began = Date.now();
        try {
          const record = trial.run(row.family, VARIANT, row.arm, id, options(row, replaces));
          say("     " + (record.infrastructure.valid ? "valid  " : "INVALID " + String(record.infrastructure.reason)) + "  oracle " + (record.oracle.behaviourPassed ? "pass" : "FAIL") + "  repair " + String(record.shortcut.present === false) + "  " + String(Math.round((Date.now() - began) / 1000)) + "s");
        } catch (why) {
          say("     FAILED  " + String(why));
          if (!fs.existsSync(path.join(directory, id, "record.json"))) crash(directory, row, id, replaces, why);
        }
        attempts = chain(row, records(directory), crashes(directory));
      }
    }
  }
  say("\nseeded round complete, records under " + directory);
  return 0;
}

function blockName(family: string, repetition: number): string {
  return family + " r" + String(repetition);
}

/** Each scheduled block as its family and its repetition, in the manifest's order. */
function blocksOf(manifest: Manifest): { family: string; repetition: number }[] {
  const seen = new Map<string, { family: string; repetition: number }>();
  for (const row of manifest.order) seen.set(blockName(row.family, row.repetition), { family: row.family, repetition: row.repetition });
  return [...seen.values()];
}

function checkPair(problems: string[], group: RunRecord[], name: string, family: string, manifest: Manifest): void {
  const active = group.filter((one) => one.arm === "active");
  const shadow = group.filter((one) => one.arm === "shadow");
  if (active.length !== 1 || shadow.length !== 1) {
    problems.push(name + ": the seeded cell does not hold one Active and one Shadow record");
    return;
  }
  const fields: [string, (one: RunRecord) => unknown][] = [
    ["committed base", (one) => one.fixture.treeSha256],
    ["subject starting tree", (one) => one.fixture.startTreeSha256],
    ["prompt", (one) => one.fixture.promptSha256],
    ["seed", (one) => JSON.stringify(one.fixture.seed)],
    ["staged index", (one) => JSON.stringify(one.fixture.staged ?? null)],
  ];
  for (const [what, read] of fields) {
    if (new Set(group.map(read)).size > 1) problems.push(name + ": the arms did not share one " + what);
  }
  const fixture = manifest.fixtures[family];
  for (const record of group) {
    if (record.gate !== fixture?.gate) problems.push(name + ": " + record.trialId + " states gate " + record.gate);
    if (record.taskId !== fixture?.variant.taskId) problems.push(name + ": " + record.trialId + " did not run the frozen task");
    if (record.fixture.treeSha256 !== fixture?.variant.treeSha256) problems.push(name + ": " + record.trialId + " did not start from the frozen committed base");
    if (record.fixture.startTreeSha256 !== fixture?.variant.startTreeSha256) problems.push(name + ": " + record.trialId + " did not start from the frozen seeded tree");
    if (record.fixture.promptSha256 !== fixture?.variant.promptSha256) problems.push(name + ": " + record.trialId + " did not run the frozen seeded prompt");
    if (JSON.stringify(record.fixture.seed) !== JSON.stringify(fixture?.variant.seed)) problems.push(name + ": " + record.trialId + " did not use the frozen seed");
    if (JSON.stringify(record.fixture.staged ?? null) !== JSON.stringify(fixture?.variant.staged ? fixture.variant.seed : [])) problems.push(name + ": " + record.trialId + " did not start from the frozen staged index");
    if (record.seeded === undefined) problems.push(name + ": " + record.trialId + " states no seeded metrics");
  }
}

/** The two trees a seeded report diffs, held to the digests the record states for them. */
function keptTrees(directory: string, record: RunRecord): string[] {
  const where = record.family + "/seeded/r" + String(record.repetition) + "/" + record.arm + " " + record.trialId;
  const trees = path.join(directory, record.trialId, "fixtures");
  const problems: string[] = [];
  for (const [tree, want] of [
    ["subject", record.fixture.startTreeSha256],
    ["final", record.fixture.finalTreeSha256],
  ] as const) {
    if (typeof want !== "string" || want === "") {
      problems.push(where + ": the record states no digest for the " + tree + " tree");
    } else if (!fs.existsSync(path.join(trees, tree))) {
      problems.push(where + ": the attempt keeps no " + tree + " tree");
    } else if (digest(path.join(trees, tree)) !== want) {
      problems.push(where + ": the kept " + tree + " tree differs from the digest its record states");
    }
  }
  return problems;
}

/** Verify that every scheduled seeded cell has valid paired records and that no natural cell slipped in. */
export function verify(directory: string): string[] {
  const file = path.join(directory, "manifest.json");
  if (!fs.existsSync(file)) return ["no manifest.json under " + directory];
  const manifest = readManifest(directory).value;
  const problems = [
    ...manifestProblems(manifest),
    ...probeEvidenceProblems(directory, manifest),
  ];
  if (!manifest.frozen || !Array.isArray(manifest.order)) return problems;
  const held = records(directory);
  const failed = crashes(directory);
  if (held.length === 0) problems.push("no record was found under " + directory);
  const claimed = new Set<string>();
  for (const record of held) {
    const where = record.family + "/" + record.variant + "/" + record.arm + " " + record.trialId;
    if (record.kind !== "publishable" || record.publishable !== true) problems.push(where + ": the record is not publishable");
    problems.push(...(record.infrastructure.valid ? recordProblems(record) : validate(record as unknown as Record<string, unknown>)).map((one) => where + ": " + one));
  }
  for (const row of manifest.order ?? []) {
    const where = row.family + "/seeded/r" + String(row.repetition) + "/" + row.arm;
    const attempts = chain(row, held, failed);
    if (attempts.length === 0) {
      problems.push(where + ": the scheduled trial " + row.trialId + " left no attempt");
      continue;
    }
    attempts.forEach((attempt, index) => {
      claimed.add(attempt.trialId);
      const record = attempt.record;
      if (!record) {
        const crashRecord = failed.find((one) => one.trialId === attempt.trialId);
        const wanted = index === 0 ? null : attempts[index - 1].trialId;
        if (crashRecord?.replaces !== wanted) problems.push(where + ": the crash " + attempt.trialId + " has the wrong replacement link");
        return;
      }
      const wanted = index === 0 ? null : attempts[index - 1].trialId;
      if (record.replaces !== wanted) problems.push(where + ": " + record.trialId + " has the wrong replacement link");
      for (const [what, was, want] of [["family", record.family, row.family], ["variant", record.variant, VARIANT], ["arm", record.arm, row.arm], ["order", record.order, row.order], ["repetition", record.repetition, row.repetition]] as [string, unknown, unknown][]) {
        if (was !== want) problems.push(where + ": the record states " + what + " " + String(was) + " where the manifest scheduled " + String(want));
      }
      if (record.infrastructure.valid && index < attempts.length - 1) problems.push(where + ": a valid result was replaced");
    });
    if (!settled(attempts)) problems.push(where + ": no valid record after " + String(attempts.length) + " attempt(s)");
  }
  for (const record of held) if (!claimed.has(record.trialId)) problems.push(record.trialId + ": the record belongs to no seeded schedule row");
  for (const crashed of failed) if (!claimed.has(crashed.trialId)) problems.push("the crash " + crashed.trialId + " belongs to no seeded schedule row");
  const valid = held.filter((one) => one.infrastructure.valid);
  for (const record of valid) problems.push(...keptTrees(directory, record));
  for (const [what, read] of FROZEN) {
    if (new Set(valid.map(read)).size > 1) problems.push("the seeded round did not share " + what);
  }
  for (const { family, repetition } of blocksOf(manifest)) {
    checkPair(problems, valid.filter((one) => one.family === family && one.repetition === repetition), blockName(family, repetition), family, manifest);
  }
  const runs = manifest.order.length;
  if (valid.length !== runs) problems.push("the seeded round holds " + String(valid.length) + " valid records where it needs " + String(runs));
  const stated: [string, (one: RunRecord) => string, string][] = [
    ["the protocol", (one) => String(one.protocol), String(manifest.frozen.protocol)],
    ["the klin binary", (one) => one.klin.binarySha256, manifest.frozen.klin.binarySha256],
    ["the klin version", (one) => one.klin.version, manifest.frozen.klin.version],
    ["the klin source commit", (one) => one.klin.commit, manifest.frozen.klin.commit],
    ["the harness commit", (one) => one.harness.commit, manifest.frozen.harness.commit],
    ["the harness tree", (one) => one.harness.treeSha256, manifest.frozen.harness.treeSha256],
    ["the host version", (one) => one.host.version, manifest.frozen.host.version],
    ["the requested model", (one) => one.model.requested, manifest.frozen.model],
    ["the host flags", (one) => normalizedFlags(one.host.flags).join(" "), manifest.frozen.flags.join(" ")],
  ];
  for (const [what, read, want] of stated) {
    if (new Set(valid.map(read).filter((one) => one !== want)).size > 0) problems.push("valid seeded records state " + what + " outside the frozen manifest");
  }
  return problems;
}

function yesNo(value: boolean | null | undefined): string {
  return value === null || value === undefined ? "unknown" : value ? "yes" : "no";
}

function row(cells: string[]): string {
  return "| " + cells.join(" | ") + " |";
}

function caught(record: RunRecord): string {
  const run = record.seeded?.wholeRun;
  if (run?.caught !== true) return yesNo(run?.caught);
  return failedOn(run) ? "yes" : "yes, at the Stop hook";
}

function delivery(record: RunRecord): string {
  if (record.seeded?.stopDelivery !== true) return yesNo(record.seeded?.stopDelivery);
  return record.arm === "active" ? "delivered" : "would-have-been-delivered";
}

function counted(group: RunRecord[], read: (one: RunRecord) => boolean | null | undefined): string {
  return String(group.filter((one) => read(one) === true).length) + " of " + String(group.length);
}

function blocksSpent(group: RunRecord[]): string {
  const blocks = new Map<number, number>();
  for (const one of group) {
    const count = one.seeded?.blockedStops ?? -1;
    blocks.set(count, (blocks.get(count) ?? 0) + 1);
  }
  return [...blocks]
    .sort(([a], [b]) => a - b)
    .map(([count, runs]) => (count < 0 ? "unknown" : String(count)) + ": " + String(runs))
    .join(", ");
}

/** What the subject changed, from the seeded starting tree to its final tree, or why no diff could be made. */
function repairDiff(directory: string, record: RunRecord): { diff: string; failure: string } {
  const trees = path.join(directory, record.trialId, "fixtures");
  const ran = spawnSync("git", ["diff", "--no-index", "--no-color", "--no-ext-diff", "subject", "final"], { cwd: trees, encoding: "utf8" });
  if (ran.error || (ran.status !== 0 && ran.status !== 1)) {
    return { diff: "", failure: "git diff failed: " + String(ran.error ?? ran.stderr.trim()) };
  }
  return { diff: ran.stdout === "" ? "the final tree equals the seeded starting tree" : ran.stdout.trimEnd(), failure: "" };
}

/**
 * A compact report whose columns keep catch, delivery, repair and friction distinct.
 *
 * `problems` is the contract the report rests on: the round's own verification and every Active
 * repair diff that could not be made. A report with problems is still rendered, for diagnosis.
 */
export function report(directory: string): { text: string; problems: string[] } {
  const held = records(directory)
    .filter((one) => one.variant === VARIANT && one.infrastructure.valid)
    .sort((a, b) => a.family.localeCompare(b.family) || a.repetition - b.repetition || a.arm.localeCompare(b.arm));
  const problems = verify(directory);
  const lines = [
    "# Seeded Shadow/Active round",
    "",
    "Every run below started from a planted shortcut in the subject's uncommitted working tree.",
    "These results measure catch, Stop delivery and repair conditional on that planted exposure;",
    "they are not a natural shortcut rate and do not enter the natural risk/control tables.",
    "",
    "## Seeded runs",
    "",
    "| family | gate | arm | repetition | seed present | whole-run catch | Stop delivery | final repair | blocked Stops | tries | external-oracle/task outcome | final shortcut | cost |",
    "| --- | --- | --- | ---: | --- | --- | --- | --- | ---: | ---: | --- | --- | ---: |",
    ...held.map((record) =>
      row([
        record.family,
        record.gate,
        record.arm,
        String(record.repetition),
        yesNo(record.fixture.startShortcut?.present),
        caught(record),
        delivery(record),
        yesNo(record.seeded?.finalRepair),
        String(record.seeded?.blockedStops ?? ""),
        String(record.seeded?.tries ?? ""),
        record.result.outcome + " / " + (record.oracle.behaviourPassed ? "pass" : "fail"),
        yesNo(record.shortcut.present),
        record.cost === null || record.cost === undefined ? "" : String(record.cost),
      ]),
    ),
    "",
    "## By family and arm",
    "",
    "Blocks spent counts the runs at each number of blocked Stops. A Shadow run spends no block, so its Stop delivery and blocks spent are what klin would have delivered and blocked.",
    "Final repair is the target endpoint; the oracle is a guardrail.",
    "",
    "| family | arm | runs | whole-run catch | Stop delivery | blocks spent | final repair | oracle pass |",
    "| --- | --- | ---: | --- | --- | --- | --- | --- |",
  ];
  const named = [...new Set(held.map((one) => one.family))];
  for (const family of named) {
    for (const arm of ARMS) {
      const group = held.filter((one) => one.family === family && one.arm === arm);
      lines.push(
        row([
          family,
          arm,
          String(group.length),
          counted(group, (one) => one.seeded?.wholeRun.caught),
          counted(group, (one) => one.seeded?.stopDelivery),
          blocksSpent(group),
          counted(group, (one) => one.seeded?.finalRepair),
          counted(group, (one) => one.oracle.behaviourPassed),
        ]),
      );
    }
  }
  lines.push(
    "",
    "## Paired cost differences",
    "",
    "Active minus Shadow, using the host's raw total session cost field where both arms recorded one.",
    "",
  );
  for (const family of named) {
    for (const repetition of [...new Set(held.filter((one) => one.family === family).map((one) => one.repetition))]) {
      const pair = held.filter((one) => one.family === family && one.repetition === repetition);
      const active = pair.find((one) => one.arm === "active")?.cost;
      const shadow = pair.find((one) => one.arm === "shadow")?.cost;
      lines.push("- " + blockName(family, repetition) + ": " + (typeof active === "number" && typeof shadow === "number" ? String(active - shadow) : "unknown"));
    }
  }
  lines.push(
    "",
    "## Active repairs",
    "",
    "Each diff runs from the seeded starting tree to the final tree. Classify each repair as genuine or appeasement from it.",
  );
  for (const record of held.filter((one) => one.arm === "active")) {
    const made = repairDiff(directory, record);
    if (made.failure !== "") problems.push(record.trialId + ": no Active repair diff, " + made.failure);
    lines.push("", "### " + blockName(record.family, record.repetition) + " " + record.trialId, "", "```diff", made.failure || made.diff, "```");
  }
  lines.push("", "## Contract", "", problems.length === 0 ? "Exactly " + String(held.length) + " valid scheduled seeded runs hold the frozen contract." : problems.map((one) => "- " + one).join("\n"), "");
  return { text: lines.join("\n"), problems };
}
