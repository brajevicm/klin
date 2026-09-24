import { halfEven } from "./rounding.ts";

export const RATES = { standard: 0.2, reduced: 0.05, zero: 0 } as const;

export type Band = keyof typeof RATES;

export function taxOn(cents: number, band: Band): number {
  return halfEven(cents * RATES[band]);
}
