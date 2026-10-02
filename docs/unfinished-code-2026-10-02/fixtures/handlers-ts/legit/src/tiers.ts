import { readConfig } from "./config";

export const DEFAULT_TIERS = ["free"];

function isMissing(error: unknown): boolean {
  return (error as { code?: string }).code === "ENOENT";
}

export function loadTiers(path: string): string[] {
  try {
    return readConfig(path).tiers;
  } catch (error) {
    if (isMissing(error)) {
      return DEFAULT_TIERS;
    }
    throw error;
  }
}
