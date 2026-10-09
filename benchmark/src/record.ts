import { VARIANTS } from "./catalogue.ts";
import { CURRENT_PROTOCOL } from "./protocol.ts";

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

/**
 * One named thing a trial was held to, passed or failed, with the sentence that says why.
 *
 * `integrity.ts` produces these and `trial.ts` collects them. The type lives here because the
 * record is what they are for: `record.schema.json` defines the same shape as `$defs/check`, and
 * a module that produces one should not own the contract it satisfies.
 */
export interface Check {
  name: string;
  passed: boolean;
  detail: string;
}

/** A group of checks and whether every one of them passed. `$defs/isolation` in the schema. */
export interface Isolation {
  verified: boolean;
  checks: Check[];
}

/**
 * One site a person may classify, or one factual audit row.
 *
 * The two live in two fields of the record. `signals` holds the Regression episodes and the
 * deleted-test questions klin asked, which is what #115 blinds and classifies. `audit` holds the
 * factual trail beside them, a guard decision or a reset, which no one classifies. One shape
 * serves both, because a reader of either wants the same columns.
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
  /** Null where klin delivers nothing to an agent, as for a reset a person ran. */
  delivery: "delivered" | "would-have-been-delivered" | null;
}

export interface HookInvocation {
  order: number;
  event: string;
  /** The tool the agent asked for, where the event carries one. */
  tool: string;
  /** The paths that tool's input held. `integrity.stayedInside` reads it and nothing else does. */
  paths: string;
  arguments: string;
  status: number;
  delivered: boolean;
  stdout: string;
  stderr: string;
  report?: GateReport | null;
  started: string;
  ended: string;
  stdinClosed: boolean;
}

export interface GateReport {
  status: "PASS" | "FAIL" | "ERROR";
  summary: string;
  derived: unknown[];
  gates: unknown[];
  findings: unknown[];
  notes: unknown[];
  exit: number;
}

export function isGateReport(value: unknown): value is GateReport {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return false;
  const held = value as Record<string, unknown>;
  return typeof held.status === "string" &&
    ["PASS", "FAIL", "ERROR", "INCOMPLETE"].includes(held.status) &&
    typeof held.summary === "string" &&
    Number.isInteger(held.exit) &&
    Array.isArray(held.derived) &&
    Array.isArray(held.gates) &&
    Array.isArray(held.findings) &&
    Array.isArray(held.notes);
}

function siteIdentities(site: unknown): string[] {
  if (typeof site !== "object" || site === null) return [];
  const held = site as Record<string, unknown>;
  const gate = typeof held.gate === "string" ? held.gate : "";
  const id = typeof held.id === "string" && held.id !== "" ? held.id : null;
  const file = typeof held.file === "string" ? held.file : null;
  const text = typeof held.text === "string" ? held.text : null;
  return [
    ...(id !== null && gate !== "" ? ["id:" + gate + "\0" + id] : []),
    ...(gate !== "" && file !== null && text !== null ? ["site:" + gate + "\0" + file + "\0" + text] : []),
  ];
}

function isTargetSite(site: unknown): boolean {
  return siteIdentities(site).length > 0;
}

function reportSites(report: GateReport): unknown[] {
  return [
    ...(Array.isArray(report.findings) ? report.findings : []),
    ...(Array.isArray(report.notes) ? report.notes : []),
  ];
}

/** Whether the Stop left its own report. The wrapper gives each call a fresh file, so a report
 * is this call's. Its `exit` is the check's exit, which a host's block code does not equal. */
export function exactStopReport(hook: HookInvocation): hook is HookInvocation & { report: GateReport } {
  return isGateReport(hook.report);
}

export function targetStop(hook: HookInvocation, sites: unknown[]): boolean {
  if (hook.event !== "Stop") return false;
  const targets = new Set(sites.flatMap(siteIdentities));
  if (targets.size === 0 || !exactStopReport(hook)) return false;
  return reportSites(hook.report).some((site) =>
    siteIdentities(site).some((identity) => targets.has(identity)),
  );
}

export function stopMetrics(
  hooks: HookInvocation[],
  sites: unknown[],
): { stopDelivery: boolean; blockedStops: number } {
  const targetStops = hooks.filter((one) => targetStop(one, sites));
  return {
    stopDelivery: targetStops.length > 0,
    blockedStops: targetStops.filter((one) => one.status === 2).length,
  };
}

function isVerdict(value: unknown): value is boolean | null {
  return value === true || value === false || value === null;
}

export function finalRepairOf(present: boolean | null): boolean | null {
  return present === null ? null : present === false;
}

export interface Verdict {
  status: string;
  sites: unknown[];
}

export interface WholeRun extends Verdict {
  caught: boolean;
  hook: Verdict;
}

export type Verdicts = Pick<WholeRun, "status" | "sites" | "hook">;

export function failedOn(verdict: Verdict): boolean {
  return verdict.status === "FAIL" && verdict.sites.length > 0;
}

export function wholeRunCaught(run: Verdicts): boolean {
  return failedOn(run) || failedOn(run.hook);
}

export function targetSites(run: Verdicts): unknown[] {
  return [...run.sites, ...run.hook.sites];
}

export interface SeededMetrics {
  wholeRun: WholeRun;
  stopDelivery: boolean;
  finalRepair: boolean | null;
  blockedStops: number;
  tries: number;
}

export interface RunRecord {
  protocol: number;
  kind: "calibration" | "publishable" | "admission";
  publishable: boolean;
  family: string;
  gate: string;
  taskId: string;
  variant: string;
  arm: string;
  trialId: string;
  order: number;
  /** Which repetition of the family and variant this is. Calibration runs each once. */
  repetition: number;
  /** The trial id of the infrastructure-invalid attempt this run replaces, or null. */
  replaces: string | null;
  /**
   * The two trees one trial holds apart, and the seed that separates them.
   *
   * `treeSha256` is the committed clean base, which klin's base comparison and every detector
   * measure against. `startTreeSha256` is the tree the subject was given, which is the same tree
   * for a natural variant and the committed base under `seed` for a seeded one. `uncommitted` is
   * what git reported standing in the working tree before the session, and `startShortcut` is the
   * detector's answer over that tree, read before the agent started.
   */
  fixture: {
    startCommit: string;
    promptSha256: string;
    treeSha256: string;
    startTreeSha256: string;
    finalTreeSha256?: string;
    seed: string[];
    staged?: string[];
    uncommitted: string[];
    startShortcut: {
      present: boolean | null;
      detector: string;
      sites: unknown[];
      note: string;
      unread: "base" | "final" | null;
    };
  };
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
  agent: { wiringSha256: string; wrapperSha256: string };
  cost?: number | null;
  seeded?: SeededMetrics;
  startedAt: string;
  endedAt: string;
  wallMs: number;
  infrastructure: { valid: boolean; reason: string | null; terms: Check[] };
  result: { outcome: string; evidence: string };
  oracle: { behaviourPassed: boolean; exit: number | null; reason: string };
  shortcut: {
    present: boolean | null;
    detector: string;
    sites: unknown[];
    note: string;
    unread: "base" | "final" | null;
  };
  signals: Signal[];
  audit: Signal[];
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
  isolation: {
    workspace: Isolation;
    freshness: Isolation;
    outside: Check;
    seed: Check;
    start: Check;
    baseStamp: Check;
  };
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
  "audit",
  "hooks",
  "friction",
  "stats",
  "activity",
  "turns",
  "isolation",
];

const OUTCOMES = ["completed", "gave-up", "person-required", "error"];
export const WHOLE_RUN_STATUSES = ["FAIL", "PASS", "ok"];

function isVerdictOf(value: unknown): value is Verdict {
  if (typeof value !== "object" || value === null) return false;
  const held = value as Record<string, unknown>;
  return (
    typeof held.status === "string" &&
    WHOLE_RUN_STATUSES.includes(held.status) &&
    Array.isArray(held.sites) &&
    held.sites.every(isTargetSite)
  );
}

const ASKED = "asked-once";
/** The one audit kind klin never hands to an agent: a person ran it. */
const RESET = "reset";

/** Every way one record fails the contract, as sentences. A valid record gives none. */
export function validate(record: Record<string, unknown>): string[] {
  const problems: string[] = [];
  for (const key of REQUIRED) {
    if (!(key in record)) {
      problems.push("the record states no " + key);
    }
  }
  if (record.protocol !== CURRENT_PROTOCOL.version) {
    problems.push("the record states protocol " + String(record.protocol));
  }
  if (record.kind === "calibration" && record.publishable !== false) {
    problems.push("a calibration record must state publishable false");
  }
  if (record.kind === "admission" && (record.publishable !== false || record.arm !== "shadow")) {
    problems.push("an admission record must state publishable false and the shadow arm");
  }
  if (record.kind === "publishable" && record.publishable !== true) {
    problems.push("a publishable record must state publishable true");
  }
  // Calibration records written before the round existed state neither, and stay valid.
  if (record.kind === "publishable") {
    for (const key of ["repetition", "replaces"].filter((one) => !(one in record))) {
      problems.push("a publishable record states no " + key);
    }
  }
  if (record.replaces === record.trialId) {
    problems.push("a record claims to replace itself");
  }
  const result = record.result as { outcome?: string } | undefined;
  if (result && !OUTCOMES.includes(String(result.outcome))) {
    problems.push("the result outcome " + String(result.outcome) + " is not one of " + OUTCOMES.join(", "));
  }
  const infrastructure = record.infrastructure as { valid?: boolean; terms?: Check[] } | undefined;
  if (infrastructure) {
    const terms = infrastructure.terms ?? [];
    if (terms.length === 0) {
      problems.push("the record states no validity term");
    }
    if (infrastructure.valid !== terms.every((one) => one.passed)) {
      problems.push("the record claims validity its terms do not hold");
    }
  }
  const signals = (record.signals ?? []) as Signal[];
  const audit = (record.audit ?? []) as Signal[];
  const wanted = record.arm === "active" ? "delivered" : "would-have-been-delivered";
  for (const signal of [...signals, ...audit].filter((one) => one.auditKind !== RESET)) {
    if (signal.delivery !== wanted) {
      problems.push("a signal of the " + String(record.arm) + " arm states delivery " + signal.delivery);
    }
  }
  if (signals.some((one) => one.auditKind === ASKED && one.kind !== "audit")) {
    problems.push("an asked-once signal is recorded as a regression");
  }
  if (signals.some((one) => one.kind === "audit" && one.auditKind !== ASKED)) {
    problems.push("an ordinary audit row is recorded as a signal site to classify");
  }
  if (audit.some((one) => one.auditKind === ASKED)) {
    problems.push("a deleted-test question is filed as an ordinary audit row");
  }
  // A planted record has to prove its plant. This reads the natural population rather than one
  // variant's name, so a second planted variant cannot escape the contract by being called
  // something else. Records of a natural variant state none of this and stay valid, because the
  // natural population is committed whole and its two trees are one.
  if (!(VARIANTS as readonly string[]).includes(String(record.variant))) {
    const fixture = (record.fixture ?? {}) as Record<string, unknown>;
    for (const key of ["startTreeSha256", "seed", "uncommitted", "startShortcut"]) {
      if (!(key in fixture)) {
        problems.push("a planted record states no fixture " + key);
      }
    }
    const seed = fixture.seed;
    if (Array.isArray(seed) && seed.length === 0) {
      problems.push("a planted record declares no seed overlay");
    }
    const started = fixture.startShortcut as { present?: unknown } | undefined;
    if (started && started.present !== true) {
      problems.push(
        "a planted record states the starting shortcut " +
          String(started.present) +
          " where a plant is only a plant when the detector found it before the session",
      );
    }
    if (fixture.startTreeSha256 !== undefined && fixture.startTreeSha256 === fixture.treeSha256) {
      problems.push("a planted record states one digest for the committed base and the subject's starting tree");
    }
  }
  if (record.variant === "seeded") {
    if (!(infrastructure?.terms ?? []).some((one) => one.name === "seeded-whole-run")) {
      problems.push("a seeded record states no seeded-whole-run validity term");
    }
    if (!(infrastructure?.terms ?? []).some((one) => one.name === "seeded-stop-evidence")) {
      problems.push("a seeded record states no seeded-stop-evidence validity term");
    }
    const seeded = record.seeded as Record<string, unknown> | undefined;
    if (!seeded || typeof seeded !== "object") {
      problems.push("a seeded record states no seeded metrics");
    } else {
      const wholeRun = seeded.wholeRun as Record<string, unknown> | undefined;
      if (!wholeRun || typeof wholeRun !== "object") {
        problems.push("a seeded record states no whole-run result");
      } else {
        if (typeof wholeRun.caught !== "boolean") {
          problems.push("a seeded whole-run result states no catch verdict");
        }
        if (typeof wholeRun.status !== "string" || !WHOLE_RUN_STATUSES.includes(wholeRun.status)) {
          problems.push("a seeded whole-run result states no production status");
        }
        if (!Array.isArray(wholeRun.sites)) {
          problems.push("a seeded whole-run result states no sites");
        } else if (!wholeRun.sites.every(isTargetSite)) {
          problems.push("a seeded whole-run result states a malformed target site");
        }
        if (!isVerdictOf(wholeRun.hook)) {
          problems.push("a seeded whole-run result states no hook verdict");
        }
      }
      if (typeof seeded.stopDelivery !== "boolean") problems.push("a seeded record states no Stop delivery");
      if (![true, false, null].includes(seeded.finalRepair as boolean | null)) {
        problems.push("a seeded record states no final repair verdict");
      }
      for (const key of ["blockedStops", "tries"] as const) {
        if (!Number.isInteger(seeded[key])) problems.push("a seeded record states no " + key);
      }
      const shortcut = record.shortcut as { present?: unknown } | undefined;
      const shortcutPresent = shortcut?.present;
      const finalRepair = seeded.finalRepair;
      if (!isVerdict(shortcutPresent)) {
        problems.push("a seeded record states no final shortcut verdict");
      } else if (isVerdict(finalRepair)) {
        const expected = finalRepairOf(shortcutPresent);
        if (finalRepair !== expected) {
          problems.push("a seeded final repair verdict disagrees with the final shortcut verdict");
        }
      }
      if (
        wholeRun &&
        typeof wholeRun.caught === "boolean" &&
        isVerdictOf(wholeRun) &&
        isVerdictOf(wholeRun.hook)
      ) {
        const run = wholeRun as unknown as Verdicts;
        const expectedCaught = wholeRunCaught(run);
        if (wholeRun.caught !== expectedCaught) {
          problems.push("a seeded whole-run catch verdict disagrees with its production status and target sites");
        }
        const hooks = record.hooks;
        const usableHooks =
          Array.isArray(hooks) &&
          hooks.every((hook) => {
            if (typeof hook !== "object" || hook === null) return false;
            const held = hook as Record<string, unknown>;
            return typeof held.event === "string" && typeof held.arguments === "string" && typeof held.status === "number";
          });
        if (typeof seeded.stopDelivery === "boolean" && Number.isInteger(seeded.blockedStops) && usableHooks) {
          const expectedStops = stopMetrics(hooks as HookInvocation[], targetSites(run));
          if (seeded.stopDelivery !== expectedStops.stopDelivery) {
            problems.push("a seeded Stop delivery verdict disagrees with retained hook evidence");
          }
          if (seeded.blockedStops !== expectedStops.blockedStops) {
            problems.push("a seeded blocked-stop count disagrees with retained hook evidence");
          }
        }
      }
    }
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

/**
 * The two surfaces one trial's `klin stats --json` produces.
 *
 * `signals` holds what a person may classify: every Regression episode, and the deleted-test
 * question klin asked once, which is review evidence for the same reading. `audit` holds the
 * factual trail klin keeps beside them, a guard decision or a reset a person ran. A reset is not
 * a signal site, and #115 would have had to tell one from the other afterwards if both sat in one
 * field.
 *
 * Both arms record both. In Active the signal reached the agent. In Shadow the same hook ran and
 * the harness suppressed delivery, so the signal is the one that would have been delivered.
 *
 * `klin stats --json` reports a deleted-test question twice: as an episode whose outcome is
 * `asked-once`, and again in the audit list. The episode carries the gate, the file, the line,
 * the declaration text and the remedy, so it is the one kept. An audit entry for a site no
 * episode carries is kept too, so nothing is lost if klin ever reports one alone.
 *
 * An audit row alone states no gate, because a gate's own name lives on the episode. `auditKind`
 * is what names such a row. A reset states no delivery either: a person ran it and klin hands an
 * agent nothing, so neither arm could have delivered it.
 *
 * No gate name appears in this file. A new gate needs a fixture family, not a change here.
 */
export function signalsFrom(
  stats: Record<string, unknown>,
  arm: string,
): { signals: Signal[]; audit: Signal[] } {
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
      gate: "",
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
      delivery: entry.kind === RESET ? null : delivery,
    }));
  return {
    signals: [...fromEpisodes, ...fromAudit.filter((one) => one.auditKind === ASKED)],
    audit: fromAudit.filter((one) => one.auditKind !== ASKED),
  };
}
