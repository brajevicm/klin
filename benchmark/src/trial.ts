import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import * as paths from "./paths.ts";
import { copyTree, digest, sha256 } from "./trees.ts";
import { family as familyNamed, type ArmName, type VariantName } from "./catalogue.ts";
import * as workspace from "./workspace.ts";
import * as session from "./session.ts";
import * as oracle from "./oracle.ts";
import * as integrity from "./integrity.ts";
import { signalsFrom, validate, type HookInvocation, type RunRecord } from "./record.ts";

/** One trial: one family, one variant, one arm, one fresh repository and one fresh session. */

export interface TrialOptions extends session.SessionOptions {
  order: number;
  kind: "calibration" | "publishable";
  control: string;
}

function inRepo(args: string[]): string {
  try {
    return execFileSync("git", args, { cwd: paths.REPO, encoding: "utf8" }).trim();
  } catch {
    return "";
  }
}

/**
 * The repository's HEAD when the trial ran, and whether anything was uncommitted.
 *
 * This is where the harness came from. It is not proof of where the binary came from: a stale
 * build carries an older commit's behaviour under today's HEAD. `klin.binarySha256` is the
 * binary's own identity, and it is what a later reader should trust.
 */
function provenance(): { commit: string; dirty: boolean } {
  return { commit: inRepo(["rev-parse", "HEAD"]), dirty: inRepo(["status", "--porcelain"]) !== "" };
}

function binarySha256(binary: string): string {
  return fs.existsSync(binary) ? sha256(fs.readFileSync(binary)) : "";
}

function outcomeOf(ran: session.SessionResult, hooks: HookInvocation[]): {
  outcome: string;
  evidence: string;
} {
  const asked = hooks.some(
    (hook) => hook.delivered && hook.stdout.includes('"permissionDecision":"ask"'),
  );
  const agent = ran.agent ?? {};
  const subtype = String(agent.subtype ?? "");
  const isError = agent.is_error === true;
  if (ran.timedOut) {
    return { outcome: "gave-up", evidence: "the session reached the harness timeout" };
  }
  if (subtype.startsWith("error_max_turns") || subtype === "error_budget_exceeded") {
    return { outcome: "gave-up", evidence: "the host reported " + subtype };
  }
  if (asked && (isError || ran.exit !== 0)) {
    return { outcome: "person-required", evidence: "a guard question reached a session with no person" };
  }
  if (ran.exit === 0 && !isError) {
    return { outcome: "completed", evidence: "the host reported " + (subtype || "success") };
  }
  return { outcome: "error", evidence: "the host exited " + String(ran.exit) + " " + subtype };
}

function friction(
  hooks: HookInvocation[],
  signals: { tries: number | null }[],
  ran: session.SessionResult,
): RunRecord["friction"] {
  const gates = hooks.filter((hook) => hook.arguments.startsWith("gate"));
  const guards = hooks.filter((hook) => hook.arguments.startsWith("guard"));
  const denials = (ran.agent?.permission_denials ?? []) as unknown[];
  return {
    blockedStops: gates.filter((hook) => hook.status === 2).length,
    gateRuns: gates.length,
    guardRefusals: guards.filter((hook) => hook.status === 2).length,
    tries: signals.reduce((sum, one) => sum + (one.tries ?? 0), 0),
    hostDenials: denials.length,
  };
}

/**
 * Every model the host reports it ran, sorted.
 *
 * The host lists more than one: the model the session asked for, and the small model it uses for
 * its own housekeeping. Naming only the first would mis-record the trial's model, as calibration
 * on 2026-09-17 found.
 */
function modelsRan(ran: session.SessionResult): string | null {
  const usage = (ran.agent?.modelUsage ?? {}) as Record<string, unknown>;
  const named = Object.keys(usage).sort();
  return named.length > 0 ? named.join(", ") : null;
}

function write(where: string, name: string, held: unknown): void {
  fs.mkdirSync(where, { recursive: true });
  fs.writeFileSync(path.join(where, name), JSON.stringify(held, null, 2) + "\n");
}

export function run(
  familyName: string,
  variantName: VariantName,
  arm: ArmName,
  trialId: string,
  options: TrialOptions,
): RunRecord {
  const found = familyNamed(familyName);
  const variant = found.variants[variantName];
  const control = path.join(options.control, trialId);
  fs.mkdirSync(control, { recursive: true });

  const here = provenance();
  const configDir = session.configFor(options, trialId);
  const place = workspace.materialize(variant, trialId);
  const freshness = integrity.freshness(place.repo, place.state, configDir, place.commits);
  const isolation = integrity.judge(variant, found.spec.gate, place.repo, control);

  const ran = session.run(place, variant.prompt, arm === "active", options, configDir);
  isolation.checks.push(integrity.stillHidden(variant, place.repo));
  isolation.verified = isolation.checks.every((one) => one.passed);

  // The tree copies sit under a directory named `fixtures`, which is in klin's built-in skip
  // set, so records kept inside this repository are never measured as klin's own source.
  const trees = path.join(control, "fixtures");
  const base = workspace.startingTree(variant, path.join(trees, "base"));
  const final = path.join(trees, "final");
  fs.rmSync(final, { recursive: true, force: true });
  copyTree(place.repo, final);

  const judged = oracle.judge(variant, base, final, path.join(trees, "scoring"));
  const stats = session.stats(place.repo, place.state, options.klinBin, ["--since", "1d"]) as Record<
    string,
    unknown
  >;
  const bySession = session.stats(place.repo, place.state, options.klinBin, ["--session"]);
  const hooks = session.hookEvidence(place.hooks);
  const signals = signalsFrom(stats, arm);
  const activity = (stats.activity ?? {}) as Record<string, number>;

  const record: RunRecord = {
    protocol: paths.PROTOCOL,
    kind: options.kind,
    publishable: options.kind === "publishable",
    family: familyName,
    gate: found.spec.gate,
    taskId: variant.taskId,
    variant: variantName,
    arm,
    trialId,
    order: options.order,
    fixture: {
      startCommit: place.startCommit,
      promptSha256: variant.promptSha256,
      treeSha256: place.treeSha256,
    },
    harness: {
      commit: here.commit,
      dirty: here.dirty,
      treeSha256: digest(path.join(paths.BENCHMARK, "src")),
    },
    klin: {
      commit: here.commit,
      version: session.klinVersion(options.klinBin),
      binarySha256: binarySha256(options.klinBin),
    },
    host: {
      name: "claude-code",
      version: session.hostVersion(),
      flags: ran.flags,
      flagsSha256: ran.flagsSha256,
      isolatedConfiguration: configDir !== "",
      memory: session.memory(configDir),
    },
    model: { requested: options.model, reported: modelsRan(ran) },
    agent: {
      configSha256: sha256(fs.readFileSync(place.settings)),
      configurationDigest: sha256(ran.flags.join(" ") + "|" + options.model),
    },
    startedAt: ran.startedAt,
    endedAt: ran.endedAt,
    wallMs: ran.wallMs,
    infrastructure: {
      valid: isolation.verified && freshness.verified && ran.agent !== null,
      reason:
        isolation.verified && freshness.verified && ran.agent !== null
          ? null
          : "isolation, freshness or the host result was not whole",
    },
    result: outcomeOf(ran, hooks),
    oracle: {
      behaviourPassed: judged.behaviour.passed,
      exit: judged.behaviour.exit,
      reason: judged.behaviour.reason,
    },
    shortcut: {
      present: judged.shortcut.present,
      detector: judged.shortcut.detector ?? variant.shortcut.detector,
      sites: judged.shortcut.sites,
      note: judged.shortcut.note,
    },
    signals,
    hooks,
    friction: friction(hooks, signals, ran),
    stats,
    activity: { klinMs: activity.klin_ms ?? null },
    turns: (ran.agent?.num_turns as number | undefined) ?? null,
    isolation: { workspace: isolation, freshness },
  };

  write(control, "record.json", record);
  write(control, "agent.json", { ...ran, stdout: ran.stdout, agent: ran.agent });
  write(control, "behaviour.json", judged.behaviour);
  write(control, "stats-session.json", bySession);
  copyTree(place.hooks, path.join(control, "hooks"));

  const problems = validate(record as unknown as Record<string, unknown>);
  if (problems.length > 0) {
    write(control, "record-problems.json", problems);
  }
  return record;
}
