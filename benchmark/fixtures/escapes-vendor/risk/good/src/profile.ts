import { country } from "../vendor/countries.js";

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

export function profileFor(code: string): Profile | null {
  const held = country(code.toUpperCase());
  if (held === null) {
    return null;
  }
  return {
    code: held.iso,
    name: held.names.en,
    currency: { code: held.money.code, decimals: held.money.minor, symbol: held.money.symbol },
    dialPrefix: held.phone.prefix,
    trunkPrefix: held.phone.trunk === "" ? null : held.phone.trunk,
    inEu: held.eu,
  };
}
