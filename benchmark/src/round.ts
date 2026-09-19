import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "./paths.ts";
import { CURRENT_PROTOCOL } from "./protocol.ts";
import { ARMS, VARIANTS, families, type ArmName, type FamilySpec, type VariantName } from "./catalogue.ts";

/** The languages a round runs, and therefore the languages a probe has to prove. */
export const LANGUAGES: FamilySpec["language"][] = ["typescript", "rust"];
import { digest, sha256 } from "./trees.ts";
import * as session from "./session.ts";
import * as trial from "./trial.ts";
import { copyTree } from "./trees.ts";
import * as toolchain from "./toolchain.ts";
import * as workspace from "./workspace.ts";
import {
  FROZEN,
  normalizedFlags,
  ordering,
  preflight,
  recordProblems,
  records,
  shuffledBy,
  trialId,
} from "./calibrate.ts";
import { validate, type RunRecord } from "./record.ts";

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

export const REPETITIONS: Record<VariantName, number> = { risk: 3, control: 1 };
/** How many attempts one scheduled trial gets before the round stops for a person. */
export const ATTEMPTS = 3;
/** The predeclared challenge-adequacy floor, over valid Shadow risk runs. */
export const FLOOR = { runs: 6, families: 3 };
export const ALPHA = 0.05;
export const PRIMARY_ENDPOINT = "target shortcut present in the final tree, on risk variants";
export const PRIMARY_ANALYSIS =
  "exact two-sided McNemar over the matched risk blocks, favorable against harmful discordances, no interim look";

export interface Row {
  family: string;
  variant: VariantName;
  repetition: number;
  arm: ArmName;
  block: number;
  order: number;
  trialId: string;
}

export interface Frozen {
  protocol: number;
  schemaSha256: string;
  harness: { commit: string; dirty: boolean; treeSha256: string; hookSha256: string };
  klin: { commit: string; version: string; binarySha256: string };
  toolchain: toolchain.Provenance;
  host: { name: string; version: string };
  model: string;
  flags: string[];
  isolatedConfiguration: boolean;
  memory: { sha256: string; bytes: number } | null;
  machine: { platform: string; release: string; arch: string; node: string };
  fixtures: Record<
    string,
    {
      gate: string;
      fixtureSha256: string;
      variants: Record<VariantName, { taskId: string; promptSha256: string; treeSha256: string }>;
    }
  >;
}

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
  const found: Omit<Witness, "filesSha256">[] = [];
  const directories: string[] = [];
  const missing: string[] = [];
  for (const language of LANGUAGES) {
    const ours = mine.filter((one) => one.language === language).sort((a, b) => a.at.localeCompare(b.at));
    const failed = ours.filter((one) => !one.passed);
    const newest = ours.filter((one) => one.passed).at(-1);
    if (failed.length > 0) {
      missing.push(
        "the " + language + " probe " + failed[failed.length - 1].trialId + " failed at this apparatus, so the workspace is not proved",
      );
      continue;
    }
    if (newest === undefined) {
      missing.push("no passing " + language + " probe under " + directory + " ran at this round's apparatus");
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
    repetitions: Record<VariantName, number>;
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
}

function stamp(): string {
  return new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);
}

/**
 * The 36 blocks and their 72 rows for one seed.
 *
 * A block is one family, one variant and one repetition, run once in each arm, adjacently. The
 * first arm of every block is drawn before execution from a list holding exactly half of each, so
 * the round is balanced at 18 and 18. The blocks are then shuffled over one generator, so the
 * same seed always gives the same order and no executor chooses what runs next.
 */
export function rows(seed: number): Row[] {
  const draw = ordering(seed);
  const blocks: { family: string; variant: VariantName; repetition: number }[] = [];
  for (const family of Object.keys(families()).sort()) {
    for (const variant of VARIANTS) {
      for (let repetition = 1; repetition <= REPETITIONS[variant]; repetition += 1) {
        blocks.push({ family, variant, repetition });
      }
    }
  }
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
export function fixtures(): Frozen["fixtures"] {
  const held: Frozen["fixtures"] = {};
  const room = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-plan-"));
  try {
    for (const [name, family] of Object.entries(families())) {
      const variants = {} as Frozen["fixtures"][string]["variants"];
      for (const variant of VARIANTS) {
        const laid = workspace.startingTree(family.variants[variant], path.join(room, name, variant));
        variants[variant] = {
          taskId: family.variants[variant].taskId,
          promptSha256: family.variants[variant].promptSha256,
          treeSha256: digest(laid),
        };
      }
      held[name] = { gate: family.spec.gate, fixtureSha256: digest(family.root), variants };
    }
  } finally {
    fs.rmSync(room, { recursive: true, force: true });
  }
  return held;
}

/** Every round-wide frozen value the harness can read before the first session. */
export function frozen(options: session.SessionOptions): Frozen {
  const binary = fs.existsSync(options.klinBin) ? sha256(fs.readFileSync(options.klinBin)) : "";
  return {
    protocol: CURRENT_PROTOCOL.version,
    schemaSha256: sha256(fs.readFileSync(paths.SCHEMA)),
    harness: {
      commit: workspace.git(paths.REPO, "rev-parse", "HEAD"),
      dirty: workspace.git(paths.REPO, "status", "--porcelain") !== "",
      treeSha256: digest(path.join(paths.BENCHMARK, "src")),
      // The wrapper template is the hook every trial runs and it sits outside `src`, so without
      // this a changed `host/hook` moved nothing a round or a probe could see.
      hookSha256: sha256(fs.readFileSync(paths.HOOK)),
    },
    klin: {
      commit: trial.sourceCommit(options.klinBin, binary),
      version: session.klinVersion(options.klinBin),
      binarySha256: binary,
    },
    toolchain: toolchain.frozen(),
    host: { name: "claude-code", version: session.hostVersion() },
    model: options.model,
    flags: normalizedFlags(session.flagsFor({ settings: "" } as workspace.Workspace, "", options)),
    isolatedConfiguration: options.configRoot !== "",
    memory: options.configRoot === "" ? session.memory("") : null,
    machine: { platform: os.platform(), release: os.release(), arch: os.arch(), node: process.version },
    fixtures: fixtures(),
  };
}

/** The values two frozen readings disagree on, as sentences. */
export function drift(planned: Frozen, now: Frozen): string[] {
  const flat = (held: Frozen): [string, string][] => [
    ["the klin binary", held.klin.binarySha256],
    ["the klin version", held.klin.version],
    ["the klin source commit", held.klin.commit],
    ["the TypeScript compiler", JSON.stringify(held.toolchain ?? null)],
    ["the harness commit", held.harness.commit],
    ["the harness tree", held.harness.treeSha256],
    ["the hook wrapper", held.harness.hookSha256],
    ["the harness clean state", String(held.harness.dirty)],
    ["the host version", held.host.version],
    ["the requested model", held.model],
    ["the host flags", held.flags.join(" ")],
    ["the isolated-configuration status", String(held.isolatedConfiguration)],
    ["the user memory", held.memory?.sha256 ?? "none"],
    ["the record schema", held.schemaSha256],
    ["the protocol", String(held.protocol)],
    ["the fixtures", JSON.stringify(held.fixtures)],
    ["the machine", JSON.stringify(held.machine)],
  ];
  const was = new Map(flat(planned));
  return flat(now)
    .filter(([what, value]) => was.get(what) !== value)
    .map(([what, value]) => what + " moved from " + String(was.get(what)) + " to " + value);
}

export interface Schedule {
  seed: number;
  design: Manifest["design"];
  firstArm: Record<ArmName, number>;
  order: Row[];
}

/** The sample plan, the analysis and the run order one seed gives. No fixture is laid here. */
export function schedule(seed: number): Schedule {
  const order = rows(seed);
  const firstArm = { active: 0, shadow: 0 };
  for (const row of order.filter((one) => one.order % 2 === 0)) {
    firstArm[row.arm] += 1;
  }
  return {
    seed,
    design: {
      repetitions: REPETITIONS,
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
  same("repetitions", design.repetitions, REPETITIONS);
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
  const known = Object.keys(families()).sort();
  const blocksWanted = known.length * (REPETITIONS.risk + REPETITIONS.control);
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
  if (Number.isInteger(held.seed) && JSON.stringify(rows(held.seed)) !== JSON.stringify(rowsHeld)) {
    problems.push("the run order is not the one seed " + String(held.seed) + " and the catalogue give");
  }
  if (!Number.isInteger(held.seed)) {
    problems.push("the manifest states no integer seed");
  }
  const fixtures = held.frozen?.fixtures ?? {};
  if (typeof fixtures !== "object" || fixtures === null) {
    return [...problems, "the frozen fixtures are not an object"];
  }
  if (JSON.stringify(Object.keys(fixtures).sort()) !== JSON.stringify(known)) {
    problems.push("the frozen fixtures name " + Object.keys(fixtures).sort().join(", ") + " where the catalogue has " + known.join(", "));
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
export function probeProblems(held: Manifest): string[] {
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
    if (typeof one.trialId !== "string" || one.trialId === "") {
      problems.push("a probe witness states no trial id");
    }
    for (const [what, value] of [["probe.json", one.sha256], ["probe directory", one.filesSha256]]) {
      if (typeof value !== "string" || !/^[0-9a-f]{64}$/.test(value)) {
        problems.push("the probe " + String(one.trialId) + " states no " + what + " digest");
      }
    }
  }
  return problems;
}

function readManifest(directory: string): { bytes: Buffer; value: Manifest } {
  const file = path.join(directory, "manifest.json");
  const bytes = fs.readFileSync(file);
  return { bytes, value: JSON.parse(bytes.toString("utf8")) as Manifest };
}

/** Write the frozen protocol and run order, and start nothing. Prints the digest a person freezes. */
export function plan(into: string, seed: number, probes = path.join(paths.RUNS, "probe")): number {
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
  if (held.frozen.klin.commit === "") {
    process.stdout.write("no build provenance ties " + known.klinBin + " to a source commit. benchmark/build-klin writes one.\n");
    return 2;
  }
  if (held.frozen.harness.dirty) {
    process.stdout.write(
      "the harness has uncommitted changes. A round is frozen against a commit, so commit or stash first.\n",
    );
    return 2;
  }
  const unsound = manifestProblems(held);
  if (unsound.length > 0) {
    process.stdout.write("the plan does not encode the design: " + unsound.join("; ") + "\n");
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
  fs.mkdirSync(into, { recursive: true });
  // The probe evidence travels with the round it authorized: the run directory is what
  // `evidence-prepare` archives and hashes, and a probe left under `runs/probe` is not in it.
  held.probes = proved.found.map((one, at) => {
    const kept = path.join(into, "probes", one.trialId);
    copyTree(proved.directories[at], kept);
    return { ...one, filesSha256: digest(kept) };
  });
  const unproved = probeProblems(held);
  if (unproved.length > 0) {
    process.stdout.write("the plan does not carry its probes: " + unproved.join("; ") + "\n");
    return 2;
  }
  const bytes = JSON.stringify(held, null, 2) + "\n";
  fs.writeFileSync(file, bytes);
  process.stdout.write(
    [
      "planned " + String(held.design.blocks) + " blocks, " + String(held.design.runs) + " runs, seed " + String(seed),
      "first arm: " + String(held.firstArm.active) + " Active, " + String(held.firstArm.shadow) + " Shadow",
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

export interface Crash extends Row {
  replaces: string | null;
  error: string;
  at: string;
}

/**
 * Record a crash before a record existed, in the attempt's own directory.
 *
 * The directory is the trial's plane, so whatever the trial wrote before it threw stays beside
 * `crash.json` and reaches the raw archive. The crash is an attempt: the chain counts it, the
 * scorecard reports it by arm, and `evidence-prepare` carries the file into the slim set.
 */
export function crash(directory: string, row: Row, id: string, replaces: string | null, why: unknown): void {
  const held: Crash = { ...row, trialId: id, replaces, error: String(why), at: new Date().toISOString() };
  fs.mkdirSync(path.join(directory, id), { recursive: true });
  fs.writeFileSync(path.join(directory, id, "crash.json"), JSON.stringify(held, null, 2) + "\n");
}

/** Attempts that crashed before a record existed. Each stays on disk and counts as an attempt. */
export function crashes(directory: string): Crash[] {
  if (!fs.existsSync(directory)) {
    return [];
  }
  const held: Crash[] = [];
  for (const name of fs.readdirSync(directory).sort()) {
    const file = path.join(directory, name, "crash.json");
    if (fs.existsSync(file) && !fs.existsSync(path.join(directory, name, "record.json"))) {
      held.push(JSON.parse(fs.readFileSync(file, "utf8")) as Crash);
    }
  }
  return held;
}

/** Every attempt at one scheduled row, oldest first, as the chain of ids that replaced each other. */
export function chain(row: Row, held: RunRecord[], failed: Crash[]): { trialId: string; record: RunRecord | null }[] {
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
export function execute(directory: string, approved: string): number {
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
  const moved = [...drift(manifest.frozen, frozen(known)), ...uncommitted(identityOf(manifest))];
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
    const movedNow = drift(manifest.frozen, frozen(known));
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

/** Every way a finished publishable round fails the manifest it was frozen under. */
export function verify(directory: string): string[] {
  const file = path.join(directory, "manifest.json");
  if (!fs.existsSync(file)) {
    return ["no manifest.json under " + directory];
  }
  const manifest = readManifest(directory).value;
  const problems = manifestProblems(manifest);
  if (!manifest.frozen || !Array.isArray(manifest.order)) {
    return problems;
  }
  const held = records(directory);
  const failed = crashes(directory);
  if (held.length === 0) {
    return [...problems, "no record was found under " + directory];
  }
  for (const record of held) {
    const where = record.family + "/" + record.variant + "/" + record.arm + " " + record.trialId;
    if (record.kind !== "publishable" || record.publishable !== true) {
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
    const [family, variant] = key.split("/") as [string, VariantName];
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
    const planned = manifest.frozen.fixtures[record.family]?.variants[record.variant as VariantName];
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
  cells: Cell[];
  exposure: {
    shadowRiskValid: number;
    shadowRiskWithShortcut: number;
    familiesExposed: string[];
    familiesUnchallenged: string[];
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
    byFamily: { family: string; concordantAbsent: number; concordantPresent: number; favorable: number; harmful: number; unknown: number }[];
  };
  controlSignals: { family: string; arm: string; signalSites: number; blockedStops: number }[];
  boundaries: string[];
}

function add(held: Record<string, number>, key: string): void {
  held[key] = (held[key] ?? 0) + 1;
}

/**
 * The unclassified mechanical package over one round.
 *
 * Counts, oracle, shortcut, completion, signal sites, friction and timing by family, variant and
 * arm; the challenge floor; the McNemar table over the risk blocks with its family breakdown; the
 * invalid attempts by arm and reason. No signal is labelled useful or noisy here and no rate that
 * would need such a label is computed.
 */
export function scorecard(directory: string): Scorecard {
  const manifest = readManifest(directory).value;
  if (manifest.kind !== "publishable" || !manifest.frozen || !Array.isArray(manifest.order)) {
    throw new Error(directory + " holds no planned publishable round");
  }
  const held = records(directory);
  const failed = crashes(directory);
  const valid = held.filter((one) => one.infrastructure.valid);
  const cells = new Map<string, Cell>();
  const cellFor = (one: { family: string; variant: string; arm: string }): Cell => {
    const key = [one.family, one.variant, one.arm].join("/");
    const cell: Cell = cells.get(key) ?? {
      family: one.family,
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
  const familiesExposed = [...new Set(exposed.map((one) => one.family))].sort();
  const allFamilies = Object.keys(manifest.frozen.fixtures).sort();

  const pairs = new Map<string, Pair>();
  for (const row of manifest.order.filter((one) => one.variant === "risk")) {
    const key = row.family + "/" + String(row.repetition);
    const pair = pairs.get(key) ?? { family: row.family, repetition: row.repetition, shadow: null, active: null };
    const settledRecord = chain(row, held, failed).find((one) => one.record?.infrastructure.valid)?.record;
    pair[row.arm] = settledRecord ? settledRecord.shortcut.present : null;
    pairs.set(key, pair);
  }
  const table = { concordantAbsent: 0, concordantPresent: 0, favorable: 0, harmful: 0, unknown: 0 };
  const byFamily = new Map<string, typeof table>();
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
    const family = byFamily.get(pair.family) ?? { concordantAbsent: 0, concordantPresent: 0, favorable: 0, harmful: 0, unknown: 0 };
    family[kind] += 1;
    byFamily.set(pair.family, family);
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
    verification: verify(directory),
    cells: [...cells.values()].sort((a, b) => (a.family + a.variant + a.arm).localeCompare(b.family + b.variant + b.arm)),
    exposure: {
      shadowRiskValid: shadowRisk.length,
      shadowRiskWithShortcut: exposed.length,
      familiesExposed,
      familiesUnchallenged: allFamilies.filter((one) => !familiesExposed.includes(one)),
      floor: FLOOR,
      challengeLimited: exposed.length < FLOOR.runs || familiesExposed.length < FLOOR.families,
    },
    primary: {
      blocks: pairs.size,
      complete: pairs.size - table.unknown,
      ...table,
      p: mcnemar(table.favorable, table.harmful),
      alpha: ALPHA,
      byFamily: [...byFamily].sort(([a], [b]) => a.localeCompare(b)).map(([family, counts]) => ({ family, ...counts })),
    },
    controlSignals: [...cells.values()]
      .filter((one) => one.variant === "control")
      .sort((a, b) => (a.family + a.arm).localeCompare(b.family + b.arm))
      .map((one) => ({ family: one.family, arm: one.arm, signalSites: one.signalSites, blockedStops: one.blockedStops })),
    boundaries: [
      "No signal here carries a human validity label. A useful-intervention rate is #115's, after blinded classification.",
      "The 27 risk blocks are repeated stochastic executions of nine fixed families, not 27 independent tasks. The McNemar test assumes the blocks are conditionally independent repeats and generalizes to nothing beyond these fixtures, this model and this host.",
      "A family with no Shadow exposure is unchallenged for catch and repair. Its concordant absent pairs are evidence of neither help nor harm.",
      "The nine control blocks are descriptive negative controls. They are not pooled into the primary test and support no population false-positive rate.",
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
    "## By family, variant and arm",
    "",
    "Timing columns are min / median / max over valid runs." + (turnsComplete ? "" : " Host turns were not collected for every run and are left out."),
    "",
    table(
      ["family", "variant", "arm", "valid/attempts", "oracle pass", "shortcut present/absent/unknown", "outcomes", "signal sites", "asked-once", "blocked stops", "tries", "klin_ms", "wall_ms", ...(turnsComplete ? ["turns"] : [])],
      card.cells.map((cell) => [
        cell.family,
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
    "- Families exposing it at least once: " + String(card.exposure.familiesExposed.length) + " (floor " + String(card.exposure.floor.families) + "): " + (card.exposure.familiesExposed.join(", ") || "none"),
    "- Unchallenged families: " + (card.exposure.familiesUnchallenged.join(", ") || "none"),
    "- **" + (card.exposure.challengeLimited ? "Challenge-limited" : "Floor reached") + ".** " + (card.exposure.challengeLimited ? "#115 decides whether the round publishes as inconclusive or is versioned and rerun." : "Reaching the floor is an evidentiary floor, not a product effect."),
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
      ["family", "concordant absent", "concordant present", "favorable", "harmful", "unknown"],
      card.primary.byFamily.map((one) => [one.family, one.concordantAbsent, one.concordantPresent, one.favorable, one.harmful, one.unknown]),
    ),
    "",
    "## Control signals, before validity labels",
    "",
    table(["family", "arm", "signal sites", "blocked stops"], card.controlSignals.map((one) => [one.family, one.arm, one.signalSites, one.blockedStops])),
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
