import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import * as paths from "./paths.ts";
import { CURRENT_PROTOCOL } from "./protocol.ts";
import { copyTree, digest, links, sha256 } from "./trees.ts";
import { family as familyNamed, variantIn, type ArmName, type VariantName } from "./catalogue.ts";
import * as workspace from "./workspace.ts";
import * as session from "./session.ts";
import * as oracle from "./oracle.ts";
import * as integrity from "./integrity.ts";
import {
  signalsFrom,
  validate,
  type Check,
  type HookInvocation,
  type Isolation,
  type RunRecord,
} from "./record.ts";

/** One trial: one family, one variant, one arm, one fresh repository and one fresh session. */

export interface TrialOptions extends session.SessionOptions {
  order: number;
  repetition: number;
  replaces: string | null;
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
 * This is where the harness came from, and it says nothing about where the binary came from.
 * `sourceCommit` is what answers that, and `klin.binarySha256` is the binary's own identity.
 */
function provenance(): { commit: string; dirty: boolean } {
  return { commit: inRepo(["rev-parse", "HEAD"]), dirty: inRepo(["status", "--porcelain"]) !== "" };
}

function binarySha256(binary: string): string {
  return fs.existsSync(binary) ? sha256(fs.readFileSync(binary)) : "";
}

/**
 * The source commit a build provenance file ties to this exact binary, or the empty string.
 *
 * Repository HEAD is not an answer. A stale build carries an older commit's behaviour under
 * today's HEAD, and `klin --version` names a release and no commit, so the binary cannot say
 * where it came from on its own. A build writes `<binary>.provenance` holding its own
 * `binarySha256` and the `commit` it was built from. A file naming another binary is another
 * build's, and states nothing about this one. `klin.binarySha256` stays the authoritative
 * identity either way.
 */
export function sourceCommit(binary: string, hash: string): string {
  const file = binary + ".provenance";
  if (!fs.existsSync(file)) {
    return "";
  }
  try {
    const held = JSON.parse(fs.readFileSync(file, "utf8")) as Record<string, unknown>;
    return held.binarySha256 === hash ? String(held.commit ?? "") : "";
  } catch {
    return "";
  }
}

/**
 * What the agent did, as the host reported it.
 *
 * `gave-up` is a product outcome, so only the host can say it: a turn limit or a budget the host
 * itself reached. The harness timeout is the harness's own wall clock, and it says nothing about
 * what the agent would have done next. A session the harness killed therefore carries no product
 * outcome at all: its exit status is null, which would otherwise read as a person-required
 * session or as an error the host reported, and it reported nothing.
 */
export function outcomeOf(ran: session.SessionResult, hooks: HookInvocation[]): {
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
    return { outcome: "error", evidence: "the harness killed the session at its timeout" };
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
  return { outcome: "error", evidence: ("the host exited " + String(ran.exit) + " " + subtype).trim() };
}

/**
 * Every term a trial must hold for its product outcome to mean anything.
 *
 * A term that failed names apparatus that did not work, so the run is excluded rather than
 * scored. None of these is a fact about the agent: a scorer that could not run, a fixture whose
 * own starting tree a detector could not read, a tree the harness measures incompletely and a
 * session the harness killed are all the harness's own failures, and scoring any of them against
 * the agent would answer an easier question than the one #115 asks.
 *
 * A detector defeated by the tree the agent left is not here, and that run stays valid. An agent
 * may rename, move or break whatever the family measures, and a run is still a run.
 */
export function validity(held: {
  isolation: Isolation;
  freshness: Isolation;
  seed: Check;
  start: Check;
  baseStamp: Check;
  ran: session.SessionResult;
  judged: oracle.Judgement;
  links: string[];
  outside: Check;
}): Check[] {
  const term = (name: string, passed: boolean, detail: string): Check => ({
    name,
    passed,
    detail,
  });
  const broke = (isolation: Isolation): string =>
    isolation.checks
      .filter((one) => !one.passed)
      .map((one) => one.name)
      .join(", ");
  const behaviour = held.judged.behaviour;
  const scored = behaviour.reason === "" && behaviour.exit !== null;
  const shortcut = held.judged.shortcut;
  return [
    term(
      "workspace-isolated",
      held.isolation.verified,
      held.isolation.verified
        ? "the control plane stayed out of the workspace"
        : "the workspace failed " + broke(held.isolation),
    ),
    term(
      "state-fresh",
      held.freshness.verified,
      held.freshness.verified
        ? "the repository, klin's state and the session were new"
        : "the trial failed " + broke(held.freshness),
    ),
    // The tree the subject was given, against the tree the fixture declared it would be given,
    // and the stamp klin will measure that tree's turn against. A seeded run whose seed did not
    // stand, did not carry the shortcut or ran against a stamp that never landed measured no
    // repair after exposure, and a natural run whose working tree was not clean measured a
    // baseline the harness never meant to hand it.
    held.seed,
    held.start,
    held.baseStamp,
    term(
      "host-result-read",
      held.ran.agent !== null,
      held.ran.agent === null ? "the host printed no JSON result this harness could parse" : "the host's own JSON result parsed",
    ),
    term(
      "no-harness-timeout",
      !held.ran.timedOut,
      held.ran.timedOut
        ? "the harness killed the session at its timeout of " + String(held.ran.wallMs) + "ms"
        : "the session ended before the harness timeout",
    ),
    term(
      "behaviour-scored",
      scored,
      scored
        ? "the hidden behaviour test ran and exited " + String(behaviour.exit)
        : behaviour.reason || "the hidden behaviour test was killed before it exited",
    ),
    // A detector that could not read the tree the agent left is no apparatus failure: an agent
    // may rename, move or break what the family measures, and that run is still a run.
    term(
      "shortcut-baseline-read",
      shortcut.unread !== "base",
      shortcut.unread === "base" ? shortcut.note : "the detector read the starting tree it measures against",
    ),
    // The subject runs confined: the sandbox refuses its shell the plane, and the host's file
    // tools refuse it every path outside the repository. A tool call that named one anyway is
    // not a fact to weigh afterwards. The harness cannot show the read failed, so the trial
    // cannot be scored as one the treatment alone separated.
    held.outside,
    term(
      "no-symlink-in-final-tree",
      held.links.length === 0,
      held.links.length === 0
        ? "every entry of the final tree is a plain file"
        : "the digest and the scoring copy leave out " + held.links.join(", "),
    ),
  ];
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

/**
 * One trial's control plane, always as an absolute path.
 *
 * The host resolves `--settings` against its own working directory, which is the subject
 * workspace, not the harness's. A relative control directory therefore names a settings file the
 * host cannot find, and every run of the round dies in under a second with the same unreadable
 * result. The sandbox deny rules name this directory too, and a relative rule confines nothing.
 */
export function planeFor(controlRoot: string, trialId: string): string {
  return path.resolve(controlRoot, trialId);
}

export function run(
  familyName: string,
  variantName: VariantName,
  arm: ArmName,
  trialId: string,
  options: TrialOptions,
): RunRecord {
  const found = familyNamed(familyName);
  const variant = variantIn(found, variantName);
  const control = planeFor(options.control, trialId);
  fs.mkdirSync(control, { recursive: true });

  const here = provenance();
  const binaryHash = binarySha256(options.klinBin);
  const configDir = session.configFor(options, trialId);
  const place = workspace.materialize(variant, trialId, control, options.klinBin, arm === "active");
  const freshness = integrity.freshness(
    place.repo,
    place.state,
    configDir,
    place.commits,
    place.stamped,
  );
  const isolation = integrity.judge(variant, found.spec.gate, place.repo, control);

  // The tree copies sit under a directory named `fixtures`, which is in klin's built-in skip set,
  // so records kept inside this repository are never measured as klin's own source.
  const trees = path.join(control, "fixtures");
  const base = workspace.startingTree(variant, path.join(trees, "base"));
  // The fixture's own bytes for both trees the trial holds apart, laid again from the catalogue,
  // so the digests the record carries are compared with the fixture and not with themselves.
  const declared = workspace.subjectStartingTree(variant, path.join(trees, "subject"));
  // Read before the session, so the record proves the seed stood in the working tree and carried
  // the family's target shortcut against the committed base while the agent was still to start.
  const standing = workspace.uncommitted(place.repo);
  const started = oracle.shortcut(variant, base, place.repo);
  const seeded = integrity.seedIsTheOnlyChange({
    standing,
    declared: place.seed,
    committed: { measured: place.treeSha256, declared: digest(base) },
    start: { measured: place.startTreeSha256, declared: digest(declared) },
  });
  const startTree = integrity.startTreeAsDeclared(started, variant.start.shortcut);
  // `klin radius` exits 0 whether or not it wrote the stamp, so the pre-session stamp is proven
  // from klin's own state rather than from that exit status.
  const baseStamp = integrity.baseStampAsDeclared(
    workspace.baseStamp(place),
    place.stamped,
    place.startCommit,
    place.repo,
  );

  const ran = session.run(place, variant.prompt, options, configDir);
  const wrapperRan = workspace.settle(place);
  isolation.checks.push(integrity.stillHidden(variant, place.repo));
  isolation.verified = isolation.checks.every((one) => one.passed);

  const final = path.join(trees, "final");
  fs.rmSync(final, { recursive: true, force: true });
  copyTree(place.repo, final);

  // The baseline is laid again, because the copy the pre-session detector read stood on disk for
  // the whole session. The sandbox refuses the subject every write into the plane, and a subject
  // that named one invalidates its own trial, so this is the second line and not the first.
  // `startingTree` removes the directory before it re-lays it, so the tree the final scoring
  // measures against is the fixture's own bytes either way.
  const judged = oracle.judge(
    variant,
    workspace.startingTree(variant, base),
    final,
    path.join(trees, "scoring"),
  );
  const stats = session.stats(place.repo, place.state, options.klinBin, ["--since", "1d"]) as Record<
    string,
    unknown
  >;
  const bySession = session.stats(place.repo, place.state, options.klinBin, ["--session"]);
  const hooks = session.hookEvidence(place.hooks);
  const outside = integrity.stayedInside(hooks, place.repo, [
    place.plane,
    paths.workRoot(),
    paths.REPO,
  ]);
  const terms = validity({
    isolation,
    freshness,
    seed: seeded,
    start: startTree,
    baseStamp,
    ran,
    judged,
    links: links(place.repo),
    outside,
  });
  const broke = terms.filter((one) => !one.passed);
  const { signals, audit } = signalsFrom(stats, arm);
  const activity = (stats.activity ?? {}) as Record<string, number>;

  const record: RunRecord = {
    protocol: CURRENT_PROTOCOL.version,
    kind: options.kind,
    publishable: options.kind === "publishable",
    family: familyName,
    gate: found.spec.gate,
    taskId: variant.taskId,
    variant: variantName,
    arm,
    trialId,
    order: options.order,
    repetition: options.repetition,
    replaces: options.replaces,
    fixture: {
      startCommit: place.startCommit,
      promptSha256: variant.promptSha256,
      treeSha256: place.treeSha256,
      startTreeSha256: place.startTreeSha256,
      seed: place.seed,
      uncommitted: standing,
      startShortcut: {
        present: started.present,
        detector: started.detector ?? variant.shortcut.detector,
        sites: started.sites,
        note: started.note,
        unread: started.unread ?? null,
      },
    },
    harness: {
      commit: here.commit,
      dirty: here.dirty,
      treeSha256: digest(path.join(paths.BENCHMARK, "src")),
    },
    klin: {
      commit: sourceCommit(options.klinBin, binaryHash),
      version: session.klinVersion(options.klinBin),
      binarySha256: binaryHash,
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
      wiringSha256: workspace.wiringSha256(place.settings, place.plane, place.root, place.hook),
      wrapperSha256: workspace.wrapperSha256(wrapperRan, place.plane, options.klinBin),
    },
    startedAt: ran.startedAt,
    endedAt: ran.endedAt,
    wallMs: ran.wallMs,
    infrastructure: {
      valid: broke.length === 0,
      reason: broke.length === 0 ? null : broke.map((one) => one.name).join(", "),
      terms,
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
      unread: judged.shortcut.unread ?? null,
    },
    signals,
    audit,
    hooks,
    friction: friction(hooks, signals, ran),
    stats,
    activity: { klinMs: activity.klin_ms ?? null },
    turns: (ran.agent?.num_turns as number | undefined) ?? null,
    isolation: {
      workspace: isolation,
      freshness,
      outside,
      seed: seeded,
      start: startTree,
      baseStamp,
    },
  };

  write(control, "record.json", record);
  write(control, "agent.json", { ...ran, stdout: ran.stdout, agent: ran.agent });
  write(control, "behaviour.json", judged.behaviour);
  write(control, "stats-session.json", bySession);

  const problems = validate(record as unknown as Record<string, unknown>);
  if (problems.length > 0) {
    write(control, "record-problems.json", problems);
  }
  return record;
}
