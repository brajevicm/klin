import type { Invoice } from "../invoice.ts";

export function encodeSubject(invoice: Invoice): string {
  return `Invoice ${invoice.number}, due ${invoice.due}`;
}
