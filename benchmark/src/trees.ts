import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const IGNORED = new Set([".git", "node_modules", "target", ".klin"]);

/**
 * Every file under `root` by its relative path, sorted, less the directories a build writes.
 *
 * A symbolic link is neither listed nor copied, because the walk asks for a directory or a plain
 * file and a link is neither. No fixture ships one. An agent that creates one would leave it out
 * of the digest and out of the scoring copy, which is a fixture defect to record if it happens.
 */
export function files(root: string): string[] {
  const found: string[] = [];
  const walk = (directory: string, prefix: string): void => {
    let entries: fs.Dirent[];
    try {
      entries = fs.readdirSync(directory, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries.sort((a, b) => (a.name < b.name ? -1 : 1))) {
      if (IGNORED.has(entry.name)) {
        continue;
      }
      const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
      if (entry.isDirectory()) {
        walk(path.join(directory, entry.name), relative);
      } else if (entry.isFile()) {
        found.push(relative);
      }
    }
  };
  walk(root, "");
  return found.sort();
}

export function read(root: string, relative: string): string {
  return fs.readFileSync(path.join(root, relative), "utf8");
}

/**
 * Copy every file of `source` over `target`, and return the relative paths written.
 *
 * A file named `REMOVE` at the root of the overlay is a list of paths to delete from the target,
 * one per line, and is never copied. It is how an overlay states a move or a deletion.
 */
export function overlay(source: string, target: string): string[] {
  if (!fs.existsSync(source)) {
    return [];
  }
  const removals = path.join(source, "REMOVE");
  if (fs.existsSync(removals)) {
    for (const line of fs.readFileSync(removals, "utf8").split("\n")) {
      const relative = line.trim();
      if (relative.length > 0 && !relative.startsWith("#")) {
        fs.rmSync(path.join(target, relative), { recursive: true, force: true });
      }
    }
  }
  const written = files(source).filter((relative) => relative !== "REMOVE");
  for (const relative of written) {
    const destination = path.join(target, relative);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.copyFileSync(path.join(source, relative), destination);
  }
  return written;
}

/** A hash of the tree: every relative path and its bytes, in path order. */
export function digest(root: string): string {
  const running = createHash("sha256");
  for (const relative of files(root)) {
    running.update(relative);
    running.update("\0");
    running.update(fs.readFileSync(path.join(root, relative)));
    running.update("\0");
  }
  return running.digest("hex");
}

/** A copy of a tree the oracle may write into, without the repository's history. */
export function copyTree(source: string, target: string): void {
  fs.mkdirSync(target, { recursive: true });
  for (const relative of files(source)) {
    const destination = path.join(target, relative);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.copyFileSync(path.join(source, relative), destination);
  }
}

export function sha256(text: string | Buffer): string {
  return createHash("sha256").update(text).digest("hex");
}
