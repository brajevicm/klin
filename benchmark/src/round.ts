import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "./paths.ts";
import { ARMS, VARIANTS, families, type ArmName, type VariantName } from "./catalogue.ts";
import { digest, sha256 } from "./trees.ts";
import * as session from "./session.ts";
import * as trial from "./trial.ts";
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
  harness: { commit: string; dirty: boolean; treeSha256: string };
  klin: { commit: string; version: string; binarySha256: string };
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

/** Every round-wide frozen value the harness can read before the first session. */
export function frozen(options: session.SessionOptions): Frozen {
  const binary = fs.existsSync(options.klinBin) ? sha256(fs.readFileSync(options.klinBin)) : "";
  const fixtures: Frozen["fixtures"] = {};
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
      fixtures[name] = { gate: family.spec.gate, fixtureSha256: digest(family.root), variants };
    }
  } finally {
    fs.rmSync(room, { recursive: true, force: true });
  }
  return {
    protocol: paths.PROTOCOL,
    schemaSha256: sha256(fs.readFileSync(paths.SCHEMA)),
    harness: {
      commit: workspace.git(paths.REPO, "rev-parse", "HEAD"),
      dirty: workspace.git(paths.REPO, "status", "--porcelain") !== "",
      treeSha256: digest(path.join(paths.BENCHMARK, "src")),
    },
    klin: {
      commit: trial.sourceCommit(options.klinBin, binary),
      version: session.klinVersion(options.klinBin),
      binarySha256: binary,
    },
    host: { name: "claude-code", version: session.hostVersion() },
    model: options.model,
    flags: normalizedFlags(session.flagsFor({ settings: "" } as workspace.Workspace, "", options)),
    isolatedConfiguration: options.configRoot !== "",
    memory: options.configRoot === "" ? session.memory("") : null,
    machine: { platform: os.platform(), release: os.release(), arch: os.arch(), node: process.version },
    fixtures,
  };
}

/** The values two frozen readings disagree on, as sentences. Machine facts are informational. */
export function drift(planned: Frozen, now: Frozen): string[] {
  const flat = (held: Frozen): [string, string][] => [
    ["the klin binary", held.klin.binarySha256],
    ["the klin version", held.klin.version],
    ["the klin source commit", held.klin.commit],
    ["the harness commit", held.harness.commit],
    ["the harness tree", held.harness.treeSha256],
    ["the harness clean state", String(held.harness.dirty)],
    ["the host version", held.host.version],
    ["the requested model", held.model],
    ["the host flags", held.flags.join(" ")],
    ["the isolated-configuration status", String(held.isolatedConfiguration)],
    ["the user memory", held.memory?.sha256 ?? "none"],
    ["the record schema", held.schemaSha256],
    ["the protocol", String(held.protocol)],
    ["the fixtures", JSON.stringify(held.fixtures)],
  ];
  const was = new Map(flat(planned));
  return flat(now)
    .filter(([what, value]) => was.get(what) !== value)
    .map(([what, value]) => what + " moved from " + String(was.get(what)) + " to " + value);
}

export function manifestOf(seed: number, options: session.SessionOptions): Manifest {
  const order = rows(seed);
  const firstArm = { active: 0, shadow: 0 };
  for (const row of order.filter((one) => one.order % 2 === 0)) {
    firstArm[row.arm] += 1;
  }
  return {
    protocol: paths.PROTOCOL,
    kind: "publishable",
    publishable: true,
    seed,
    plannedAt: new Date().toISOString(),
    design: {
      repetitions: REPETITIONS,
      blocks: order.length / 2,
      runs: order.length,
      attemptsPerTrial: ATTEMPTS,
      floor: FLOOR,
      alpha: ALPHA,
      primaryEndpoint: "target shortcut present in the final tree, on risk variants",
      primaryAnalysis:
        "exact two-sided McNemar over the matched risk blocks, favorable against harmful discordances, no interim look",
    },
    firstArm,
    frozen: frozen(options),
    order,
  };
}

function readManifest(directory: string): { bytes: Buffer; value: Manifest } {
  const file = path.join(directory, "manifest.json");
  const bytes = fs.readFileSync(file);
  return { bytes, value: JSON.parse(bytes.toString("utf8")) as Manifest };
}

/** Write the frozen protocol and run order, and start nothing. Prints the digest a person freezes. */
export function plan(into: string, seed: number): number {
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
  const held = manifestOf(seed, known);
  if (held.frozen.klin.commit === "") {
    process.stdout.write("no build provenance ties " + known.klinBin + " to a source commit. benchmark/build-klin writes one.\n");
    return 2;
  }
  fs.mkdirSync(into, { recursive: true });
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
      "No session ran. Review the manifest, record its digest in the issue, then: node benchmark/src/cli.ts execute " + into,
    ].join("\n") + "\n",
  );
  return 0;
}

interface Failed {
  trialId: string;
  order: number;
}

/** Attempts that crashed before a record existed. Each stays on disk and counts as an attempt. */
function failedAttempts(directory: string): Failed[] {
  if (!fs.existsSync(directory)) {
    return [];
  }
  return fs
    .readdirSync(directory)
    .filter((name) => name.endsWith("-failed.json"))
    .map((name) => JSON.parse(fs.readFileSync(path.join(directory, name), "utf8")) as Failed);
}

/** Every attempt at one scheduled row, oldest first, as the chain of ids that replaced each other. */
export function chain(row: Row, held: RunRecord[], failed: Failed[]): { trialId: string; record: RunRecord | null }[] {
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
export function execute(directory: string): number {
  const file = path.join(directory, "manifest.json");
  if (!fs.existsSync(file)) {
    process.stdout.write("no manifest.json under " + directory + ". Plan first: node benchmark/src/cli.ts plan --into " + directory + "\n");
    return 2;
  }
  const first = readManifest(directory);
  const manifest = first.value;
  if (manifest.kind !== "publishable" || manifest.publishable !== true || !manifest.frozen) {
    process.stdout.write(file + " is not a planned publishable round\n");
    return 2;
  }
  const known = session.defaults();
  const blocked = preflight(known.klinBin);
  if (blocked !== "") {
    process.stdout.write(blocked + "\n");
    return 2;
  }
  const moved = drift(manifest.frozen, frozen(known));
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
    let crashed = failedAttempts(directory);
    if (held.every((row) => settled(chain(row, onDisk, crashed)))) {
      continue;
    }
    if (!fs.readFileSync(file).equals(first.bytes)) {
      say("stopped before block " + String(block) + ": manifest.json changed under the round");
      return 2;
    }
    const hostNow = session.hostVersion();
    const binaryNow = fs.existsSync(known.klinBin) ? sha256(fs.readFileSync(known.klinBin)) : "";
    if (hostNow !== manifest.frozen.host.version || binaryNow !== manifest.frozen.klin.binarySha256) {
      say(
        "stopped before block " + String(block) + ": the host is " + hostNow + " where the round froze " + manifest.frozen.host.version +
          (binaryNow === manifest.frozen.klin.binarySha256 ? "" : ", and the klin binary changed"),
      );
      return 2;
    }
    for (const row of held.sort((a, b) => a.order - b.order)) {
      onDisk = records(directory);
      crashed = failedAttempts(directory);
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
            attempts = chain(row, records(directory), failedAttempts(directory));
            continue;
          }
          fs.writeFileSync(
            path.join(directory, id + "-failed.json"),
            JSON.stringify({ ...row, trialId: id, replaces, error: String(why) }, null, 2) + "\n",
          );
        }
        attempts = chain(row, records(directory), failedAttempts(directory));
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
  const problems: string[] = [];
  if (manifest.kind !== "publishable" || manifest.publishable !== true) {
    problems.push("the manifest is not a publishable round");
  }
  if (!manifest.frozen) {
    return [...problems, "the manifest states no frozen protocol"];
  }
  const planned = rows(manifest.seed);
  if (JSON.stringify(planned) !== JSON.stringify(manifest.order)) {
    problems.push("the run order is not the one seed " + String(manifest.seed) + " and the catalogue give");
  }
  const firsts = manifest.order.filter((one) => one.order % 2 === 0);
  const actives = firsts.filter((one) => one.arm === "active").length;
  if (Math.abs(actives - (firsts.length - actives)) > 1) {
    problems.push("first arms are " + String(actives) + " Active against " + String(firsts.length - actives) + " Shadow");
  }
  const held = records(directory);
  const failed = failedAttempts(directory);
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
      const record = attempt.record;
      if (!record) {
        return;
      }
      const wanted = index === 0 ? null : attempts[index - 1].trialId;
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
  const stated: [string, (one: RunRecord) => string, string][] = [
    ["the klin binary", (one) => one.klin.binarySha256, manifest.frozen.klin.binarySha256],
    ["the klin version", (one) => one.klin.version, manifest.frozen.klin.version],
    ["the klin source commit", (one) => one.klin.commit, manifest.frozen.klin.commit],
    ["the harness commit", (one) => one.harness.commit, manifest.frozen.harness.commit],
    ["the host version", (one) => one.host.version, manifest.frozen.host.version],
    ["the requested model", (one) => one.model.requested, manifest.frozen.model],
    ["the host flags", (one) => normalizedFlags(one.host.flags).join(" "), manifest.frozen.flags.join(" ")],
  ];
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
  const failed = failedAttempts(directory);
  const valid = held.filter((one) => one.infrastructure.valid);
  const cells = new Map<string, Cell>();
  for (const record of held) {
    const key = [record.family, record.variant, record.arm].join("/");
    const cell: Cell = cells.get(key) ?? {
      family: record.family,
      variant: record.variant,
      arm: record.arm,
      attempts: 0,
      valid: 0,
      invalid: 0,
      replacements: 0,
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
    const row = manifest.order.find((row) => row.order === one.order);
    if (row) {
      add(invalidByArm[row.arm], "harness-crash");
    }
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
      replacements: held.filter((one) => one.replaces).length,
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
