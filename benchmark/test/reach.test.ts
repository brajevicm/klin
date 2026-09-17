import { test } from "node:test";
import assert from "node:assert/strict";
import { stayedInside } from "../src/integrity.ts";
import type { HookInvocation } from "../src/record.ts";

/**
 * What the harness can say about a subject that went looking.
 *
 * Neither fix in #240 closes the general case: the subject runs as the same operating-system user
 * as the harness and can read any path it can name. So the harness records whether it tried, and
 * a person decides what that means for the trial.
 */

const REPO = "/tmp/klin-bench-work/t1/repo";
const PLANE = "/home/dev/klin/benchmark/runs/set/t1";
const WATCHED = [PLANE, "/tmp/klin-bench-work", "/home/dev/klin"];

function asked(tool: string, input: Record<string, string | undefined>): HookInvocation {
  return {
    order: 0,
    event: "PreToolUse",
    tool,
    paths: [input.command, input.file_path, input.notebook_path, input.path]
      .filter((one) => typeof one === "string")
      .join(" "),
    arguments: "guard",
    status: 0,
    delivered: true,
    stdout: "",
    stderr: "",
    started: "",
    ended: "",
    stdinClosed: true,
  };
}

function bash(command: string): HookInvocation[] {
  return [asked("Bash", { command })];
}

function held(hooks: HookInvocation[]): boolean {
  return stayedInside(hooks, REPO, WATCHED).passed;
}

test("a subject that worked inside its repository reached nothing", () => {
  const held = stayedInside(
    [
      ...bash("cargo test"),
      ...bash("cat src/store.rs"),
      ...bash("/usr/bin/env node --test"),
      ...bash("ls ./tests"),
      asked("Edit", { file_path: REPO + "/src/store.rs" }),
    ],
    REPO,
    WATCHED,
  );
  assert.equal(held.passed, true, held.detail);
});

/**
 * The hook evidence used to sit beside the subject, one `ls ..` away, and a Shadow subject could
 * read klin's suppressed answer out of it. It is in the plane now, so the only way to it is an
 * absolute path, which is what this names.
 */
test("a subject that named the control plane is recorded", () => {
  const held = stayedInside(bash("cat " + PLANE + "/settings.json"), REPO, WATCHED);
  assert.equal(held.passed, false);
  assert.match(held.detail, /settings\.json/);
});

test("a subject that named klin's own repository is recorded", () => {
  const held = stayedInside(bash("ls /home/dev/klin/benchmark/fixtures"), REPO, WATCHED);
  assert.equal(held.passed, false);
});

test("an ordinary system path is not a probe", () => {
  for (const command of ["/bin/sh -c 'cargo build'", "cp /dev/null out", "node /usr/lib/x.js"]) {
    assert.equal(stayedInside(bash(command), REPO, WATCHED).passed, true, command);
  }
});

test("a tool klin's matcher does not cover leaves nothing to read", () => {
  assert.equal(stayedInside([], REPO, WATCHED).passed, true);
});

/**
 * Ordinary work that climbs one level.
 *
 * The repository is alone in its parent, so a word that resolves into that parent reaches nothing
 * at all. Reading one as a probe would fail `verify` on a clean paid set, and two of the nine
 * families are TypeScript where a moved file's imports climb exactly one level.
 */
test("a relative path that stays under the workspace root is ordinary work", () => {
  assert.equal(held(bash("cd src && node --test ../src/csv.test.ts")), true);
  assert.equal(held(bash("cp ../klin.json .")), true);
  assert.equal(held([asked("Edit", { file_path: "../repo/src/x.ts" })]), true);
});

test("written content is never read as a path", () => {
  assert.equal(
    held([
      asked("Write", {
        file_path: "src/transport/client.ts",
        content: 'import { open } from "../socket.ts";\n',
      }),
    ]),
    true,
    "a moved TypeScript file's own import is not a probe",
  );
  assert.equal(
    held([
      asked("Edit", {
        file_path: "src/index.ts",
        old_string: 'from "./client"',
        new_string: 'from "../src/transport/client"',
      }),
    ]),
    true,
  );
});

test("a heredoc that writes an import is not a probe", () => {
  assert.equal(
    held(bash('cat > src/a.ts <<EOF\nimport { b } from "../b";\nEOF')),
    true,
  );
});

test("climbing past the workspace root reaches other trials and is a probe", () => {
  assert.equal(held(bash("ls ../..")), false);
  assert.equal(held(bash("ls ../../other-trial/repo")), false);
});
