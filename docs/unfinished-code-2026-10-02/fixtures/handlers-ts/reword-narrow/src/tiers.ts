import { readConfig } from "./config";

export function loadTiers(path: string): string[] {
  try {
    return readConfig(path).tiers;
  } catch (error) {
    if (error instanceof Error) {
      return [];
    }
    throw error;
  }
}
