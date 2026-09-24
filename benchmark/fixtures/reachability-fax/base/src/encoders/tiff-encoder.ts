import { total, type Invoice } from "../invoice.ts";

const WIDTH = 72;

export function encodeTiff(invoice: Invoice): string {
  const rows = [`INVOICE ${invoice.number}`, ...invoice.lines.map((line) => line.text), `TOTAL ${total(invoice)}`];
  return ["II*", ...rows.map((row) => row.toUpperCase().slice(0, WIDTH).padEnd(WIDTH, "."))].join("\n");
}
