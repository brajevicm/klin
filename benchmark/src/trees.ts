import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const IGNORED = new Set([".git", "node_modules", "target", ".klin"]);

/**
 * Every entry under `root` that `keep` accepts, by relative path, sorted, less the directories a
 * build writes.
 *
 * `readdirSync` does not follow a symbolic link, so a link is a directory to neither `files` nor
 * this walk, and the walk never descends through one.
 */
function walk(
  root: string,
  keep: (entry: fs.Dirent) => boolean,
  skip: ReadonlySet<string>,
): string[] {
  const found: string[] = [];
  const step = (directory: string, prefix: string): void => {
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
      if (skip.has(relative)) {
        continue;
      }
      if (keep(entry)) {
        found.push(relative);
      }
      if (entry.isDirectory()) {
        step(path.join(directory, entry.name), relative);
      }
    }
  };
  step(root, "");
  return found.sort();
}

const NOTHING: ReadonlySet<string> = new Set();

/**
 * Every plain file under `root` by its relative path, sorted.
 *
 * A symbolic link is neither listed nor copied, so a tree holding one is measured incompletely.
 * `links` is what finds that, and a trial whose final tree holds one is invalid.
 */
export function files(root: string, skip: ReadonlySet<string> = NOTHING): string[] {
  return walk(root, (entry) => entry.isFile(), skip);
}

/** Every symbolic link under `root`, which `files`, `digest` and `copyTree` all leave out. */
export function links(root: string): string[] {
  return walk(root, (entry) => entry.isSymbolicLink(), NOTHING);
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

/**
 * A hash of the tree: every relative path and its bytes, in path order.
 *
 * `skip` names relative paths the walk does not enter, so a caller can digest part of a tree. A
 * digest taken with a `skip` naming nothing the tree holds is the digest of the whole tree, which
 * is what lets a directory be added beside a frozen identity without moving it.
 */
export function digest(root: string, skip: ReadonlySet<string> = NOTHING): string {
  const running = createHash("sha256");
  for (const relative of files(root, skip)) {
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
