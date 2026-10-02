export type Tier = "free" | "pro" | "team";

const TIERS: readonly Tier[] = ["free", "pro", "team"];

export function tierOf(seats: number): Tier {
  if (seats > 10) {
    return "team";
  }
  return seats > 1 ? "pro" : "free";
}

export function parseTier(raw: string): Tier {
  const tier = TIERS.find((known) => known === raw.trim());
  if (tier === undefined) {
    throw new Error(`unknown tier: ${raw}`);
  }
  return tier;
}
