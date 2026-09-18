import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "./paths.ts";
import { copyTree, files, overlay } from "./trees.ts";
import { families, VARIANTS, type Family, type Variant } from "./catalogue.ts";
import * as oracle from "./oracle.ts";
import * as workspace from "./workspace.ts";
import * as integrity from "./integrity.ts";

/**
 * The deterministic fixture and oracle self-tests.
 *
 * Nothing here starts an agent, reaches the network or reads a calibration record. Every case is
 * decided by the fixture trees on disk, so a fixture or oracle defect is found before any live
 * trial is paid for.
 */

export interface Case {
  family: string;
  variant: string;
  name: string;
  passed: boolean;
  detail: string;
}

function held(family: Family, variant: Variant): string[] {
  const words = [
    "klin",
    family.name,
    family.spec.gate,
    family.spec.gate.replace("-", "_"),
    family.spec.gate.replace("_", "-"),
    "shortcut",
    "benchmark",
    "oracle",
    "shadow",
    "calibration",
    variant.shortcut.detector,
    ...((family.spec as unknown as { forbidden?: string[] }).forbidden ?? []),
  ];
  const prompt = variant.prompt.toLowerCase();
  return [...new Set(words)].filter((word) => prompt.includes(word.toLowerCase()));
}

function scratch(name: string): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-" + name + "-"));
}

function overlaid(variant: Variant, starting: string, which: string, into: string): string {
  fs.rmSync(into, { recursive: true, force: true });
  copyTree(starting, into);
  overlay(path.join(variant.root, which), into);
  return into;
}

function judge(name: string, passed: boolean, detail: string, family: Family, variant: Variant): Case {
  return { family: family.name, variant: variant.name, name, passed, detail };
}

const KLIN = process.env.KLIN_BIN ?? path.join(paths.REPO, "target", "release", "klin");

/**
 * The gates the production Stop hook names as failing over a tree, through the real binary.
 *
 * This runs `klin gate --hook --changed`, the command the hook runs, over a repository whose
 * base is the starting tree and whose working tree is the known-bad one. It is the only way to
 * know whether a family's target gate reaches an agent at the end of a turn.
 */
function gatesTheHookNames(starting: string, bad: string, room: string): string[] {
  const repo = path.join(room, "hooked");
  fs.rmSync(repo, { recursive: true, force: true });
  copyTree(starting, repo);
  workspace.git(repo, "init", "--quiet");
  workspace.git(repo, "add", "-A");
  workspace.git(repo, "commit", "--quiet", "-m", "the starting tree");
  for (const relative of files(repo)) {
    fs.rmSync(path.join(repo, relative));
  }
  copyTree(bad, repo);
  const state = path.join(room, "hooked-state");
  const payload = JSON.stringify({ hook_event_name: "Stop", session_id: "selftest" });
  const ran = spawnSync(KLIN, ["gate", "--hook", "--changed"], {
    cwd: repo,
    input: payload,
    encoding: "utf8",
    timeout: 300_000,
    env: { ...process.env, KLIN_STATE_DIR: state },
  });
  return ((ran.stdout ?? "") + (ran.stderr ?? ""))
    .split("\n")
    .filter((line) => /^\s{2}(FAIL|ERR)\s/.test(line))
    .map((line) => line.trim().split(/\s+/)[1]);
}

function casesFor(family: Family, variant: Variant, room: string): Case[] {
  const starting = workspace.startingTree(variant, path.join(room, "start"));
  const words = held(family, variant);
  const cases: Case[] = [
    judge(
      "the prompt names no gate, arm or expected failure",
      words.length === 0,
      words.length === 0 ? "the prompt is clean" : "the prompt names " + words.join(", "),
      family,
      variant,
    ),
  ];

  const clean = oracle.shortcut(variant, starting, starting);
  cases.push(
    judge(
      "the starting tree carries no target shortcut",
      clean.present === false,
      JSON.stringify(clean.sites) + " " + clean.note,
      family,
      variant,
    ),
  );

  const good = overlaid(variant, starting, "good", path.join(room, "good"));
  const goodBehaviour = oracle.behaviour(variant, good, path.join(room, "good-scored"));
  const goodShortcut = oracle.shortcut(variant, starting, good);
  cases.push(
    judge(
      "the oracle passes a known-good tree",
      goodBehaviour.passed,
      goodBehaviour.reason || (goodBehaviour.stderr || goodBehaviour.stdout).slice(-600),
      family,
      variant,
    ),
    judge(
      "a known-good tree carries no target shortcut",
      goodShortcut.present === false,
      JSON.stringify(goodShortcut.sites) + " " + goodShortcut.note,
      family,
      variant,
    ),
  );

  const bad = overlaid(variant, starting, "bad", path.join(room, "bad"));
  const badBehaviour = oracle.behaviour(variant, bad, path.join(room, "bad-scored"));
  const badShortcut = oracle.shortcut(variant, starting, bad);
  cases.push(
    judge(
      "the oracle fails a known-bad tree",
      !badBehaviour.passed,
      "exit " + String(badBehaviour.exit),
      family,
      variant,
    ),
    judge(
      "the detector finds the target shortcut in a known-bad tree",
      badShortcut.present === true,
      JSON.stringify(badShortcut.sites) + " " + badShortcut.note,
      family,
      variant,
    ),
  );

  if (fs.existsSync(KLIN)) {
    const named = gatesTheHookNames(starting, bad, room);
    const fired = named.includes(family.spec.gate);
    cases.push(
      judge(
        "the production hook " + (variant.hookFires ? "flags" : "stays silent on") + " the known-bad tree",
        fired === variant.hookFires,
        "the hook named [" + named.join(", ") + "], and the fixture records " + String(variant.hookFires),
        family,
        variant,
      ),
    );
  }

  const isolation = integrity.judge(variant, family.spec.gate, starting, path.join(room, "control"));
  for (const one of isolation.checks) {
    cases.push(judge("workspace isolation: " + one.name, one.passed, one.detail, family, variant));
  }
  return cases;
}

function configurations(found: Record<string, Family>): Map<string, string> {
  const held = new Map<string, string>();
  for (const [name, family] of Object.entries(found)) {
    const file = path.join(family.root, "base", "klin.json");
    held.set(name, fs.existsSync(file) ? fs.readFileSync(file, "utf8") : "");
  }
  return held;
}

export function run(only: string[]): Case[] {
  const found = families();
  const chosen = only.length > 0 ? only : Object.keys(found).sort();
  const shared = integrity.sameConfiguration(configurations(found));
  const cases: Case[] = [
    { family: "every", variant: "-", name: shared.name, passed: shared.passed, detail: shared.detail },
  ];
  for (const name of chosen) {
    const family = found[name];
    if (!family) {
      throw new Error("no fixture family named " + name);
    }
    for (const variantName of VARIANTS) {
      const room = scratch(name + "-" + variantName);
      try {
        cases.push(...casesFor(family, family.variants[variantName], room));
      } finally {
        fs.rmSync(room, { recursive: true, force: true });
      }
    }
  }
  return cases;
}
