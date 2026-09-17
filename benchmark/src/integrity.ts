import fs from "node:fs";
import path from "node:path";
import { files, read } from "./trees.ts";
import type { Variant } from "./catalogue.ts";

/**
 * Subject-workspace isolation.
 *
 * The benchmark control plane must not be visible to the coding agent. These checks demonstrate
 * that for one trial: the workspace holds no hidden oracle, no arm assignment, no scoring
 * metadata and no expected shortcut, and no path leads from the workspace into the control
 * plane.
 */

export interface Check {
  name: string;
  passed: boolean;
  detail: string;
}

export interface Isolation {
  verified: boolean;
  checks: Check[];
}

/**
 * The overlay whose files must never reach a subject workspace.
 *
 * `good/` and `bad/` are self-test scaffolding. The harness never copies them into a workspace,
 * and their relative paths are the fixture's own, so they are not a leak to look for here.
 */
const HIDDEN = ["oracle"];
const TEXT = [".rs", ".ts", ".tsx", ".js", ".mjs", ".json", ".md", ".toml", ".lock", ".txt", ".sh"];

function check(name: string, passed: boolean, detail: string): Check {
  return { name, passed, detail };
}

function inside(outer: string, inner: string): boolean {
  const relative = path.relative(path.resolve(outer), path.resolve(inner));
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

/**
 * Every word a subject workspace must not carry, as a whole word, in a path or in a file.
 *
 * The list names the family, the gate it targets and the detector that scores it, so a fixture
 * can never name its own expected failure mode. It also names the control plane's own words. The
 * arm is not on the list because the harness never writes the arm into a workspace: the
 * `control-plane-outside-workspace` check is what holds that.
 */
export function forbidden(variant: Variant, gate: string, control: string): string[] {
  return [
    ...new Set([
      variant.family,
      gate,
      gate.replace(/-/g, "_"),
      gate.replace(/_/g, "-"),
      variant.shortcut.detector,
      "calibration",
      "oracle",
      "shortcut",
      "KLIN_BENCH_DELIVER",
      control,
    ]),
  ];
}

function escapeWord(word: string): string {
  return word.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function holds(text: string, words: string[]): boolean {
  return words.some((word) =>
    new RegExp("(^|[^A-Za-z0-9_])" + escapeWord(word) + "([^A-Za-z0-9_]|$)", "i").test(text),
  );
}

function hiddenPaths(variant: Variant): string[] {
  const held: string[] = ["prompt.md"];
  for (const name of HIDDEN) {
    const root = path.join(variant.root, name);
    if (fs.existsSync(root)) {
      held.push(...files(root));
    }
  }
  return [...new Set(held)];
}

function pathsHolding(workspace: string, words: string[]): string[] {
  return files(workspace).filter((relative) => holds(relative, words));
}

/**
 * klin's own configuration is exempt.
 *
 * A klin configuration names the gates it pins, and the ticket allows the agent to know that the
 * repository carries one. Every fixture ships the same configuration, so it tells nothing about
 * which gate a task targets. `sameConfiguration` below is what holds that.
 */
const EXEMPT = new Set(["klin.json"]);

function contentsHolding(workspace: string, words: string[]): string[] {
  return files(workspace).filter(
    (relative) =>
      !EXEMPT.has(relative) &&
      TEXT.includes(path.extname(relative)) &&
      holds(read(workspace, relative), words),
  );
}

/**
 * Judge one workspace against its control directory, as the harness materialized it.
 *
 * `control` is where the harness keeps the arm, the run record, the hook evidence and the
 * scoring copies.
 *
 * This runs before the session, because the question is what the harness put in front of the
 * agent. Judging the final tree instead would fail a run whose agent happened to write the gate's
 * ordinary English name in a comment, and that agent read nothing it should not have.
 * `stillHidden` is the narrower check for afterwards.
 */
export function judge(variant: Variant, gate: string, workspace: string, control: string): Isolation {
  const words = forbidden(variant, gate, control);
  const hidden = hiddenPaths(variant).filter((relative) =>
    fs.existsSync(path.join(workspace, relative)),
  );
  const leakingPaths = pathsHolding(workspace, words);
  const leakingContents = contentsHolding(workspace, words);
  const checks: Check[] = [
    check(
      "control-plane-outside-workspace",
      !inside(workspace, control) && !inside(control, workspace),
      "workspace " + workspace + ", control " + control,
    ),
    check(
      "no-hidden-oracle-in-workspace",
      hidden.length === 0,
      hidden.length === 0 ? "no hidden file is present" : hidden.join(", "),
    ),
    check(
      "no-metadata-in-paths",
      leakingPaths.length === 0,
      leakingPaths.length === 0 ? "no path names the family, gate, arm or variant" : leakingPaths.join(", "),
    ),
    check(
      "no-metadata-in-contents",
      leakingContents.length === 0,
      leakingContents.length === 0 ? "no file names the family, gate, arm or variant" : leakingContents.join(", "),
    ),
  ];
  return { verified: checks.every((one) => one.passed), checks };
}

/**
 * Freshness of what a trial must not share with another trial.
 *
 * `config` is the host configuration directory this trial will run under, or the empty string
 * where the trial runs under the operator's own. An empty string is reported as a fact rather
 * than a failure: the host keys its credential by the configuration directory, so a per-trial
 * directory needs a credential of its own. Every record keeps the memory digest either way, so a
 * reader can tell whether the two arms saw one configuration.
 */
export function freshness(
  repo: string,
  state: string,
  config: string,
  commits: number,
): Isolation {
  const isolated = config !== "";
  const checks: Check[] = [
    check("fresh-repository", commits === 1, String(commits) + " commit(s) before the session"),
    check(
      "fresh-klin-state",
      !fs.existsSync(state) || fs.readdirSync(state).length === 0,
      "klin state at " + state,
    ),
    check(
      "fresh-host-configuration",
      !isolated || !fs.existsSync(config) || fs.readdirSync(config).length === 0,
      isolated
        ? "the trial has its own host configuration at " + config
        : "the trial shares the operator's host configuration, and the record keeps its memory digest",
    ),
    check("workspace-is-its-own-repository", fs.existsSync(path.join(repo, ".git")), repo),
  ];
  return { verified: checks.every((one) => one.passed), checks };
}

/**
 * Every family ships the same klin configuration.
 *
 * A configuration that pinned one gate in one family and not in another would tell the agent
 * which gate its task targets, through the one file the agent is allowed to see.
 */
export function sameConfiguration(configurations: Map<string, string>): Check {
  const distinct = new Set(configurations.values());
  const named = [...configurations]
    .filter(([, text]) => text !== [...distinct][0])
    .map(([family]) => family);
  return check(
    "one configuration across every family",
    distinct.size === 1,
    distinct.size === 1 ? "every family ships one klin.json" : named.join(", ") + " differ",
  );
}

/**
 * The narrow check for after the session: no hidden oracle file reached the workspace.
 *
 * An agent may write anything it likes into its own tree, including the gate's ordinary English
 * name. What it may never hold is one of the oracle's files or the task prompt, because those
 * would have had to come from outside the workspace.
 */
export function stillHidden(variant: Variant, workspace: string): Check {
  const found = hiddenPaths(variant).filter((relative) =>
    fs.existsSync(path.join(workspace, relative)),
  );
  return check(
    "no-hidden-oracle-after-the-session",
    found.length === 0,
    found.length === 0 ? "no hidden file reached the workspace" : found.join(", "),
  );
}
