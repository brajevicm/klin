import { fileURLToPath } from "node:url";
import path from "node:path";
import os from "node:os";

const here = path.dirname(fileURLToPath(import.meta.url));

export const BENCHMARK = path.resolve(here, "..");
export const REPO = path.resolve(BENCHMARK, "..");
export const FIXTURES = path.join(BENCHMARK, "fixtures");
export const HOOK = path.join(BENCHMARK, "host", "hook");
export const WITNESS = path.join(BENCHMARK, "host", "witness");
export const RUNS = path.join(BENCHMARK, "runs");
export const SCHEMA = path.join(BENCHMARK, "record.schema.json");

/**
 * Where subject workspaces are materialized. The default sits under the system temporary
 * directory, so no relative path leads from a subject workspace to the control plane under
 * `benchmark/runs`.
 */
export function workRoot(): string {
  const named = process.env.KLIN_BENCH_WORK;
  if (named) {
    return path.resolve(named);
  }
  return path.join(os.tmpdir(), "klin-bench-work");
}
