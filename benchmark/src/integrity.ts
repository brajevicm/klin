import fs from "node:fs";
import path from "node:path";
import { files, read } from "./trees.ts";
import type { Variant } from "./catalogue.ts";
import type { Check, HookInvocation, Isolation } from "./record.ts";
import { signsNothing } from "./workspace.ts";

/**
 * Subject-workspace isolation.
 *
 * The benchmark control plane must not be visible to the coding agent. These checks demonstrate
 * that for one trial: the workspace holds no hidden oracle, no arm assignment, no scoring
 * metadata and no expected shortcut, and no path leads from the workspace into the control
 * plane.
 */

/**
 * The overlay whose files must never reach a subject workspace.
 *
 * A variant's exemplar trees are self-test scaffolding. The harness never copies them into a
 * workspace, and their relative paths are the fixture's own, so they are not a leak to look for
 * here.
 */
const HIDDEN = ["oracle"];
const TEXT = [".rs", ".ts", ".tsx", ".js", ".mjs", ".json", ".md", ".toml", ".lock", ".txt", ".sh"];

function check(name: string, passed: boolean, detail: string): Check {
  return { name, passed, detail };
}

/** Whether two paths name one place, through their symbolic links. */
function samePath(one: string, other: string): boolean {
  return real(one) === real(other);
}

export function inside(outer: string, inner: string): boolean {
  const relative = path.relative(real(outer), real(inner));
  const escaped = relative === ".." || relative.startsWith(".." + path.sep);
  return relative === "" || (!escaped && !path.isAbsolute(relative));
}

/**
 * Every word a subject workspace must not carry, as a whole word, in a path or in a file.
 *
 * The list names the family, the gate it targets and the detector that scores it, so a fixture
 * can never name its own expected failure mode. It also names the control plane's own words.
 *
 * The arm is not on the list, because the arm is no longer a word anywhere a workspace could
 * carry it. It reaches the wrapper as an argument of the hook command in the plane's settings
 * file, and `control-plane-outside-workspace` is what holds the plane out of reach.
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
 *
 * `stamped` says the harness took one stamp over the committed base before laying a seed over it.
 * A seeded trial's state therefore holds exactly that one worktree entry and no journal, and a
 * natural trial's state holds nothing at all. Either way the state is this trial's own: the
 * harness removes it before it materializes anything.
 */
export function freshness(
  repo: string,
  state: string,
  config: string,
  commits: number,
  stamped = false,
): Isolation {
  const isolated = config !== "";
  const entries = fs.existsSync(state) ? fs.readdirSync(state) : [];
  const unsigned = signsNothing(repo);
  const checks: Check[] = [
    check("fresh-repository", commits === 1, String(commits) + " commit(s) before the session"),
    check(
      "fresh-klin-state",
      entries.length === (stamped ? 1 : 0),
      (stamped
        ? "klin state holds this trial's own stamp over the committed base, " +
          String(entries.length) +
          " worktree entry(s), at "
        : "klin state at ") + state,
    ),
    check(
      "fresh-host-configuration",
      !isolated || !fs.existsSync(config) || fs.readdirSync(config).length === 0,
      isolated
        ? "the trial has its own host configuration at " + config
        : "the trial shares the operator's host configuration, and the record keeps its memory digest",
    ),
    check("workspace-is-its-own-repository", fs.existsSync(path.join(repo, ".git")), repo),
    check(
      "commits-unsigned",
      unsigned,
      unsigned
        ? "the repository's own configuration sets commit.gpgsign to false, over the operator's global one"
        : "the subject's git reads commit.gpgsign as other than false, so its commit reaches a keyring the sandbox refuses",
    ),
  ];
  return { verified: checks.every((one) => one.passed), checks };
}

/**
 * What stood uncommitted in the repository before the session, against what the variant declared.
 *
 * The harness equated three trees until #260: the committed tree, the subject's starting tree and
 * the detector's baseline. A seeded variant separates the first two by exactly its declared seed,
 * and this is what proves the separation is exactly that and nothing else. A natural variant
 * declares no seed, so its working tree has to stand clean.
 */
export function seedIsTheOnlyChange(read: {
  standing: string[];
  declared: string[];
  committed: { measured: string; declared: string };
  start: { measured: string; declared: string };
}): Check {
  const want = [...read.declared].sort();
  const held = [...read.standing].sort();
  const named = (list: string[]): string => (list.length === 0 ? "nothing" : list.join(", "));
  const broke: string[] = [];
  if (want.length !== held.length || want.some((one, at) => one !== held[at])) {
    broke.push("the working tree held " + named(held) + " where the variant declares " + named(want));
  }
  // The path set alone says only which files changed. These two say the bytes are the fixture's
  // own, so a seed that wrote the right path with the wrong content cannot pass.
  if (read.committed.measured !== read.committed.declared) {
    broke.push(
      "the committed tree digests " +
        read.committed.measured.slice(0, 12) +
        " where the fixture's own base gives " +
        read.committed.declared.slice(0, 12),
    );
  }
  if (read.start.measured !== read.start.declared) {
    broke.push(
      "the subject's starting tree digests " +
        read.start.measured.slice(0, 12) +
        " where the fixture's own base under its declared seed gives " +
        read.start.declared.slice(0, 12),
    );
  }
  return check(
    "seed-as-declared",
    broke.length === 0,
    broke.length > 0
      ? broke.join("; ")
      : want.length === 0
        ? "the working tree stood clean before the session, at the fixture's own bytes"
        : "the declared seed " +
          named(want) +
          " was the only uncommitted change, at the fixture's own bytes",
  );
}

/**
 * Whether the stamp klin measures a seeded trial's turn against was really taken, and taken over
 * the committed clean base.
 *
 * `klin radius` exits 0 whether or not it wrote a stamp: `turn::run` returns `Ok(0)` on every
 * path, and the write that persists the stamp returns a boolean the caller discards. So the exit
 * status proves nothing, and a seeded trial whose stamp did not land would let its own session
 * start photograph the seed as prior work while the harness still called the trial valid.
 *
 * The verdict is the condition that decides whether the stamp survives. A stamp moves on a first
 * session or when the last stop ended green (SPEC 6.2), so only a stamp this reading finds `red`
 * is one the subject's own session start will keep.
 *
 * A trial that took no stamp is held to the opposite: klin's state must hold nothing at all.
 */
export function baseStampAsDeclared(
  read: {
    worktrees: string[];
    entries: string[];
    turn: { commit?: string; parent?: string; verdict?: string } | null;
    ref: string;
    repository: string;
  },
  stamped: boolean,
  base: string,
  repo: string,
): Check {
  const name = "base-stamp-as-declared";
  if (!stamped) {
    return check(
      name,
      read.worktrees.length === 0,
      read.worktrees.length === 0
        ? "the trial took no pre-session stamp and klin's state held nothing"
        : "the trial took no pre-session stamp and klin's state holds " + read.worktrees.join(", "),
    );
  }
  const broke: string[] = [];
  if (read.worktrees.length !== 1) {
    broke.push("klin's state holds " + String(read.worktrees.length) + " worktree entries, not one");
  }
  for (const wanted of ["turn", "index", "repository"]) {
    if (!read.entries.includes(wanted)) {
      broke.push("the state entry holds no " + wanted);
    }
  }
  if (read.entries.includes("journal.jsonl")) {
    broke.push("the state entry already holds a journal, so something ran before the session");
  }
  if (read.repository !== "" && !samePath(read.repository, repo)) {
    broke.push("the state entry names the repository " + read.repository + ", not " + repo);
  }
  if (read.turn === null) {
    broke.push("the turn stamp is missing or does not parse, so klin wrote none");
  } else {
    if (read.turn.parent !== base) {
      broke.push(
        "the stamp names the parent " +
          String(read.turn.parent) +
          " where the committed base is " +
          base,
      );
    }
    if (read.turn.verdict !== "red") {
      broke.push(
        "the stamp reads " +
          String(read.turn.verdict) +
          ", and only a red stamp survives the subject's own session start",
      );
    }
    if (read.turn.commit === undefined || read.turn.commit === "") {
      broke.push("the stamp names no commit of its own");
    } else if (read.ref !== read.turn.commit) {
      broke.push(
        "refs/worktree/klin/turn resolves to " +
          (read.ref === "" ? "nothing" : read.ref) +
          " where the stamp names " +
          read.turn.commit,
      );
    }
  }
  return check(
    name,
    broke.length === 0,
    broke.length === 0
      ? "a red stamp over the committed base " +
        base.slice(0, 12) +
        " stood in klin's state, with its ref resolved and no journal beside it"
      : broke.join("; "),
  );
}

/**
 * Whether the tree the subject started from carries what the variant declared.
 *
 * A natural variant declares a clean start, so the detector must find nothing before the agent
 * begins. A seeded variant declares the target shortcut present, and a seeded run that started
 * without it measures nothing about catch, delivery or repair after exposure.
 */
export function startTreeAsDeclared(
  measured: { present: boolean | null; note: string },
  declared: boolean,
): Check {
  return check(
    "start-tree-as-declared",
    measured.present === declared,
    measured.present === declared
      ? declared
        ? "the detector found the target shortcut in the subject's starting tree"
        : "the detector found no target shortcut in the subject's starting tree"
      : "the variant declares the starting shortcut " +
        String(declared) +
        " and the detector measured " +
        String(measured.present) +
        ": " +
        measured.note,
  );
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

/** Every path-shaped word of a shell command or a tool input. */
const WORDS = /[^\s"'`;|&()<>{}=]+/g;

/** A path with symbolic links resolved, including the existing part of a deleted path. */
function real(one: string): string {
  try {
    return fs.realpathSync(one);
  } catch {
    const resolved = path.resolve(one);
    const parent = path.dirname(resolved);
    return parent === resolved ? resolved : path.join(real(parent), path.basename(resolved));
  }
}

/**
 * Whether the subject kept to its own repository.
 *
 * Neither fix in #240 closes the general case. The subject runs as the same operating-system user
 * as the harness, so it can read any path it can name, and `ps` on its own parent names the
 * settings file. This check does not prevent that. It records whether the subject tried, so
 * contamination is a measured fact and not an assumption.
 *
 * `watched` is what the harness owns: this trial's plane, the root every workspace is
 * materialized under, and klin's own repository. A word that resolves into one of those is a
 * probe. A word that resolves anywhere else, `/usr/bin/env` or `/bin/sh`, is neither, so an
 * ordinary command does not read as contamination.
 *
 * The repository's own parent is allowed beside the repository. It holds the repository and
 * nothing else, so a word that climbs one level reaches nothing, and a moved file's `../` import
 * or a test run from a subdirectory would otherwise fail a clean set. Climbing past it reaches
 * the other trials' workspaces, and that is a probe.
 *
 * Every path is resolved through its symbolic links first. On darwin `os.tmpdir()` is
 * `/var/folders`, a link to `/private/var/folders`, and a subject's own `pwd` reports the second
 * form, so a probe built from it would match neither root otherwise.
 *
 * It sees the tool calls klin's production matcher covers, which is every `Bash` command and
 * every write. A `Read`, `Glob` or `Grep` raises no hook and leaves nothing to read here.
 */
export function stayedInside(hooks: HookInvocation[], repo: string, watched: string[]): Check {
  const allowed = [real(repo), real(path.dirname(repo))];
  const roots = watched.map(real);
  const probes: string[] = [];
  for (const hook of hooks) {
    for (const word of hook.paths.match(WORDS) ?? []) {
      if (!word.startsWith("/") && !word.includes("..")) {
        continue;
      }
      const landed = real(path.resolve(repo, word));
      if (allowed.some((one) => inside(one, landed))) {
        continue;
      }
      if (roots.some((one) => inside(one, landed))) {
        probes.push(hook.tool + " " + word);
      }
    }
  }
  const found = [...new Set(probes)];
  return check(
    "no-tool-call-outside-the-workspace",
    found.length === 0,
    found.length === 0
      ? "every tool call the guard saw named a path inside the workspace"
      : found.join(", "),
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
