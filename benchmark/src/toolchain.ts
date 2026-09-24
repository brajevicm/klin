import fs from "node:fs";
import path from "node:path";
import * as paths from "./paths.ts";
import { sha256 } from "./trees.ts";

export const TYPESCRIPT_VERSION = "5.9.3";
export const TYPESCRIPT_SHA256 = "2cffde0b8c6760dfb0b5b0382bbb7e00ba6a8b2d981b9205b256a700a481d983";
const PACKAGE = path.join(paths.BENCHMARK, "node_modules", "typescript", "package.json");
const COMPILER = path.join(paths.BENCHMARK, "node_modules", "typescript", "lib", "tsc.js");

export interface Provenance {
  package: "typescript";
  version: string;
  path: string;
  sha256: string;
}

/** Read the compiler already prepared for the benchmark; this never installs or contacts npm. */
export function current(): Provenance | null {
  try {
    const packageData = JSON.parse(fs.readFileSync(PACKAGE, "utf8")) as { version?: string };
    if (packageData.version !== TYPESCRIPT_VERSION) return null;
    const digest = sha256(fs.readFileSync(COMPILER));
    if (digest !== TYPESCRIPT_SHA256) return null;
    return {
      package: "typescript",
      version: packageData.version,
      path: COMPILER,
      sha256: digest,
    };
  } catch {
    return null;
  }
}

/**
 * Lend the prepared compiler to a tree whose lockfile pins the same one, as its own `node_modules`.
 *
 * A subject installs a locked compiler with `npm ci`, and a copy the self-test runs has no
 * registry. A tree that locks any other version, or holds a `node_modules` already, is left alone.
 */
export function lend(tree: string): void {
  const own = path.join(tree, "node_modules");
  const lock = path.join(tree, "package-lock.json");
  if (!current() || fs.existsSync(own) || !fs.existsSync(lock)) return;
  const locked = JSON.parse(fs.readFileSync(lock, "utf8")) as {
    packages?: Record<string, { version?: string }>;
  };
  if (locked.packages?.["node_modules/typescript"]?.version !== TYPESCRIPT_VERSION) return;
  fs.symlinkSync(path.join(paths.BENCHMARK, "node_modules"), own);
}

export function requirement(): string {
  return current()
    ? ""
    : "the pinned TypeScript compiler is not prepared at " +
        COMPILER +
        ". Run benchmark/prepare before selftest, probe or plan";
}

export function frozen(): Provenance {
  const held = current();
  if (held) return held;
  throw new Error(requirement());
}

export function describe(): string {
  const held = current();
  return held
    ? "TypeScript " + held.version + " at " + held.path + " (SHA-256 " + held.sha256 + ")"
    : requirement();
}
