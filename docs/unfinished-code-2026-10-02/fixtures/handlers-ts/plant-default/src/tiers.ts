import { readConfig } from "./config";

export function loadTiers(path: string): string[] {
  try {
    return readConfig(path).tiers;
  } catch {
    return [];
  }
}
