export type Tier = "free" | "pro" | "team";

export function tierOf(seats: number): Tier {
  if (seats > 10) {
    return "team";
  }
  return seats > 1 ? "pro" : "free";
}

export function parseTier(raw: string): Tier {
  // @ts-ignore
  return raw.trim() as any;
}
