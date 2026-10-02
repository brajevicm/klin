import { readFile } from "node:fs/promises";

export async function loadTiers(path: string): Promise<string[]> {
  return readFile(path, "utf8")
    .then((text) => (JSON.parse(text) as { tiers: string[] }).tiers)
    .catch(() => []);
}
