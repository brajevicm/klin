import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "../src/paths.ts";
import { CURRENT_PROTOCOL } from "../src/protocol.ts";
import { TYPESCRIPT_SHA256 } from "../src/toolchain.ts";
import { execFileSync } from "node:child_process";
import { CLAIM, RULE, claim, claimRef, cohortOf, orderOf, rubricSha256, summarize, type Apparatus, type Candidate, type Manifest, type Place } from "../src/admission.ts";
import type { RunRecord } from "../src/record.ts";
import { sha256 } from "../src/trees.ts";

/** An admission set on disk, as `calibrate --population admission` leaves one. No session runs. */

const BASE = JSON.parse(fs.readFileSync(path.join(paths.BENCHMARK, "test", "live-record.json"), "utf8")) as Record<string, unknown>;

export const APPARATUS: Apparatus = {
  protocol: CURRENT_PROTOCOL.version,
  schemaSha256: "s",
  harness: { commit: "h", dirty: false, treeSha256: "ht", hookSha256: "hook" },
  confinement: "sandbox",
  execution: "e",
  toolchain: { package: "typescript", version: "5.9.3", path: "/tsc.js", sha256: TYPESCRIPT_SHA256 },
  host: { name: "claude-code", version: "2.1.276 (Claude Code)" },
  model: "sonnet",
  flags: ["--print"],
  isolatedConfiguration: false,
  memory: null,
  machine: { platform: "test", release: "0", arch: "x", node: "v0" },
};

export interface Outcome {
  valid?: boolean;
  shortcut: boolean;
  oracle?: boolean;
}

export function candidate(name: string, gate: string, order: number): Candidate {
  const identity = (variant: string) => ({ taskId: "t-" + name + variant, promptSha256: "p-" + name + variant, treeSha256: "tr-" + name + variant });
  return { candidate: name, gate, order, fixtureSha256: "f-" + name, variants: { risk: identity("risk"), control: identity("control") } };
}

export function recordFor(row: Manifest["order"][number], trialId: string, outcome: Outcome, frozen?: Candidate, apparatus: Apparatus = APPARATUS): RunRecord {
  const valid = outcome.valid ?? true;
  const planned = frozen?.variants[row.variant as "risk" | "control"] ?? {
    taskId: "t-" + row.family + row.variant,
    promptSha256: "p-" + row.family + row.variant,
    treeSha256: "tr-" + row.family + row.variant,
  };
  return {
    ...BASE,
    protocol: CURRENT_PROTOCOL.version,
    kind: "admission",
    publishable: false,
    family: row.family,
    variant: row.variant,
    arm: "shadow",
    taskId: planned.taskId,
    order: row.order,
    repetition: row.repetition,
    trialId,
    replaces: null,
    audit: [],
    signals: [],
    hooks: [],
    fixture: { startCommit: "c", promptSha256: planned.promptSha256, treeSha256: planned.treeSha256 },
    harness: { commit: apparatus.harness.commit, dirty: apparatus.harness.dirty, treeSha256: apparatus.harness.treeSha256 },
    klin: { commit: "k", version: "klin 0.9", binarySha256: "kb" },
    host: {
      name: "claude-code",
      version: apparatus.host.version,
      flags: apparatus.flags,
      flagsSha256: "x",
      isolatedConfiguration: apparatus.isolatedConfiguration,
      memory: apparatus.memory,
    },
    model: { requested: apparatus.model, reported: apparatus.model },
    agent: { wiringSha256: "w", wrapperSha256: "wr" },
    infrastructure: valid
      ? { valid: true, reason: null, terms: [{ name: "state-fresh", passed: true, detail: "" }] }
      : { valid: false, reason: "host-result-read", terms: [{ name: "host-result-read", passed: false, detail: "no JSON" }] },
    result: { outcome: "completed", evidence: "" },
    oracle: { behaviourPassed: outcome.oracle ?? true, exit: 0, reason: "" },
    shortcut: { present: outcome.shortcut, detector: "d", sites: [], note: "", unread: null },
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

export function write(where: string, directory: string, record: RunRecord): void {
  fs.mkdirSync(path.join(where, directory), { recursive: true });
  fs.writeFileSync(path.join(where, directory, "record.json"), JSON.stringify(record) + "\n");
}

/**
 * An admission set on disk. Each candidate's outcomes are its three risk runs, then its control.
 * A candidate `declared` holds and `outcomes` does not is one the set did not run.
 */
export function setOnDisk(
  declared: Candidate[],
  outcomes: Record<string, Outcome[]>,
  at: { retries?: string; under?: string; apparatus?: Apparatus; remote?: string } = {},
): { where: string; manifest: Manifest } {
  const where = at.retries
    ? path.join(at.retries, "retry")
    : fs.mkdtempSync(path.join(at.under ?? fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-admission-root-")), "klin-bench-admission-"));
  fs.mkdirSync(where, { recursive: true });
  const names = declared.filter((one) => one.candidate in outcomes).map((one) => one.candidate);
  const manifest: Manifest = {
    protocol: CURRENT_PROTOCOL.version,
    kind: "admission",
    population: "admission",
    publishable: false,
    seed: 1,
    startedAt: "2026-09-24T00:00:00Z",
    rule: RULE,
    rubric: rubricSha256() as string,
    apparatus: at.apparatus ?? APPARATUS,
    declared,
    candidates: declared.filter((one) => names.includes(one.candidate)),
    order: orderOf(names, 1),
    cohort: "",
    first: at.retries ? sha256(fs.readFileSync(path.join(at.retries, "manifest.json"))) : null,
  };
  manifest.cohort = cohortOf(manifest);
  fs.writeFileSync(path.join(where, "manifest.json"), JSON.stringify(manifest) + "\n");
  if (!at.retries) {
    const remote = at.remote ?? placeOf(path.dirname(where)).remote;
    const commit = claim(remote, manifest.rubric, where);
    if (commit !== null) {
      fs.writeFileSync(path.join(where, CLAIM), JSON.stringify({ remote, ref: claimRef(manifest.rubric), commit }) + "\n");
    }
  }
  for (const row of manifest.order) {
    const at = row.variant === "risk" ? row.repetition - 1 : RULE.runs + row.repetition - 1;
    write(where, row.trialId, recordFor(row, row.trialId, outcomes[row.family][at], declared.find((one) => one.candidate === row.family), manifest.apparatus));
  }
  return { where, manifest };
}

export const risk = (shortcut: boolean, oracle = true): Outcome => ({ shortcut, oracle });
export const clean: Outcome = { shortcut: false };
export const admitted = [risk(true), risk(true), risk(true), clean];
export const invalid = [risk(true), risk(true), { valid: false, shortcut: true }, clean];

/** The verdict `calibrate` writes once a set verifies. */
export function withVerdict(where: string): string {
  fs.writeFileSync(path.join(where, "admission.json"), JSON.stringify(summarize(where), null, 2) + "\n");
  return where;
}

export function digestOf(file: string): string {
  return sha256(fs.readFileSync(file));
}

/** A root for first sets, with a bare repository beside them as the shared remote that holds the claims. */
export function placeOf(root: string): Place {
  const remote = path.join(root, ".claims.git");
  if (!fs.existsSync(remote)) {
    execFileSync("git", ["init", "--quiet", "--bare", remote]);
  }
  return { root, remote };
}

/** The place a set made by `setOnDisk` lives in, where `final` looks for its rivals and its claim. */
export function rootOf(where: string): Place {
  return placeOf(path.dirname(where));
}
