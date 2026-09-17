import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import * as paths from "./paths.ts";
import { copyTree, overlay, digest, sha256 } from "./trees.ts";
import type { Variant } from "./catalogue.ts";

/**
 * The subject workspace and the control plane.
 *
 * Every trial gets a fresh standalone Git repository, a fresh klin state directory and a fresh
 * host session. The repository is the only thing in its own parent directory, so no `ls ..` from
 * the subject's working directory reaches anything the harness owns.
 *
 * The arm, the scoring metadata, the expected shortcut, the external oracle, klin's state, the
 * hook wrapper, the host settings and the hook evidence all live in the plane. The plane sits
 * inside this repository and the workspace sits under the system temporary directory, so the two
 * share no ancestor but the filesystem root and no relative path leads from one to the other.
 */

export interface Workspace {
  /** The workspace root. It holds `repo` and nothing else. */
  root: string;
  repo: string;
  /** The control plane for this trial. Nothing under it is named in the subject's environment. */
  plane: string;
  hook: string;
  settings: string;
  state: string;
  hooks: string;
  startCommit: string;
  treeSha256: string;
  commits: number;
}

const GIT = [
  "-c",
  "user.name=klin benchmark",
  "-c",
  "user.email=benchmark@example.invalid",
  "-c",
  "commit.gpgsign=false",
  "-c",
  "init.defaultBranch=main",
];

export function git(repo: string, ...args: string[]): string {
  const ran = spawnSync("git", [...GIT, ...args], { cwd: repo, encoding: "utf8" });
  if (ran.status !== 0) {
    throw new Error("git " + args.join(" ") + " failed: " + (ran.stderr ?? ""));
  }
  return (ran.stdout ?? "").trim();
}

/**
 * The hosts a fixture task may need, and nothing else.
 *
 * Both package managers are here because the families are Rust and TypeScript. Neither registry
 * can tell a subject which arm it is in.
 */
const REGISTRIES = [
  "registry.npmjs.org",
  "crates.io",
  "index.crates.io",
  "static.crates.io",
];

/**
 * A path and its symbolic-link-resolved form, deduplicated.
 *
 * On darwin the system temporary directory is `/var/folders`, a link to `/private/var/folders`,
 * and a subject's own `pwd` reports the second form. A rule written in one form and enforced
 * against the other would hold nothing, so both are written. `wiringSha256` normalizes both for
 * the same reason.
 */
function forms(one: string): string[] {
  let held = [one];
  try {
    held = [...new Set([one, fs.realpathSync(one)])];
  } catch {
    held = [one];
  }
  return held.sort((a, b) => b.length - a.length);
}

/**
 * Where the subject may read and write, and where it may not.
 *
 * The operating system enforces this, not the layout. `denied` is everything the harness owns:
 * this trial's plane, the root every workspace is materialized under, and klin's own repository,
 * which holds the fixtures, the hidden oracles and every other trial's records. The repository
 * is re-opened inside the denied region, because it sits under that root.
 *
 * `allowUnsandboxedCommands: false` is what makes the boundary hold: without it the host retries
 * a refused command outside the sandbox, and the subject reaches the plane on the second try.
 *
 * The sandbox covers `Bash` and its children. `Read`, `Glob` and `Grep` are the host's own file
 * tools and no sandbox holds them, so `blockReadsOutsideWorkingDirectories` is what refuses them
 * the same paths.
 *
 * `~/.cargo` and `~/.npm` are writable, and the two package registries are reachable. Four
 * families are Rust and five are TypeScript, and one task asks the agent to bring a dependency
 * in at an exact version, which it cannot record without the registry that states it. A subject
 * that cannot install or take cargo's own package lock measures a different task. Neither
 * directory nor either registry holds anything about this benchmark.
 *
 * `strictAllowlist` is what keeps any other host from reaching the permission flow. A headless
 * session has no one to answer a network prompt, so a domain that is not named here is refused
 * outright and the command fails, rather than the trial stalling until the harness times out.
 */
function confinement(repo: string, denied: string[]): Record<string, unknown> {
  const out = denied.flatMap(forms);
  const own = forms(repo);
  return {
    sandbox: {
      enabled: true,
      autoAllowBashIfSandboxed: true,
      allowUnsandboxedCommands: false,
      filesystem: {
        denyRead: out,
        allowRead: own,
        denyWrite: out,
        allowWrite: [...own, "~/.cargo", "~/.npm"],
      },
      network: { allowedDomains: REGISTRIES, strictAllowlist: true },
    },
    permissions: { blockReadsOutsideWorkingDirectories: true },
  };
}

/**
 * The host settings for one trial, which carry the arm and the subject's confinement.
 *
 * The arm reaches the wrapper as an argument of the hook command, and the settings file lives in
 * the plane. Neither the subject's environment nor any path beside its repository states it.
 *
 * The events, the matcher and the timeouts are klin's own production wiring. The matcher is the
 * one klin installs, so the trial runs the lifecycle a person's repository runs. The hooks
 * themselves are the host's own lifecycle and no sandbox holds them, so the wrapper still writes
 * the plane the subject cannot read.
 */
function settingsFor(
  place: { hook: string; plane: string; repo: string },
  klinBin: string,
  deliver: boolean,
): string {
  const quoted = (one: string): string => JSON.stringify(one);
  const command = (args: string): string =>
    [quoted(place.hook), quoted(place.plane), quoted(klinBin), deliver ? "1" : "0", args].join(" ");
  return JSON.stringify(
    {
      ...confinement(place.repo, [place.plane, paths.workRoot(), paths.REPO]),
      hooks: {
        SessionStart: [{ hooks: [{ type: "command", command: command("radius"), timeout: 60 }] }],
        UserPromptSubmit: [
          { hooks: [{ type: "command", command: command("radius"), timeout: 60 }] },
        ],
        PreToolUse: [
          {
            matcher: "Write|Edit|MultiEdit|NotebookEdit|Bash|apply_patch|mcp__.*",
            hooks: [{ type: "command", command: command("guard"), timeout: 60 }],
          },
        ],
        Stop: [
          {
            hooks: [
              { type: "command", command: command("gate --hook --changed"), timeout: 900 },
            ],
          },
        ],
      },
    },
    null,
    2,
  );
}

/**
 * Lay one variant's starting tree into `into`.
 *
 * The subject workspace and the tree a detector compares against are laid by this one function,
 * so the baseline a shortcut is measured against is always the tree the agent was given.
 */
function layStartingTree(variant: Variant, into: string): string {
  copyTree(path.join(variant.familyRoot, "base"), into);
  overlay(path.join(variant.root, "overlay"), into);
  if (!fs.existsSync(path.join(into, "klin.json"))) {
    fs.writeFileSync(path.join(into, "klin.json"), "{}\n");
  }
  return into;
}

/**
 * The settings file the trial ran under, by digest, with its three per-trial values named.
 *
 * The file itself cannot be compared across arms: it names the plane and the workspace, whose
 * paths both carry the trial id, and it carries the arm. So those two paths become `<plane>` and
 * `<work>`, the arm digit becomes `<arm>`, and what the digest still attests is the real bytes of
 * the real file: the hook table, the matcher, the timeouts, the binary the wrapper runs and every
 * sandbox and permission rule the subject ran under. Two arms that differ here did not run one
 * configuration, and a file one of them truncated or hand-edited says so.
 *
 * Only those three normalize away. A changed sandbox rule, tool permission, hook event or timeout
 * is a real difference and changes the digest.
 *
 * The arm is the one digit that follows a command's escaped closing quote, because a command is
 * a JSON string inside the settings file and the klin path before the arm ends in one. No other
 * number in the file sits in that position.
 */
export function wiringSha256(settings: string, plane: string, work: string): string {
  let text = fs.readFileSync(settings, "utf8");
  for (const one of forms(plane)) {
    text = text.split(one).join("<plane>");
  }
  for (const one of forms(work)) {
    text = text.split(one).join("<work>");
  }
  return sha256(text.replace(/\\" [01] /g, '\\" <arm> '));
}

/**
 * Materialize one trial's subject workspace and its control plane.
 *
 * `deliver` is the treatment, and it is written into the plane's settings file rather than into
 * the environment the host hands the subject.
 */
export function materialize(
  variant: Variant,
  trialId: string,
  plane: string,
  klinBin: string,
  deliver: boolean,
): Workspace {
  const root = path.join(paths.workRoot(), trialId);
  fs.rmSync(root, { recursive: true, force: true });
  const repo = path.join(root, "repo");
  const hook = path.join(plane, "hook");
  const settings = path.join(plane, "settings.json");
  const state = path.join(plane, "state");
  const hooks = path.join(plane, "hooks");
  fs.mkdirSync(repo, { recursive: true });
  fs.mkdirSync(plane, { recursive: true });
  fs.rmSync(state, { recursive: true, force: true });
  fs.rmSync(hooks, { recursive: true, force: true });

  layStartingTree(variant, repo);

  fs.copyFileSync(paths.HOOK, hook);
  fs.chmodSync(hook, 0o755);
  fs.writeFileSync(settings, settingsFor({ hook, plane, repo }, klinBin, deliver) + "\n");

  const treeSha256 = digest(repo);
  git(repo, "init", "--quiet");
  git(repo, "add", "-A");
  git(repo, "commit", "--quiet", "-m", "The starting tree");
  const startCommit = git(repo, "rev-parse", "HEAD");
  const commits = git(repo, "rev-list", "--count", "HEAD");

  return {
    root,
    repo,
    plane,
    hook,
    settings,
    state,
    hooks,
    startCommit,
    treeSha256,
    commits: Number(commits),
  };
}

/** The starting tree again, on its own, so a detector can compare against it. */
export function startingTree(variant: Variant, into: string): string {
  fs.rmSync(into, { recursive: true, force: true });
  fs.mkdirSync(into, { recursive: true });
  return layStartingTree(variant, into);
}
