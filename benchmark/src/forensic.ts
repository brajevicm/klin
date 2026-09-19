import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";

/**
 * Copying and hashing for evidence, which is not the same job as copying a source tree.
 *
 * `trees.ts` skips `.git`, `node_modules`, `target` and `.klin`, and steps over a symbolic link,
 * because it answers what a fixture or a subject tree is. Evidence has to answer what was on
 * disk. So nothing here is skipped, and a symbolic link or a device node is refused rather than
 * quietly left out: a copy that dropped a file would still hash, and the hash would attest a
 * tree that never existed.
 */

/** Every regular file under `root`, by path relative to it, in a stable order. */
export function files(root: string): string[] {
  const held: string[] = [];
  const visit = (directory: string): void => {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      const at = path.join(directory, entry.name);
      const shown = path.relative(root, at).split(path.sep).join("/");
      if (entry.isSymbolicLink()) {
        throw new Error("evidence cannot hold the symbolic link " + shown);
      }
      if (entry.isDirectory()) {
        visit(at);
      } else if (entry.isFile()) {
        held.push(shown);
      } else {
        throw new Error("evidence cannot hold the non-file " + shown);
      }
    }
  };
  visit(root);
  return held.sort();
}

/** One digest over every file's path and bytes, so a renamed or edited file moves it. */
export function digest(root: string): string {
  const whole = createHash("sha256");
  for (const name of files(root)) {
    const bytes = fs.readFileSync(path.join(root, name));
    whole.update(name);
    whole.update("\0");
    whole.update(createHash("sha256").update(bytes).digest("hex"));
    whole.update("\n");
  }
  return whole.digest("hex");
}

/** Copy every file under `from` into `into`, with the same refusals. */
export function copy(from: string, into: string): void {
  fs.rmSync(into, { recursive: true, force: true });
  for (const name of files(from)) {
    const target = path.join(into, name);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.copyFileSync(path.join(from, name), target);
  }
}
