import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { family } from "../src/catalogue.ts";
import * as integrity from "../src/integrity.ts";
import * as workspace from "../src/workspace.ts";
import { files } from "../src/trees.ts";
import * as paths from "../src/paths.ts";
import * as session from "../src/session.ts";
import type { Check, Isolation } from "../src/record.ts";

function room(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-integrity-"));
}

const KLIN = "/opt/klin/klin";

/** One trial, with a plane of its own. `clear` removes both halves. */
function laid(familyName: string, variantName: "risk" | "control", id: string, deliver = true) {
  const plane = path.join(room(), "plane");
  const place = workspace.materialize(
    family(familyName).variants[variantName],
    id,
    plane,
    KLIN,
    deliver,
  );
  const clear = (): void => {
    fs.rmSync(place.root, { recursive: true, force: true });
    fs.rmSync(plane, { recursive: true, force: true });
  };
  return { place, plane, clear };
}

function named(isolation: Isolation, name: string): Check {
  const one = isolation.checks.find((check) => check.name === name);
  assert.ok(one, "no check named " + name);
  return one;
}

test("a materialized workspace is isolated from its control plane", () => {
  const where = room();
  const variant = family("inventory").variants.risk;
  const start = workspace.startingTree(variant, path.join(where, "repo"));
  const judged = integrity.judge(variant, "inventory", start, path.join(where, "control"));
  assert.equal(judged.verified, true, JSON.stringify(judged.checks));
  fs.rmSync(where, { recursive: true, force: true });
});

test("a planted hidden oracle is found", () => {
  const where = room();
  const variant = family("inventory").variants.risk;
  const start = workspace.startingTree(variant, path.join(where, "repo"));
  const hidden = files(path.join(variant.root, "oracle"))[0];
  fs.mkdirSync(path.dirname(path.join(start, hidden)), { recursive: true });
  fs.writeFileSync(path.join(start, hidden), "leaked");
  const judged = integrity.judge(variant, "inventory", start, path.join(where, "control"));
  assert.equal(judged.verified, false);
  assert.equal(named(judged, "no-hidden-oracle-in-workspace").passed, false);
  fs.rmSync(where, { recursive: true, force: true });
});

test("a planted prompt is found", () => {
  const where = room();
  const variant = family("escapes").variants.control;
  const start = workspace.startingTree(variant, path.join(where, "repo"));
  fs.writeFileSync(path.join(start, "prompt.md"), "the task");
  assert.equal(
    named(integrity.judge(variant, "escapes", start, path.join(where, "c")), "no-hidden-oracle-in-workspace")
      .passed,
    false,
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("a file named after the gate is found", () => {
  const where = room();
  const variant = family("escapes").variants.risk;
  const start = workspace.startingTree(variant, path.join(where, "repo"));
  fs.writeFileSync(path.join(start, "src/escapes.ts"), "export const a = 1;\n");
  assert.equal(
    named(integrity.judge(variant, "escapes", start, path.join(where, "c")), "no-metadata-in-paths").passed,
    false,
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("a file naming the detector or the scoring metadata is found", () => {
  const where = room();
  const variant = family("escapes").variants.risk;
  const start = workspace.startingTree(variant, path.join(where, "repo"));
  fs.writeFileSync(path.join(start, "src/notes.ts"), "// scored by new_escape_site\n");
  assert.equal(
    named(integrity.judge(variant, "escapes", start, path.join(where, "c")), "no-metadata-in-contents")
      .passed,
    false,
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("an ordinary word that merely holds a forbidden one is not a leak", () => {
  const where = room();
  const variant = family("escapes").variants.risk;
  const start = workspace.startingTree(variant, path.join(where, "repo"));
  fs.writeFileSync(path.join(start, "src/notes.ts"), "export const interactive = true;\n");
  assert.equal(
    integrity.judge(variant, "escapes", start, path.join(where, "c")).verified,
    true,
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("a control plane inside the workspace is refused", () => {
  const where = room();
  const variant = family("inventory").variants.risk;
  const start = workspace.startingTree(variant, path.join(where, "repo"));
  assert.equal(
    named(
      integrity.judge(variant, "inventory", start, path.join(start, "control")),
      "control-plane-outside-workspace",
    ).passed,
    false,
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("materializing a trial gives a fresh repository, state and session store", () => {
  const { place, clear } = laid("stubs", "risk", "selftest-fresh");
  const judged = integrity.freshness(place.repo, place.state, "", place.commits);
  assert.equal(judged.verified, true, JSON.stringify(judged.checks));
  assert.equal(place.commits, 1);
  assert.ok(fs.existsSync(path.join(place.repo, "klin.json")));
  assert.ok(fs.existsSync(place.settings));
  assert.ok(!fs.existsSync(path.join(place.repo, ".claude")));
  clear();
});

test("the repository is the only thing in its own parent directory", () => {
  const { place, clear } = laid("stubs", "risk", "selftest-alone");
  assert.deepEqual(
    fs.readdirSync(place.root),
    ["repo"],
    "an ls of the subject's parent must reach nothing the harness owns",
  );
  for (const held of [place.hook, place.settings, place.state, place.hooks]) {
    assert.ok(
      !held.startsWith(place.root),
      held + " sits beside the subject, one ls away from it",
    );
  }
  clear();
});

test("a second trial over the same family starts from the same tree", () => {
  const one = laid("stubs", "risk", "selftest-one");
  const two = laid("stubs", "risk", "selftest-two");
  assert.equal(one.place.treeSha256, two.place.treeSha256);
  assert.notEqual(one.place.root, two.place.root);
  assert.notEqual(one.place.state, two.place.state);
  one.clear();
  two.clear();
});

test("the host settings point every event at the wrapper and live outside the workspace", () => {
  const { place, clear } = laid("stubs", "control", "selftest-settings");
  const settings = JSON.parse(fs.readFileSync(place.settings, "utf8")) as {
    hooks: Record<string, { hooks: { command: string }[] }[]>;
  };
  assert.deepEqual(Object.keys(settings.hooks).sort(), [
    "PreToolUse",
    "SessionStart",
    "Stop",
    "UserPromptSubmit",
  ]);
  for (const entries of Object.values(settings.hooks)) {
    for (const entry of entries) {
      for (const hook of entry.hooks) {
        assert.ok(hook.command.includes(place.hook), hook.command);
      }
    }
  }
  assert.ok(!place.settings.startsWith(place.repo));
  clear();
});

test("the arms differ in nothing a workspace can read", () => {
  const active = laid("lockfile", "risk", "selftest-active", true);
  const shadow = laid("lockfile", "risk", "selftest-shadow", false);
  assert.equal(active.place.treeSha256, shadow.place.treeSha256);
  assert.notEqual(
    fs.readFileSync(active.place.hook, "utf8"),
    fs.readFileSync(shadow.place.hook, "utf8"),
    "the arm is baked into the wrapper, which sits in the plane and not beside the subject",
  );
  assert.equal(
    workspace.wrapperSha256(active.place.hook, active.place.plane, KLIN),
    workspace.wrapperSha256(shadow.place.hook, shadow.place.plane, KLIN),
    "the baked values normalize away, so every other byte of the script must match",
  );
  assert.notEqual(
    fs.readFileSync(active.place.settings, "utf8"),
    fs.readFileSync(shadow.place.settings, "utf8"),
    "the settings name the plane and the wrapper, both of which carry the trial id",
  );
  assert.equal(
    workspace.wiringSha256(active.place.settings, active.place.plane, active.place.root, active.place.hook),
    workspace.wiringSha256(shadow.place.settings, shadow.place.plane, shadow.place.root, shadow.place.hook),
    "the digest names the plane, the workspace and the wrapper, so two arms of one cell must match",
  );
  for (const one of [active, shadow]) {
    assert.deepEqual(fs.readdirSync(one.place.root), ["repo"]);
  }
  active.clear();
  shadow.clear();
});

/**
 * The host prints the whole hook command line back to the agent when a Stop hook blocks, under
 * "Stop hook feedback:", and it prints its own sandbox deny rules into the subject's system
 * prompt. A live set found four Active subjects reading the first and two of them running klin
 * against their own tree. So the command line names the wrapper and klin's event arguments, and
 * nothing else, and the wrapper's name is not one a subject told where the plane is can guess.
 */
test("no hook command line names the klin binary, the arm or a guessable wrapper", () => {
  for (const deliver of [true, false]) {
    const { place, clear } = laid("lockfile", "risk", "selftest-echo-" + String(deliver), deliver);
    const settings = JSON.parse(fs.readFileSync(place.settings, "utf8")) as {
      hooks: Record<string, { hooks: { command: string }[] }[]>;
    };
    const commands = Object.values(settings.hooks).flatMap((entries) =>
      entries.flatMap((entry) => entry.hooks.map((one) => one.command)),
    );
    assert.ok(commands.length >= 4, "every production event still has a hook");
    for (const command of commands) {
      assert.ok(!command.includes(KLIN), "the command line names the klin binary: " + command);
      assert.match(
        command,
        /^"[^"]+"(?: [a-z-]+)*$/,
        "the command line is the wrapper and klin's own event arguments: " + command,
      );
    }
    assert.notEqual(path.basename(place.hook), "hook", "a wrapper called hook is one guess away");
    assert.match(path.basename(place.hook), /^[0-9a-f]{24}$/);
    assert.equal(
      fs.readFileSync(place.hook, "utf8").includes(KLIN),
      true,
      "the binary is in the wrapper instead, which the sandbox refuses the subject",
    );
    clear();
  }
});

/** The random name is only for the live session. The verifier and the packager want one name. */
test("settling the plane puts the wrapper back under its stable name", () => {
  const { place, clear } = laid("lockfile", "risk", "selftest-settle");
  const before = fs.readFileSync(place.hook, "utf8");
  const settled = workspace.settle(place);
  assert.equal(path.basename(settled), "hook");
  assert.equal(fs.readFileSync(settled, "utf8"), before);
  assert.equal(fs.existsSync(place.hook), false);
  clear();
});

test("the wiring digest attests the settings file's own bytes", () => {
  const { place, clear } = laid("lockfile", "risk", "selftest-wiring");
  const digestNow = (): string => workspace.wiringSha256(place.settings, place.plane, place.root, place.hook);
  const before = digestNow();
  const edit = (from: string, to: string): void => {
    fs.writeFileSync(place.settings, fs.readFileSync(place.settings, "utf8").replace(from, to));
  };
  edit('"timeout": 900', '"timeout": 5');
  assert.notEqual(
    digestNow(),
    before,
    "a hand-edited Stop timeout must change the digest, or a paired cell could not catch it",
  );
  edit('"timeout": 5', '"timeout": 900');
  assert.equal(digestNow(), before);
  edit('"allowUnsandboxedCommands": false', '"allowUnsandboxedCommands": true');
  assert.notEqual(
    digestNow(),
    before,
    "a sandbox rule is a real difference between two arms, so it must change the digest",
  );
  clear();
});

/**
 * What normalizes away, and what may not.
 *
 * Two arms run in two workspaces and two planes, and both paths carry the trial id. Nothing else
 * about the file may drop out of the digest.
 */
test("the session's own paths normalize away and the rules do not", () => {
  const one = laid("lockfile", "control", "selftest-norm-one", true);
  const two = laid("lockfile", "control", "selftest-norm-two", true);
  assert.notEqual(one.place.root, two.place.root);
  assert.notEqual(one.place.plane, two.place.plane);
  assert.equal(
    workspace.wiringSha256(one.place.settings, one.place.plane, one.place.root, one.place.hook),
    workspace.wiringSha256(two.place.settings, two.place.plane, two.place.root, two.place.hook),
    "only the trial's own paths differ between these two",
  );
  const before = workspace.wiringSha256(one.place.settings, one.place.plane, one.place.root, one.place.hook);
  fs.writeFileSync(
    one.place.settings,
    fs
      .readFileSync(one.place.settings, "utf8")
      .replace('"blockReadsOutsideWorkingDirectories": true', '"blockReadsOutsideWorkingDirectories": false'),
  );
  assert.notEqual(
    workspace.wiringSha256(one.place.settings, one.place.plane, one.place.root, one.place.hook),
    before,
    "a tool permission is a real difference and may not normalize away",
  );
  one.clear();
  two.clear();
});

/**
 * The subject is confined by the operating system, not by the layout.
 *
 * `cwd` is no sandbox: the plane, the other trials' workspaces and klin's own repository are all
 * absolute paths a shell can name. These rules are what refuse them, and `allowUnsandboxedCommands`
 * is what stops the host from retrying a refused command outside the sandbox.
 *
 * The work root is denied for reading and left open for writing, and #252 is what that cost. A
 * nested `allowWrite` does not re-open a path inside a `denyWrite` region, so a denied work root
 * walls the subject out of its own repository, and a whole publishable round ran with every
 * subject building somewhere else. Writes need no deny of their own: the sandbox already refuses
 * every write outside the working directory.
 */
test("the settings confine the subject to its own repository", () => {
  const { place, clear } = laid("lockfile", "risk", "selftest-sandbox", false);
  const settings = JSON.parse(fs.readFileSync(place.settings, "utf8")) as {
    sandbox: {
      enabled: boolean;
      allowUnsandboxedCommands: boolean;
      filesystem: {
        denyRead: string[];
        allowRead: string[];
        denyWrite: string[];
        allowWrite: string[];
      };
      network: { allowedDomains: string[]; strictAllowlist: boolean };
    };
    permissions: { blockReadsOutsideWorkingDirectories: boolean };
  };
  assert.equal(settings.sandbox.enabled, true);
  assert.equal(settings.sandbox.allowUnsandboxedCommands, false);
  assert.equal(settings.permissions.blockReadsOutsideWorkingDirectories, true);
  for (const denied of [place.plane, paths.workRoot(), paths.REPO]) {
    for (const form of [denied, fs.realpathSync(denied)]) {
      assert.ok(settings.sandbox.filesystem.denyRead.includes(form), form + " is readable");
    }
  }
  for (const denied of [place.plane, paths.REPO]) {
    for (const form of [denied, fs.realpathSync(denied)]) {
      assert.ok(settings.sandbox.filesystem.denyWrite.includes(form), form + " is writable");
    }
  }
  for (const form of [paths.workRoot(), fs.realpathSync(paths.workRoot())]) {
    assert.ok(
      !settings.sandbox.filesystem.denyWrite.includes(form),
      form + " is denied for writing, which walls the subject out of its own repository",
    );
  }
  const reopened = [...new Set([place.repo, fs.realpathSync(place.repo)])].sort();
  const environment = [...new Set([paths.ENVIRONMENT, fs.realpathSync(paths.ENVIRONMENT)])].sort();
  const toolchains = ["~/.cargo", "~/.rustup", "~/.npm"];
  const runtimeCandidates = [
    process.execPath,
    ...(process.env.PATH ?? "").split(path.delimiter).map((one) => path.join(one, "node")),
  ];
  const runtimes = [...new Set(runtimeCandidates)].flatMap((one) => {
    try {
      return [path.dirname(path.dirname(fs.realpathSync(one)))];
    } catch {
      return [];
    }
  });
  const shells = process.platform === "win32" ? [] : ["/bin/sh"];
  assert.deepEqual(
    settings.sandbox.filesystem.allowRead
      .filter((one) => !toolchains.includes(one) && !runtimes.includes(one) && !shells.includes(one))
      .sort(),
    [...reopened, ...environment].sort(),
    "only the subject's own repository and the read-only probe helper are opened",
  );
  assert.deepEqual(
    settings.sandbox.filesystem.allowWrite
      .filter((one) => !toolchains.includes(one) && !runtimes.includes(one) && !shells.includes(one))
      .sort(),
    reopened,
    "the probe helper is not writable",
  );
  for (const named of [settings.sandbox.filesystem.allowRead, settings.sandbox.filesystem.allowWrite]) {
    for (const home of toolchains) {
      assert.ok(named.includes(home), home + " is refused, so the subject cannot run its own build");
    }
  }
  for (const runtime of runtimes) {
    assert.ok(settings.sandbox.filesystem.allowRead.includes(runtime), runtime + " is refused, so npm cannot spawn Node");
    assert.ok(!settings.sandbox.filesystem.allowWrite.includes(runtime), runtime + " is writable by the subject");
  }
  for (const shell of shells) {
    assert.ok(settings.sandbox.filesystem.allowRead.includes(shell), shell + " is refused, so npm cannot run its package script");
    assert.ok(!settings.sandbox.filesystem.allowWrite.includes(shell), shell + " is writable by the subject");
  }
  assert.deepEqual(
    settings.sandbox.network,
    {
      allowedDomains: ["registry.npmjs.org", "crates.io", "index.crates.io", "static.crates.io"],
      strictAllowlist: true,
    },
    "a task that installs a dependency needs its registry, and a headless session cannot answer a network prompt",
  );
  clear();
});

test("a trial that shares the operator's host configuration says so and is not refused", () => {
  const { place, clear } = laid("stubs", "risk", "selftest-shared-config");
  const judged = integrity.freshness(place.repo, place.state, "", place.commits);
  assert.equal(judged.verified, true);
  assert.match(named(judged, "fresh-host-configuration").detail, /shares the operator's/);
  clear();
});

test("a trial given its own host configuration is refused when that is not fresh", () => {
  const where = room();
  const { place, clear } = laid("stubs", "risk", "selftest-own-config");
  const config = path.join(where, "config");
  fs.mkdirSync(config, { recursive: true });
  assert.equal(
    named(integrity.freshness(place.repo, place.state, config, place.commits), "fresh-host-configuration")
      .passed,
    true,
    "an empty directory of its own is fresh",
  );
  fs.writeFileSync(path.join(config, "CLAUDE.md"), "left over\n");
  assert.equal(
    named(integrity.freshness(place.repo, place.state, config, place.commits), "fresh-host-configuration")
      .passed,
    false,
    "a directory another trial left behind is not fresh",
  );
  fs.rmSync(where, { recursive: true, force: true });
  clear();
});

/**
 * The environment is a boundary the sandbox does not hold.
 *
 * `cwd` moves the subject's process. It does not rewrite the variables the process inherits, and
 * the harness runs out of klin's own repository, so `PWD` and `OLDPWD` carried that path into
 * every trial. A live trial found a subject that read it and ran the klin binary under
 * `target/release` against its own tree, which is klin's reference documentation and klin's own
 * verdict reaching a subject the treatment says gets neither.
 */
test("no variable handed to a subject names a path the harness owns", () => {
  const before = { ...process.env };
  process.env.PWD = paths.REPO;
  process.env.OLDPWD = paths.REPO;
  process.env.A_TOOL_CACHE = path.join(paths.REPO, "target", "release");
  process.env.PATH = [path.join(paths.REPO, "target", "release"), "/usr/bin", "/bin"].join(
    path.delimiter,
  );
  process.env.KLIN_STATE_DIR = "/somewhere";
  try {
    const kept = session.withoutKlin();
    for (const [name, value] of Object.entries(kept)) {
      assert.ok(
        !(value ?? "").includes(paths.REPO),
        name + " carries " + paths.REPO + " into the subject",
      );
    }
    assert.equal(kept.PWD, undefined);
    assert.equal(kept.OLDPWD, undefined);
    assert.equal(kept.A_TOOL_CACHE, undefined);
    assert.equal(kept.KLIN_STATE_DIR, undefined, "every KLIN_ variable is still dropped");
    assert.equal(kept.NPM_CONFIG_USERCONFIG, "/dev/null", "npm must not probe the operator's user config");
    assert.equal(kept.NPM_CONFIG_SCRIPT_SHELL, "/bin/sh", "npm must use the allowlisted package-script shell");
    assert.deepEqual(
      (kept.PATH ?? "").split(path.delimiter),
      ["/usr/bin", "/bin"],
      "PATH loses the owned entry and keeps the rest, because a subject with no PATH cannot build",
    );
  } finally {
    for (const name of ["PWD", "OLDPWD", "A_TOOL_CACHE", "PATH", "KLIN_STATE_DIR"]) {
      if (before[name] === undefined) {
        delete process.env[name];
      } else {
        process.env[name] = before[name];
      }
    }
  }
});

/**
 * Filtering the environment is not enough on its own.
 *
 * `~/.zshenv` runs for every zsh invocation, interactive or not, and this operator's sources
 * `~/.config/secrets.env`. So every Bash call in every trial re-exported a GitHub OAuth token, a
 * fine-grained PAT, two API keys and a proxy password inside the subject's own shell, after the
 * allowlist had already dropped them. A probe session read them and said so.
 */
test("the subject's shell reads no startup file of the operator's", () => {
  const home = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-home-"));
  fs.writeFileSync(path.join(home, ".zshenv"), "export A_PERSONAL_SECRET=from-the-profile\n");
  const asked = ["-lc", 'printf "%s" "${A_PERSONAL_SECRET-}"'];
  const read = (extra: Record<string, string>): string =>
    spawnSync("/bin/zsh", asked, {
      encoding: "utf8",
      env: { HOME: home, PATH: "/usr/bin:/bin", ...extra },
    }).stdout ?? "";

  assert.equal(read({}), "from-the-profile", "the fixture profile must export something to hide");

  const kept = session.withoutKlin();
  assert.ok(kept.ZDOTDIR, "no startup directory reaches the subject");
  assert.equal(fs.readFileSync(path.join(kept.ZDOTDIR, ".zshenv"), "utf8"), "");
  assert.equal(
    read({ ZDOTDIR: kept.ZDOTDIR }),
    "",
    "a startup file of the operator's still reached the subject's shell",
  );
  fs.rmSync(home, { recursive: true, force: true });
});
