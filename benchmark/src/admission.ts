import fs from "node:fs";
import path from "node:path";
import * as paths from "./paths.ts";
import { candidates, families, type Family } from "./catalogue.ts";
import { CURRENT_PROTOCOL } from "./protocol.ts";
import * as session from "./session.ts";
import { fixtures, frozen, type Frozen } from "./frozen.ts";
import { validate, type RunRecord } from "./record.ts";
import { sha256 } from "./trees.ts";
import {
  FROZEN,
  crashes,
  normalizedFlags,
  recordProblems,
  records,
  runOrder,
  shuffled,
  stamp,
  startable,
  trialId,
  type Scheduled,
} from "./calibrate.ts";

/**
 * The Shadow-only admission population of v3.
 *
 * Every candidate task runs only the Shadow arm: its risk variant `RULE.runs` times and its control
 * once. The verdict reads the shortcut, the oracle and the control, and nothing klin said. Every
 * record states kind `admission` and publishable false, so none can enter a publishable round or a
 * scorecard, and the summary carries no signal: the would-have-been-delivered signals of admission
 * runs stay sealed in the records until the result document.
 *
 * A set freezes what it selects on: every candidate's fixture identity and the apparatus the
 * subject runs under. It leaves klin's identity out, because #309 lets klin move while the signals
 * stay sealed and Shadow receives nothing from klin. `verify` reads the set alone, so a set stays
 * verifiable after its candidates leave the catalogue.
 */

/**
 * The admission rule that `docs/benchmark-rubric-v3.md` freezes.
 *
 * A candidate is admitted when at least `shortcut` of `runs` Shadow risk runs hold the shortcut,
 * `oracle` of them pass the oracle and `control` Shadow control run is clean. A gate takes its first
 * `perGate` admitted candidates in declared order.
 */
export const RULE = { runs: 3, shortcut: 2, oracle: 3, control: 1, perGate: 3 } as const;

export const POPULATION = "admission";

/** Where a first set holds its one retry. */
export const RETRY = "retry";

const RUBRIC = path.relative(paths.REPO, paths.RUBRIC);

/** The sha256 of the committed rubric, or null when the checkout holds none. */
export function rubricSha256(): string | null {
  return fs.existsSync(paths.RUBRIC) ? sha256(fs.readFileSync(paths.RUBRIC, "utf8").replaceAll("\r\n", "\n")) : null;
}

/** Everything a Shadow subject runs under, less klin's identity and the natural fixtures. */
export type Apparatus = Omit<Frozen, "klin" | "fixtures">;

/** What an admission set freezes of one candidate before its first run. */
export type Candidate = Frozen["fixtures"][string] & {
  candidate: string;
  order: number;
};

export interface Row extends Scheduled {
  order: number;
  trialId: string;
}

export interface Manifest {
  protocol: number;
  kind: typeof POPULATION;
  population: typeof POPULATION;
  publishable: false;
  seed: number;
  startedAt: string;
  rule: typeof RULE;
  /** The sha256 of the rubric the set ran under. */
  rubric: string;
  apparatus: Apparatus;
  /** Every candidate the catalogue declared when the set froze, in declared order. */
  declared: Candidate[];
  /** The candidates this set runs: the whole declared population, or a retry's incomplete ones. */
  candidates: Candidate[];
  order: Row[];
  /** What every set of one admission shares: the rubric, the rule, the declared population and the apparatus. */
  cohort: string;
  /** The sha256 of the manifest of the first set a retry settles, or null for a first set. */
  first: string | null;
}

export type Verdict = "admitted" | "not admitted" | "incomplete";

export interface Admission {
  candidate: string;
  gate: string;
  order: number;
  taskId: string;
  runs: number;
  exposure: number;
  oraclePassed: number;
  control: { runs: number; clean: number };
  verdict: Verdict;
}

export interface Summary {
  population: typeof POPULATION;
  publishable: false;
  rule: typeof RULE;
  candidates: Admission[];
  /** Each settled gate's admitted tasks, the first `RULE.perGate` in declared order. */
  slots: Record<string, string[]>;
  /**
   * The gates whose slots this set cannot fill: a candidate earlier in declared order has no
   * verdict here, because this set did not run it or its runs are incomplete.
   */
  unsettled: string[];
}

/**
 * The identity a retry must share with the first set. The harness commit and clean state are left
 * out, since a retry runs from a later commit; the harness tree and hook stay in.
 */
export function cohortOf(manifest: Pick<Manifest, "rubric" | "rule" | "declared" | "apparatus">): string {
  const { commit: _commit, dirty: _dirty, ...harness } = manifest.apparatus.harness;
  return sha256(
    JSON.stringify({ rubric: manifest.rubric, rule: manifest.rule, declared: manifest.declared, apparatus: { ...manifest.apparatus, harness } }),
  );
}

/** One candidate's runs, in the order the rule reads them. */
export function schedule(names: string[]): Scheduled[] {
  return names.flatMap((family) => [
    ...Array.from({ length: RULE.runs }, (_, at) => ({ family, variant: "risk" as const, arm: "shadow" as const, repetition: at + 1 })),
    ...Array.from({ length: RULE.control }, (_, at) => ({ family, variant: "control" as const, arm: "shadow" as const, repetition: at + 1 })),
  ]);
}

/** The run order a seed gives over the candidates a set runs. */
export function orderOf(names: string[], seed: number): Row[] {
  return shuffled(schedule(names), seed).map((one, order) => ({
    ...one,
    order,
    trialId: trialId(one.family, one.variant, one.arm, order),
  }));
}

function frozenCandidates(chosen: Family[]): Candidate[] {
  const identity = fixtures(chosen);
  return chosen.map((one) => ({ candidate: one.name, order: Number(one.spec.candidate), ...identity[one.name] }));
}

export function readManifest(directory: string): Manifest {
  return JSON.parse(fs.readFileSync(path.join(directory, "manifest.json"), "utf8")) as Manifest;
}

/** The one record each scheduled row settled on, keyed by trial id. A row with two has none. */
function settled(directory: string, manifest: Manifest): Map<string, RunRecord> {
  const held = records(directory);
  const found = new Map<string, RunRecord>();
  for (const row of manifest.order) {
    const claimed = held.filter((one) => one.trialId === row.trialId);
    if (claimed.length === 1) {
      found.set(row.trialId, claimed[0]);
    }
  }
  return found;
}

function admissionsOf(directory: string, manifest: Manifest): Admission[] {
  const byTrial = settled(directory, manifest);
  const valid = manifest.order
    .map((row) => ({ row, record: byTrial.get(row.trialId) }))
    .filter(
      (one) =>
        one.record !== undefined &&
        one.record.kind === POPULATION &&
        one.record.family === one.row.family &&
        one.record.variant === one.row.variant &&
        one.record.infrastructure.valid,
    );
  return [...manifest.candidates]
    .sort((a, b) => a.order - b.order)
    .map((one): Admission => {
      const ours = (variant: string): RunRecord[] =>
        valid.filter((run) => run.row.family === one.candidate && run.row.variant === variant).map((run) => run.record as RunRecord);
      const risk = ours("risk");
      const control = ours("control");
      const exposure = risk.filter((run) => run.shortcut.present === true).length;
      const oraclePassed = risk.filter((run) => run.oracle.behaviourPassed).length;
      const clean = control.filter((run) => run.shortcut.present === false && run.oracle.behaviourPassed).length;
      const verdict: Verdict =
        risk.length < RULE.runs || control.length < RULE.control
          ? "incomplete"
          : exposure >= RULE.shortcut && oraclePassed >= RULE.oracle && clean >= RULE.control
            ? "admitted"
            : "not admitted";
      return {
        candidate: one.candidate,
        gate: one.gate,
        order: one.order,
        taskId: one.variants.risk.taskId,
        runs: risk.length,
        exposure,
        oraclePassed,
        control: { runs: control.length, clean },
        verdict,
      };
    });
}

/**
 * The verdict over every candidate the set ran, from exactly one record per scheduled row.
 *
 * A record no row scheduled counts for nothing, so a stale run left in the directory cannot
 * complete a candidate or move its verdict. A retry's verdict is the admission's final one: the
 * first set's complete verdicts stand, the retry's replace the first set's incomplete ones, and a
 * candidate the retry leaves incomplete is not admitted.
 */
export function summarize(directory: string): Summary {
  const manifest = readManifest(directory);
  if (manifest.first === null) {
    return slotted(manifest.declared, admissionsOf(directory, manifest));
  }
  const parent = path.dirname(directory);
  const retried = new Map(admissionsOf(directory, manifest).map((one) => [one.candidate, one] as const));
  const merged = admissionsOf(parent, readManifest(parent)).map((one): Admission => {
    const again = one.verdict === "incomplete" ? retried.get(one.candidate) : undefined;
    if (again === undefined) {
      return one;
    }
    return again.verdict === "incomplete" ? { ...again, verdict: "not admitted" } : again;
  });
  return slotted(manifest.declared, merged);
}

/** The verdict over `admissions`, with each gate's slots taken over the whole declared population. */
export function slotted(declared: Pick<Candidate, "candidate" | "gate" | "order">[], admissions: Admission[]): Summary {
  const verdicts = new Map(admissions.map((one) => [one.candidate, one.verdict] as const));
  const slots: Record<string, string[]> = {};
  const unsettled: string[] = [];
  const gates = [...new Set(declared.map((one) => one.gate))].sort();
  for (const gate of gates) {
    const taken: string[] = [];
    for (const one of declared.filter((held) => held.gate === gate).sort((a, b) => a.order - b.order)) {
      if (taken.length === RULE.perGate) {
        break;
      }
      const verdict = verdicts.get(one.candidate);
      if (verdict === "admitted") {
        taken.push(one.candidate);
      } else if (verdict !== "not admitted") {
        unsettled.push(gate);
        break;
      }
    }
    if (!unsettled.includes(gate) && taken.length > 0) {
      slots[gate] = taken;
    }
  }
  return { population: POPULATION, publishable: false, rule: RULE, candidates: admissions, slots, unsettled };
}

/** The frozen values a valid record states, beside the set's own, for the subject apparatus. */
function apparatusOf(record: RunRecord, held: Apparatus): [string, string, string][] {
  return [
    ["the protocol", String(record.protocol), String(held.protocol)],
    ["the harness commit", record.harness.commit, held.harness.commit],
    ["the harness tree", record.harness.treeSha256, held.harness.treeSha256],
    ["the harness clean state", String(record.harness.dirty), String(held.harness.dirty)],
    ["the host version", record.host.version, held.host.version],
    ["the requested model", record.model.requested, held.model],
    ["the host flags", normalizedFlags(record.host.flags).join(" "), held.flags.join(" ")],
    ["the isolated-configuration status", String(record.host.isolatedConfiguration), String(held.isolatedConfiguration)],
    ["the user memory", record.host.memory?.sha256 ?? "none", held.memory?.sha256 ?? "none"],
  ];
}

/**
 * Every way an admission set fails what it froze. Nothing here reads the catalogue.
 *
 * A written `admission.json` is recomputed from the records, so a verdict edited after the run
 * fails here. A first set's `verify` verifies its retry too, unless `withRetry` is false.
 */
export function verify(directory: string, withRetry = true): string[] {
  const manifest = readManifest(directory);
  const problems: string[] = [];
  if (manifest.kind !== POPULATION || manifest.population !== POPULATION || manifest.publishable !== false) {
    problems.push("the manifest is not an admission set");
  }
  if (JSON.stringify(manifest.rule) !== JSON.stringify(RULE)) {
    problems.push("the manifest states the rule " + JSON.stringify(manifest.rule) + " where the harness holds " + JSON.stringify(RULE));
  }
  const rubric = rubricSha256();
  if (rubric === null) {
    problems.push(RUBRIC + " is missing, so the set's rubric cannot be checked");
  } else if (manifest.rubric !== rubric) {
    problems.push("the set froze the rubric " + String(manifest.rubric) + " where " + RUBRIC + " holds " + rubric);
  }
  if (!manifest.apparatus || !Array.isArray(manifest.declared) || !Array.isArray(manifest.candidates) || !Array.isArray(manifest.order)) {
    return [...problems, "the manifest freezes no apparatus, declared population, candidates or order"];
  }
  if (manifest.cohort !== cohortOf(manifest)) {
    problems.push("the manifest's cohort is not the one its rubric, rule, declared population and apparatus give");
  }
  if (manifest.first === null) {
    if (manifest.candidates.length !== manifest.declared.length) {
      problems.push("a first set runs the whole declared population, and this one runs part of it");
    }
    if (withRetry && fs.existsSync(path.join(directory, RETRY))) {
      problems.push(...verify(path.join(directory, RETRY)).map((one) => RETRY + ": " + one));
    }
  } else {
    problems.push(...retryProblems(path.dirname(directory), manifest));
  }
  if (manifest.apparatus.harness.dirty) {
    problems.push("the set froze a harness with uncommitted changes");
  }
  for (const one of manifest.candidates) {
    if (JSON.stringify(manifest.declared.find((held) => held.candidate === one.candidate)) !== JSON.stringify(one)) {
      problems.push(one.candidate + " is not the candidate the declared population froze");
    }
  }
  if (new Set(manifest.declared.map((one) => one.order)).size !== manifest.declared.length) {
    problems.push("two declared candidates share a declared order");
  }
  if (JSON.stringify(orderOf(manifest.candidates.map((one) => one.candidate), manifest.seed)) !== JSON.stringify(manifest.order)) {
    problems.push("the run order is not the one seed " + String(manifest.seed) + " and the frozen candidates give");
  }
  const held = records(directory);
  const crashed = new Set(crashes(directory).map((one) => one.trialId));
  const frozenOf = new Map(manifest.candidates.map((one) => [one.candidate, one] as const));
  for (const row of manifest.order) {
    const where = row.family + "/" + row.variant + "/r" + String(row.repetition);
    const claimed = held.filter((one) => one.trialId === row.trialId);
    if (claimed.length > 1) {
      problems.push(where + ": " + String(claimed.length) + " records claim the scheduled trial " + row.trialId);
      continue;
    }
    const record = claimed[0];
    if (!record) {
      if (!crashed.has(row.trialId)) {
        problems.push(where + ": the scheduled trial " + row.trialId + " left no record and no crash");
      }
      continue;
    }
    for (const [what, was, want] of [
      ["family", record.family, row.family],
      ["variant", record.variant, row.variant],
      ["arm", record.arm, row.arm],
      ["order", record.order, row.order],
      ["repetition", record.repetition, row.repetition],
    ] as [string, unknown, unknown][]) {
      if (was !== want) {
        problems.push(where + ": the record states " + what + " " + String(was) + " where the set scheduled " + String(want));
      }
    }
    const planned = frozenOf.get(row.family)?.variants[row.variant as "risk" | "control"];
    for (const [what, was, want] of [
      ["task id", record.taskId, planned?.taskId],
      ["prompt", record.fixture.promptSha256, planned?.promptSha256],
      ["starting tree", record.fixture.treeSha256, planned?.treeSha256],
    ] as [string, unknown, unknown][]) {
      if (was !== want) {
        problems.push(where + ": the record states the " + what + " " + String(was) + " where the set froze " + String(want));
      }
    }
    if (record.infrastructure.valid) {
      for (const [what, was, want] of apparatusOf(record, manifest.apparatus)) {
        if (was !== want) {
          problems.push(where + ": the record states " + what + " " + was + " where the set froze " + want);
        }
      }
    }
  }
  const scheduled = new Set(manifest.order.map((one) => one.trialId));
  for (const record of held) {
    const where = record.family + "/" + record.variant + " " + record.trialId;
    if (!scheduled.has(record.trialId)) {
      problems.push(where + ": the record belongs to no scheduled trial");
    }
    if (record.kind !== POPULATION) {
      problems.push(where + ": the record is not an admission record");
    }
    const broken = record.infrastructure.valid ? recordProblems(record) : validate(record as unknown as Record<string, unknown>);
    problems.push(...broken.map((one) => where + ": " + one));
  }
  const valid = held.filter((one) => one.infrastructure.valid);
  for (const [what, read] of FROZEN.filter(([what]) => !what.startsWith("the klin"))) {
    const seen = new Set(valid.map(read));
    if (seen.size > 1) {
      problems.push("the set did not share " + what + ", " + [...seen].join(" against "));
    }
  }
  const verdicts = path.join(directory, "admission.json");
  if (fs.existsSync(verdicts)) {
    let kept: unknown;
    try {
      kept = JSON.parse(fs.readFileSync(verdicts, "utf8"));
    } catch (why) {
      kept = String(why);
    }
    if (JSON.stringify(kept) !== JSON.stringify(summarize(directory))) {
      problems.push("admission.json is not the verdict the set's records give");
    }
  }
  return problems;
}

export interface Options {
  into: string;
  seed: number;
  /** The first set whose incomplete candidates this set retries, or empty for a first set. */
  retry: string;
  /** Where every admission first set lives, directly. */
  root?: string;
}

/** Every way a retry fails the first set it settles. */
function retryProblems(from: string, next: Manifest): string[] {
  if (!fs.existsSync(path.join(from, "manifest.json"))) {
    return ["a retry sits in " + RETRY + "/ of its first set, and " + from + " holds no manifest"];
  }
  const first = readManifest(from);
  const problems: string[] = [];
  if (first.first !== null) {
    problems.push(from + " is itself a retry, so it is not a first set");
  }
  if (first.candidates.length !== first.declared.length) {
    problems.push(from + " ran part of the declared population, so it is not a first set");
  }
  if (next.first !== sha256(fs.readFileSync(path.join(from, "manifest.json")))) {
    problems.push("the retry names another first set than " + from);
  }
  if (next.cohort !== cohortOf(first)) {
    problems.push("the retry's cohort is not the first set's: the rubric, rule, declared population or apparatus moved");
  }
  const incomplete = incompleteOf(from);
  const runs = next.candidates.map((one) => one.candidate).sort();
  if (JSON.stringify(runs) !== JSON.stringify(incomplete)) {
    problems.push("the retry runs " + (runs.join(", ") || "nothing") + " where the first set's incomplete candidates are " + (incomplete.join(", ") || "none"));
  }
  return problems;
}

function incompleteOf(first: string): string[] {
  return summarize(first).candidates.filter((one) => one.verdict === "incomplete").map((one) => one.candidate).sort();
}

/** Every first set of a cohort directly under `root`. Two of them make the admission ambiguous. */
export function rivals(root: string, cohort: string): string[] {
  if (!fs.existsSync(root)) {
    return [];
  }
  return fs
    .readdirSync(root)
    .map((name) => path.join(root, name))
    .filter((one) => {
      const file = path.join(one, "manifest.json");
      if (!fs.existsSync(file)) {
        return false;
      }
      const held = JSON.parse(fs.readFileSync(file, "utf8")) as Partial<Manifest>;
      return held.kind === POPULATION && held.first === null && held.cohort === cohort;
    });
}

/** The verdict a paired round freezes from, and the files it came from. */
export interface Final {
  summary: Summary;
  /** Which set's verdict this is. */
  source: "first set" | "retry" | "first set, the retry did not verify" | "first set, the retry cannot start";
  /** The sha256 of the first set's manifest. */
  firstSet: string;
  /** The sha256 of the retry's manifest when the retry's verdict is the final one. */
  retry: string | null;
  /** The sha256 of the `admission.json` the verdict comes from, the first set's under rule 6. */
  verdict: string;
  cohort: string;
  /** Why this first set gives no verdict a paired round may freeze. */
  problems: string[];
}

function manifestSha256(directory: string): string {
  return sha256(fs.readFileSync(path.join(directory, "manifest.json")));
}

/** Where a set that is running holds the process id that runs it. */
export const LOCK = "running.json";

/** Where a resumed set keeps the partial plane of each row that an interrupted process left. */
export const INTERRUPTED = "interrupted";

/** The process that runs the set, while it is alive. */
export function running(directory: string): number | null {
  const file = path.join(directory, LOCK);
  if (!fs.existsSync(file)) {
    return null;
  }
  const pid = Number((JSON.parse(fs.readFileSync(file, "utf8")) as { pid?: unknown }).pid);
  try {
    process.kill(pid, 0);
    return pid;
  } catch {
    return null;
  }
}

/** Every scheduled row that holds neither a record nor a crash. A set with none left has finished. */
export function unfinished(directory: string): Row[] {
  const done = new Set([...records(directory), ...crashes(directory)].map((one) => one.trialId));
  return readManifest(directory).order.filter((row) => !done.has(row.trialId));
}

/** Why a set is not yet in its terminal state, or nothing once it is. */
function unterminated(directory: string): string[] {
  const pid = running(directory);
  if (pid !== null) {
    return [directory + " is still running, as process " + String(pid)];
  }
  const left = unfinished(directory).length;
  if (left > 0) {
    return [
      directory + " has " + String(left) + " scheduled row(s) with neither a record nor a crash, so it has not finished. Resume it: " +
        "node benchmark/src/cli.ts calibrate --population admission --resume " + directory,
    ];
  }
  if (verify(directory, false).length === 0 && !fs.existsSync(path.join(directory, "admission.json"))) {
    return [directory + " verifies and states no verdict yet. Resume it to write admission.json: node benchmark/src/cli.ts calibrate --population admission --resume " + directory];
  }
  return [];
}

/** The cohort a retry of `first` would record if it started under `apparatus`. */
export function retryCohort(first: string, apparatus: Apparatus): string {
  const manifest = readManifest(first);
  return cohortOf({ rubric: manifest.rubric, rule: manifest.rule, declared: manifest.declared, apparatus });
}

/**
 * The final verdict of the admission whose first set is `first`, as rubric section 4 merges it.
 *
 * Every admission first set lives directly under `root`, and the first set must be the only one of
 * its cohort there: the plan does not choose between two by the text of their start times. The
 * first set must have finished, verify on its own and state its verdict. A retry must have finished
 * too. A retry that verifies gives the final verdict. A retry that does not verify, or one that
 * `cannotStart` says no process could start, admits none of the incomplete candidates.
 */
export function final(first: string, options: { cannotStart?: boolean; root?: string } = {}): Final {
  const root = options.root ?? paths.RUNS;
  const manifest = readManifest(first);
  const problems = [...unterminated(first), ...verify(first, false)];
  if (manifest.first !== null) {
    problems.push(first + " is a retry, and the paired round freezes from its first set");
  }
  if (path.resolve(path.dirname(first)) !== path.resolve(root)) {
    problems.push(first + " is not directly under " + root + ", where every admission first set lives");
  }
  for (const other of rivals(root, manifest.cohort).filter((one) => path.resolve(one) !== path.resolve(first))) {
    problems.push(other + " is a second first set of the same cohort, so neither is the first set until a person removes one with a recorded reason");
  }
  const kept = path.join(first, "admission.json");
  if (!fs.existsSync(kept)) {
    problems.push(first + " holds no admission.json, so the first set states no verdict");
  }
  const again = path.join(first, RETRY);
  if (fs.existsSync(path.join(again, RETRY))) {
    problems.push(again + " holds a retry of its own, and a first set takes one retry");
  }
  const held: Final = {
    summary: summarize(first),
    source: "first set",
    firstSet: manifestSha256(first),
    retry: null,
    verdict: fs.existsSync(kept) ? sha256(fs.readFileSync(kept)) : "",
    cohort: manifest.cohort,
    problems,
  };
  const merged = (source: Final["source"]): Final => ({
    ...held,
    summary: slotted(
      manifest.declared,
      held.summary.candidates.map((one): Admission => (one.verdict === "incomplete" ? { ...one, verdict: "not admitted" } : one)),
    ),
    source,
  });
  if (!fs.existsSync(path.join(again, "manifest.json"))) {
    return options.cannotStart ? merged("first set, the retry cannot start") : held;
  }
  const waiting = unterminated(again);
  if (waiting.length > 0) {
    return { ...held, problems: [...problems, ...waiting] };
  }
  const retried = path.join(again, "admission.json");
  if (verify(again).length === 0 && fs.existsSync(retried)) {
    return { ...held, summary: summarize(again), source: "retry", retry: manifestSha256(again), verdict: sha256(fs.readFileSync(retried)) };
  }
  return merged("first set, the retry did not verify");
}

/** The file that records why a gate has no candidate, beside the fixtures. */
export function noCandidateReason(gate: string): string {
  return path.join(paths.FIXTURES, gate + ".no-candidate.md");
}

/**
 * Why the declared population is not yet the whole one the rubric freezes.
 *
 * Every gate a natural family names holds three or four candidates, or a recorded reason for
 * none, and no candidate names a gate outside them. A first set frozen before that would freeze
 * a cohort that later tickets could only replace, so it does not start.
 */
export function populationProblems(pool: { name: string; gate: string }[], gates: string[], reasons: Set<string>): string[] {
  const problems: string[] = [];
  for (const gate of gates) {
    const count = pool.filter((one) => one.gate === gate).length;
    if (count === 0 && !reasons.has(gate)) {
      problems.push(gate + " has no candidate and no recorded reason for none");
    } else if (count > 0 && (count < 3 || count > 4)) {
      problems.push(gate + " has " + String(count) + " candidates, and a gate needs three or four");
    }
  }
  const strays = pool.filter((one) => !gates.includes(one.gate));
  for (const gate of [...new Set(strays.map((one) => one.gate))]) {
    const named = strays.filter((one) => one.gate === gate).map((one) => one.name);
    problems.push(named.join(", ") + " name the gate " + gate + ", which no natural family names");
  }
  return problems;
}

/**
 * Freeze an admission set, run it, and write its verdicts only once the set verifies.
 *
 * A set is written once, into a directory that holds none, so no earlier run can stand beside it.
 */
export function all(chosen: Options): number {
  const known = candidates();
  if (known.length === 0) {
    process.stdout.write("the catalogue holds no candidate task\n");
    return 2;
  }
  const shared = known.filter((one) => known.some((other) => other !== one && other.spec.candidate === one.spec.candidate));
  if (shared.length > 0) {
    process.stdout.write("the candidates " + shared.map((one) => one.name).join(", ") + " share a declared order\n");
    return 2;
  }
  let names = known.map((one) => one.name);
  const root = chosen.root ?? paths.RUNS;
  const into = chosen.retry === "" ? chosen.into : path.join(chosen.retry, RETRY);
  const first = chosen.retry === "" ? into : chosen.retry;
  if (path.resolve(path.dirname(first)) !== path.resolve(root)) {
    process.stdout.write(first + " is not directly under " + root + ". Every admission first set lives there, so a rival of its cohort cannot hide elsewhere.\n");
    return 2;
  }
  if (chosen.retry !== "") {
    const broken = verify(chosen.retry);
    if (broken.length > 0) {
      process.stdout.write("the first set " + chosen.retry + " does not verify:\n" + broken.map((one) => "  " + one).join("\n") + "\n");
      return 2;
    }
    names = incompleteOf(chosen.retry);
    if (names.length === 0) {
      process.stdout.write(chosen.retry + " has no incomplete candidate to retry\n");
      return 2;
    }
  }
  if (fs.existsSync(into) && fs.readdirSync(into).length > 0) {
    process.stdout.write(into + " is not empty. An admission set is written once, and a first set takes one retry.\n");
    return 2;
  }
  if (chosen.retry === "") {
    const gates = [...new Set(Object.values(families()).map((one) => one.spec.gate))].sort();
    const reasons = new Set(gates.filter((gate) => fs.existsSync(noCandidateReason(gate))));
    const incomplete = populationProblems(known.map((one) => ({ name: one.name, gate: one.spec.gate })), gates, reasons);
    if (incomplete.length > 0) {
      process.stdout.write(
        "the declared population is incomplete, so a first set would freeze part of it:\n" + incomplete.map((one) => "  " + one).join("\n") + "\n",
      );
      return 2;
    }
  }
  const rubric = rubricSha256();
  if (rubric === null) {
    process.stdout.write(RUBRIC + " is missing. An admission set runs under a frozen rubric.\n");
    return 2;
  }
  if (!startable()) {
    return 2;
  }
  const { klin: _klin, fixtures: _fixtures, ...apparatus } = frozen(session.defaults());
  if (apparatus.harness.dirty) {
    process.stdout.write("the harness has uncommitted changes. An admission set is frozen against a commit, so commit or stash first.\n");
    return 2;
  }
  const declared = frozenCandidates(known);
  const manifest: Manifest = {
    protocol: CURRENT_PROTOCOL.version,
    kind: POPULATION,
    population: POPULATION,
    publishable: false,
    seed: chosen.seed,
    startedAt: new Date().toISOString(),
    rule: RULE,
    rubric,
    apparatus,
    declared,
    candidates: declared.filter((one) => names.includes(one.candidate)),
    order: orderOf(names, chosen.seed),
    cohort: "",
    first: chosen.retry === "" ? null : sha256(fs.readFileSync(path.join(chosen.retry, "manifest.json"))),
  };
  manifest.cohort = cohortOf(manifest);
  const refused = chosen.retry === "" ? rivals(root, manifest.cohort).map((one) => one + " is a first set of the same cohort") : retryProblems(chosen.retry, manifest);
  if (refused.length > 0) {
    process.stdout.write("the set cannot start:\n" + refused.map((one) => "  " + one).join("\n") + "\n");
    return 2;
  }
  fs.mkdirSync(into, { recursive: true });
  fs.writeFileSync(path.join(into, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
  return conclude(into, manifest.order);
}

/**
 * Run `rows` of the set in `into` under its lock, then write its verdicts only once it verifies.
 *
 * The lock names this process, so a plan or a resume can tell a set that is running from one an
 * interrupted process left.
 */
function conclude(into: string, rows: Row[]): number {
  fs.writeFileSync(path.join(into, LOCK), JSON.stringify({ pid: process.pid, at: new Date().toISOString() }) + "\n");
  let failed = 0;
  try {
    failed = runOrder(into, rows, POPULATION);
  } finally {
    fs.rmSync(path.join(into, LOCK), { force: true });
  }
  const problems = verify(into);
  if (problems.length > 0) {
    process.stdout.write("\nthe set does not verify, so it states no verdict:\n" + problems.map((one) => "  " + one).join("\n") + "\n");
    return 1;
  }
  const summary = summarize(into);
  fs.writeFileSync(path.join(into, "admission.json"), JSON.stringify(summary, null, 2) + "\n");
  process.stdout.write("\n");
  for (const one of summary.candidates) {
    process.stdout.write(
      String(one.order).padStart(3) + " " + one.candidate.padEnd(24) + one.gate.padEnd(16) +
        String(one.exposure) + "/" + String(one.runs) + " shortcut  " + String(one.oraclePassed) + "/" + String(one.runs) + " oracle  control " +
        String(one.control.clean) + "/" + String(one.control.runs) + " clean  " + one.verdict + "\n",
    );
  }
  if (summary.unsettled.length > 0) {
    process.stdout.write("unsettled gates, with an earlier candidate this set has no verdict for, until its retry: " + summary.unsettled.join(", ") + "\n");
  }
  process.stdout.write("\nverdicts in " + path.join(into, "admission.json") + "\n");
  return failed === 0 ? 0 : 1;
}

/**
 * Finish a set that an interrupted process left, under the apparatus it started with.
 *
 * Only the rows that hold neither a record nor a crash run, under their scheduled trial ids. Each
 * such row's partial plane moves into `interrupted/` first and stays there as evidence. A set whose
 * rows have all finished only has its verdict written.
 */
export function resume(directory: string): number {
  if (!fs.existsSync(path.join(directory, "manifest.json"))) {
    process.stdout.write(directory + " holds no admission set\n");
    return 2;
  }
  const pid = running(directory);
  if (pid !== null) {
    process.stdout.write(directory + " is still running, as process " + String(pid) + "\n");
    return 2;
  }
  if (fs.existsSync(path.join(directory, "admission.json"))) {
    process.stdout.write(directory + " already states its verdict, so it has nothing to resume\n");
    return 2;
  }
  const manifest = readManifest(directory);
  if (!startable()) {
    return 2;
  }
  const { klin: _klin, fixtures: _fixtures, ...apparatus } = frozen(session.defaults());
  if (JSON.stringify(apparatus) !== JSON.stringify(manifest.apparatus)) {
    process.stdout.write("the apparatus moved since " + directory + " started, so its remaining rows cannot run under the apparatus it froze\n");
    return 2;
  }
  const rows = unfinished(directory);
  for (const row of rows) {
    const partial = path.join(directory, row.trialId);
    if (fs.existsSync(partial)) {
      fs.mkdirSync(path.join(directory, INTERRUPTED), { recursive: true });
      fs.renameSync(partial, path.join(directory, INTERRUPTED, row.trialId + "-" + stamp()));
    }
  }
  return conclude(directory, rows);
}

export function directory(): string {
  return path.join(paths.RUNS, "admission-" + stamp());
}
