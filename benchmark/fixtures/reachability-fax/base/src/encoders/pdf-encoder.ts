import { total, type Invoice } from "../invoice.ts";

export function encodePdf(invoice: Invoice): string {
  const text = invoice.lines.map((line, index) => `(${line.text}) Tj ${index}`);
  return ["%PDF-1.7", `% ${invoice.number}`, ...text, `(Total ${total(invoice)}) Tj`, "%%EOF"].join("\n");
}
