import { spawnSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import * as paths from "./paths.ts";
import { copyTree, files, overlay, digest, sha256 } from "./trees.ts";
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
  /** Where the probe's witness hook keeps the host's own PostToolUse payloads. Empty in a trial. */
  seen: string;
  startCommit: string;
  /**
   * The committed clean base, by digest.
   *
   * klin's base comparison and every detector measure against this tree, so a shortcut the seed
   * carries is new relative to it.
   */
  treeSha256: string;
  /** The tree the subject starts from: the committed base under any declared seed overlay. */
  startTreeSha256: string;
  /** The relative paths the declared seed wrote. Empty where the variant declares no seed. */
  seed: string[];
  /** Whether the harness stamped the committed base before laying the seed over it. */
  stamped: boolean;
  commits: number;
}

/** The ref klin writes its turn stamp to, which a seeded trial's stamp must still resolve. */
const TURN_REF = "refs/worktree/klin/turn";

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
 * The toolchain homes, which the subject must read to run its own build.
 *
 * `cargo` and `rustc` on the host machine are rustup shims that resolve a toolchain under
 * `~/.rustup`, and cargo takes its registry cache from `~/.cargo`. A subject that cannot read
 * these cannot compile the four Rust families at all, so its session measures a different task
 * than the one the fixture states. `~/.npm` is the same for the five TypeScript families.
 *
 * All three are writable as well as readable, because a first build populates the caches. None
 * of them holds anything about this benchmark, the arm or the expected shortcut.
 */
const TOOLCHAINS = ["~/.cargo", "~/.rustup", "~/.npm"];

/** Node runtimes on PATH, which npm must spawn for a package suite. Read-only in the subject. */
function nodeRuntimes(): string[] {
  const candidates = [
    process.execPath,
    ...(process.env.PATH ?? "").split(path.delimiter).map((one) => path.join(one, "node")),
  ];
  return [
    ...new Set(
      candidates.flatMap((one) => {
        try {
          return [path.dirname(path.dirname(fs.realpathSync(one)))];
        } catch {
          return [];
        }
      }),
    ),
  ];
}

const NODE_RUNTIMES = nodeRuntimes();

/** npm runs POSIX package scripts through this interpreter. Read-only in the subject. */
const SCRIPT_SHELLS = process.platform === "win32" ? [] : ["/bin/sh"];

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
 * The operating system enforces this, not the layout. Everything the harness owns is denied:
 * this trial's plane, the root every workspace is materialized under, and klin's own repository,
 * which holds the fixtures, the hidden oracles and every other trial's records.
 *
 * The read and write denials are not the same list, because the two rules do not behave the
 * same way. The repository sits under the work root, and a nested `allowWrite` does not re-open
 * a path inside a `denyWrite` region: the deny wins, and the subject cannot write in its own
 * repository at all. `denyRead` has no such problem, so the work root stays there. Writes need
 * no deny of their own: the sandbox already refuses every write outside the working directory,
 * which is what keeps one trial out of another trial's workspace. #252 is where this was found,
 * after a whole publishable round ran with every subject building outside its own repository.
 *
 * `allowUnsandboxedCommands: false` is what makes the boundary hold: without it the host retries
 * a refused command outside the sandbox, and the subject reaches the plane on the second try.
 *
 * The sandbox covers `Bash` and its children. `Read`, `Glob` and `Grep` are the host's own file
 * tools and no sandbox holds them, so `blockReadsOutsideWorkingDirectories` is what refuses them
 * the same paths.
 *
 * `TOOLCHAINS` is readable and writable, and the two package registries are reachable. Four
 * families are Rust and five are TypeScript, and one task asks the agent to bring a dependency
 * in at an exact version, which it cannot record without the registry that states it. A subject
 * that cannot install or take cargo's own package lock measures a different task. Neither those
 * directories nor either registry holds anything about this benchmark.
 *
 * `strictAllowlist` is what keeps any other host from reaching the permission flow. A headless
 * session has no one to answer a network prompt, so a domain that is not named here is refused
 * outright and the command fails, rather than the trial stalling until the harness times out.
 */
function confinement(
  repo: string,
  deniedRead: string[],
  deniedWrite: string[],
): Record<string, unknown> {
  const own = forms(repo);
  return {
    sandbox: {
      enabled: true,
      autoAllowBashIfSandboxed: true,
      allowUnsandboxedCommands: false,
      filesystem: {
        denyRead: deniedRead.flatMap(forms),
        allowRead: [...own, ...TOOLCHAINS, ...NODE_RUNTIMES, ...SCRIPT_SHELLS],
        denyWrite: deniedWrite.flatMap(forms),
        allowWrite: [...own, ...TOOLCHAINS],
      },
      network: { allowedDomains: REGISTRIES, strictAllowlist: true },
    },
    permissions: { blockReadsOutsideWorkingDirectories: true },
  };
}

/**
 * The confinement every trial of this round will run under, as one digest.
 *
 * The enumerated frozen values do not hold it. `KLIN_BENCH_WORK`, and `TMPDIR` when that is
 * unset, move the root every workspace is materialized under, and that root is a `denyRead` rule,
 * the placement of the subject's own repository, the owned-path test and the environment filter.
 * A probe run under one work root would otherwise authorize a round run under another.
 *
 * The three per-trial paths are named rather than real, so the digest is a function of the rules
 * and not of a trial. The work root is deliberately left as it stands, because it is one of the
 * rules.
 */
export function confinementSha256(): string {
  return sha256(
    JSON.stringify(
      confinement("<repo>", ["<plane>", paths.workRoot(), paths.REPO], ["<plane>", paths.REPO]),
    ),
  );
}

/**
 * One trial's copy of the hook wrapper, with the three values it must not name on a command line
 * substituted into it.
 *
 * Single-quoted, because a path may hold a space and nothing here is meant to expand. A path that
 * held a single quote would break the script, so it is refused rather than escaped: the harness
 * owns all three, and none of them has ever held one.
 */
export function wrapper(plane: string, klinBin: string, deliver: boolean): string {
  const shell = (one: string): string => {
    if (one.includes("'")) {
      throw new Error("the wrapper cannot carry a path holding a single quote: " + one);
    }
    return "'" + one + "'";
  };
  return fs
    .readFileSync(paths.HOOK, "utf8")
    .replace("@PLANE@", shell(plane))
    .replace("@KLIN@", shell(klinBin))
    .replace("@DELIVER@", deliver ? "1" : "0");
}

/**
 * The host settings for one trial, which carry the subject's confinement.
 *
 * The hook command is the wrapper and klin's own event arguments, and nothing else. The plane,
 * the klin binary and the arm are baked into the wrapper, because the host shows the agent the
 * whole command line when a Stop hook blocks. Neither the subject's environment nor any path
 * beside its repository states the arm either.
 *
 * The events, the matcher and the timeouts are klin's own production wiring. The matcher is the
 * one klin installs, so the trial runs the lifecycle a person's repository runs. The hooks
 * themselves are the host's own lifecycle and no sandbox holds them, so the wrapper still writes
 * the plane the subject cannot read.
 */
function settingsFor(place: { hook: string; plane: string; repo: string; witness: string }): string {
  const quoted = (one: string): string => JSON.stringify(one);
  const command = (args: string): string => [quoted(place.hook), args].join(" ");
  return JSON.stringify(
    {
      ...confinement(
        place.repo,
        [place.plane, paths.workRoot(), paths.REPO],
        [place.plane, paths.REPO],
      ),
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
          // `Read`, `Glob` and `Grep` are the host's own file tools. No sandbox holds them and
          // klin's production matcher does not cover them, so the probe alone watches them: this
          // records what the subject asked for, and the `PostToolUse` entry below records what
          // the host answered. A read the operating system refused raises the first and not the
          // second, and a read that succeeded carries the file into the second.
          ...(place.witness === ""
            ? []
            : [
                {
                  matcher: "Read|Glob|Grep",
                  hooks: [{ type: "command", command: quoted(place.witness), timeout: 60 }],
                },
              ]),
        ],
        Stop: [
          {
            hooks: [
              { type: "command", command: command("gate --hook --changed"), timeout: 900 },
            ],
          },
        ],
        // The probe alone wires this, and it is the probe's trusted record of what a Bash call
        // asked for and what it printed. It relaxes no sandbox rule and no tool permission, so
        // the confinement a probe proves is the confinement a trial runs under.
        ...(place.witness === ""
          ? {}
          : {
              PostToolUse: [
                {
                  matcher: "Bash|Read|Glob|Grep",
                  hooks: [{ type: "command", command: quoted(place.witness), timeout: 60 }],
                },
              ],
            }),
      },
    },
    null,
    2,
  );
}

/**
 * Lay one variant's committed clean base into `into`.
 *
 * This is the tree the harness commits, the tree klin's base comparison reads and the tree every
 * detector measures against. For a natural variant it is also the tree the agent is given. For a
 * seeded variant the agent is given this tree under the declared seed, and the two are held apart
 * on purpose: a shortcut the seed carries has to be new relative to the commit.
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
 * Lay the variant's declared seed over a committed tree, and return the paths it wrote.
 *
 * A seed is a colleague's uncommitted work. It goes on after the commit and is never committed,
 * so it is the only change standing in the working tree when the subject's session begins.
 */
function laySeed(variant: Variant, into: string): string[] {
  return variant.seed === "" ? [] : overlay(path.join(variant.root, variant.seed), into);
}

/**
 * Stamp the committed base as the turn klin measures the seed against.
 *
 * klin's hook window is the turn stamp, not the commit (SPEC 6.1). On a first session the stamp
 * moves to the working tree as it stands, which treats a person's uncommitted work as prior
 * (SPEC 6.2). A seed laid before the subject's session would therefore be inherited debt, the
 * Stop would stay silent, and a seeded trial would measure nothing.
 *
 * So the harness takes one stamp over the committed clean base, before the seed goes on. The
 * subject's own session start then finds a state directory that exists, so the stamp stays and
 * the seed is new at every stop. This is what makes the record's committed base the tree klin
 * compares against, which is the identity the seeded design rests on.
 *
 * It writes `repository`, `turn` and `index` and no journal, so the trial's own signals, stops
 * and `klin_ms` are still the session's alone.
 */
function stampCommittedBase(repo: string, state: string, klinBin: string): void {
  const env = Object.fromEntries(
    Object.entries(process.env).filter(([name]) => !name.startsWith("KLIN_")),
  );
  const ran = spawnSync(klinBin, ["radius"], {
    cwd: repo,
    input: JSON.stringify({ hook_event_name: "SessionStart", session_id: "base" }),
    encoding: "utf8",
    timeout: 300_000,
    env: { ...env, KLIN_STATE_DIR: state },
  });
  if (ran.error || ran.status !== 0) {
    throw new Error(
      "the committed base could not be stamped, so a seed laid over it would read as prior work: " +
        (ran.error?.message ?? "klin radius exited " + String(ran.status) + " " + (ran.stderr ?? "")),
    );
  }
}

/**
 * Every path git reports as changed in the working tree, by relative path, sorted.
 *
 * Two plain listings rather than one `status --porcelain`, because both print a path per line and
 * neither prints a status code, a rename arrow or anything else to parse off the front. A rename
 * appears here as the old path and the new one, which is what a declared seed's own path list
 * names too.
 */
export function uncommitted(repo: string): string[] {
  const changed = git(repo, "diff", "--name-only", "HEAD");
  const untracked = git(repo, "ls-files", "--others", "--exclude-standard");
  return [...new Set([...changed.split("\n"), ...untracked.split("\n")])]
    .map((one) => one.trim())
    .filter((one) => one.length > 0)
    .sort();
}

/**
 * The settings file the trial ran under, by digest, with its three per-trial values named.
 *
 * The file itself cannot be compared across arms: it names the plane and the workspace, whose
 * paths both carry the trial id, and the wrapper, whose name is random per trial. So those two
 * paths become `<plane>` and `<work>`, the wrapper becomes `<hook>`, and what the digest still
 * attests is the real bytes of the real file: the hook table, the matcher, the timeouts and
 * every sandbox and permission rule the subject ran under. Two arms that differ here did not run
 * one configuration, and a file one of them truncated or hand-edited says so.
 *
 * The arm is no longer here to normalize. It lives in the wrapper, which `wrapperSha256` holds
 * to the same standard.
 *
 * Only those three normalize away. A changed sandbox rule, tool permission, hook event or timeout
 * is a real difference and changes the digest.
 */
export function wiringSha256(settings: string, plane: string, work: string, hook: string): string {
  let text = fs.readFileSync(settings, "utf8");
  text = text.split(hook).join("<hook>");
  for (const one of forms(plane)) {
    text = text.split(one).join("<plane>");
  }
  for (const one of forms(work)) {
    text = text.split(one).join("<work>");
  }
  return sha256(text);
}

/**
 * The wrapper the trial ran, by digest, with the three baked values named.
 *
 * The wrapper now differs between the arms by construction, so a raw digest of it would fail
 * every paired cell. The plane becomes `<plane>`, the klin binary becomes `<klin>` and the arm
 * becomes `<arm>`. What the digest still attests is every other byte of the script: how it writes
 * the payload, that it closes the pipe, what evidence it keeps, and that it suppresses on 0 and
 * delivers on 1. Two arms running different wrapper logic say so here.
 */
export function wrapperSha256(hook: string, plane: string, klinBin: string): string {
  let text = fs.readFileSync(hook, "utf8");
  text = text.split("'" + plane + "'").join("<plane>");
  text = text.split("'" + klinBin + "'").join("<klin>");
  return sha256(text.replace(/^deliver=[01]$/m, "deliver=<arm>"));
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
  observe = false,
): Workspace {
  const root = path.join(paths.workRoot(), trialId);
  fs.rmSync(root, { recursive: true, force: true });
  const repo = path.join(root, "repo");
  const hook = path.join(plane, randomBytes(12).toString("hex"));
  const settings = path.join(plane, "settings.json");
  const state = path.join(plane, "state");
  const hooks = path.join(plane, "hooks");
  const seen = path.join(plane, "witness");
  fs.mkdirSync(repo, { recursive: true });
  fs.mkdirSync(plane, { recursive: true });
  fs.rmSync(state, { recursive: true, force: true });
  fs.rmSync(hooks, { recursive: true, force: true });

  layStartingTree(variant, repo);

  fs.writeFileSync(hook, wrapper(plane, klinBin, deliver));
  fs.chmodSync(hook, 0o755);
  const witness = observe ? path.join(plane, randomBytes(12).toString("hex")) : "";
  if (witness !== "") {
    fs.rmSync(path.join(plane, "witness"), { recursive: true, force: true });
    fs.writeFileSync(witness, fs.readFileSync(paths.WITNESS, "utf8").replace("@PLANE@", "'" + plane + "'"));
    fs.chmodSync(witness, 0o755);
  }
  fs.writeFileSync(settings, settingsFor({ hook, plane, repo, witness }) + "\n");

  const treeSha256 = digest(repo);
  git(repo, "init", "--quiet");
  git(repo, "add", "-A");
  git(repo, "commit", "--quiet", "-m", "The starting tree");
  const startCommit = git(repo, "rev-parse", "HEAD");
  const commits = git(repo, "rev-list", "--count", "HEAD");
  const stamped = variant.seed !== "";
  if (stamped) {
    stampCommittedBase(repo, state, klinBin);
  }
  const seed = laySeed(variant, repo);
  const startTreeSha256 = digest(repo);

  return {
    root,
    repo,
    plane,
    hook,
    settings,
    state,
    hooks,
    seen: witness === "" ? "" : seen,
    startCommit,
    treeSha256,
    startTreeSha256,
    seed,
    stamped,
    commits: Number(commits),
  };
}

/**
 * Rename the wrapper to `hook` once the session is over.
 *
 * The random name is only needed while a subject is running. Afterwards the plane is read by the
 * verifier and packaged by `evidence-prepare`, both of which want one stable name. Returns where
 * the wrapper now is.
 */
export function settle(place: Workspace): string {
  const stable = path.join(place.plane, "hook");
  if (place.hook !== stable) {
    fs.renameSync(place.hook, stable);
  }
  return stable;
}

/** The committed clean base again, on its own, so a detector can compare against it. */
export function startingTree(variant: Variant, into: string): string {
  fs.rmSync(into, { recursive: true, force: true });
  fs.mkdirSync(into, { recursive: true });
  return layStartingTree(variant, into);
}

/** The tree the subject starts from again: the committed clean base under any declared seed. */
export function subjectStartingTree(variant: Variant, into: string): string {
  const laid = startingTree(variant, into);
  laySeed(variant, laid);
  return laid;
}

/**
 * What klin's state and the repository hold after the pre-session stamp, as plain facts.
 *
 * `integrity.baseStampAsDeclared` judges these. The reading lives here because this module owns
 * git and the plane's layout, and the judging lives there because that is where a trial's other
 * named contracts are written.
 */
export interface BaseStamp {
  /** The worktree directories klin's state root holds. A trial owns its state, so this is 0 or 1. */
  worktrees: string[];
  /** The files that one worktree entry holds, by name. */
  entries: string[];
  /** The `turn` file as klin wrote it, or null where it is missing or does not parse. */
  turn: { commit?: string; parent?: string; verdict?: string } | null;
  /** The object `refs/worktree/klin/turn` resolves to, or the empty string. */
  ref: string;
  /** The repository path the state entry names, or the empty string. */
  repository: string;
}

/** Read klin's state and the turn ref, without judging either. */
export function baseStamp(place: Workspace): BaseStamp {
  const worktrees = fs.existsSync(place.state)
    ? fs.readdirSync(place.state).sort()
    : [];
  const one = worktrees.length === 1 ? path.join(place.state, worktrees[0]) : "";
  const held = (name: string): string => {
    try {
      return fs.readFileSync(path.join(one, name), "utf8");
    } catch {
      return "";
    }
  };
  let turn: BaseStamp["turn"] = null;
  try {
    turn = one === "" ? null : (JSON.parse(held("turn")) as BaseStamp["turn"]);
  } catch {
    turn = null;
  }
  let ref = "";
  try {
    ref = git(place.repo, "rev-parse", "--verify", "--quiet", TURN_REF);
  } catch {
    ref = "";
  }
  return {
    worktrees,
    entries: one === "" || !fs.existsSync(one) ? [] : fs.readdirSync(one).sort(),
    turn,
    ref,
    repository: held("repository").trim(),
  };
}

/** The paths a variant's declared seed writes, read from the overlay the fixture ships. */
export function seedPaths(variant: Variant): string[] {
  return variant.seed === "" ? [] : files(path.join(variant.root, variant.seed)).filter((one) => one !== "REMOVE");
}
