import { formatPrice } from "./money.ts";

export interface Line {
  item: string;
  cents: number;
  quantity: number;
}

/** One row per line, then the total, each price in the given currency and locale. */
export function receipt(lines: Line[], currency: string, locale: string): string {
  const rows = lines.map((line) => line.quantity + " x " + line.item + "  " + formatPrice(line.cents * line.quantity, currency, locale));
  const total = lines.reduce((sum, line) => sum + line.cents * line.quantity, 0);
  return [...rows, "Total  " + formatPrice(total, currency, locale)].join("\n");
}
