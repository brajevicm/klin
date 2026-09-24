import { total, type Invoice } from "../invoice.ts";

function escaped(text: string): string {
  return text.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}

export function encodeHtml(invoice: Invoice): string {
  const rows = invoice.lines.map((line) => `<li>${escaped(line.text)}</li>`).join("");
  return `<h1>Invoice ${escaped(invoice.number)}</h1><ul>${rows}</ul><p>Total ${total(invoice)}</p>`;
}
