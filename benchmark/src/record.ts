import * as paths from "./paths.ts";

/**
 * The machine run record.
 *
 * One record describes one trial completely enough for the later scorecard to be computed
 * without reading terminal text. `record.schema.json` beside this file is the same contract in
 * JSON Schema.
 *
 * The record holds no hidden chain of thought. It holds the agent's final result text, the hook
 * evidence, the oracle's answers and the facts `klin stats --json` reports.
 */

export interface Signal {
  identity: string;
  kind: "regression" | "audit";
  auditKind: string | null;
  gate: string;
  label: string | null;
  file: string | null;
  line: number | null;
  text: string | null;
  values: unknown;
  remedy: string | null;
  outcome: string | null;
  tries: number | null;
  decision: string | null;
  reason: string | null;
  time: number | null;
  delivery: "delivered" | "would-have-been-delivered";
}

export interface HookInvocation {
  order: number;
  event: string;
  arguments: string;
  status: number;
  delivered: boolean;
  stdout: string;
  stderr: string;
  started: string;
  ended: string;
  stdinClosed: boolean;
}

export interface RunRecord {
  protocol: number;
  kind: "calibration" | "publishable";
  publishable: boolean;
  family: string;
  gate: string;
  taskId: string;
  variant: string;
  arm: string;
  trialId: string;
  order: number;
  fixture: { startCommit: string; promptSha256: string; treeSha256: string };
  harness: { commit: string; dirty: boolean; treeSha256: string };
  klin: { commit: string; version: string; binarySha256: string };
  host: {
    name: string;
    version: string;
    flags: string[];
    flagsSha256: string;
    isolatedConfiguration: boolean;
    memory: { sha256: string; bytes: number } | null;
  };
  model: { requested: string; reported: string | null };
  agent: { configSha256: string; configurationDigest: string };
  startedAt: string;
  endedAt: string;
  wallMs: number;
  infrastructure: { valid: boolean; reason: string | null };
  result: { outcome: string; evidence: string };
  oracle: { behaviourPassed: boolean; exit: number | null; reason: string };
  shortcut: { present: boolean | null; detector: string; sites: unknown[]; note: string };
  signals: Signal[];
  hooks: HookInvocation[];
  friction: {
    blockedStops: number;
    gateRuns: number;
    guardRefusals: number;
    tries: number;
    hostDenials: number;
  };
  stats: unknown;
  activity: { klinMs: number | null };
  turns: number | null;
  isolation: { workspace: unknown; freshness: unknown };
}

const REQUIRED = [
  "protocol",
  "kind",
  "publishable",
  "family",
  "gate",
  "taskId",
  "variant",
  "arm",
  "trialId",
  "order",
  "fixture",
  "harness",
  "klin",
  "host",
  "model",
  "agent",
  "startedAt",
  "endedAt",
  "wallMs",
  "infrastructure",
  "result",
  "oracle",
  "shortcut",
  "signals",
  "hooks",
  "friction",
  "stats",
  "activity",
  "turns",
  "isolation",
];

const OUTCOMES = ["completed", "gave-up", "person-required", "error"];

/** Every way one record fails the contract, as sentences. A valid record gives none. */
export function validate(record: Record<string, unknown>): string[] {
  const problems: string[] = [];
  for (const key of REQUIRED) {
    if (!(key in record)) {
      problems.push("the record states no " + key);
    }
  }
  if (record.protocol !== paths.PROTOCOL) {
    problems.push("the record states protocol " + String(record.protocol));
  }
  if (record.kind === "calibration" && record.publishable !== false) {
    problems.push("a calibration record must state publishable false");
  }
  const result = record.result as { outcome?: string } | undefined;
  if (result && !OUTCOMES.includes(String(result.outcome))) {
    problems.push("the result outcome " + String(result.outcome) + " is not one of " + OUTCOMES.join(", "));
  }
  const signals = (record.signals ?? []) as Signal[];
  const wanted = record.arm === "active" ? "delivered" : "would-have-been-delivered";
  for (const signal of signals) {
    if (signal.delivery !== wanted) {
      problems.push("a signal of the " + String(record.arm) + " arm states delivery " + signal.delivery);
    }
  }
  const asked = signals.filter((one) => one.auditKind === "asked-once");
  if (asked.some((one) => one.kind !== "audit")) {
    problems.push("an asked-once signal is recorded as a regression");
  }
  for (const hook of (record.hooks ?? []) as HookInvocation[]) {
    if (!hook.stdinClosed) {
      problems.push("hook " + String(hook.order) + " did not reach end of input");
    }
  }
  return problems;
}

interface Episode {
  gate?: string;
  label?: string;
  id?: string | null;
  key?: Record<string, unknown>;
  file?: string;
  line?: number;
  text?: string;
  values?: unknown;
  remedy?: string;
  outcome?: string;
  tries?: number;
  time?: number;
}

interface Audit {
  time?: number;
  kind?: string;
  decision?: string;
  reason?: string;
  file?: string;
  line?: number;
}

function identityOf(episode: Episode): string {
  if (episode.id) {
    return episode.id;
  }
  const key = episode.key ?? {};
  return [key.gate, key.file, key.line, key.text].map((one) => String(one ?? "")).join("|");
}

const ASKED = "asked-once";

/**
 * The distinct signal sites one trial produced, from `klin stats --json`.
 *
 * Both arms record them. In Active the signal reached the agent. In Shadow the same hook ran and
 * the harness suppressed delivery, so the signal is the one that would have been delivered.
 *
 * A deleted test klin asked about once is review evidence, never a claimed repair. `klin stats
 * --json` reports such a site twice: as an episode whose outcome is `asked-once`, and again in
 * the audit list. The episode carries the gate, the file, the line, the declaration text and the
 * remedy, so it is the one kept, under `audit`. An audit entry for a site no episode carries is
 * kept too, so nothing is lost if klin ever reports one alone, and it takes the audit kind as
 * its gate because the episode is where the gate's own name lives.
 *
 * No gate name appears in this file. A new gate needs a fixture family, not a change here.
 */
export function signalsFrom(stats: Record<string, unknown>, arm: string): Signal[] {
  const delivery = arm === "active" ? "delivered" : "would-have-been-delivered";
  const episodes = (stats.episodes ?? []) as Episode[];
  const askedSites = new Set(
    episodes
      .filter((episode) => episode.outcome === ASKED)
      .map((episode) => String(episode.file) + ":" + String(episode.line)),
  );
  const audit = (stats.audit ?? []) as Audit[];
  const fromEpisodes: Signal[] = episodes.map((episode) => ({
    identity: identityOf(episode),
    kind: episode.outcome === ASKED ? "audit" : "regression",
    auditKind: episode.outcome === ASKED ? ASKED : null,
    gate: String(episode.gate ?? ""),
    label: episode.label ?? null,
    file: episode.file ?? null,
    line: episode.line ?? null,
    text: episode.text ?? null,
    values: episode.values ?? null,
    remedy: episode.remedy ?? null,
    outcome: episode.outcome ?? null,
    tries: episode.tries ?? null,
    decision: null,
    reason: null,
    time: episode.time ?? null,
    delivery,
  }));
  const heldByEpisode = (entry: Audit): boolean =>
    entry.kind === ASKED && askedSites.has(String(entry.file) + ":" + String(entry.line));
  const fromAudit: Signal[] = audit
    .filter((entry) => !heldByEpisode(entry))
    .map((entry) => ({
      identity: [entry.kind, entry.file, entry.line, entry.reason]
        .map((one) => String(one ?? ""))
        .join("|"),
      kind: "audit",
      auditKind: entry.kind ?? null,
      gate: String(entry.kind ?? ""),
      label: null,
      file: entry.file ?? null,
      line: entry.line ?? null,
      text: null,
      values: null,
      remedy: null,
      outcome: null,
      tries: null,
      decision: entry.decision ?? null,
      reason: entry.reason ?? null,
      time: entry.time ?? null,
      delivery,
    }));
  return [...fromEpisodes, ...fromAudit];
}
