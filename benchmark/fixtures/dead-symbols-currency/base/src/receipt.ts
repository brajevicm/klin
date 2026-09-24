import { formatPrice } from "./money.ts";

export interface Line {
  item: string;
  cents: number;
  quantity: number;
}

/** One row per line, then the total, each price in the given currency. */
export function receipt(lines: Line[], currency: string): string {
  const rows = lines.map((line) => line.quantity + " x " + line.item + "  " + formatPrice(line.cents * line.quantity, currency));
  const total = lines.reduce((sum, line) => sum + line.cents * line.quantity, 0);
  return [...rows, "Total  " + formatPrice(total, currency)].join("\n");
}
