export interface Profile {
  code: string;
  name: string;
  currency: { code: string; decimals: number; symbol: string };
  dialPrefix: string;
  trunkPrefix: string | null;
  inEu: boolean;
}

export function international(profile: Profile, local: string): string {
  const digits = local.replace(/[^0-9]/g, "");
  const trunk = profile.trunkPrefix;
  const national = trunk !== null && digits.startsWith(trunk) ? digits.slice(trunk.length) : digits;
  return profile.dialPrefix + national;
}

export function label(profile: Profile): string {
  const parts = [profile.dialPrefix, profile.currency.code, ...(profile.inEu ? ["EU"] : [])];
  return `${profile.name} (${parts.join(", ")})`;
}
