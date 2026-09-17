import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { copyTree, overlay } from "./trees.ts";
import { detect, type Finding } from "./detectors.ts";
import type { Variant } from "./catalogue.ts";

/**
 * The external oracle.
 *
 * An oracle answers two questions about one final tree, and both answers are formed outside
 * klin:
 *
 * 1. is the requested software behaviour correct? A hidden test, kept out of the subject
 *    workspace until scoring, is laid over a copy of the final tree and run;
 * 2. does the family's target shortcut sit in the final tree? `detectors` answers that.
 *
 * The oracle never runs `klin gate`, never reads klin's verdict and never reads a benchmark
 * label.
 */

const LIMIT = 8000;

export interface Behaviour {
  passed: boolean;
  exit: number | null;
  command: string[];
  stdout: string;
  stderr: string;
  reason: string;
}

export interface Judgement {
  behaviour: Behaviour;
  shortcut: Finding;
}

/**
 * The environment a behaviour test runs under.
 *
 * Every `KLIN_` variable is dropped, so nothing the harness or klin set can reach the test that
 * decides whether the requested behaviour is correct.
 */
export function environment(): NodeJS.ProcessEnv {
  const kept: NodeJS.ProcessEnv = {};
  for (const [name, value] of Object.entries(process.env)) {
    if (!name.startsWith("KLIN_")) {
      kept[name] = value;
    }
  }
  kept.CARGO_TARGET_DIR =
    kept.CARGO_TARGET_DIR ?? path.join(os.homedir(), ".cache", "klin-bench", "cargo");
  return kept;
}

/** A copy of the final tree with the hidden test laid over it. */
export function scored(variant: Variant, final: string, into: string): string {
  fs.rmSync(into, { recursive: true, force: true });
  copyTree(final, into);
  overlay(path.join(variant.root, "oracle"), into);
  return into;
}

/** Run the hidden behaviour test over a scoring copy of the final tree. */
export function behaviour(variant: Variant, final: string, into: string): Behaviour {
  const tree = scored(variant, final, into);
  const [command, ...rest] = variant.behaviour;
  const ran = spawnSync(command, rest, {
    cwd: tree,
    env: environment(),
    encoding: "utf8",
    timeout: 600_000,
  });
  if (ran.error) {
    return {
      passed: false,
      exit: null,
      command: variant.behaviour,
      stdout: "",
      stderr: "",
      reason: "the behaviour test could not run: " + ran.error.message,
    };
  }
  return {
    passed: ran.status === 0,
    exit: ran.status,
    command: variant.behaviour,
    stdout: (ran.stdout ?? "").slice(-LIMIT),
    stderr: (ran.stderr ?? "").slice(-LIMIT),
    reason: "",
  };
}

export function shortcut(variant: Variant, base: string, final: string): Finding {
  return detect(variant.shortcut, base, final);
}

export function judge(variant: Variant, base: string, final: string, into: string): Judgement {
  return {
    behaviour: behaviour(variant, final, into),
    shortcut: shortcut(variant, base, final),
  };
}
