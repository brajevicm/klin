import { unlinkSync } from "node:fs";
import { readConfig } from "./config";

export function loadTiers(path: string): string[] {
  const tiers = readConfig(path).tiers;
  try {
    unlinkSync(`${path}.lock`);
  } catch {
    // The lock file may already be gone; the read above does not need it.
  }
  return tiers;
}
