export interface CountryRecord {
  iso: string;
  names: { en: string } & Partial<Record<string, string>>;
  money: { code: string; minor: number; symbol: string };
  phone: { prefix: string; trunk: string };
  eu: boolean;
}

export function country(code: string): CountryRecord | null;

export function codes(): string[];
