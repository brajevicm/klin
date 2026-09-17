import fs from "node:fs";
import path from "node:path";
import * as paths from "./paths.ts";
import { cells, families, type ArmName, type VariantName } from "./catalogue.ts";
import { sha256 } from "./trees.ts";
import * as session from "./session.ts";
import * as trial from "./trial.ts";
import type { Check, RunRecord } from "./record.ts";
import { validate } from "./record.ts";

/**
 * Calibration: one live trial per family, variant and arm.
 *
 * Calibration proves the apparatus, not the product. Every record it writes states
 * `publishable: false`, and `verify` refuses a set that says otherwise.
 */

export function stamp(): string {
  return new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);
}

/** A small seeded generator, so one seed always gives one scheduled order. */
function ordering(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let drawn = Math.imul(state ^ (state >>> 15), 1 | state);
    drawn = (drawn + Math.imul(drawn ^ (drawn >>> 7), 61 | drawn)) ^ drawn;
    return ((drawn ^ (drawn >>> 14)) >>> 0) / 4294967296;
  };
}

function shuffled<T>(items: T[], seed: number): T[] {
  const draw = ordering(seed);
  const held = [...items];
  for (let index = held.length - 1; index > 0; index -= 1) {
    const swap = Math.floor(draw() * (index + 1));
    [held[index], held[swap]] = [held[swap], held[index]];
  }
  return held;
}

function trialId(family: string, variant: string, arm: string, order: number): string {
  return sha256([family, variant, arm, order].join(":")).slice(0, 12);
}

function options(into: string, order: number): trial.TrialOptions {
  return { ...session.defaults(), order, kind: "calibration", control: into };
}

/** One ad-hoc trial. Its id carries the clock, so a second run beside the first keeps both. */
export function one(family: string, variant: string, arm: string, into: string): number {
  const blocked = preflight(session.defaults().klinBin);
  if (blocked !== "") {
    process.stdout.write(blocked + "\n");
    return 2;
  }
  const id = trialId(family, variant, arm, Date.now());
  const record = trial.run(
    family,
    variant as VariantName,
    arm as ArmName,
    id,
    options(into, 0),
  );
  process.stdout.write(
    [
      record.family,
      record.variant,
      record.arm,
      record.result.outcome,
      "oracle " + (record.oracle.behaviourPassed ? "pass" : "fail"),
      "shortcut " + String(record.shortcut.present),
      String(record.signals.length) + " signals",
      String(record.audit.length) + " audit rows",
    ].join("  ") + "\n",
  );
  return record.infrastructure.valid ? 0 : 1;
}

/**
 * Refuse to start a paid run without a klin binary that answers.
 *
 * Without this the hook exits 127 on every event, every trial runs with klin effectively absent,
 * and `verify` only says so after the whole set has been paid for.
 *
 * A binary with no build provenance beside it is a warning and not a refusal. It runs, and every
 * record of the set states no klin source commit. `preflight` cannot write that file itself: it
 * sees the binary and it can ask git for HEAD, and it cannot know that the one built the other.
 */
export function preflight(binary: string): string {
  if (!fs.existsSync(binary)) {
    return "no klin binary at " + binary + ". Build it with: benchmark/build-klin";
  }
  const named = session.klinVersion(binary);
  return named === "" ? "the binary at " + binary + " did not answer --version" : "";
}

export interface CalibrateOptions {
  into: string;
  only: string[];
  seed: number;
}

export function all(chosen: CalibrateOptions): number {
  const known = session.defaults();
  const blocked = preflight(known.klinBin);
  if (blocked !== "") {
    process.stdout.write(blocked + "\n");
    return 2;
  }
  if (!fs.existsSync(known.klinBin + ".provenance")) {
    process.stdout.write(
      "no build provenance beside " +
        known.klinBin +
        ". Every record will state no klin source commit. benchmark/build-klin writes one.\n",
    );
  }
  const wanted = cells().filter(
    (cell) => chosen.only.length === 0 || chosen.only.includes(cell.family),
  );
  const order = shuffled(wanted, chosen.seed);
  fs.mkdirSync(chosen.into, { recursive: true });
  fs.writeFileSync(
    path.join(chosen.into, "manifest.json"),
    JSON.stringify(
      {
        protocol: paths.PROTOCOL,
        kind: "calibration",
        publishable: false,
        seed: chosen.seed,
        startedAt: new Date().toISOString(),
        model: known.model,
        klinVersion: session.klinVersion(known.klinBin),
        hostVersion: session.hostVersion(),
        order: order.map((cell, index) => ({
          ...cell,
          order: index,
          trialId: trialId(cell.family, cell.variant, cell.arm, index),
        })),
      },
      null,
      2,
    ) + "\n",
  );

  let failed = 0;
  order.forEach((cell, index) => {
    const id = trialId(cell.family, cell.variant, cell.arm, index);
    process.stdout.write(
      String(index + 1) + "/" + String(order.length) + " " + cell.family + " " + cell.variant + " " + cell.arm + "\n",
    );
    try {
      const record = trial.run(cell.family, cell.variant, cell.arm, id, options(chosen.into, index));
      if (!record.infrastructure.valid) {
        failed += 1;
      }
    } catch (why) {
      failed += 1;
      fs.writeFileSync(
        path.join(chosen.into, id + "-failed.json"),
        JSON.stringify({ ...cell, order: index, error: String(why) }, null, 2) + "\n",
      );
    }
  });
  process.stdout.write("\nrecords under " + chosen.into + "\n");
  return failed === 0 ? 0 : 1;
}

/**
 * The host flags less the two that differ between any two trials by construction.
 *
 * `--session-id` is a fresh UUID per trial and `--settings` names the plane, whose path carries
 * the trial id. Everything else the host was told is a frozen variable, so it is compared.
 */
export function normalizedFlags(flags: string[]): string[] {
  const kept: string[] = [];
  for (let at = 0; at < flags.length; at += 1) {
    if (flags[at] === "--session-id" || flags[at] === "--settings") {
      at += 1;
      continue;
    }
    kept.push(flags[at]);
  }
  return kept;
}

/**
 * Everything two arms of one cell must hold alike, and what to call each one.
 *
 * A cell is one family and one variant, run twice. The arm is the only thing that may differ. A
 * pair that differs in any of these measured a second difference beside the treatment, and #115
 * could not tell the two apart afterwards.
 *
 * `model.reported` is not here. The host names its housekeeping model beside the session's, and a
 * run that needed no housekeeping names fewer for a legitimate reason, so the difference is
 * reported and the cell is not failed for it.
 */
const FROZEN: [string, (one: RunRecord) => string][] = [
  ["the klin binary", (one) => one.klin.binarySha256],
  ["the klin version", (one) => one.klin.version],
  ["the klin source commit", (one) => one.klin.commit],
  [
    "the harness",
    (one) => [one.harness.commit, one.harness.treeSha256, String(one.harness.dirty)].join(" "),
  ],
  ["the host version", (one) => one.host.version],
  ["the requested model", (one) => one.model.requested],
  ["the host flags", (one) => normalizedFlags(one.host.flags).join(" ")],
  ["the hook wiring", (one) => one.agent.wiringSha256],
  ["the hook wrapper", (one) => one.agent.wrapperSha256],
  ["the isolated-configuration status", (one) => String(one.host.isolatedConfiguration)],
  ["the user memory", (one) => one.host.memory?.sha256 ?? "none"],
];

export function records(directory: string): RunRecord[] {
  if (!fs.existsSync(directory)) {
    return [];
  }
  const held: RunRecord[] = [];
  for (const name of fs.readdirSync(directory).sort()) {
    const file = path.join(directory, name, "record.json");
    if (fs.existsSync(file)) {
      held.push(JSON.parse(fs.readFileSync(file, "utf8")) as RunRecord);
    }
  }
  return held;
}

/** Every way a calibration set fails what the protocol requires of it. */
export function verify(directory: string): string[] {
  const held = records(directory);
  const problems: string[] = [];
  if (held.length === 0) {
    return ["no record was found under " + directory];
  }
  for (const record of held) {
    const where = record.family + "/" + record.variant + "/" + record.arm;
    for (const problem of validate(record as unknown as Record<string, unknown>)) {
      problems.push(where + ": " + problem);
    }
    if (record.publishable) {
      problems.push(where + ": a calibration record claims to be publishable");
    }
    // Isolation and freshness are two of the terms, so neither needs a check of its own here.
    for (const term of (record.infrastructure.terms ?? []).filter((one) => !one.passed)) {
      problems.push(where + ": " + term.name + " failed, " + term.detail);
    }
    const delivered = record.hooks.filter((hook) => hook.delivered);
    if (record.arm === "shadow" && delivered.length > 0) {
      problems.push(where + ": the shadow arm delivered " + String(delivered.length) + " hook answers");
    }
    if (record.arm === "active" && record.hooks.length > 0 && delivered.length !== record.hooks.length) {
      problems.push(where + ": the active arm suppressed a hook answer");
    }
    const blocked = record.hooks.filter(
      (hook) => hook.arguments.startsWith("gate") && hook.status === 2,
    );
    for (const hook of blocked) {
      if (hook.stderr.trim().length === 0) {
        problems.push(where + ": a blocked stop recorded no report to audit");
      }
    }
    if ((record.stats as { error?: string }).error) {
      problems.push(where + ": klin stats --json could not be read");
    }
    // The run stays valid: an agent may rename or move what the family measures. A person still
    // has to see that this trial carries no shortcut answer.
    if (record.shortcut.present === null) {
      problems.push(where + ": the detector answered nothing, " + record.shortcut.note);
    }
    // The same fact is a validity term, so an invalid run above already names it. This reads the
    // record's own field, so a record whose terms and whose isolation disagree is named too.
    const outside = record.isolation.outside as Check | undefined;
    if (outside?.passed === false) {
      problems.push(where + ": the subject named a path outside its workspace, " + outside.detail);
    }
    if (record.klin.commit === "") {
      problems.push(where + ": no build provenance ties " + record.klin.binarySha256.slice(0, 12) + " to a source commit");
    }
    for (const signal of record.signals) {
      if (signal.auditKind === "asked-once" && signal.kind !== "audit") {
        problems.push(where + ": a deleted-test question was counted as a regression");
      }
    }
    if (record.oracle.reason.includes("klin")) {
      problems.push(where + ": the oracle named klin");
    }
    // Under the trial's confinement a denial has two readings, and the record cannot tell them
    // apart: the sandbox refused a subject that went looking, or it refused a call the task
    // needed. The first is a fact about the subject and the second changes what was measured, so
    // a person reads `hooks` and `isolation.outside` and decides.
    if (record.friction.hostDenials > 0) {
      problems.push(
        where +
          ": the host refused " +
          String(record.friction.hostDenials) +
          " tool call(s) of its own, so either the subject went looking or the trial did not run the task the fixture states",
      );
    }
  }
  const byCell = new Map<string, RunRecord[]>();
  for (const record of held) {
    const key = record.family + "/" + record.variant;
    byCell.set(key, [...(byCell.get(key) ?? []), record]);
  }
  for (const [key, group] of byCell) {
    const trees = new Set(group.map((one) => one.fixture.treeSha256));
    const prompts = new Set(group.map((one) => one.fixture.promptSha256));
    if (trees.size > 1) {
      problems.push(key + ": the arms did not start from one tree");
    }
    if (prompts.size > 1) {
      problems.push(key + ": the arms did not run one prompt");
    }
    for (const [what, read] of FROZEN) {
      const held = new Set(group.map(read));
      if (held.size > 1) {
        problems.push(key + ": the arms did not share " + what + ", " + [...held].join(" against "));
      }
    }
  }
  const states = new Set(held.map((one) => one.trialId));
  if (states.size !== held.length) {
    problems.push("two trials shared one trial id");
  }
  // A set run with `--only` is smaller than the protocol on purpose, and its manifest says how
  // much smaller. Without a manifest the whole protocol is what a set is judged against.
  const manifest = path.join(directory, "manifest.json");
  const stated = fs.existsSync(manifest)
    ? (JSON.parse(fs.readFileSync(manifest, "utf8")) as { order?: unknown[] })
    : {};
  const expected = stated.order?.length ?? Object.keys(families()).length * 4;
  if (held.length !== expected) {
    problems.push(
      "the set holds " + String(held.length) + " records where it should hold " + String(expected),
    );
  }
  return problems;
}
