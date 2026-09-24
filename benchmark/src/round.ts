import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "./paths.ts";
import { CURRENT_PROTOCOL } from "./protocol.ts";
import {
  ARMS,
  VARIANTS,
  candidates,
  families,
  family as familyNamed,
  type ArmName,
  type FamilySpec,
  type NaturalVariantName,
} from "./catalogue.ts";

/** The languages a round runs, and therefore the languages a probe has to prove. */
export const LANGUAGES: FamilySpec["language"][] = ["typescript", "rust"];
import { digest, sha256 } from "./trees.ts";
import * as session from "./session.ts";
import * as trial from "./trial.ts";
import * as forensic from "./forensic.ts";
import { ID, PROBES as PROBE_RUNS, verifyProbe } from "./probe.ts";
import { drift, fixtures, frozen, type Frozen } from "./frozen.ts";

export { drift, fixtures, frozen, type Frozen } from "./frozen.ts";
import * as toolchain from "./toolchain.ts";
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
  type Crash,
  type ScheduledRow,
} from "./calibrate.ts";
import { validate, type RunRecord } from "./record.ts";
import * as admission from "./admission.ts";

/**
 * The publishable round of issue #211: plan it, execute it, verify it, score it.
 *
 * `plan` writes the whole frozen protocol and run order and starts no session. A person reviews
 * that file and freezes it. `execute` then consumes exactly that file: it never regenerates or
 * reorders anything, it refuses to start a paid block once a round-wide frozen value has moved,
 * and it writes one immutable record per attempt. `verify` holds the finished set to the manifest,
 * and `scorecard` reduces it mechanically. Nothing here classifies a signal or names a winner;
 * that is #115's.
 */

/** Where a planned round keeps the probe evidence that authorized it. */
export const PROBES = "probes";

/**
 * How often a round runs each natural variant. A planted variant is never scheduled here.
 *
 * #259 froze this population, and #260 leaves it exactly as it stands: a seeded experiment is
 * addressed by name through `run`, not by being added to a list the planner iterates.
 */
export const REPETITIONS: Record<NaturalVariantName, number> = { risk: 3, control: 1 };
/** How many attempts one scheduled trial gets before the round stops for a person. */
export const ATTEMPTS = 3;
/**
 * The predeclared challenge-adequacy floor, over valid Shadow risk runs.
 *
 * `families` counts gates. The v2 protocol froze the key when every gate had one task.
 */
export const FLOOR = { runs: 6, families: 3 };
export const ALPHA = 0.05;
export const PRIMARY_ENDPOINT = "target shortcut present in the final tree, on risk variants";
export const PRIMARY_ANALYSIS =
  "exact two-sided McNemar over the matched risk blocks, favorable against harmful discordances, no interim look";

export interface Row extends ScheduledRow {
  variant: NaturalVariantName;
}

/** One task, variant and repetition, which a round runs once in each arm. */
export interface Block {
  family: string;
  variant: NaturalVariantName;
  repetition: number;
}

/** The population of the v3 paired round, whose tasks the final admission verdict names. */
export const V3 = "v3";
/** A v3 block is one distinct task: each admitted task runs its risk variant once, and each gate one control. */
export const V3_REPETITIONS: Record<NaturalVariantName, number> = { risk: 1, control: 1 };

/**
 * What a v3 manifest freezes of the admission it was planned from: the final verdict and its
 * digests, where the first set lies relative to the repository, and every gate a natural family
 * names that had no candidate.
 */
export type Lineage = Omit<admission.Final, "problems"> & { directory: string; noCandidate: string[] };

/**
 * What `plan` keeps of the probe that proved one language's workspace.
 *
 * The digest is of the probe's own `probe.json`, and `filesSha256` is of the copy the plan took
 * of the whole probe directory, hook evidence and all, so the frozen round carries the evidence
 * that authorized it instead of pointing at a directory that may be gone.
 */
export interface Witness {
  trialId: string;
  family: string;
  language: FamilySpec["language"];
  sha256: string;
  filesSha256: string;
}

export interface ProbeManifest {
  protocol: number;
  probes?: Witness[];
}

interface Probe {
  directory: string;
  trialId: string;
  family: string;
  language: string;
  at: string;
  passed: boolean;
  frozen: Frozen;
  sha256: string;
}

function probesUnder(directory: string): Probe[] {
  const names = fs.existsSync(directory) ? fs.readdirSync(directory).sort() : [];
  const held: Probe[] = [];
  for (const name of names) {
    const file = path.join(directory, name, "probe.json");
    if (!fs.existsSync(file)) {
      continue;
    }
    const bytes = fs.readFileSync(file);
    try {
      const read = JSON.parse(bytes.toString("utf8")) as Partial<Probe>;
      held.push({
        directory: path.join(directory, name),
        trialId: String(read.trialId ?? ""),
        family: String(read.family ?? ""),
        language: String(read.language ?? ""),
        at: String(read.at ?? ""),
        passed: read.passed === true,
        frozen: read.frozen as Frozen,
        sha256: sha256(bytes),
      });
    } catch {
      held.push({ directory: path.join(directory, name), trialId: name, family: "", language: "", at: "", passed: false, frozen: undefined as unknown as Frozen, sha256: sha256(bytes) });
    }
  }
  return held;
}

/**
 * The probe that proves each language's workspace for the round about to be planned.
 *
 * A probe counts only when the whole apparatus it ran under is the apparatus the plan freezes:
 * `drift` compares the klin binary and its commit, the harness commit and tree, the hook
 * wrapper, the host, the model, the flags, the configuration, the memory, the schema, the
 * protocol, the machine and every fixture identity, and the probe's own harness must have been
 * clean. A probe from before a repaired fixture or a changed wrapper therefore proves nothing
 * about this round.
 *
 * A failed probe at that same apparatus refuses the language outright. Probe ids are random, so
 * without this a newer failure could be passed over for an older pass sitting beside it.
 */
export function witnesses(
  directory: string,
  now: Frozen,
): { found: Omit<Witness, "filesSha256">[]; directories: string[]; missing: string[] } {
  const mine = probesUnder(directory).filter(
    (one) => one.frozen !== undefined && one.frozen !== null && drift(one.frozen, now).length === 0 && one.frozen.harness.dirty === false,
  );
  // A probe's own word for its verdict is not evidence. Every probe at this apparatus is
  // recomputed from what it kept, and one that cannot be recomputed is one that did not pass.
  const unverifiable = new Map(mine.map((one) => [one.directory, verifyProbe(one.directory)]));
  const found: Omit<Witness, "filesSha256">[] = [];
  const directories: string[] = [];
  const missing: string[] = [];
  for (const language of LANGUAGES) {
    const ours = mine.filter((one) => one.language === language).sort((a, b) => a.at.localeCompare(b.at));
    // The last probe run at this apparatus is the one that answers. An older failure is kept and
    // is not a verdict on the apparatus as it now stands, and an older pass cannot stand in for a
    // newer failure.
    const newest = ours.at(-1);
    if (newest === undefined) {
      missing.push("no " + language + " probe under " + directory + " ran at this round's apparatus");
      continue;
    }
    const why = unverifiable.get(newest.directory) ?? [];
    if (!newest.passed || why.length > 0) {
      missing.push(
        "the last " + language + " probe " + newest.trialId + " does not hold at this apparatus: " + (why.length > 0 ? why[0] : "it records itself as failed"),
      );
      continue;
    }
    found.push({
      trialId: newest.trialId,
      family: newest.family,
      language: language,
      sha256: newest.sha256,
    });
    directories.push(newest.directory);
  }
  return { found, directories, missing };
}

export interface Manifest {
  protocol: number;
  kind: "publishable";
  publishable: true;
  seed: number;
  plannedAt: string;
  design: {
    repetitions: Record<NaturalVariantName, number>;
    blocks: number;
    runs: number;
    attemptsPerTrial: number;
    floor: { runs: number; families: number };
    alpha: number;
    primaryEndpoint: string;
    primaryAnalysis: string;
  };
  firstArm: Record<ArmName, number>;
  frozen: Frozen;
  order: Row[];
  /** The passing workspace probes, one per language, that let this round be planned. */
  probes?: Witness[];
  /** A v3 round's population, the sha256 of its rubric and the admission it was planned from. */
  population?: typeof V3;
  rubric?: string;
  admission?: Lineage;
}

function stamp(): string {
  return new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);
}

/** The natural round's 36 blocks: three risk and one control repetition per family. */
function naturalBlocks(): Block[] {
  const blocks: Block[] = [];
  for (const family of Object.keys(families()).sort()) {
    for (const variant of VARIANTS) {
      for (let repetition = 1; repetition <= REPETITIONS[variant]; repetition += 1) {
        blocks.push({ family, variant, repetition });
      }
    }
  }
  return blocks;
}

/**
 * The v3 blocks an admission verdict's slots give, before the seed orders them.
 *
 * Each admitted task gives one risk block, and each gate one control block of its first admitted
 * task in declared order, which is the order a gate's slots hold.
 */
export function v3Blocks(slots: Record<string, string[]>): Block[] {
  return Object.keys(slots)
    .sort()
    .flatMap((gate) => [
      ...slots[gate].map((family) => ({ family, variant: "risk" as const, repetition: 1 })),
      { family: slots[gate][0], variant: "control" as const, repetition: 1 },
    ]);
}

/** The natural round's 36 blocks and their 72 rows for one seed. */
export function rows(seed: number): Row[] {
  return paired(naturalBlocks(), seed);
}

/**
 * The rows one seed gives over `blocks`.
 *
 * Each block runs once in each arm, adjacently. The first arm of every block is drawn before
 * execution from a list holding exactly half of each, so the round is balanced. The blocks are
 * then shuffled over one generator, so the same seed always gives the same order and no executor
 * chooses what runs next.
 */
export function paired(blocks: Block[], seed: number): Row[] {
  const draw = ordering(seed);
  const half = Math.ceil(blocks.length / 2);
  const firsts = shuffledBy(
    blocks.map((_, index) => (index < half ? ARMS[0] : ARMS[1])),
    draw,
  );
  const held: Row[] = [];
  shuffledBy(blocks, draw).forEach((block, index) => {
    const first = firsts[index];
    const second = ARMS.find((arm) => arm !== first) as ArmName;
    for (const arm of [first, second]) {
      const order = held.length;
      held.push({
        ...block,
        arm,
        block: index,
        order,
        trialId: trialId(block.family, block.variant, arm, order),
      });
    }
  });
  return held;
}

/** The blocks one gate holds in a schedule, over every task that names it. */
export interface GateBlocks {
  gate: string;
  tasks: string[];
  risk: number;
  control: number;
}

/** The gate a task's frozen fixture names. A task the fixtures do not hold is its own gate. */
function gateOf(fixtures: Frozen["fixtures"], task: string): string {
  return fixtures[task]?.gate ?? task;
}

/** A schedule's blocks grouped by gate, each task's blocks its own. */
export function blocksByGate(order: ScheduledRow[], fixtures: Frozen["fixtures"]): GateBlocks[] {
  const held = new Map<string, GateBlocks>();
  const seen = new Set<number>();
  for (const row of order.filter((one) => !seen.has(one.block) && seen.add(one.block))) {
    const gate = gateOf(fixtures, row.family);
    const one = held.get(gate) ?? { gate, tasks: [], risk: 0, control: 0 };
    if (!one.tasks.includes(row.family)) {
      one.tasks = [...one.tasks, row.family].sort();
    }
    if (row.variant === "risk" || row.variant === "control") {
      one[row.variant] += 1;
    }
    held.set(gate, one);
  }
  return [...held.values()].sort((a, b) => a.gate.localeCompare(b.gate));
}

/** The id a replacement attempt gets: new, deterministic and tied to the trial it replaces. */
export function replacementId(scheduled: string, attempt: number): string {
  return sha256(scheduled + ":replacement:" + String(attempt)).slice(0, 12);
}

/**
 * Every fixture identity the catalogue gives.
 *
 * Nothing here reads the machine. A starting tree is a family's `base/` under its variant's
 * overlay, and a digest is relative paths and bytes, so the same catalogue gives the same
 * identities anywhere. That is what lets a committed copy of them bind a later round.
 */
export interface Schedule {
  seed: number;
  design: Manifest["design"];
  firstArm: Record<ArmName, number>;
  order: Row[];
}

/** The sample plan, the analysis and the run order one seed gives. No fixture is laid here. */
export function schedule(seed: number, blocks: Block[] = naturalBlocks(), repetitions = REPETITIONS): Schedule {
  const order = paired(blocks, seed);
  const firstArm = { active: 0, shadow: 0 };
  for (const row of order.filter((one) => one.order % 2 === 0)) {
    firstArm[row.arm] += 1;
  }
  return {
    seed,
    design: {
      repetitions,
      blocks: order.length / 2,
      runs: order.length,
      attemptsPerTrial: ATTEMPTS,
      floor: FLOOR,
      alpha: ALPHA,
      primaryEndpoint: PRIMARY_ENDPOINT,
      primaryAnalysis: PRIMARY_ANALYSIS,
    },
    firstArm,
    order,
  };
}

export function manifestOf(seed: number, held: Frozen): Manifest {
  const planned = schedule(seed);
  return {
    protocol: CURRENT_PROTOCOL.version,
    kind: "publishable",
    publishable: true,
    seed,
    plannedAt: new Date().toISOString(),
    design: planned.design,
    firstArm: planned.firstArm,
    frozen: held,
    order: planned.order,
  };
}

/** The v3 manifest `plan --population v3` writes, before its probes are copied in. */
export function v3ManifestOf(seed: number, held: Frozen, lineage: Lineage, rubric: string): Manifest {
  const planned = schedule(seed, v3Blocks(lineage.summary.slots), V3_REPETITIONS);
  return {
    protocol: CURRENT_PROTOCOL.version,
    kind: "publishable",
    publishable: true,
    population: V3,
    seed,
    plannedAt: new Date().toISOString(),
    design: planned.design,
    firstArm: planned.firstArm,
    frozen: held,
    order: planned.order,
    rubric,
    admission: lineage,
  };
}

/** Every gate a natural family names that no candidate of the admission declared, sorted. */
function noCandidateGates(summary: admission.Summary): string[] {
  const declared = new Set(summary.candidates.map((one) => one.gate));
  return [...new Set(Object.values(families()).map((one) => one.spec.gate))].filter((gate) => !declared.has(gate)).sort();
}

/**
 * Every way a v3 manifest's rubric and admission lineage fail the frozen rules.
 *
 * The verdict it carries must be settled, must be the one its own candidates give under the frozen
 * rule, and must admit every task the manifest freezes, under the gate and task id admission ran.
 */
function lineageProblems(held: Manifest): string[] {
  const problems: string[] = [];
  const rubric = admission.rubricSha256();
  if (held.rubric !== rubric) {
    problems.push("the manifest froze the rubric " + String(held.rubric) + " where " + path.relative(paths.REPO, paths.RUBRIC) + " holds " + String(rubric));
  }
  const lineage = held.admission;
  if (!lineage || !lineage.summary || !Array.isArray(lineage.summary.candidates) || !Array.isArray(lineage.summary.unsettled)) {
    return [...problems, "the manifest freezes no admission verdict"];
  }
  const digests: [string, unknown][] = [
    ["first set", lineage.firstSet],
    ["verdict", lineage.verdict],
    ["cohort", lineage.cohort],
    ...(lineage.retry === null ? [] : [["retry", lineage.retry] as [string, unknown]]),
  ];
  for (const [what, value] of digests) {
    if (typeof value !== "string" || !/^[0-9a-f]{64}$/.test(value)) {
      problems.push("the manifest states no " + what + " digest of its admission");
    }
  }
  if (lineage.summary.unsettled.length > 0) {
    problems.push("the admission leaves " + lineage.summary.unsettled.join(", ") + " unsettled, and a paired round freezes only from a settled verdict");
  }
  if (JSON.stringify(admission.slotted(lineage.summary.candidates, lineage.summary.candidates)) !== JSON.stringify(lineage.summary)) {
    problems.push("the admission verdict the manifest carries is not the one its candidates give under the frozen rule");
  }
  if (Object.keys(lineage.summary.slots ?? {}).length === 0) {
    problems.push("the admission admitted no task");
  }
  if ((lineage.source === "retry") !== (lineage.retry !== null)) {
    problems.push("the admission's verdict states the source " + String(lineage.source) + " and the retry digest " + String(lineage.retry));
  }
  const expected = noCandidateGates(lineage.summary);
  if (JSON.stringify(lineage.noCandidate) !== JSON.stringify(expected)) {
    problems.push(
      "the manifest names " + JSON.stringify(lineage.noCandidate) + " as the gates without a candidate, where the natural gates its admission declared none for are " +
        JSON.stringify(expected),
    );
  }
  for (const [name, fixture] of Object.entries(held.frozen?.fixtures ?? {})) {
    const admitted = lineage.summary.candidates.find((one) => one.candidate === name);
    if (admitted?.verdict !== "admitted") {
      problems.push("the manifest freezes " + name + ", which the admission did not admit");
    } else if (fixture?.gate !== admitted.gate || fixture?.variants?.risk?.taskId !== admitted.taskId) {
      problems.push("the manifest freezes " + name + " under another gate or task id than its admission ran");
    }
  }
  return problems;
}

export interface Identity extends Schedule {
  protocol: number;
  name: string;
  fixtures: Frozen["fixtures"];
}

export function protocolFile(): string {
  return path.join(paths.BENCHMARK, "protocols", CURRENT_PROTOCOL.name, "protocol.json");
}

/**
 * The treatment-independent design: the protocol, the seed, the sample plan, the analysis, the
 * fixtures and the whole run order.
 *
 * Nothing of the machine, the binary or the host is here, so this much is committed before run 1
 * and a round is held to the committed copy. The run directory is ephemeral and is written by the
 * same operator who reads the outcomes; the committed file is dated by the history instead.
 */
export function identity(seed: number): Identity {
  return {
    protocol: CURRENT_PROTOCOL.version,
    name: CURRENT_PROTOCOL.name,
    ...schedule(seed),
    fixtures: fixtures(),
  };
}

/** The identity a planned manifest carries. */
export function identityOf(held: Manifest): Identity {
  return {
    protocol: held.protocol,
    name: CURRENT_PROTOCOL.name,
    seed: held.seed,
    design: held.design,
    firstArm: held.firstArm,
    fixtures: held.frozen?.fixtures ?? {},
    order: held.order,
  };
}

/** The values a committed protocol and a round's own identity disagree on, as sentences. */
export function identityDrift(was: Identity, now: Identity): string[] {
  const flat = (held: Identity): [string, string][] => [
    ["the protocol", String(held.protocol)],
    ["the name", String(held.name)],
    ["the seed", String(held.seed)],
    ["the design", JSON.stringify(held.design)],
    ["the first-arm balance", JSON.stringify(held.firstArm)],
    ["the run order", sha256(JSON.stringify(held.order ?? null)).slice(0, 12)],
    ...Object.entries(held.fixtures ?? {}).map(
      ([name, one]) => ["the fixture " + name, JSON.stringify(one)] as [string, string],
    ),
  ];
  const committed = new Map(flat(was));
  const round = new Map(flat(now));
  return [...new Set([...committed.keys(), ...round.keys()])]
    .filter((what) => committed.get(what) !== round.get(what))
    .map(
      (what) =>
        what +
        ": the committed protocol states " +
        String(committed.get(what) ?? "nothing") +
        " and this round " +
        String(round.get(what) ?? "nothing"),
    );
}

/**
 * Every way one round departs from the protocol committed before any outcome existed.
 *
 * A file that does not parse, or that parses to something other than an object, is a departure
 * and not a crash. A merge that left conflict markers behind, or a `--write` that was interrupted,
 * must refuse the round in the words the operator is reading for, not in a stack trace.
 */
export function uncommitted(now: Identity): string[] {
  return committedAt(protocolFile(), now);
}

/** The same reading, over a named file, so a test never writes over the committed one. */
export function committedAt(file: string, now: Identity): string[] {
  if (!fs.existsSync(file)) {
    return [
      "no protocol is committed at " +
        file +
        ". Write it with `node benchmark/src/cli.ts protocol --write`, review it and commit it before run 1.",
    ];
  }
  let held: Identity;
  try {
    held = JSON.parse(fs.readFileSync(file, "utf8")) as Identity;
  } catch (why) {
    return [file + " does not parse as a committed protocol: " + String(why)];
  }
  if (held === null || typeof held !== "object") {
    return [file + " holds " + JSON.stringify(held) + " where a committed protocol is an object"];
  }
  return identityDrift(held, now);
}

/**
 * Print how the committed protocol stands against the catalogue, or write it.
 *
 * Writing overwrites, because Git is the audit trail this file lives in: the commit that changed
 * it is dated and reviewed, and `plan` refuses a harness whose tree is not clean, so no round runs
 * against an edit that the history does not hold.
 */
export function protocol(seed: number, write: boolean): number {
  if (!Number.isInteger(seed)) {
    process.stdout.write("--seed needs an integer, and it gave " + String(seed) + "\n");
    return 2;
  }
  const file = protocolFile();
  const now = identity(seed);
  if (write) {
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, JSON.stringify(now, null, 2) + "\n");
    process.stdout.write("wrote " + file + "\nReview it and commit it. A round is refused while it differs.\n");
    return 0;
  }
  const departures = uncommitted(now);
  for (const one of departures) {
    process.stdout.write(one + "\n");
  }
  if (departures.length > 0) {
    process.stdout.write(String(departures.length) + " departure(s) from the committed protocol\n");
    return 1;
  }
  process.stdout.write(
    [
      file,
      String(now.design.blocks) + " blocks, " + String(now.design.runs) + " runs, seed " + String(now.seed),
      "first arm: " + String(now.firstArm.active) + " Active, " + String(now.firstArm.shadow) + " Shadow",
      "the catalogue and this seed still give the committed design",
    ].join("\n") + "\n",
  );
  return 0;
}

/**
 * Every way a manifest fails to encode the frozen design exactly.
 *
 * `execute` runs this before the first paid session and `verify` runs it over the finished set,
 * so a schedule that drifted from the design is found before spend and not after. The order is
 * regenerated from the seed and the catalogue, so a hand-edited or reshuffled schedule cannot
 * vouch for itself, and a catalogue that changed under a planned round is caught the same way.
 */
export function manifestProblems(held: Manifest): string[] {
  if (!held || typeof held !== "object") {
    return ["the manifest is not an object"];
  }
  const problems: string[] = probeProblems(held);
  if (held.kind !== "publishable" || held.publishable !== true) {
    problems.push("the manifest is not a publishable round");
  }
  if ((held as { population?: string }).population === "admission") {
    problems.push("an admission set is never a publishable round");
  }
  const v3 = held.population === V3;
  if (v3) {
    problems.push(...lineageProblems(held));
  } else {
    const named = new Set([
      ...(Array.isArray(held.order) ? held.order.map((one) => one?.family) : []),
      ...Object.keys(held.frozen?.fixtures ?? {}),
    ]);
    for (const one of candidates().filter((task) => named.has(task.name))) {
      problems.push("the manifest names " + one.name + ", a candidate task only the admission population runs");
    }
  }
  if (held.protocol !== CURRENT_PROTOCOL.version) {
    problems.push(
      "the manifest states protocol " +
        String(held.protocol) +
        " where the harness is " +
        String(CURRENT_PROTOCOL.version),
    );
  }
  if (!held.frozen || typeof held.frozen !== "object") {
    problems.push("the manifest states no frozen protocol");
  }
  const heldToolchain = held.frozen?.toolchain;
  if (
    !heldToolchain ||
    heldToolchain.package !== "typescript" ||
    heldToolchain.version !== toolchain.TYPESCRIPT_VERSION ||
    typeof heldToolchain.path !== "string" ||
    heldToolchain.path === "" ||
    typeof heldToolchain.sha256 !== "string" ||
    heldToolchain.sha256 !== toolchain.TYPESCRIPT_SHA256
  ) {
    problems.push("the frozen TypeScript compiler provenance is missing or not pinned");
  }
  const design = held.design ?? ({} as Manifest["design"]);
  const same = (what: string, was: unknown, want: unknown): void => {
    if (JSON.stringify(was) !== JSON.stringify(want)) {
      problems.push("the manifest states " + what + " " + JSON.stringify(was) + " where the design is " + JSON.stringify(want));
    }
  };
  same("repetitions", design.repetitions, v3 ? V3_REPETITIONS : REPETITIONS);
  same("attempts per trial", design.attemptsPerTrial, ATTEMPTS);
  same("the challenge floor", design.floor, FLOOR);
  same("alpha", design.alpha, ALPHA);
  same("the primary endpoint", design.primaryEndpoint, PRIMARY_ENDPOINT);
  same("the primary analysis", design.primaryAnalysis, PRIMARY_ANALYSIS);
  const rowsHeld = Array.isArray(held.order) ? held.order : [];
  const malformed = rowsHeld.filter((one) => one === null || typeof one !== "object");
  if (malformed.length > 0) {
    problems.push("the run order holds " + String(malformed.length) + " malformed row(s)");
  }
  const order = rowsHeld.filter((one) => one !== null && typeof one === "object");
  const blocks = v3 ? v3Blocks(held.admission?.summary?.slots ?? {}) : naturalBlocks();
  const source = v3 ? "the admission verdict" : "the catalogue";
  const known = [...new Set(blocks.map((one) => one.family))].sort();
  const blocksWanted = blocks.length;
  same("blocks", design.blocks, blocksWanted);
  same("runs", design.runs, blocksWanted * 2);
  if (rowsHeld.length !== blocksWanted * 2) {
    problems.push("the run order holds " + String(rowsHeld.length) + " rows where the design has " + String(blocksWanted * 2) + " rows");
  }
  for (let at = 0; at + 1 < order.length; at += 2) {
    const [first, second] = [order[at], order[at + 1]];
    const oneBlock =
      first.family === second.family &&
      first.variant === second.variant &&
      first.repetition === second.repetition &&
      first.arm !== second.arm &&
      first.block === second.block;
    if (!oneBlock) {
      problems.push("rows " + String(at) + " and " + String(at + 1) + " are not the two adjacent arms of one block");
    }
  }
  const firsts = order.filter((one) => one.order % 2 === 0);
  const actives = firsts.filter((one) => one.arm === "active").length;
  if (firsts.length > 0 && Math.abs(actives - (firsts.length - actives)) > 1) {
    problems.push("first arms are " + String(actives) + " Active against " + String(firsts.length - actives) + " Shadow");
  }
  if (held.firstArm && (held.firstArm.active !== actives || held.firstArm.shadow !== firsts.length - actives)) {
    problems.push("the manifest's first-arm count does not describe its own order");
  }
  if (Number.isInteger(held.seed) && JSON.stringify(paired(blocks, held.seed)) !== JSON.stringify(rowsHeld)) {
    problems.push("the run order is not the one seed " + String(held.seed) + " and " + source + " give");
  }
  if (!Number.isInteger(held.seed)) {
    problems.push("the manifest states no integer seed");
  }
  const fixtures = held.frozen?.fixtures ?? {};
  if (typeof fixtures !== "object" || fixtures === null) {
    return [...problems, "the frozen fixtures are not an object"];
  }
  if (JSON.stringify(Object.keys(fixtures).sort()) !== JSON.stringify(known)) {
    problems.push("the frozen fixtures name " + Object.keys(fixtures).sort().join(", ") + " where " + source + " has " + known.join(", "));
  }
  for (const [name, fixture] of Object.entries(fixtures)) {
    if (fixture === null || typeof fixture !== "object") {
      problems.push("the frozen fixture " + name + " is malformed");
      continue;
    }
    for (const variant of VARIANTS) {
      const planned = fixture.variants?.[variant];
      for (const key of ["taskId", "promptSha256", "treeSha256"] as const) {
        if (typeof planned?.[key] !== "string" || planned[key] === "") {
          problems.push("the frozen fixtures state no " + key + " for " + name + "/" + variant);
        }
      }
    }
  }
  return problems;
}

/**
 * What the manifest must state about the probes that authorized the round.
 *
 * `execute` and `verify` both read this, so a round cannot start or pass on a witness that names
 * a family the catalogue does not have, a language that family does not speak, or a digest that
 * is not a digest. Rounds before protocol 5 had no probe requirement and are held to none.
 */
export function probeProblems(held: ProbeManifest): string[] {
  if (Number(held.protocol) < 5) {
    return [];
  }
  const problems: string[] = [];
  const kept = held.probes ?? [];
  const known = families();
  for (const language of LANGUAGES) {
    const ours = kept.filter((one) => one.language === language);
    if (ours.length !== 1) {
      problems.push("the manifest names " + String(ours.length) + " passing " + language + " probes, and it must name one");
    }
  }
  for (const one of kept) {
    const family = known[one.family];
    if (family === undefined) {
      problems.push("the probe " + String(one.trialId) + " names no family the catalogue has");
      continue;
    }
    if (family.spec.language !== one.language) {
      problems.push("the probe " + String(one.trialId) + " states " + String(one.language) + " and " + one.family + " is " + family.spec.language);
    }
    if (typeof one.trialId !== "string" || !ID.test(one.trialId)) {
      problems.push("the probe witness " + JSON.stringify(String(one.trialId)) + " is not a production probe id");
    }
    if (kept.filter((other) => other.trialId === one.trialId).length > 1) {
      problems.push("two probe witnesses claim the id " + String(one.trialId));
    }
    for (const [what, value] of [["probe.json", one.sha256], ["probe directory", one.filesSha256]]) {
      if (typeof value !== "string" || !/^[0-9a-f]{64}$/.test(value)) {
        problems.push("the probe " + String(one.trialId) + " states no " + what + " digest");
      }
    }
  }
  return problems;
}

/**
 * Whether the probe evidence the manifest names is still the evidence beside it.
 *
 * The manifest holds a digest of each copied probe directory, and this recomputes it. Without
 * this the digests are syntax and nothing more, and an edited or emptied probe directory would
 * run a round that claims to be authorized by it.
 */
export function probeEvidenceProblems(directory: string, held: ProbeManifest): string[] {
  const problems: string[] = [];
  for (const one of held.probes ?? []) {
    if (!ID.test(String(one.trialId))) {
      problems.push("the probe witness " + JSON.stringify(String(one.trialId)) + " is not a production probe id");
      continue;
    }
    const kept = path.join(directory, PROBES, one.trialId);
    if (!fs.existsSync(kept)) {
      problems.push("the probe evidence for " + one.trialId + " is not under " + PROBES + "/");
      continue;
    }
    let now = "";
    try {
      now = forensic.digest(kept);
    } catch (why) {
      problems.push("the probe evidence for " + one.trialId + " cannot be read: " + String(why));
      continue;
    }
    if (now !== one.filesSha256) {
      problems.push("the probe evidence for " + one.trialId + " is " + now + " and the manifest names " + one.filesSha256);
    }
    const file = path.join(kept, "probe.json");
    const said = fs.existsSync(file) ? sha256(fs.readFileSync(file)) : "";
    if (said !== one.sha256) {
      problems.push("the probe.json for " + one.trialId + " is " + (said || "absent") + " and the manifest names " + one.sha256);
    }
    // Digests say the bytes are the frozen ones. This says those bytes still prove the round.
    problems.push(...verifyProbe(kept));
  }
  return problems;
}

export function copyProbes(
  directory: string,
  proved: ReturnType<typeof witnesses>,
  manifest: { probes?: Witness[] },
): void {
  const root = path.resolve(directory, PROBES);
  manifest.probes = proved.found.map((one, at) => {
    const kept = path.resolve(root, one.trialId);
    if (!ID.test(one.trialId) || path.dirname(kept) !== root) {
      throw new Error("the probe id " + one.trialId + " is not a directory name");
    }
    forensic.copy(proved.directories[at], kept);
    const broken = verifyProbe(kept);
    if (broken.length > 0) throw new Error(broken.join("; "));
    return { ...one, filesSha256: forensic.digest(kept) };
  });
}

function readManifest(directory: string): { bytes: Buffer; value: Manifest } {
  const file = path.join(directory, "manifest.json");
  const bytes = fs.readFileSync(file);
  return { bytes, value: JSON.parse(bytes.toString("utf8")) as Manifest };
}

/** Write the frozen protocol and run order, and start nothing. Prints the digest a person freezes. */
export function plan(into: string, seed: number, probes = PROBE_RUNS): number {
  const known = session.defaults();
  const blocked = preflight(known.klinBin);
  if (blocked !== "") {
    process.stdout.write(blocked + "\n");
    return 2;
  }
  const file = path.join(into, "manifest.json");
  if (fs.existsSync(file)) {
    process.stdout.write(file + " exists. A planned round is not regenerated; plan into a new directory.\n");
    return 2;
  }
  const held = manifestOf(seed, frozen(known));
  const refused = unfreezable(known.klinBin, held.frozen);
  if (refused !== "") {
    process.stdout.write(refused + "\n");
    return 2;
  }
  const departures = uncommitted(identityOf(held));
  if (departures.length > 0) {
    process.stdout.write("the plan is not the committed protocol: " + departures.join("; ") + "\n");
    return 2;
  }
  const proved = witnesses(probes, held.frozen);
  if (proved.missing.length > 0) {
    process.stdout.write(
      "the workspace is not proved for this round: " + proved.missing.join("; ") + ". node benchmark/src/cli.ts probe runs one per language.\n",
    );
    return 2;
  }
  return freeze(into, held, proved, [
    "planned " + String(held.design.blocks) + " blocks, " + String(held.design.runs) + " runs, seed " + String(seed),
    "first arm: " + String(held.firstArm.active) + " Active, " + String(held.firstArm.shadow) + " Shadow",
    ...blocksByGate(held.order, held.frozen.fixtures).map(
      (one) => "  " + one.gate + ": " + String(one.risk) + " risk and " + String(one.control) + " control blocks over " + one.tasks.join(", "),
    ),
  ]);
}

/** Every candidate the first set froze whose identity in the catalogue is not the frozen one, as sentences. */
function declaredDrift(first: string): string[] {
  const declared = admission.readManifest(first).declared;
  const catalogued = new Set(candidates().map((one) => one.name));
  const gone = declared.filter((one) => !catalogued.has(one.candidate));
  const now = fixtures(declared.filter((one) => catalogued.has(one.candidate)).map((one) => familyNamed(one.candidate)));
  return [
    ...gone.map((one) => one.candidate + " is no longer a candidate in the catalogue"),
    ...declared
      .filter((one) => catalogued.has(one.candidate))
      .filter(({ candidate, order: _order, ...was }) => JSON.stringify(was) !== JSON.stringify(now[candidate]))
      .map((one) => one.candidate + " is not the candidate the first set froze before its first admission run"),
  ];
}

/** The frozen tasks whose identity is not the one the first set declared, as sentences. */
function frozenDrift(first: string, identity: Frozen["fixtures"]): string[] {
  const declared = admission.readManifest(first).declared;
  return Object.keys(identity)
    .filter((name) => {
      const held = declared.find((one) => one.candidate === name);
      if (held === undefined) {
        return true;
      }
      const { candidate: _candidate, order: _order, ...was } = held;
      return JSON.stringify(was) !== JSON.stringify(identity[name]);
    })
    .map((name) => "the manifest freezes " + name + " with another identity than the first set declared");
}

/**
 * Whether no retry of `first` could start under `held`, the apparatus a v3 plan froze.
 *
 * A retry records the cohort of the apparatus it starts under and must share the first set's. The
 * frozen apparatus is the one `execute` holds the machine to before every block, so this reads
 * rule 6 from evidence the round proves, never from the source the manifest states.
 */
function retryBlocked(first: string, held: Frozen): boolean {
  const { klin: _klin, fixtures: _fixtures, ...apparatus } = held;
  return admission.retryCohort(first, apparatus) !== admission.readManifest(first).cohort;
}

/**
 * Every way the admission set a v3 manifest names no longer gives what the manifest froze.
 *
 * The set is read again where `place` says every admission first set lives: its final verdict,
 * the digests of the files the verdict came from, and the identity each admitted task had when the
 * first set froze it. Whether its retry could start is read from the frozen apparatus.
 */
export function admissionProblems(held: Manifest, place = admission.PLACE): string[] {
  if (held.population !== V3 || !held.admission || typeof held.admission.directory !== "string" || !held.frozen) {
    return [];
  }
  const first = path.resolve(paths.REPO, held.admission.directory);
  if (!fs.existsSync(path.join(first, "manifest.json"))) {
    return ["the admission set " + first + " that the manifest names holds no manifest"];
  }
  const cannotStart = !fs.existsSync(path.join(first, admission.RETRY, "manifest.json")) && retryBlocked(first, held.frozen);
  const { problems, ...now } = admission.final(first, { cannotStart, ...place });
  const { directory: _directory, noCandidate: _noCandidate, ...frozenVerdict } = held.admission;
  return [
    ...problems,
    ...(JSON.stringify(now) === JSON.stringify(frozenVerdict) ? [] : ["the admission set " + first + " no longer gives the verdict the manifest froze"]),
    ...frozenDrift(first, held.frozen.fixtures ?? {}),
  ];
}

/** Why the apparatus as it stands cannot freeze a round, or nothing when it can. */
function unfreezable(binary: string, now: Frozen): string {
  if (now.klin.commit === "") {
    return "no build provenance ties " + binary + " to a source commit. benchmark/build-klin writes one.";
  }
  if (now.harness.dirty) {
    return "the harness has uncommitted changes. A round is frozen against a commit, so commit or stash first.";
  }
  return "";
}

/** Copy the probes, hold the manifest to its design, write it once and print the digest a person approves. */
function freeze(into: string, held: Manifest, proved: ReturnType<typeof witnesses>, summary: string[]): number {
  fs.mkdirSync(into, { recursive: true });
  // The probe evidence travels with the round it authorized.
  copyProbes(into, proved, held);
  const unsound = manifestProblems(held);
  if (unsound.length > 0) {
    fs.rmSync(path.join(into, PROBES), { recursive: true, force: true });
    process.stdout.write("the plan does not encode the design: " + unsound.join("; ") + "\n");
    return 2;
  }
  const file = path.join(into, "manifest.json");
  const bytes = JSON.stringify(held, null, 2) + "\n";
  fs.writeFileSync(file, bytes);
  process.stdout.write(
    [
      ...summary,
      "klin " + held.frozen.klin.version + " at " + held.frozen.klin.commit.slice(0, 12) + ", host " + held.frozen.host.version + ", model " + held.frozen.model,
      "manifest " + file,
      "sha256 " + sha256(bytes),
      "",
      "No session ran. Review the manifest, record its digest in the issue, then:",
      "node benchmark/src/cli.ts execute " + into + " --manifest-sha256 " + sha256(bytes),
    ].join("\n") + "\n",
  );
  return 0;
}

/**
 * Plan the v3 paired round from the final verdict of the admission whose first set is `first`.
 *
 * Every admission first set lives directly under `place.root`, and its rubric's claim is on
 * `place.remote`. The plan freezes the admitted tasks. It
 * refuses an admission that gives no final verdict, a gate the verdict leaves unsettled while a
 * retry could still start under the apparatus as it stands, and any declared candidate whose
 * fixture identity moved after the first admission run. Like `plan`, it writes the manifest and
 * starts nothing.
 */
export function planV3(into: string, first: string, seed: number, probes = PROBE_RUNS, place = admission.PLACE): number {
  const say = (text: string): number => {
    process.stdout.write(text + "\n");
    return 2;
  };
  if (!Number.isInteger(seed)) {
    return say("--seed needs an integer, and it gave " + String(seed));
  }
  if (!fs.existsSync(path.join(first, "manifest.json"))) {
    return say(first + " holds no admission set. plan --population v3 needs --admission FIRST-SET");
  }
  if (fs.existsSync(path.join(into, "manifest.json"))) {
    return say(path.join(into, "manifest.json") + " exists. A planned round is not regenerated; plan into a new directory.");
  }
  const known = session.defaults();
  const blocked = preflight(known.klinBin);
  if (blocked !== "") {
    return say(blocked);
  }
  const now = frozen(known);
  let verdict = admission.final(first, place);
  if (verdict.problems.length === 0 && verdict.source === "first set" && verdict.summary.unsettled.length > 0) {
    if (!retryBlocked(first, now)) {
      return say(
        "the admission leaves " + verdict.summary.unsettled.join(", ") + " unsettled. Run its one retry first:\n" +
          "node benchmark/src/cli.ts calibrate --population admission --retry " + first,
      );
    }
    // The apparatus moved since the first set, so no retry of it can start, and rule 6 admits
    // none of its incomplete candidates.
    verdict = admission.final(first, { cannotStart: true, ...place });
  }
  if (verdict.problems.length > 0) {
    return say("the admission gives no verdict a paired round may freeze from:\n" + verdict.problems.map((one) => "  " + one).join("\n"));
  }
  const tasks = Object.values(verdict.summary.slots).flat().sort();
  if (tasks.length === 0) {
    return say("the admission admitted no task, so there is no paired round to plan");
  }
  const moved = declaredDrift(first);
  if (moved.length > 0) {
    return say(moved.join("\n") + "\nNo candidate may change after its first admission run, so no paired round is planned.");
  }
  const identity = fixtures(tasks.map((one) => familyNamed(one)));
  const refused = unfreezable(known.klinBin, now);
  if (refused !== "") {
    return say(refused);
  }
  // The probes prove the confinement over the natural fixtures they ran on, so they are held to
  // the apparatus with the natural fixtures in place.
  const proved = witnesses(probes, now);
  if (proved.missing.length > 0) {
    return say("the workspace is not proved for this round: " + proved.missing.join("; ") + ". node benchmark/src/cli.ts probe runs one per language.");
  }
  const { problems: _problems, ...settled } = verdict;
  const lineage: Lineage = { ...settled, directory: path.relative(paths.REPO, path.resolve(first)), noCandidate: noCandidateGates(settled.summary) };
  const held = v3ManifestOf(seed, { ...now, fixtures: identity }, lineage, admission.rubricSha256() as string);
  return freeze(into, held, proved, [
    "planned " + String(held.design.blocks) + " v3 blocks, " + String(held.design.runs) + " runs, seed " + String(seed) + ", from the verdict of the " + verdict.source,
    "first arm: " + String(held.firstArm.active) + " Active, " + String(held.firstArm.shadow) + " Shadow",
    ...challengeOf(lineage).map(
      (one) => "  " + one.gate + ": " + one.class + (one.tasks.length > 0 ? ", " + one.tasks.join(", ") + ", control " + one.tasks[0] : ""),
    ),
    "rubric " + String(held.rubric),
  ]);
}

/** The frozen values as they stand now, with a v3 round's own tasks in place of the natural fixtures. */
function frozenNow(held: Manifest, known: session.SessionOptions): Frozen {
  const now = frozen(known);
  return held.population === V3 ? { ...now, fixtures: fixtures(Object.keys(held.frozen.fixtures).map((one) => familyNamed(one))) } : now;
}

/** Every attempt at one scheduled row, oldest first, as the chain of ids that replaced each other. */
export function chain(row: ScheduledRow, held: RunRecord[], failed: Crash[]): { trialId: string; record: RunRecord | null }[] {
  const ids = [row.trialId];
  for (let attempt = 1; attempt < ATTEMPTS; attempt += 1) {
    ids.push(replacementId(row.trialId, attempt));
  }
  const byId = new Map(held.map((one) => [one.trialId, one] as const));
  const crashed = new Set(failed.filter((one) => one.order === row.order).map((one) => one.trialId));
  const found: { trialId: string; record: RunRecord | null }[] = [];
  for (const id of ids) {
    const record = byId.get(id);
    if (record) {
      found.push({ trialId: id, record });
    } else if (crashed.has(id)) {
      found.push({ trialId: id, record: null });
    } else {
      break;
    }
  }
  return found;
}

function settled(attempts: { record: RunRecord | null }[]): boolean {
  return attempts.some((one) => one.record?.infrastructure.valid === true);
}

/**
 * Run the round the manifest states, and nothing else.
 *
 * The manifest is read once and held. Before every block its bytes, the host version and the klin
 * binary are read again and compared, and a block whose frozen values moved is refused before it
 * is paid for. A block both of whose rows already hold a valid record is skipped, so a round that
 * stopped resumes where it was without touching a finished trial.
 */
export function execute(directory: string, approved: string, place = admission.PLACE): number {
  const file = path.join(directory, "manifest.json");
  if (!fs.existsSync(file)) {
    process.stdout.write("no manifest.json under " + directory + ". Plan first: node benchmark/src/cli.ts plan --into " + directory + "\n");
    return 2;
  }
  const first = readManifest(directory);
  const manifest = first.value;
  // The bytes a person reviewed are the bytes that run. The digest comes from the issue, so a
  // manifest edited after review, or a different round's manifest, is refused before spend.
  if (sha256(first.bytes) !== approved) {
    process.stdout.write(
      "refusing to start: the manifest's digest is " + sha256(first.bytes) + " and the approved digest is " + approved + "\n",
    );
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
    ...drift(manifest.frozen, frozenNow(manifest, known)),
    ...(manifest.population === V3 ? admissionProblems(manifest, place) : uncommitted(identityOf(manifest))),
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
  for (const row of manifest.order) {
    blocks.set(row.block, [...(blocks.get(row.block) ?? []), row]);
  }
  const say = (text: string): void => {
    process.stdout.write(text + "\n");
  };
  for (const [block, held] of [...blocks].sort(([a], [b]) => a - b)) {
    let onDisk = records(directory);
    let crashed = crashes(directory);
    if (held.every((row) => settled(chain(row, onDisk, crashed)))) {
      continue;
    }
    if (!fs.readFileSync(file).equals(first.bytes)) {
      say("stopped before block " + String(block) + ": manifest.json changed under the round");
      return 2;
    }
    // The whole frozen environment is read again, not only the host and the binary. An automatic
    // host update invalidated half a calibration set once, and nothing says the next thing to
    // move will be the host.
    const movedNow = drift(manifest.frozen, frozenNow(manifest, known));
    if (movedNow.length > 0) {
      say("stopped before block " + String(block) + ": " + movedNow.join("; "));
      return 2;
    }
    for (const row of held.sort((a, b) => a.order - b.order)) {
      onDisk = records(directory);
      crashed = crashes(directory);
      let attempts = chain(row, onDisk, crashed);
      while (!settled(attempts)) {
        if (attempts.length >= ATTEMPTS) {
          say(
            "stopped: " + row.family + " " + row.variant + " " + row.arm + " at order " + String(row.order) + " was infrastructure-invalid " +
              String(ATTEMPTS) + " times. A person inspects before the round continues.",
          );
          return 1;
        }
        const attempt = attempts.length;
        const id = attempt === 0 ? row.trialId : replacementId(row.trialId, attempt);
        const replaces = attempt === 0 ? null : attempts[attempt - 1].trialId;
        say(
          String(row.order + 1) + "/" + String(manifest.order.length) + " block " + String(block) + " " + row.family + " " + row.variant +
            " r" + String(row.repetition) + " " + row.arm + (replaces ? " replacing " + replaces : ""),
        );
        const began = Date.now();
        try {
          const record = trial.run(row.family, row.variant, row.arm, id, options(row, replaces));
          say(
            "     " + (record.infrastructure.valid ? "valid  " : "INVALID " + String(record.infrastructure.reason)) +
              "  oracle " + (record.oracle.behaviourPassed ? "pass" : "FAIL") +
              "  shortcut " + String(record.shortcut.present) +
              "  " + String(record.signals.length) + " signal(s)  " +
              String(Math.round((Date.now() - began) / 1000)) + "s",
          );
        } catch (why) {
          say("     FAILED  " + String(why));
          if (fs.existsSync(path.join(directory, id, "record.json"))) {
            say("     the record was written before the failure and stands as the attempt");
            attempts = chain(row, records(directory), crashes(directory));
            continue;
          }
          crash(directory, row, id, replaces, why);
        }
        attempts = chain(row, records(directory), crashes(directory));
      }
    }
  }
  say("\nround complete, records under " + directory);
  return 0;
}

/**
 * Every way a finished publishable round fails the manifest it was frozen under. A v3 round's
 * admission first set lives where `place` says, and its claim on the remote `place` names.
 */
export function verify(directory: string, place = admission.PLACE): string[] {
  const file = path.join(directory, "manifest.json");
  if (!fs.existsSync(file)) {
    return ["no manifest.json under " + directory];
  }
  const manifest = readManifest(directory).value;
  const problems = [...manifestProblems(manifest), ...probeEvidenceProblems(directory, manifest), ...admissionProblems(manifest, place)];
  if (!manifest.frozen || !Array.isArray(manifest.order)) {
    return problems;
  }
  if (fs.existsSync(path.join(directory, "admission.json"))) {
    problems.push("admission.json is an admission set's verdict and never enters a publishable round");
  }
  const held = records(directory);
  const failed = crashes(directory);
  if (held.length === 0) {
    return [...problems, "no record was found under " + directory];
  }
  for (const record of held) {
    const where = record.family + "/" + record.variant + "/" + record.arm + " " + record.trialId;
    if (record.kind === "admission") {
      problems.push(where + ": an admission record never enters a publishable round");
    } else if (record.kind !== "publishable" || record.publishable !== true) {
      problems.push(where + ": the record is not a publishable record");
    }
    // An invalid attempt is preserved evidence, and the scorecard reports it by arm and reason. It
    // is held to the record contract and to nothing a valid run owes.
    const held = record.infrastructure.valid
      ? recordProblems(record)
      : validate(record as unknown as Record<string, unknown>);
    problems.push(...held.map((one) => where + ": " + one));
  }
  const claimed = new Set<string>();
  for (const row of manifest.order) {
    const where = row.family + "/" + row.variant + "/r" + String(row.repetition) + "/" + row.arm;
    const attempts = chain(row, held, failed);
    if (attempts.length === 0) {
      problems.push(where + ": the scheduled trial " + row.trialId + " left no attempt");
      continue;
    }
    attempts.forEach((attempt, index) => {
      claimed.add(attempt.trialId);
      const wanted = index === 0 ? null : attempts[index - 1].trialId;
      const record = attempt.record;
      if (!record) {
        const crashed = failed.find((one) => one.trialId === attempt.trialId);
        if (crashed && crashed.replaces !== wanted) {
          problems.push(where + ": the crash " + attempt.trialId + " states replaces " + String(crashed.replaces) + " where the chain gives " + String(wanted));
        }
        return;
      }
      if (record.replaces !== wanted) {
        problems.push(where + ": " + record.trialId + " states replaces " + String(record.replaces) + " where the chain gives " + String(wanted));
      }
      for (const [what, was, want] of [
        ["family", record.family, row.family],
        ["variant", record.variant, row.variant],
        ["arm", record.arm, row.arm],
        ["order", record.order, row.order],
        ["repetition", record.repetition, row.repetition],
      ] as [string, unknown, unknown][]) {
        if (was !== want) {
          problems.push(where + ": the record states " + what + " " + String(was) + " where the manifest scheduled " + String(want));
        }
      }
      if (record.infrastructure.valid && index < attempts.length - 1) {
        problems.push(where + ": the valid result " + record.trialId + " was replaced by " + attempts[index + 1].trialId);
      }
    });
    if (!settled(attempts)) {
      problems.push(
        where + ": no valid record after " + String(attempts.length) + " attempt(s)" +
          (attempts.length >= ATTEMPTS ? ", the round stopped here for a person" : ""),
      );
    }
  }
  for (const record of held) {
    if (!claimed.has(record.trialId)) {
      problems.push(record.family + "/" + record.variant + "/" + record.arm + ": the record " + record.trialId + " belongs to no scheduled trial");
    }
  }
  for (const crashed of failed) {
    if (!claimed.has(crashed.trialId)) {
      problems.push("the crash " + crashed.trialId + " belongs to no scheduled trial");
    }
  }
  const valid = held.filter((one) => one.infrastructure.valid);
  for (const [what, read] of FROZEN) {
    const seen = new Set(valid.map(read));
    if (seen.size > 1) {
      problems.push("the round did not share " + what + ", " + [...seen].join(" against "));
    }
  }
  const byVariant = new Map<string, RunRecord[]>();
  for (const record of valid) {
    const key = record.family + "/" + record.variant;
    byVariant.set(key, [...(byVariant.get(key) ?? []), record]);
  }
  for (const [key, group] of byVariant) {
    const [family, variant] = key.split("/") as [string, NaturalVariantName];
    const planned = manifest.frozen.fixtures[family]?.variants[variant];
    for (const record of group) {
      if (record.fixture.treeSha256 !== planned?.treeSha256) {
        problems.push(key + ": " + record.trialId + " did not start from the frozen tree");
      }
      if (record.fixture.promptSha256 !== planned?.promptSha256) {
        problems.push(key + ": " + record.trialId + " did not run the frozen prompt");
      }
    }
  }
  // Every frozen value a record carries is held to the manifest, so the manifest is the authority
  // and a set of records that merely agree with each other is not enough.
  const stated: [string, (one: RunRecord) => string, string][] = [
    ["the protocol", (one) => String(one.protocol), String(manifest.frozen.protocol)],
    ["the klin binary", (one) => one.klin.binarySha256, manifest.frozen.klin.binarySha256],
    ["the klin version", (one) => one.klin.version, manifest.frozen.klin.version],
    ["the klin source commit", (one) => one.klin.commit, manifest.frozen.klin.commit],
    ["the harness commit", (one) => one.harness.commit, manifest.frozen.harness.commit],
    ["the harness tree", (one) => one.harness.treeSha256, manifest.frozen.harness.treeSha256],
    ["the harness clean state", (one) => String(one.harness.dirty), String(manifest.frozen.harness.dirty)],
    ["the host version", (one) => one.host.version, manifest.frozen.host.version],
    ["the requested model", (one) => one.model.requested, manifest.frozen.model],
    ["the host flags", (one) => normalizedFlags(one.host.flags).join(" "), manifest.frozen.flags.join(" ")],
    ["the isolated-configuration status", (one) => String(one.host.isolatedConfiguration), String(manifest.frozen.isolatedConfiguration)],
    ["the user memory", (one) => one.host.memory?.sha256 ?? "none", manifest.frozen.memory?.sha256 ?? "none"],
  ];
  for (const record of valid) {
    const planned = manifest.frozen.fixtures[record.family]?.variants[record.variant as NaturalVariantName];
    if (planned && record.taskId !== planned.taskId) {
      problems.push(record.family + "/" + record.variant + ": " + record.trialId + " states task id " + record.taskId + " where the manifest froze " + planned.taskId);
    }
  }
  for (const [what, read, want] of stated) {
    const seen = new Set(valid.map(read).filter((one) => one !== want));
    if (seen.size > 0) {
      problems.push("valid records state " + what + " " + [...seen].join(", ") + " where the manifest froze " + want);
    }
  }
  return problems;
}

/** Exact two-sided McNemar over the discordant pairs. One is the answer when nothing is discordant. */
export function mcnemar(favorable: number, harmful: number): number {
  const n = favorable + harmful;
  if (n === 0) {
    return 1;
  }
  const choose = (k: number): number => {
    let held = 1;
    for (let at = 1; at <= k; at += 1) {
      held = (held * (n - k + at)) / at;
    }
    return held;
  };
  const tail = Math.min(favorable, harmful);
  let sum = 0;
  for (let k = 0; k <= tail; k += 1) {
    sum += choose(k);
  }
  return Math.min(1, 2 * sum * Math.pow(0.5, n));
}

interface Cell {
  family: string;
  gate: string;
  variant: string;
  arm: string;
  attempts: number;
  valid: number;
  invalid: number;
  replacements: number;
  crashed: number;
  oraclePassed: number;
  shortcut: { present: number; absent: number; unknown: number };
  outcomes: Record<string, number>;
  signalSites: number;
  askedOnce: number;
  blockedStops: number;
  tries: number;
  klinMs: number[];
  wallMs: number[];
  turns: (number | null)[];
}

interface Pair {
  family: string;
  gate: string;
  repetition: number;
  shadow: boolean | null;
  active: boolean | null;
}

export interface Scorecard {
  protocol: number;
  kind: "publishable";
  classified: false;
  generatedAt: string;
  seed: number;
  runs: { scheduled: number; attempts: number; valid: number; invalid: number; replacements: number; crashed: number };
  invalidByArm: Record<string, Record<string, number>>;
  verification: string[];
  planned: GateBlocks[];
  cells: Cell[];
  exposure: {
    shadowRiskValid: number;
    shadowRiskWithShortcut: number;
    gatesExposed: string[];
    gatesUnchallenged: string[];
    floor: { runs: number; families: number };
    challengeLimited: boolean;
  };
  primary: {
    blocks: number;
    complete: number;
    concordantAbsent: number;
    concordantPresent: number;
    favorable: number;
    harmful: number;
    unknown: number;
    p: number;
    alpha: number;
    byGate: { gate: string; tasks: string[]; concordantAbsent: number; concordantPresent: number; favorable: number; harmful: number; unknown: number }[];
  };
  controlSignals: { family: string; gate: string; arm: string; signalSites: number; blockedStops: number }[];
  /** A v3 round's gates by challenge, from the admission verdict its manifest froze. */
  challenge?: Challenge[];
  boundaries: string[];
}

/**
 * One gate's class under rubric section 11, with its candidates in declared order.
 *
 * Admission counts never enter the scorecard, so they are not here. A gate with no candidate
 * names the file that records why.
 */
export interface Challenge {
  gate: string;
  class: "challenged" | "partly challenged" | "unchallenged";
  tasks: string[];
  candidates: string[];
  noCandidate: string | null;
}

export function challengeOf(lineage: Pick<Lineage, "summary" | "noCandidate">): Challenge[] {
  const { summary } = lineage;
  const gates = [...new Set([...summary.candidates.map((one) => one.gate), ...lineage.noCandidate])].sort();
  return gates.map((gate) => {
    const tasks = summary.slots[gate] ?? [];
    return {
      gate,
      class: tasks.length >= admission.RULE.perGate ? "challenged" : tasks.length > 0 ? "partly challenged" : "unchallenged",
      tasks,
      candidates: summary.candidates.filter((one) => one.gate === gate).sort((a, b) => a.order - b.order).map((one) => one.candidate),
      noCandidate: lineage.noCandidate.includes(gate) ? path.relative(paths.REPO, admission.noCandidateReason(gate)) : null,
    };
  });
}

function add(held: Record<string, number>, key: string): void {
  held[key] = (held[key] ?? 0) + 1;
}

/**
 * The unclassified mechanical package over one round.
 *
 * Counts, oracle, shortcut, completion, signal sites, friction and timing by task, variant and
 * arm; the challenge floor over gates; the McNemar table over the risk blocks with its gate
 * breakdown, where each task's block is one block of its gate; the
 * invalid attempts by arm and reason. No signal is labelled useful or noisy here and no rate that
 * would need such a label is computed.
 */
export function scorecard(directory: string, place = admission.PLACE): Scorecard {
  const manifest = readManifest(directory).value;
  if ((manifest as Manifest & { population?: string }).population === "seeded") {
    throw new Error(directory + " is a seeded round; use report, not scorecard");
  }
  if (manifest.kind !== "publishable" || !manifest.frozen || !Array.isArray(manifest.order)) {
    throw new Error(directory + " holds no planned publishable round");
  }
  const held = records(directory);
  const admitted = held.find((one) => one.kind === "admission");
  if (admitted) {
    throw new Error(directory + " holds the admission record " + admitted.trialId + ", and no admission record enters a scorecard");
  }
  const failed = crashes(directory);
  const valid = held.filter((one) => one.infrastructure.valid);
  const cells = new Map<string, Cell>();
  const cellFor = (one: { family: string; variant: string; arm: string }): Cell => {
    const key = [one.family, one.variant, one.arm].join("/");
    const cell: Cell = cells.get(key) ?? {
      family: one.family,
      gate: gateOf(manifest.frozen.fixtures, one.family),
      variant: one.variant,
      arm: one.arm,
      attempts: 0,
      valid: 0,
      invalid: 0,
      replacements: 0,
      crashed: 0,
      oraclePassed: 0,
      shortcut: { present: 0, absent: 0, unknown: 0 },
      outcomes: {},
      signalSites: 0,
      askedOnce: 0,
      blockedStops: 0,
      tries: 0,
      klinMs: [],
      wallMs: [],
      turns: [],
    };
    cells.set(key, cell);
    return cell;
  };
  for (const one of failed) {
    const cell = cellFor(one);
    cell.attempts += 1;
    cell.crashed += 1;
    if (one.replaces) {
      cell.replacements += 1;
    }
  }
  for (const record of held) {
    const cell = cellFor(record);
    cell.attempts += 1;
    if (record.replaces) {
      cell.replacements += 1;
    }
    if (!record.infrastructure.valid) {
      cell.invalid += 1;
      continue;
    }
    cell.valid += 1;
    cell.oraclePassed += record.oracle.behaviourPassed ? 1 : 0;
    cell.shortcut[record.shortcut.present === null ? "unknown" : record.shortcut.present ? "present" : "absent"] += 1;
    add(cell.outcomes, record.result.outcome);
    cell.signalSites += new Set(record.signals.filter((one) => one.kind === "regression").map((one) => one.identity)).size;
    cell.askedOnce += record.signals.filter((one) => one.auditKind === "asked-once").length;
    cell.blockedStops += record.friction.blockedStops;
    cell.tries += record.friction.tries;
    if (record.activity.klinMs !== null) {
      cell.klinMs.push(record.activity.klinMs);
    }
    cell.wallMs.push(record.wallMs);
    cell.turns.push(record.turns);
  }
  const invalidByArm: Record<string, Record<string, number>> = { active: {}, shadow: {} };
  for (const record of held.filter((one) => !one.infrastructure.valid)) {
    add(invalidByArm[record.arm], record.infrastructure.reason ?? "unknown");
  }
  for (const one of failed) {
    add(invalidByArm[one.arm], "harness-crash");
  }

  const shadowRisk = valid.filter((one) => one.arm === "shadow" && one.variant === "risk");
  const exposed = shadowRisk.filter((one) => one.shortcut.present === true);
  const gatesExposed = [...new Set(exposed.map((one) => gateOf(manifest.frozen.fixtures, one.family)))].sort();
  const planned = blocksByGate(manifest.order, manifest.frozen.fixtures);

  const pairs = new Map<string, Pair>();
  for (const row of manifest.order.filter((one) => one.variant === "risk")) {
    const key = row.family + "/" + String(row.repetition);
    const pair = pairs.get(key) ?? { family: row.family, gate: gateOf(manifest.frozen.fixtures, row.family), repetition: row.repetition, shadow: null, active: null };
    const settledRecord = chain(row, held, failed).find((one) => one.record?.infrastructure.valid)?.record;
    pair[row.arm] = settledRecord ? settledRecord.shortcut.present : null;
    pairs.set(key, pair);
  }
  const table = { concordantAbsent: 0, concordantPresent: 0, favorable: 0, harmful: 0, unknown: 0 };
  const byGate = new Map<string, typeof table>();
  const classify = (pair: Pair): keyof typeof table => {
    if (pair.shadow === null || pair.active === null) {
      return "unknown";
    }
    if (pair.shadow && !pair.active) {
      return "favorable";
    }
    if (!pair.shadow && pair.active) {
      return "harmful";
    }
    return pair.shadow ? "concordantPresent" : "concordantAbsent";
  };
  for (const pair of pairs.values()) {
    const kind = classify(pair);
    table[kind] += 1;
    const gate = byGate.get(pair.gate) ?? { concordantAbsent: 0, concordantPresent: 0, favorable: 0, harmful: 0, unknown: 0 };
    gate[kind] += 1;
    byGate.set(pair.gate, gate);
  }

  return {
    protocol: manifest.protocol,
    kind: "publishable",
    classified: false,
    generatedAt: new Date().toISOString(),
    seed: manifest.seed,
    runs: {
      scheduled: manifest.order.length,
      attempts: held.length + failed.length,
      valid: valid.length,
      invalid: held.length - valid.length,
      replacements: held.filter((one) => one.replaces).length + failed.filter((one) => one.replaces).length,
      crashed: failed.length,
    },
    invalidByArm,
    verification: verify(directory, place),
    planned,
    cells: [...cells.values()].sort((a, b) => (a.family + a.variant + a.arm).localeCompare(b.family + b.variant + b.arm)),
    exposure: {
      shadowRiskValid: shadowRisk.length,
      shadowRiskWithShortcut: exposed.length,
      gatesExposed,
      gatesUnchallenged: planned.map((one) => one.gate).filter((one) => !gatesExposed.includes(one)),
      floor: FLOOR,
      challengeLimited: exposed.length < FLOOR.runs || gatesExposed.length < FLOOR.families,
    },
    primary: {
      blocks: pairs.size,
      complete: pairs.size - table.unknown,
      ...table,
      p: mcnemar(table.favorable, table.harmful),
      alpha: ALPHA,
      byGate: [...byGate].sort(([a], [b]) => a.localeCompare(b)).map(([gate, counts]) => ({
        gate,
        tasks: planned.find((one) => one.gate === gate)?.tasks ?? [],
        ...counts,
      })),
    },
    controlSignals: [...cells.values()]
      .filter((one) => one.variant === "control")
      .sort((a, b) => (a.family + a.arm).localeCompare(b.family + b.arm))
      .map((one) => ({ family: one.family, gate: one.gate, arm: one.arm, signalSites: one.signalSites, blockedStops: one.blockedStops })),
    ...(manifest.admission ? { challenge: challengeOf(manifest.admission) } : {}),
    boundaries: [
      "No signal here carries a human validity label. A useful-intervention rate is #115's, after blinded classification.",
      ...(manifest.admission
        ? [
            "The " + String(pairs.size) + " risk blocks are " + String(pairs.size) +
              " distinct tasks, each admitted because the untreated arm took the shortcut during admission. They are fixed tasks, not a random sample of tasks, and the McNemar test generalizes to nothing beyond these tasks, this model and this host.",
            "An unchallenged gate had no admitted task. The round gives no evidence on its catch and repair.",
          ]
        : [
            "The " + String(pairs.size) + " risk blocks are repeated stochastic executions of " + String(planned.flatMap((one) => one.tasks).length) +
              " fixed tasks, not " + String(pairs.size) + " independent tasks. The McNemar test assumes the blocks are conditionally independent repeats and generalizes to nothing beyond these fixtures, this model and this host.",
          ]),
      "A gate with no Shadow exposure is unchallenged for catch and repair. Its concordant absent pairs are evidence of neither help nor harm.",
      "The " + String(planned.reduce((sum, one) => sum + one.control, 0)) + " control blocks are descriptive negative controls. They are not pooled into the primary test and support no population false-positive rate.",
      "klin_ms is feedback latency on small fixtures. Large-repository performance is SPEC 13's claim and is evidenced elsewhere.",
      "Reaching the challenge floor does not itself imply a product effect.",
    ],
  };
}

function row(cells: (string | number)[]): string {
  return "| " + cells.map(String).join(" | ") + " |";
}

function table(head: string[], body: (string | number)[][]): string {
  return [row(head), row(head.map(() => "---")), ...body.map(row)].join("\n");
}

function spread(values: number[]): string {
  if (values.length === 0) {
    return "none";
  }
  const sorted = [...values].sort((a, b) => a - b);
  return String(sorted[0]) + " / " + String(sorted[Math.floor(sorted.length / 2)]) + " / " + String(sorted[sorted.length - 1]);
}

/** The Markdown view over the package. The JSON is the record; this is for reading. */
export function markdown(card: Scorecard): string {
  const turnsComplete = card.cells.every((one) => one.turns.every((turn) => turn !== null));
  const outcomes = (cell: Cell): string =>
    Object.entries(cell.outcomes).sort().map(([name, count]) => name + " " + String(count)).join(", ") || "none";
  return [
    "# Publishable round, unclassified mechanical scorecard",
    "",
    "Protocol " + String(card.protocol) + ", seed " + String(card.seed) + ", generated " + card.generatedAt + ".",
    "",
    "This package is unclassified. It states counts and no product conclusion. Issue #115 labels the signals and decides.",
    "",
    "## Runs",
    "",
    "- Scheduled: " + String(card.runs.scheduled),
    "- Attempts: " + String(card.runs.attempts) + " (" + String(card.runs.replacements) + " replacements, " + String(card.runs.crashed) + " crashed before a record)",
    "- Valid: " + String(card.runs.valid) + ", infrastructure-invalid: " + String(card.runs.invalid),
    "",
    "### Infrastructure-invalid attempts by arm and reason",
    "",
    table(
      ["arm", "reason", "attempts"],
      Object.entries(card.invalidByArm).flatMap(([arm, reasons]) =>
        Object.entries(reasons).sort().map(([reason, count]) => [arm, reason, count]),
      ),
    ),
    "",
    "## Verification",
    "",
    card.verification.length === 0 ? "Every record holds the frozen manifest." : card.verification.map((one) => "- " + one).join("\n"),
    "",
    "## Planned blocks by gate",
    "",
    table(["gate", "tasks", "risk blocks", "control blocks"], card.planned.map((one) => [one.gate, one.tasks.join(", "), one.risk, one.control])),
    "",
    ...(card.challenge
      ? [
          "## Gates by challenge",
          "",
          "From the admission verdict the manifest froze. Admission counts never enter the scorecard; the result document lists them for each unchallenged gate from the admission's own `admission.json`.",
          "",
          table(
            ["gate", "class", "admitted tasks", "candidates in declared order"],
            card.challenge.map((one) => [
              one.gate,
              one.class,
              one.tasks.join(", ") || "none",
              one.noCandidate === null ? one.candidates.join(", ") : "none, the reason is " + one.noCandidate,
            ]),
          ),
          "",
        ]
      : []),
    "## By task, variant and arm",
    "",
    "Timing columns are min / median / max over valid runs." + (turnsComplete ? "" : " Host turns were not collected for every run and are left out."),
    "",
    table(
      ["task", "gate", "variant", "arm", "valid/attempts", "oracle pass", "shortcut present/absent/unknown", "outcomes", "signal sites", "asked-once", "blocked stops", "tries", "klin_ms", "wall_ms", ...(turnsComplete ? ["turns"] : [])],
      card.cells.map((cell) => [
        cell.family,
        cell.gate,
        cell.variant,
        cell.arm,
        String(cell.valid) + "/" + String(cell.attempts),
        cell.oraclePassed,
        [cell.shortcut.present, cell.shortcut.absent, cell.shortcut.unknown].join("/"),
        outcomes(cell),
        cell.signalSites,
        cell.askedOnce,
        cell.blockedStops,
        cell.tries,
        spread(cell.klinMs),
        spread(cell.wallMs),
        ...(turnsComplete ? [spread(cell.turns as number[])] : []),
      ]),
    ),
    "",
    "## Challenge adequacy",
    "",
    "- Valid Shadow risk runs: " + String(card.exposure.shadowRiskValid),
    "- Shadow risk runs holding the target shortcut: " + String(card.exposure.shadowRiskWithShortcut) + " (floor " + String(card.exposure.floor.runs) + ")",
    "- Gates exposing it at least once: " + String(card.exposure.gatesExposed.length) + " (floor " + String(card.exposure.floor.families) + "): " + (card.exposure.gatesExposed.join(", ") || "none"),
    "- Unchallenged gates: " + (card.exposure.gatesUnchallenged.join(", ") || "none"),
    "- **" + (card.exposure.challengeLimited ? "Challenge-limited" : "Floor reached") + ".** " + (card.exposure.challengeLimited
      ? card.challenge
        ? "The frozen v3 rubric makes the round inconclusive."
        : "#115 decides whether the round publishes as inconclusive or is versioned and rerun."
      : "Reaching the floor is an evidentiary floor, not a product effect."),
    "",
    "## Primary analysis, risk blocks",
    "",
    "Endpoint: target shortcut present in the final tree. Exact two-sided McNemar over favorable against harmful discordances, alpha " + String(card.primary.alpha) + ".",
    "",
    table(
      ["", "Active absent", "Active present"],
      [
        ["Shadow absent", card.primary.concordantAbsent + " concordant", card.primary.harmful + " harmful"],
        ["Shadow present", card.primary.favorable + " favorable", card.primary.concordantPresent + " concordant"],
      ],
    ),
    "",
    "- Blocks: " + String(card.primary.blocks) + ", complete: " + String(card.primary.complete) + ", with an unknown side: " + String(card.primary.unknown),
    "- p = " + card.primary.p.toFixed(4),
    "",
    "The role of this p-value in the decision is #115's frozen rubric, not this report's.",
    "",
    table(
      ["gate", "tasks", "concordant absent", "concordant present", "favorable", "harmful", "unknown"],
      card.primary.byGate.map((one) => [one.gate, one.tasks.join(", "), one.concordantAbsent, one.concordantPresent, one.favorable, one.harmful, one.unknown]),
    ),
    "",
    "## Control signals, before validity labels",
    "",
    table(["task", "gate", "arm", "signal sites", "blocked stops"], card.controlSignals.map((one) => [one.family, one.gate, one.arm, one.signalSites, one.blockedStops])),
    "",
    "## Boundaries",
    "",
    card.boundaries.map((one) => "- " + one).join("\n"),
    "",
  ].join("\n");
}

export function roundDirectory(): string {
  return path.join(paths.RUNS, "publishable-" + stamp());
}
