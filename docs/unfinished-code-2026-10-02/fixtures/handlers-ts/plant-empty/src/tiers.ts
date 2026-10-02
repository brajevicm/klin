import { readConfig } from "./config";

export function loadTiers(path: string): string[] {
  let tiers: string[] = [];
  try {
    tiers = readConfig(path).tiers;
  } catch {}
  return tiers;
}
