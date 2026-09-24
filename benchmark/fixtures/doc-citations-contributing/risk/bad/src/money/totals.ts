import { discounted } from "./discount.ts";
import { taxOn, type Band } from "./tax.ts";

export interface Line {
  cents: number;
  quantity: number;
  band: Band;
  discountPercent?: number;
}

export interface Totals {
  net: number;
  tax: number;
  gross: number;
}

export function totals(lines: Line[]): Totals {
  let net = 0;
  let tax = 0;
  for (const line of lines) {
    const amount = discounted(line.cents * line.quantity, line.discountPercent);
    net += amount;
    tax += taxOn(amount, line.band);
  }
  return { net, tax, gross: net + tax };
}
