import { readConfig } from "./config";

export function findTiers(path: string): string[] | undefined {
  try {
    return readConfig(path).tiers;
  } catch {
    return undefined;
  }
}

export function loadTiers(path: string): string[] {
  return findTiers(path) ?? ["free"];
}
