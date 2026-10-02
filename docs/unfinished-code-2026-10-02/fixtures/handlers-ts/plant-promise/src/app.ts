import { loadTiers } from "./tiers";

export async function start(path: string): Promise<string[]> {
  return loadTiers(path);
}
