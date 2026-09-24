import { format, type Currency } from "./currency.ts";
import { totals, type Line } from "./totals.ts";

export function render(number: string, lines: Line[], currency: Currency): string {
  const sum = totals(lines);
  return [
    `Invoice ${number}`,
    `Net ${format(sum.net, currency)}`,
    `Tax ${format(sum.tax, currency)}`,
    `Total ${format(sum.gross, currency)}`,
  ].join("\n");
}
